# matey 🏴‍☠️

<a href='http://www.recurse.com' title='Made with love at the Recurse Center'><img src='https://cloud.githubusercontent.com/assets/2883345/11325206/336ea5f4-9150-11e5-9e90-d86ad31993d8.png' height='20px'/></a>

![](https://media.tenor.com/images/6f506e607e7d12273c5a21b8eafa3ed4/tenor.gif)

A BitTorrent client written in Rust.

## Usage

`matey` currently only supports a limited subset of
real-world torrent files, the primary type defined in [BEP
0003](https://www.bittorrent.org/beps/bep_0003.html). This means no support for
multiple announcers or the DHT protocol. In practice, unfortunately, this means
that most real-world torrents don't work with it, but a good deal do, such as
those distributed by mainstream Linux distributions, for instance.

`cargo run <filename>`

![](https://i.imgur.com/WlyutF1.gif)

## Goals

- User-friendliness
- Speed
- Correctness

## Non-goals

- Seeding
- Creating torrents
- Being especially robust

## Contribution

When contributing to this repository, please first discuss the change(s) you wish to make by filing an issue with the owners of this repository before making a change.
