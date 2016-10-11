//! A command which we want `docker-compose` to `exec` or `run` for us.

use std::ffi::{OsStr, OsString};

use args::ToArgs;

/// A command which can be executed.
#[derive(Debug)]
pub struct Command {
    /// The command to execute.
    pub command: OsString,
    /// The arguments to pass to the command.
    pub args: Vec<OsString>,
}

impl Command {
    /// Create a new `Command` object.
    pub fn new<S: AsRef<OsStr>>(command: S) -> Command {
        Command {
            command: command.as_ref().to_owned(),
            args: vec![],
        }
    }

    /// Add arguments to a `Command` object.  This is meant to be chained
    /// immediately after `new`, and it consumes `self` and returns it.
    pub fn with_args<A>(mut self, args: A) -> Command
        where A: IntoIterator,
              A::Item: Into<OsString>
    {
        self.args = args.into_iter().map(|arg| arg.into()).collect();
        self
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut result: Vec<OsString> = vec![];
        result.push(self.command.clone());
        result.extend(self.args.iter().cloned());
        result
    }
}

#[test]
fn command_to_args_converts_to_arguments() {
    assert_eq!(Command::new("foo").to_args(), vec![OsStr::new("foo")]);
    assert_eq!(Command::new("foo").with_args(&["--opt"]).to_args(),
               vec![OsStr::new("foo"), OsStr::new("--opt")]);
}
