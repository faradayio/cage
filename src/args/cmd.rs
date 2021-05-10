//! A command which we want `docker-compose` to `exec` or `run` for us.

use std::ffi::{OsStr, OsString};

use crate::args::ToArgs;

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

    /// Create a new `Command` object from a vec, assuming first item is the command
    pub fn from_ordered_vec(list: Vec<String>) -> Option<Command> {
        match list.split_first() {
            Some((executable, args)) => Some(Command {
                command: OsString::from(executable),
                args: args.iter().map(|arg| arg.into()).collect(),
            }),
            None => None,
        }
    }

    /// Add arguments to a `Command` object.  This is meant to be chained
    /// immediately after `new`, and it consumes `self` and returns it.
    pub fn with_args<A>(mut self, args: A) -> Command
    where
        A: IntoIterator,
        A::Item: Into<OsString>,
    {
        self.args = args.into_iter().map(|arg| arg.into()).collect();
        self
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut result: Vec<OsString> = vec![self.command.clone()];
        result.extend(self.args.iter().cloned());
        result
    }
}

#[test]
fn command_to_args_converts_to_arguments() {
    assert_eq!(Command::new("foo").to_args(), vec![OsStr::new("foo")]);
    assert_eq!(
        Command::new("foo").with_args(&["--opt"]).to_args(),
        vec![OsStr::new("foo"), OsStr::new("--opt")]
    );
}
