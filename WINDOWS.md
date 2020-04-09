# Cage on Windows

Generally speaking, `docker-compose` and Docker Engine are a work in
progress on Windows. We have highly experimental Windows support in `cage`
for people who want to try it out and let us know about any issues.

**But be warned!** Not all the test suites pass yet.  For an overview of
the current unit test status, click on the AppVeyor build badge below:

[![AppVeyor](https://img.shields.io/appveyor/ci/emk/cage.svg)](https://ci.appveyor.com/project/emk/cage)

This document should give you all the information you need on available
builds and overall guidance when working with `cage` on Windows.

## Installation from crates.io

First make sure you install rust for
`x86_64-pc-windows-gnu` [using the `rustup-init` installer][rustup].  You
should use the stable `stable` toolchain for your initial install.

Next up you'll need to get a build env going with Msys2. Grab a MinGW
installer from [Mingw-w64's sf.net page][mingw-w64 installer].

Launch the install and follow the prompts, making sure you set the arch to
`x86_64` so that it matches Rust. Thread types should be fine as Posix.

After the install succeeds (if you get download failures just keep trying),
open up the new batch script that was installed. You should be able to find
it either in your start menu or under `C:\Program Files\mingw-w64`.

From the prompt you should be able to use the `rustup` and `cargo`
commands.  Now you should be able to install cage using:

```sh
cargo install cage
```

## Building from source

If you want to make changes to `cage`, you can clone the source code from
GitHub, `cd` to the `cage` directory, and run:

```sh
cargo build
```

Assuming nothing goes wrong, you should see a binary under the `target`
directory.  To install it, run:

```sh
cargo install --path .
```

To run the unit tests (many of which are still failing), run:

```sh
cargo test
```

## Vault support

Vault will not work with the minimal build since it requires OpenSSL.  Due
to the difficulties of getting OpenSSL to build with cage presently (as in,
copying and renaming several `.lib` and `.dll` files and hunting around for
correct binaries for your platform), we don't advise that you build Vault
support yet.

This is normally fine, because you only need Vault support if you are
running Vault servers and if you're deploying your project to staging or
production.

But if you really want to try Vault on Windows, you should start with just
trying to get `cage` to build as described above first.  Then try running
`cargo build` with any arguments, which should start you off in the right
direction for what's missing. You'll likely need to grab binaries
from [openssl wiki][] and just start renaming things in order to satisfy
the build. I had luck copying the binaries into `target/release/deps` and
then continually running `cargo build --release` while I checked to see if
the binary would work for my build or not.

If you know a way to get this working repeatably, we'd love to know!

Good luck! And don't hesitate to file issues on GitHub and ask for help.

[rustup]: https://www.rustup.rs/
[mingw-w64 installer]: https://sourceforge.net/projects/mingw-w64/files/Toolchains%20targetting%20Win32/Personal%20Builds/mingw-builds/installer/
[openssl wiki]: https://wiki.openssl.org/index.php/Binaries
