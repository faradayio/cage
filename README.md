# Faraday Conductor: Orchestrates `docker-compose` for large, multi-pod apps

This is a work in progress using the
[`docker_compose`](https://github.com/emk/docker_compose-rs) library.  It's
a reimplementation of our internal, _ad hoc_ tools using the new
`docker-compose.yml` version 2 format and Rust.

## Installation

To install, we recommend using `rustup` and `cargo`

```sh
curl https://sh.rustup.rs -sSf | sh
cargo install conductor
```

## Usage

To see how to use `conductor`, run `conductor --help` (which may be newer
than this README during development):

```
conductor: Manage large, multi-pod docker-compose apps

Usage:
  conductor
  conductor (--help | --version)

Options:
    -h, --help         Show this message
    --version          Show the version of conductor

Run conductor in a directory containing a `pods` subdirectory.  For more
information, see https://github.com/faradayio/conductor.
```

## Project format

See `examples/hello` for a complete example.

```
hello
└── pods
    ├── common.env
    ├── frontend.yml
    └── overrides
        ├── development
        │   └── common.env
        ├── production
        │   ├── common.env
        │   └── frontend.yml
        └── test
            └── common.env
```
