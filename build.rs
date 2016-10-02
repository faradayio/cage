//! This script is run automatically before building any source code.  It
//! can include 3rd-party Rust dependencies and use them to generate Rust
//! source code for us to compile.
//!
//! The serde-handling portions are based on
//! https://serde.rs/codegen-hybrid.html and
//! https://travis-ci.org/emk/compose_yml

extern crate includedir_codegen;
extern crate glob;

use includedir_codegen::Compression;

fn main() {
    // Don't re-run this script unless one of the inputs has changed.
    for entry in glob::glob("data/**/*").expect("Failed to read glob pattern") {
        println!("cargo:rerun-if-changed={}", entry.unwrap().display());
    }

    // Based on example build.rs: https://github.com/tilpner/includedir
    includedir_codegen::start("DATA")
        .dir("data", Compression::None)
        .build("data.rs")
        .unwrap();

    // Handle serde, too.
    generate_serde();
}


#[cfg(feature = "serde_codegen")]
fn generate_serde() {
    use std::fs;
    extern crate glob;
    extern crate serde_codegen;

    use std::env;
    use std::path::Path;

    let out_dir = env::var_os("OUT_DIR").unwrap();

    // Switch to our `src` directory so that we have the right base for our
    // globs, and so that we won't need to strip `src/` off every path.
    env::set_current_dir("src").unwrap();

    for entry in glob::glob("**/*.in.rs").expect("Failed to read glob pattern") {
        match entry {
            Ok(src) => {
                let mut dst = Path::new(&out_dir).join(&src);

                // Change ".in.rs" to ".rs".
                dst.set_file_name(src.file_stem().expect("Failed to get file stem"));
                dst.set_extension("rs");

                // Make sure our target directory exists.  We only need
                // this if there are extra nested sudirectories under src/.
                fs::create_dir_all(dst.parent().unwrap()).unwrap();

                // Process our source file.
                serde_codegen::expand(&src, &dst).unwrap();
            }
            Err(e) => {
                panic!("Error globbing: {}", e);
            }
        }
    }
}

#[cfg(not(feature = "serde_codegen"))]
fn generate_serde() {
    // do nothing
}
