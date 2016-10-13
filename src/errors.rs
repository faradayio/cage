//! We provide fancy error-handling support thanks to the [`error_chain`
//! crate][error_chain].  The primary advantage of `error_chain` is that it
//! provides support for backtraces.  The secondary advantage of this crate
//! is that it gives us nice, structured error types.
//!
//! [error_chain]: https://github.com/brson/error-chain

#![allow(missing_docs)]
#![cfg_attr(feature="clippy", allow(redundant_closure))]

use compose_yml::v2 as dc;
use glob;
use semver;
use std::ffi::OsString;
use std::io;
use std::path::{PathBuf, StripPrefixError};
use std::string::FromUtf8Error;
use vault;

use project::PROJECT_CONFIG_PATH;
use version;

error_chain! {
    // Hook up to other libraries which also use `error_chain`.  These
    // conversions are implicit.
    links {
        dc::Error, dc::ErrorKind, Compose;
    }

    // TODO HIGH: Most of these will go away as we convert them to more
    // meaningful errors.
    foreign_links {
        FromUtf8Error, Utf8Error;
        glob::GlobError, Glob;
        glob::PatternError, GlobPattern;
        io::Error, Io;
        StripPrefixError, StripPrefix;
        vault::Error, Vault;
    }

    errors {
        /// An error occurred running an external command.
        CommandFailed(command: Vec<OsString>) {
            description("error running external command")
            display("error running '{}'", command_to_string(&command))
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

        /// An override file tried to add new services that weren't present in
        /// the file it was overriding.
        ServicesAddedInOverride(base: PathBuf, ovr: PathBuf, names: Vec<String>) {
            description("services present in override but not in base")
            display("services {:?} present in {} but not in {}",
                    &names, base.display(), ovr.display())
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
    }
}

/// Convert a command-line into a string.
fn command_to_string(command: &[OsString]) -> String {
    let cmd: Vec<_> =
        command.iter().map(|s| s.to_string_lossy().into_owned()).collect();
    cmd.join(" ")
}
