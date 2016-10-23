//! Data structures representing arguments that we can pass to
//! `docker-compose` and other command-line tools.

use std::ffi::OsString;

pub use self::act_on::ActOn;
pub use self::cmd::*;

pub mod act_on;
mod cmd;
pub mod opts;

/// Trait for types which can be converted to command-line arguments.
pub trait ToArgs {
    /// Convert to arguments suitable for `std::process::Command` or our
    /// `CommandBuilder`.
    fn to_args(&self) -> Vec<OsString>;
}
