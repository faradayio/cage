# Faraday Conductor: Orchestrates `docker-compose` for large, multi-pod apps

[![Latest version](https://img.shields.io/crates/v/conductor.svg)](https://crates.io/crates/conductor) [![License](https://img.shields.io/crates/l/conductor.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/faradayio/conductor.svg?branch=master)](https://travis-ci.org/faradayio/conductor)

This is a work in progress using the
[`docker_compose`](https://github.com/emk/docker_compose-rs) library.  It's
a reimplementation of our internal, _ad hoc_ tools using the new
`docker-compose.yml` version 2 format and Rust.

[API Documentation](https://faradayio.github.io/conductor/)

## What's this for?

- Does your app include more than one `docker-compose.yml` file?
- Does your app contain a mixture of permanently running containers and
  one-shot tasks?
- Does your app run across more than one cluster of machines?
- Do individual components of your app need their own load balancers?
- When running in development mode, do you need to replace 3rd-party
  services with local containers?

If you answer to one or more of these questions is "yes", then `conductor`
is probably for you.  It provides development and deployment tools for
complex `docker-compose` apps, following
a [convention over configuration][coc] philosophy.

[coc]: https://en.wikipedia.org/wiki/Convention_over_configuration

## Installation

To install, we recommend using `rustup` and `cargo`:

```sh
curl https://sh.rustup.rs -sSf | sh
cargo install conductor
```

We also provide [official binary releases][releases] for Mac OS X and for
Linux.  The Linux binaries are statically linked using [musl-libc][]
and [rust-musl-builder][], so they should work on any Linux distribution,
including both regular distributions and stripped down distributions like
Alpine.  Just unzip the binaries and copy them to where you want them.

[releases]: https://github.com/faradayio/conductor/releases
[musl-libc]: https://www.musl-libc.org/
[rust-musl-builder]: https://github.com/emk/rust-musl-builder

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
  new               Create a directory containing a new sample project
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
  <name>            The name of the project directory to create
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

## What's a pod?

A "pod" is a tightly-linked group of containers that are always deployed
together.  Kubernetes [defines pods][pods] as:

> A pod (as in a pod of whales or pea pod) is a group of one or more
> containers (such as Docker containers), the shared storage for those
> containers, and options about how to run the containers. Pods are always
> co-located and co-scheduled, and run in a shared context. A pod models an
> application-specific “logical host” - it contains one or more application
> containers which are relatively tightly coupled — in a pre-container
> world, they would have executed on the same physical or virtual machine.

If you're using Amazon's ECS, a pod corresponds to an ECS "task" or
"service".  If you're using Docker Swarm, a pod corresponds to a single
`docker-compose.xml` file full of services that you always launch as a
single unit.

Pods typically talk to other pods using ordinary DNS lookups or service
discovery.  If a pod accepts outside network connections, it will often do
so via a load balancer.

[pods]: http://kubernetes.io/docs/user-guide/pods/

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
