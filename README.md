# Cage: Develop and deploy complex Docker applications

[![Latest version](https://img.shields.io/crates/v/cage.svg)](https://crates.io/crates/cage) [![License](https://img.shields.io/crates/l/cage.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/faradayio/cage.svg?branch=master)](https://travis-ci.org/faradayio/cage) [![Documentation](https://img.shields.io/badge/documentation-docs.rs-yellow.svg)](https://docs.rs/cage/)

Does your project have too many Docker services? Too many git repos? Cage
makes it easy to develop complex, multi-service applications locally.  It
works with standard `docker-compose.yml` files and `docker-compose`, but
it helps bring order to the complexity:

- Cage provides a standardized project structure, much like Rails did for
  web development.
- Cage allows you to work with multiple source repositories, and to mix
  pre-built Docker images with local source code.
- Cage removes the repetitive clutter from your `docker-compose.yml` files.
- Cage provides secret management, either using a single text file
  or [Hashicorp's Vault][vault].

[vault]: https://www.vaultproject.io/

## Installation

First, you need to [install Docker][] and make sure that you
have [at least version 1.8.1][compose] of `docker-compose`:

```sh
$ docker-compose --version
docker-compose version 1.8.1, build 878cff1
```

We provide [pre-built `cage` binaries for Linux and MacOS][releases] on the
release page.  The Linux binaries
are [statically linked][rust-musl-builder] and should work on any modern
Linux distribution.  To install, you can just unzip the binaries and copy
them to `/usr/local/bin`:

```sh
unzip cage-*.zip
sudo cp cage /usr/local/bin/
rm cage-*.zip cage
```

If you would like to install from source, we recommend using `rustup` and
`cargo install`:

```sh
curl https://sh.rustup.rs -sSf | sh
cargo install cage
```

If you have [trouble][] using cage's vault integration, try installing with
`cargo` instead.

[install Docker]: https://docs.docker.com/engine/installation/
[compose]: https://github.com/docker/compose/releases
[releases]: https://github.com/faradayio/cage/releases
[rust-musl-builder]: https://github.com/emk/rust-musl-builder
[trouble]: https://github.com/faradayio/cage/issues/11

## Trying it out

Create a new application using `cage`, and list the associated Git
repositories:

```sh
$ cage new myapp
$ cd myapp
$ cage repo list
rails_hello               https://github.com/faradayio/rails_hello.git
```

Pull the pre-built Docker images associated with this application:

```sh
$ cage pull
```

Trying checking out the source code for an image locally:

```sh
$ cage repo clone rails_hello
$ cage repo list
rails_hello               https://github.com/faradayio/rails_hello.git
  Cloned at src/rails_hello
```

Start up your application:

```sh
$ cage up
Starting myapp_db_1
Starting myapp_web_1
```

You'll notice that the `src/rails_hello` directory is mounted at
`/usr/src/app` inside the `myapp_web_1` pod, so that you can make changes
locally and test them.

Run a command inside the `frontend` pod's `web` container to create a
database:

```sh
$ cage exec frontend/web rake db:create
Created database 'myapp_development'
Created database 'db/test.sqlite3'
```

We could also just specify the service name `web` instead of the full
`frontend/web`, as long as `web` is unique across all pods.

We can also package up frequently-used commands in their own, standalone
"task" pods, and run them on demand:

```sh
$ cage run migrate
Creating myapp_migrate_1
Attaching to myapp_migrate_1
myapp_migrate_1 exited with code 0
```

You should be able to access your application at http://localhost:3000/.

You may also notice that since `myapp_migrate_1` is based on the same
underlying Git repository as `myapp_web_1`, that it also has a mount of
`src/rails_hello` in the appropriate location.  If you change the source on
your host system, it will automatically show up in both containers.

We can run container-specific unit tests, which are specified by the
container, so that you can invoke any unit test framework of your choice:

```sh
$ cage test web
```

And we can access individual containers using a configurable shell:

```sh
$ cage shell web
root@21bbbb41ad4a:/usr/src/app#
```

The top-level convenience commands like `test` and `shell` make it much
easier to perform standard development tasks without knowing how individual
containers work.

For more information, check out `cage`'s help:

```sh
cage --help
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

## Reporting issues

If you encounter an issue, it might help to set the following shell
variables and re-run the command:

```sh
export RUST_BACKTRACE=1 RUST_LOG=cage=debug,compose_yml=debug
```

## Development notes

Pull requests are welcome!  If you're unsure about your idea, then please
feel free to file an issue and ask us for feedback.  We like suggestions!

### Setting up tools

When working on this code, we recommend installing the following support
tools:

```sh
cargo install rustfmt
cargo install cargo-watch
```

We also recommend installing nightly Rust, which produces better error
messages and supports extra warnings using [Clippy][]:

```sh
rustup update nightly
rustup override set nightly
```

If `nightly` produces build errors, you may need to update your compiler
and libraries to the latest versions:

```sh
rustup update nightly
cargo update
```

If that still doesn't work, try using `stable` Rust instead:

```sh
rustup override set stable
```

If you're using `nightly`, run the following in a terminal as you edit:

```sh
cargo watch "test --no-default-features --features unstable --color=always" \
    "build --no-default-features --features unstable --color=always"
```

If you're using `stable`, leave out `--no-default-features --features
unstable`:

```sh
cargo watch "test --color=always" "build --color=always"
```

Before committing your code, run:

```sh
cargo fmt
```

This will automatically reformat your code according to the project's
conventions.  We use Travis CI to verify that `cargo fmt` has been run and
that the project builds with no warnings.  If it fails, no worries—just go
ahead and fix your pull request, or ask us for help.

[Clippy]: https://github.com/Manishearth/rust-clippy

### Official releases

To make an official release, you need to be a maintainer, and you need to
have `cargo publish` permissions.  If this is the case, first edit
`Cargo.toml` to bump the version number, then regenerate `Cargo.lock`
using:

```sh
cargo build
```

Commit the release, using a commit message of the format:

```txt
v<VERSION>: <SUMMARY>

<RELEASE NOTES>
```

Then run:

```
cargo publish
git tag v$VERSION
git push; git push --tags
```

This will rebuild the official binaries using Travis CI, and upload a new version of
the crate to [crates.io](https://crates.io/crates/cage).
