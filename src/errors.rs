//! We provide fancy error-handling support thanks to the [`error_chain`
//! crate][error_chain].  The primary advantage of `error_chain` is that it
//! provides support for backtraces.  The secondary advantage of this crate
//! is that it gives us nice, structured error types.
//!
//! [error_chain]: https://github.com/brson/error-chain

#![allow(missing_docs, clippy::redundant_closure)]

use compose_yml::v2 as dc;
use glob;
use semver;
use std::ffi::OsString;
use std::io;
use std::path::{PathBuf, StripPrefixError};
use std::string::FromUtf8Error;

use crate::project::PROJECT_CONFIG_PATH;
use crate::version;

// TODO: Replace `error-chain` with `anyhow` as soon as backtraces stablize.
error_chain! {
    // TODO HIGH: Most of these will go away as we convert them to more
    // meaningful errors.
    foreign_links {
        Compose(dc::Error);
        Docker(boondock::errors::Error);
        Utf8Error(FromUtf8Error);
        Glob(glob::GlobError);
        GlobPattern(glob::PatternError);
        Io(io::Error);
        StripPrefix(StripPrefixError);
    }

    errors {
        /// An error occurred running an external command.
        CommandFailed(command: Vec<OsString>) {
            description("error running external command")
            display("error running '{}'", command_to_string(&command))
        }

        /// We could not look up our project's `RuntimeState` using Docker.
        CouldNotGetRuntimeState {
            description("error getting the project's state from Docker")
            display("error getting the project's state from Docker")
        }

        /// We failed to parse a string.
        CouldNotParse(parsing_as: &'static str, input: String) {
            description("failed to parse string")
            display("failed to parse '{}' as {}", &input, parsing_as)
        }

        /// An error occurred reading a directory.
        CouldNotReadDirectory(path: PathBuf) {
            description("could not read a directory")
            display("could not read '{}'", path.display())
        }

        /// An error occurred reading a file.
        CouldNotReadFile(path: PathBuf) {
            description("could not read a file")
            display("could not read '{}'", path.display())
        }

        /// An error occurred writing a file.
        CouldNotWriteFile(path: PathBuf) {
            description("could not write to a file")
            display("could not write to '{}'", path.display())
        }

        /// A feature was disabled at compile time.
        FeatureDisabled {
            description("feature disabled at compile time")
            display("this feature was disabled when the application was \
                     compiled (you may want to rebuild from source)")
        }

        /// This project specified that it required a different version of
        /// this tool.
        MismatchedVersion(required: semver::VersionReq) {
            description("incompatible cage version")
            display("{} specifies cage {}, but you have {}",
                    PROJECT_CONFIG_PATH.display(), &required, version())
        }

        /// An error occurred applying a plugin.
        PluginFailed(plugin: String) {
            description("plugin failed")
            display("plugin '{}' failed", &plugin)
        }

        /// An target file tried to add new services that weren't present in
        /// the file it was overriding.
        ServicesAddedInTarget(base: PathBuf, target: PathBuf, names: Vec<String>) {
            description("services present in target but not in base")
            display("services {:?} present in {} but not in {}",
                    &names, base.display(), target.display())
        }

        /// The user tried to access an undefined library.
        ///
        /// TODO LOW: This will be merged with `UnknownSource` when library
        /// keys are merged with source repo aliases.
        UnknownLibKey(lib_key: String) {
            description("unknown library")
            display("no library '{}' defined in `config/sources.yml`", &lib_key)
        }

        /// The user tried to specify a repo subdirectory on a library
        LibHasRepoSubdirectory(lib_key: String) {
            description("invalid library context URL")
            display("library '{}' may not specify a subdirectory in its git URL", &lib_key)
        }

        /// The requested target does not appear to exist.
        UnknownTarget(target_name: String) {
            description("unknown target")
            display("unknown target '{}'", &target_name)
        }

        /// The requested pod or service does not appear to exist.
        UnknownPodOrService(pod_or_service_name: String) {
            description("unknown pod or service")
            display("unknown pod or service '{}'", &pod_or_service_name)
        }

        /// The requested service does not appear to exist.
        UnknownService(service_name: String) {
            description("unknown service")
            display("unknown service '{}'", &service_name)
        }

        /// The requested source alias does not appear to exist.
        UnknownSource(source_alias: String) {
            description("unknown source alias")
            display("unknown short alias '{}' for source tree (try `cage \
                     source ls`)",
                    &source_alias)
        }

        /// We were unable to communicate with the specified Vault server.
        VaultError(url: String) {
            description("an error occurred talking to a Vault server")
            display("an error occurred talking to the Vault server at {}", &url)
        }
    }
}

impl ErrorKind {
    /// Build an `ErrorKind::CouldNotParse` value.
    pub fn parse<S>(parsing_as: &'static str, input: S) -> ErrorKind
    where
        S: Into<String>,
    {
        ErrorKind::CouldNotParse(parsing_as, input.into())
    }
}

/// Convert a command-line into a string.
fn command_to_string(command: &[OsString]) -> String {
    let cmd: Vec<_> = command
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    cmd.join(" ")
}
