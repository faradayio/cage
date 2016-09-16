//! Various commands which can be performed on a `conductor` project,
//! corresponding to CLI entry points.
//!
//! To gain access to all commands at once:
//!
//! ```
//! use conductor::cmd::*;
//! ```

pub use self::build::CommandBuild;
pub use self::exec::CommandExec;
pub use self::generate::CommandGenerate;
pub use self::pull::CommandPull;
pub use self::repo::CommandRepo;
pub use self::stop::CommandStop;
pub use self::up::CommandUp;

mod build;
mod exec;
mod generate;
mod pull;
mod repo;
mod stop;
mod up;
