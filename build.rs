//! This script is run automatically before building any source code.  It
//! can include 3rd-party Rust dependencies and use them to generate Rust
//! source code for us to compile.

extern crate includedir_codegen;

use includedir_codegen::Compression;

fn main() {
    // Based on example build.rs: https://github.com/tilpner/includedir
    includedir_codegen::start("DATA")
        .dir("data", Compression::None)
        .build("data.rs")
        .unwrap();
}
