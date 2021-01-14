use super::bencode_parser::{self, parse_bencode, Bencode};
use nom::{
    bytes::complete::{tag, take_until},
    combinator::recognize,
    error::ErrorKind,
};
use sha1::{Digest, Sha1};
use snafu::{ensure, OptionExt, ResultExt, Snafu};
use std::{
    convert::{TryFrom, TryInto},
    fmt, num,
    path::PathBuf,
    string,
};

#[derive(Clone, Debug)]
pub struct Torrent {
    pub announce: String,
    pub info: TorrentInfo,
    pub info_hash: SHA1Hash,
}

impl TryFrom<Vec<u8>> for Torrent {
    type Error = TorrentParsingError;

    fn try_from(torrent_bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let (_, torrent_bencode) =
            parse_bencode(&torrent_bytes).map_err(|_| TorrentParsingError::InvalidBencode)?;

        let mut torrent_dict = torrent_bencode
            .dict()
            .ok_or(TorrentParsingError::NotADict)?;

        let announce = String::from_utf8(
            torrent_dict
                .remove(b"announce" as &[u8])
                .and_then(|val| val.byte_string())
                .context(FieldNotFound { field: "announce" })?,
        )
        .context(InvalidString)?;

        let info = TorrentInfo::try_from(
            torrent_dict
                .remove(b"info" as &[u8])
                .context(FieldNotFound { field: "info" })?,
        )?;

        let (bytes_after_info_token, _) =
            // Rust cannot infer an error type by default, so we use nom's
            // usual (Input, ErrorKind) type. See the nom docs for details.
            take_until::<_, _, (_, ErrorKind)>("info")(torrent_bytes.as_slice())
                // take_until doesn't consume the pattern itself,
                // so we have to get rid of that part. It's guaranteed
                // to be there, so we can just unwrap this.
                .map(|(bytes, _)| tag::<_, _, (_, ErrorKind)>("info")(bytes).unwrap())
                .map_err(|_| TorrentParsingError::InvalidBencode)?;

        let (_, info_bytes) = recognize(bencode_parser::dict)(bytes_after_info_token).unwrap();

        let info_hash = SHA1Hash(Sha1::digest(info_bytes).as_slice().try_into().unwrap());

        Ok(Self {
            announce,
            info,
            info_hash,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TorrentInfo {
    pub name: String,
    pub files: Vec<TorrentFile>,
    pub piece_len: u64,
    pub pieces: Vec<SHA1Hash>,
}

// TODO: single file mode
impl TryFrom<Bencode> for TorrentInfo {
    type Error = TorrentParsingError;

    fn try_from(info_bencode: Bencode) -> Result<Self, Self::Error> {
        let mut torrent_info_dict = info_bencode.dict().ok_or(TorrentParsingError::NotADict)?;

        let name = String::from_utf8(
            torrent_info_dict
                .remove(b"name" as &[u8])
                .and_then(|val| val.byte_string())
                .context(FieldNotFound {
                    field: "info[name]",
                })?,
        )
        .context(InvalidString)?;

        let files = torrent_info_dict
            .remove(b"files" as &[u8])
            .and_then(|val| val.list())
            .context(FieldNotFound {
                field: "info[files]",
            })?
            .into_iter()
            .map(TorrentFile::try_from)
            .collect::<Result<_, _>>()?;

        let piece_len = u64::try_from(
            torrent_info_dict
                .remove(b"piece length" as &[u8])
                .and_then(|val| val.number())
                .context(FieldNotFound {
                    field: "info[piece length]",
                })?,
        )
        .context(InvalidPieceLen)?;

        let all_pieces = torrent_info_dict
            .remove(b"pieces" as &[u8])
            .and_then(|val| val.byte_string())
            .context(FieldNotFound {
                field: "info[pieces]",
            })?;

        let (pieces, remainder) = all_pieces.as_chunks();

        ensure!(remainder.is_empty(), MismatchedPieceLength);

        let pieces = pieces
            .iter()
            .map(|&hash_bytes| SHA1Hash(hash_bytes))
            .collect();

        Ok(Self {
            name,
            files,
            piece_len,
            pieces,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TorrentFile {
    pub length: u64,
    pub path: PathBuf,
}

impl TryFrom<Bencode> for TorrentFile {
    type Error = TorrentParsingError;

    fn try_from(file_bencode: Bencode) -> Result<Self, Self::Error> {
        let mut file_dict = file_bencode.dict().ok_or(TorrentParsingError::NotADict)?;

        let length = u64::try_from(
            file_dict
                .remove(b"length" as &[u8])
                .and_then(|val| val.number())
                .context(FieldNotFound {
                    field: "file[length]",
                })?,
        )
        .context(InvalidFileLen)?;

        let path = file_dict
            .remove(b"path" as &[u8])
            .and_then(|val| val.list())
            .context(FieldNotFound {
                field: "file[path]",
            })?
            .into_iter()
            .map(|val| {
                String::from_utf8(val.byte_string().context(InvalidPath)?).context(InvalidString)
            })
            .collect::<Result<_, _>>()?;

        Ok(Self { length, path })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SHA1Hash([u8; 20]);

impl fmt::Debug for SHA1Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Snafu)]
pub enum TorrentParsingError {
    #[snafu(display("Expected a dictionary, but didn't find it"))]
    NotADict,
    #[snafu(display("Attempted to decode an invalid string"))]
    InvalidString { source: string::FromUtf8Error },
    #[snafu(display("Couldn't find field {}", field))]
    FieldNotFound { field: String },
    #[snafu(display("Invalid piece length"))]
    InvalidPieceLen { source: num::TryFromIntError },
    #[snafu(display("Invalid file length"))]
    InvalidFileLen { source: num::TryFromIntError },
    #[snafu(display("Invalid file path: not a list of strings"))]
    InvalidPath,
    #[snafu(display("Found a piece with length < 20"))]
    MismatchedPieceLength,
    #[snafu(display("Provided bytes aren't valid bencode"))]
    InvalidBencode,
}
