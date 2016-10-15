//! Data structures representing arguments that we can pass to
//! `docker-compose` and other command-line tools.

use std::ffi::OsString;

pub use self::cmd::*;

mod cmd;
pub mod opts;

/// Trait for types which can be converted to command-line arguments.
pub trait ToArgs {
    /// Convert to arguments suitable for `std::process::Command` or our
    /// `CommandBuilder`.
    fn to_args(&self) -> Vec<OsString>;
}

/// The names of pods, services or both to pass to one of our commands.
#[derive(Debug)]
pub enum ActOn {
    /// Act upon all the pods and/or services associated with this project.
    All,
    /// Act upon only the named pods and/or services.
    Named(Vec<String>),
}
