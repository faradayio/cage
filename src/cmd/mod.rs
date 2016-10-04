//! Various commands which can be performed on a `conductor` project,
//! corresponding to CLI entry points.
//!
//! To gain access to all commands at once:
//!
//! ```
//! use conductor::cmd::*;
//! ```

// We're allowed to print things to the user in the `cmd` submodule.
#![cfg_attr(feature="clippy", allow(print_stdout))]

pub use self::build::CommandBuild;
pub use self::exec::CommandExec;
pub use self::generate::CommandGenerate;
pub use self::logs::CommandLogs;
pub use self::pull::CommandPull;
pub use self::repo::CommandRepo;
pub use self::run::CommandRun;
pub use self::stop::CommandStop;
pub use self::up::CommandUp;

mod build;
mod exec;
mod generate;
mod logs;
mod pull;
mod repo;
mod run;
mod stop;
mod up;
