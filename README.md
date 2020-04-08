# Cage: Develop and deploy complex Docker applications

[![Latest version](https://img.shields.io/crates/v/cage.svg)](https://crates.io/crates/cage) [![License](https://img.shields.io/crates/l/cage.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/faradayio/cage.svg?branch=master)](https://travis-ci.org/faradayio/cage) [![Build status](https://ci.appveyor.com/api/projects/status/3hn8cwckcdhpcasm/branch/master?svg=true)](https://ci.appveyor.com/project/emk/cage/branch/master) [![Documentation](https://img.shields.io/badge/documentation-docs.rs-yellow.svg)](https://docs.rs/cage/) [![Gitter](https://badges.gitter.im/faradayio/cage.svg)](https://gitter.im/faradayio/cage?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

<a href="http://cage.faraday.io/">
  <img src="http://cage.faraday.io/assets/cage-logo.svg" alt="Logo"
       width="200" height="113">
</a>

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

For more information about Cage, see
the [introductory website](http://cage.faraday.io/).

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

Note that it's [possible to build `cage` for Windows](./WINDOWS.md), but
it's still not yet officially supported.

[install Docker]: https://docs.docker.com/engine/installation/
[compose]: https://github.com/docker/compose/releases
[releases]: https://github.com/faradayio/cage/releases
[rust-musl-builder]: https://github.com/emk/rust-musl-builder
[trouble]: https://github.com/faradayio/cage/issues/11

## Trying it out

Create a new application using `cage`:

```sh
cage new myapp
cd myapp
```

Pull the pre-built Docker images associated with this application and start
up the database pod:

```sh
cage pull
cage up db
```

Run the `rake` image to initialize your database:

```sh
cage run rake db:create
cage run rake db:migrate
```

And bring up the rest of the app:

```sh
cage up
```

Let's take a look at the pods and services defined by this application:

```sh
$ cage status
db enabled type:placeholder
└─ db
frontend enabled type:service
└─ web ports:3000
rake enabled type:task
└─ rake
```

This shows us that the `web` service is listening on port 3000, so you
should be able to access the application
at [http://localhost:3000](http://localhost:3000).  But let's make a
change!  First, list the available source code for the services in this
app:

```sh
$ cage source ls
rails_hello               https://github.com/faradayio/rails_hello.git
```

Try mounting the source code for `rails_hello` into all the containers that
use it:

```sh
$ cage source mount rails_hello
$ cage up
$ cage source ls
rails_hello               https://github.com/faradayio/rails_hello.git
  Cloned at src/rails_hello (mounted)
```

You may also notice that since `myapp_rake_1` is based on the same
underlying Git repository as `myapp_web_1`, that it also has a mount of
`src/rails_hello` in the appropriate location.  If you change the source on
your host system, it will automatically show up in both containers.

Now, create an HTML file at `src/rails_hello/public/index.html`:

```html
<html>
  <head><title>Sample page</title></head>
  <body><h1>Sample page</h1></body>
</html>
```

And reload the website in your browser.  You should see the new page!

We can also run container-specific unit tests, which are specified by the
container, so that you can invoke any unit test framework of your choice:

```sh
cage test web
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

```txt
hello
└── pods
    ├── common.env
    ├── frontend.yml
    └── targets
        ├── development
        │   └── common.env
        ├── production
        │   ├── common.env
        │   └── frontend.yml
        └── test
            └── common.env
```

### File types

<table>
  <tr>
    <th>Filename</th>
    <th>Description</th>
    <th>Found in</th>
  </tr>
  <tr>
    <td><code>common.env</code>
    <td>Sets environment variables at different levels. <code>common</code> is a reserved word.</td>
    <td>Top-level <code>pods/</code> dir and also in target dirs (<code>production</code>, etc.)</td>
  </tr>
  <tr>
  <td><code>$SERVICE.yml</code></td>
  <td>Valid <a href="https://docs.docker.com/compose/compose-file/"><code>docker-compose.yml</code> version 2</a> defining images, etc.</td>
  <td>Top-level <code>pods/</code> dir and also in target dirs (<code>production</code>, etc.)</td>
  </tr>
  <tr>
  <td><code>$SERVICE.metadata.yml</code></td>
  <td>Pod-level metadata that isn't valid in <a href="https://docs.docker.com/compose/compose-file/"><code>docker-compose.yml</code> version 2</a>.</td>
  <td>Top-level <code>pods/</code> dir and also in target dirs (<code>production</code>, etc.)</td>
  </tr>
</table>

## Other commands

### cage run-script

The `run-script` command operates similarly to `npm run <script>` or
`rake <task>`. Simply define a set of named scripts in the pod's metadata:

```yml
# tasks.yml
services:
  runner:
    build: .
```

```yml
# tasks.metadata.yml

services:
  runner:
    scripts:
      populate:
        - ["npm","run","populate"]
```

By running `cage run-script populate`, cage will find all services
that have a `populate` script and run it. You can also specify a
pod or service with `cage run-script tasks populate`.

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
rustup component add rustfmt
rustup component add clippy
cargo install cargo-watch
```

Run the following in a terminal as you edit:

```sh
cargo watch -x test
```

Before committing your code, run:

```sh
cargo clippy
cargo fmt
```

This will automatically check for warnings, and reformat your code according to the project's
conventions.  We use Travis CI to verify that `cargo fmt` has been run and
that the project builds with no warnings.  If it fails, no worries—just go
ahead and fix your pull request, or ask us for help.

### On MacOS

The openssl crate needs a compatible version of the `openssl` libraries. You may be able to install them as follows:

```sh
brew install openssl
```

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

```sh
cargo publish
git tag v$VERSION
git push; git push --tags
```

This will rebuild the official binaries using Travis CI, and upload a new version of
the crate to [crates.io](https://crates.io/crates/cage).
