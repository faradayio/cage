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

// Enable as many useful Rust and Clippy warnings as we can stand.  We'd
// also enable `trivial_casts`, but we're waiting for
// https://github.com/rust-lang/rust/issues/23416.
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    clippy::all
)]
#![allow(clippy::field_reassign_with_default, clippy::unnecessary_wraps)]
// The `error_chain` documentation says we need this.
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

pub use crate::default_tags::DefaultTags;
pub use crate::errors::*;
pub use crate::pod::{AllFiles, Pod, PodType, TargetFiles};
pub use crate::project::{PodOrService, Pods, Project, ProjectConfig, Targets};
pub use crate::runtime_state::RuntimeState;
pub use crate::sources::Iter as SourceIter;
pub use crate::sources::{Source, Sources};
pub use crate::target::Target;
pub use crate::util::err;

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

/// The version of this crate.
pub fn version() -> &'static semver::Version {
    lazy_static! {
        static ref VERSION: semver::Version =
            semver::Version::parse(env!("CARGO_PKG_VERSION"))
                .expect("package version should be a valid semver");
    }
    &VERSION
}
