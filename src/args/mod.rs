//! Data structures representing arguments that we can pass to
//! `docker-compose` and other command-line tools.

use std::ffi::OsString;

pub use self::cmd::*;
pub use self::target::*;

mod cmd;
pub mod opts;
mod target;

/// Trait for types which can be converted to command-line arguments.
pub trait ToArgs {
    /// Convert to arguments suitable for `std::process::Command` or our
    /// `CommandBuilder`.
    fn to_args(&self) -> Vec<OsString>;
}
