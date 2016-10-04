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
use std::ffi::OsString;
use std::io;
use std::path::StripPrefixError;
use std::string::FromUtf8Error;
use vault;

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
    }
}

/// Convert a command-line into a string.
fn command_to_string(command: &[OsString]) -> String {
    let cmd: Vec<_> =
        command.iter().map(|s| s.to_string_lossy().into_owned()).collect();
    cmd.join(" ")
}
