//! This is the internal `cage` API.  If you're looking for documentation
//! about how to use the `cage` command-line tool, **please see the [`cage`
//! website][cage] instead.**
//!
//! ## A note about semantic versioning and API stability
//!
//! The `cage` library API is **unstable**, and it will remain so until
//! further notice.  We _do_ provide "semver" guarantees, but only for the
//! command-line interface and the on-disk project format, not for the
//! library API itself.
//!
//! If you would like to use `cage` as a library in another tool, please
//! contact the maintainers.  We may be able to stabilize parts of our API.
//!
//! ## Where to start
//!
//! Cage relies heavily on the [`compose_yml`][compose_yml] crate, which
//! represents a `docker-compose.yml` file.
//!
//! A good place to start reading through this API is the `Project` struct,
//! which represents an entire project managed by `cage`.  Most other
//! interesting types can be reached from there.
//!
//! You may also want to look the `plugins` module, which handles much of
//! our code generation and YAML transformation.  Essentially, cage works
//! like a multi-pass "compiler", where the intermediate representation is
//! a `compose_yml::v2::File` object, and each transformation plugin is a
//! analogous to a "pass" in a compiler.
//!
//! [cage]: http://cage.faraday.io/
//! [compose_yml]: http://docs.randomhacks.net/compose_yml/

// Enable clippy if our Cargo.toml file asked us to do so.
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

// Enable as many useful Rust and Clippy warnings as we can stand.  We'd
// also enable `trivial_casts`, but we're waiting for
// https://github.com/rust-lang/rust/issues/23416.
#![warn(missing_copy_implementations,
        missing_debug_implementations,
        missing_docs,
        trivial_numeric_casts,
        unsafe_code,
        unused_import_braces)]
// We disabled `unused_extern_crates` because it's failing on macro-only crates.
// We disabled `unused_qualifications` because it's failing on `try!`.
#![cfg_attr(feature="clippy", warn(cast_possible_truncation))]
#![cfg_attr(feature="clippy", warn(cast_possible_wrap))]
#![cfg_attr(feature="clippy", warn(cast_precision_loss))]
#![cfg_attr(feature="clippy", warn(cast_sign_loss))]
#![cfg_attr(feature="clippy", warn(missing_docs_in_private_items))]
#![cfg_attr(feature="clippy", warn(mut_mut))]
// We allow `println!` only in the `cmd` submodule.  If you want to print
// debugging output, using `debug!`, or have `cmd` pass you an `io::Write`
// implementation.
#![cfg_attr(feature="clippy", warn(print_stdout))]
// This allows us to use `unwrap` on `Option` values (because doing makes
// working with Regex matches much nicer) and when compiling in test mode
// (because using it in tests is idiomatic).
#![cfg_attr(all(not(test), feature="clippy"), warn(result_unwrap_used))]
#![cfg_attr(feature="clippy", warn(wrong_pub_self_convention))]

// The `error_chain` documentation says we need this.
#![recursion_limit = "1024"]

extern crate colored;
extern crate compose_yml;
extern crate boondock;
#[cfg(test)]
extern crate env_logger;
#[macro_use]
extern crate error_chain;
extern crate glob;
extern crate handlebars;
#[cfg(feature="hashicorp_vault")]
extern crate hashicorp_vault as vault;
extern crate includedir;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate phf;
#[cfg(test)]
extern crate rand;
extern crate rayon;
extern crate regex;
extern crate retry;
extern crate semver;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;
extern crate shlex;
extern crate url;

pub use default_tags::DefaultTags;
pub use errors::*;
pub use project::{PodOrService, Project, ProjectConfig, Pods, Targets};
pub use pod::{Pod, PodType, TargetFiles, AllFiles};
pub use runtime_state::RuntimeState;
pub use sources::{Sources, Source};
pub use sources::Iter as SourceIter;
pub use target::Target;
pub use util::err;

#[macro_use]
mod util;
pub mod args;
#[macro_use]
pub mod command_runner;
pub mod cmd;
mod default_tags;
pub mod dir;
mod errors;
mod ext;
pub mod hook;
pub mod plugins;
mod pod;
mod project;
mod runtime_state;
mod serde_helpers;
mod service_locations;
mod sources;
mod target;
mod template;

/// Include raw data files into our binary at compile time using the
/// `includedir_codegen` and `includedir` crates.  The actual code
/// generation is performed by our top-level `build.rs` script.
mod data {
    include!(concat!(env!("OUT_DIR"), "/data.rs"));
}

/// The version of this crate.
pub fn version() -> &'static semver::Version {
    lazy_static! {
        static ref VERSION: semver::Version =
            semver::Version::parse(env!("CARGO_PKG_VERSION"))
                .expect("package version should be a valid semver");
    }
    &VERSION
}
