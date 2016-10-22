# Cage on Windows

Generally speaking, docker-compose and docker engine are a work in progress on
Windows. This document should give you all the information you need on
available builds and overall guidance when working with cage
on Windows.

## Setup

First make sure you install rust for x86_64-pc-windows-gnu using the
`rustup-init` installer from [https://www.rustup.rs/](https://www.rustup.rs/).
You should make sure you use either `stable` or `beta` toolchains.

Next up you'll need to get a build env going with Msys2. Grab a mingw installer
from [Mingw-w64's sf.net](https://sourceforge.net/projects/mingw-w64/files/Toolchains%20targetting%20Win32/Personal%20Builds/mingw-builds/installer/)

Launch the install and follow the prompts, making sure you set the arch to
x86_64 so that it matches rust. Thread types should be fine as posix.

After the install succeeds (if you get download failures just keep trying),
open up the new bat script that was installed. You should be able to find it
either in your start menu or under `C:\Program Files\mingw-w64`.

From the prompt you should be able to get to `rustup` and `cargo` commands.

At this point you should make sure you've locally cloned the git repo for cage.
Change the current working directory of the prompt to the path that you cloned
cage to.

Now you should be able to build the binary using:
```
$ cargo build --no-default-features --features default-minimal
```

Assuming nothing goes bad, you should see a binary under the `target` directory
You can choose to work with it from there are try your luck with
`cargo install --no-default-features --features default-minimal`

### Notes

Vault will not work with the minimal build since it requires openssl.
Due to the randomness of getting openssl to build with cage presently
(as in, copying and renaming several `.lib` and `.dll` files and hunting around
for correct binaries for your platform), we don't advise that you build Vault
support yet.

If you really want to try, you should start with just trying to get the above
build to work first. Then try running `cargo build --release` which should
start you off in the right direction for what's missing. You'll likely need
to grab binaries from
[the openssl wiki](https://wiki.openssl.org/index.php/Binaries) and just start
renaming things in order to satisfy the build. I had luck copying the binaries
into `target/release/deps` and then continually running `cargo build --release`
while I checked to see if the binary would work for my build or not.

Good luck!