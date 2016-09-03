//! Utilities for running and testing shell commands.

use std::ffi::OsStr;
use std::io;
use std::process;

/// A stripped down interface based on `std::process::Command`.  We use
/// this so we can mock out shell commands during tests.
pub trait Command {
    /// Add an arugment to our command.
    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self;

    /// Add several arguments to our command.
    fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut Self {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    /// Run our command.
    fn status(&mut self) -> io::Result<process::ExitStatus>;
}

/// A factory that produces objects conforming to our `Command` wrapper
/// trait.  During tests, we'll use this to mock out the underlying system
/// and record all commands executed.
///
/// There's some unfortunate Rust lifetime trickiness with the `'a`
/// parameter, which we use to indicate the idea that the `Command`
/// returned by `build` is allowed to hold a mutable reference pointing
/// back to the `CommandBuilder`.  Usually we don't need to specify
/// lifetimes because Rust can do all the magic in the background, but here
/// we actually need to expose them.
pub trait CommandBuilder<'a> {
    /// The type of the commands we build.  Must implement our custom
    /// `Command` trait and may contain references of type `'a`.
    type Command: Command + 'a;

    /// Build a new command.
    fn build<S: AsRef<OsStr>>(&'a mut self, program: S) -> Self::Command;
}

/// Support for running operating system commands.
pub struct OsCommandBuilder;

impl<'a> CommandBuilder<'a> for OsCommandBuilder {
    type Command = process::Command;

    fn build<S: AsRef<OsStr>>(&'a mut self, program: S) -> Self::Command {
        process::Command::new(program)
    }
}

impl Command for process::Command {
    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        process::Command::arg(self, arg)
    }

    fn args<S: AsRef<OsStr>>(&mut self, args: &[S]) -> &mut Self {
        process::Command::args(self, args)
    }

    fn status(&mut self) -> io::Result<process::ExitStatus> {
        process::Command::status(self)
    }
}

#[test]
fn os_command_builder_runs_commands() {
    let mut builder = OsCommandBuilder;
    assert!(builder.build("true").status().unwrap().success());
    assert!(!builder.build("false").status().unwrap().success());
}

/// Support for running commands in test mode.
pub struct TestCommandBuilder {
    cmds: Vec<Vec<String>>
}

impl TestCommandBuilder {
    /// Create a new `TestCommandBuilder`.
    pub fn new() -> TestCommandBuilder {
        TestCommandBuilder { cmds: vec!() }
    }

    /// Access the list of commands run.
    pub fn cmds(&self) -> &[Vec<String>] {
        &self.cmds
    }
}

impl<'a> CommandBuilder<'a> for TestCommandBuilder {
    type Command = TestCommand<'a>;

    fn build<S: AsRef<OsStr>>(&'a mut self, program: S) -> Self::Command {
        self.cmds.push(vec!(program.as_ref().to_string_lossy().into_owned()));
        TestCommand { builder: self }
    }
}

/// A fake command that gets logged to a `TestCommandBuilder` instead of
/// actually getting run.
pub struct TestCommand<'a> {
    builder: &'a mut TestCommandBuilder,
}

impl<'a> Command for TestCommand<'a> {
    /// Record the command arguments in our `TestCommandBuilder`.
    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.builder.cmds.last_mut().unwrap()
            .push(arg.as_ref().to_string_lossy().into_owned());
        self
    }

    /// Always returns success.
    fn status(&mut self) -> io::Result<process::ExitStatus> {
        // There's no portable way to build an `ExitStatus` in portable
        // Rust without actually running a command, so just choose an
        // inoffensive one with the result we want.
        process::Command::new("true").status()
    }
}

#[test]
pub fn test_command_builder_logs_commands() {
    let mut builder = TestCommandBuilder::new();
    let exit_code = builder.build("git")
        .args(&["clone", "https://github.com/torvalds/linux"])
        .status().unwrap();
    assert!(exit_code.success());
    assert_eq!(builder.cmds(),
               &[&["git", "clone", "https://github.com/torvalds/linux"]]);
}
