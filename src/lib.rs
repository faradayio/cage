//! `conductor` as a reusable API, so that you can call it from other tools.

#![warn(missing_docs)]

extern crate docker_compose;
#[cfg(test)] extern crate env_logger;
extern crate glob;
extern crate handlebars;
extern crate includedir;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate phf;
extern crate rand;
extern crate regex;
extern crate retry;
extern crate rustc_serialize;
extern crate shlex;

pub use util::Error;
pub use default_tags::DefaultTags;
pub use ovr::Override;
pub use project::{Project, Pods, Overrides};
pub use pod::{Pod, OverrideFiles, AllFiles};
pub use repos::{Repos, Repo};
pub use repos::Iter as RepoIter;

#[macro_use] mod util;
#[macro_use] pub mod command_runner;
pub mod cmd;
mod default_tags;
pub mod dir;
pub mod exec;
mod ext;
mod ovr;
mod pod;
mod project;
mod repos;
mod template;

/// Include raw data files into our binary at compile time using the
/// `includedir_codegen` and `includedir` crates.  The actual code
/// generation is performed by our top-level `build.rs` script.
mod data {
    include!(concat!(env!("OUT_DIR"), "/data.rs"));
}
