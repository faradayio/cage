# Faraday Conductor: Orchestrates `docker-compose` for large, multi-pod apps

[![Latest version](https://img.shields.io/crates/v/conductor.svg)](https://crates.io/crates/conductor) [![License](https://img.shields.io/crates/l/conductor.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/faradayio/conductor.svg?branch=master)](https://travis-ci.org/faradayio/conductor)

This is a work in progress using the
[`docker_compose`](https://github.com/emk/docker_compose-rs) library.  It's
a reimplementation of our internal, _ad hoc_ tools using the new
`docker-compose.yml` version 2 format and Rust.

[API Documentation](https://faradayio.github.io/conductor/)

## Installation

To install, we recommend using `rustup` and `cargo`:

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
  conductor [options]
  conductor [options] new <name>
  conductor [options] pull
  conductor [options] up
  conductor [options] stop
  conductor [options] exec [exec options] <pod> <service> <command> [--] [<args>..]
  conductor [options] shell [exec options] <pod> <service>
  conductor [options] test <pod> <service>
  conductor [options] repo list
  conductor [options] repo clone <repo>
  conductor (--help | --version)

Commands:
  pull              Pull Docker images used by project
  up                Run project
  stop              Stop all containers associated with project
  exec              Run a command inside a container
  shell             Run an interactive shell inside a running container
  test              Run the tests associated with a service, if any
  repo list         List all git repository aliases and URLs
  repo clone        Clone a git repository using its short alias and mount it
                    into the containers that use it

Arguments:
  <repo>            Short alias for a repo (see `repo list`)
  <pod>             The name of a pod specified in `pods/`
  <service>         The name of a service in a pod

Exec options:
  -d                Run command detached in background
  --privileged      Run a command with elevated privileges
  --user <user>     User as which to run a command
  -T                Do not allocate a TTY when running a command

General options:
  -h, --help        Show this message
  --version         Show the version of conductor
  --override=<override>
                    Use overrides from the specified subdirectory of
                    `pods/overrides` [default: development]
  --default-tags=<tag_file>
                    A list of tagged image names, one per line, to
                    be used as defaults for images

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
