//! We provide fancy error-handling support thanks to the [`anyhow`
//! crate][anyhow].  The primary advantage of `anyhow` is that it
//! provides support for backtraces (when available) and easy error context.
//!
//! [anyhow]: https://github.com/dtolnay/anyhow

#![allow(missing_docs, clippy::redundant_closure)]

use std::ffi::OsString;
use std::path::PathBuf;
use thiserror::Error;

use crate::project::PROJECT_CONFIG_PATH;
use crate::version;

pub type Result<T> = anyhow::Result<T>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error running '{}'", command_to_string(.0))]
    CommandFailed(Vec<OsString>),

    #[error("error getting the project's state from Docker")]
    CouldNotGetRuntimeState,

    #[error("failed to parse '{}' as {}", .input, .parsing_as)]
    CouldNotParse {
        parsing_as: &'static str,
        input: String,
    },

    #[error("could not read '{}'", .0.display())]
    CouldNotReadDirectory(PathBuf),

    #[error("could not read '{}'", .0.display())]
    CouldNotReadFile(PathBuf),

    #[error("could not write to '{}'", .0.display())]
    CouldNotWriteFile(PathBuf),

    #[error("this feature was disabled when the application was compiled (you may want to rebuild from source)")]
    FeatureDisabled,

    #[error("{} specifies cage_version {}, but you have {}", PROJECT_CONFIG_PATH.display(), .0, version())]
    MismatchedVersion(semver::VersionReq),

    #[error("output directory {} already exists (please delete)", .0.display())]
    OutputDirectoryExists(PathBuf),

    #[error("plugin '{}' failed", .0)]
    PluginFailed(String),

    #[error("services {:?} present in {} but not in {}", .names, .base.display(), .target.display())]
    ServicesAddedInTarget {
        base: PathBuf,
        target: PathBuf,
        names: Vec<String>,
    },

    #[error("no library '{}' defined in `config/sources.yml`", .0)]
    UnknownLibKey(String),

    #[error("library '{}' may not specify a subdirectory in its git URL", .0)]
    LibHasRepoSubdirectory(String),

    #[error("unknown target '{}'", .0)]
    UnknownTarget(String),

    #[error("unknown pod or service '{}'", .0)]
    UnknownPodOrService(String),

    #[error("unknown service '{}'", .0)]
    UnknownService(String),

    #[error("unknown short alias '{}' for source tree (try `cage source ls`)", .0)]
    UnknownSource(String),

    #[error("an error occurred talking to the Vault server at {}", .0)]
    VaultError(String),
}

impl Error {
    pub fn parse<S>(parsing_as: &'static str, input: S) -> Error
    where
        S: Into<String>,
    {
        Error::CouldNotParse {
            parsing_as,
            input: input.into(),
        }
    }
}

fn command_to_string(command: &[OsString]) -> String {
    let cmd: Vec<_> = command
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    cmd.join(" ")
}
