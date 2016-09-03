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
/// back to the `CommandRunner`.  Usually we don't need to specify
/// lifetimes because Rust can do all the magic in the background, but here
/// we actually need to expose them.
pub trait CommandRunner<'a> {
    /// The type of the commands we build.  Must implement our custom
    /// `Command` trait and may contain references of type `'a`.
    type Command: Command + 'a;

    /// Build a new command.
    fn build<S: AsRef<OsStr>>(&'a mut self, program: S) -> Self::Command;
}

/// Support for running operating system commands.
pub struct OsCommandRunner;

impl<'a> CommandRunner<'a> for OsCommandRunner {
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
fn os_command_runner_runs_commands() {
    let mut runner = OsCommandRunner;
    assert!(runner.build("true").status().unwrap().success());
    assert!(!runner.build("false").status().unwrap().success());
}

/// Support for running commands in test mode.
pub struct TestCommandRunner {
    cmds: Vec<Vec<String>>
}

impl TestCommandRunner {
    /// Create a new `TestCommandRunner`.
    pub fn new() -> TestCommandRunner {
        TestCommandRunner { cmds: vec!() }
    }

    /// Access the list of commands run.
    pub fn cmds(&self) -> &[Vec<String>] {
        &self.cmds
    }
}

impl<'a> CommandRunner<'a> for TestCommandRunner {
    type Command = TestCommand<'a>;

    fn build<S: AsRef<OsStr>>(&'a mut self, program: S) -> Self::Command {
        self.cmds.push(vec!(program.as_ref().to_string_lossy().into_owned()));
        TestCommand { runner: self }
    }
}

/// A fake command that gets logged to a `TestCommandRunner` instead of
/// actually getting run.
pub struct TestCommand<'a> {
    runner: &'a mut TestCommandRunner,
}

impl<'a> Command for TestCommand<'a> {
    /// Record the command arguments in our `TestCommandRunner`.
    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.runner.cmds.last_mut().unwrap()
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
pub fn test_command_runner_logs_commands() {
    let mut runner = TestCommandRunner::new();
    let exit_code = runner.build("git")
        .args(&["clone", "https://github.com/torvalds/linux"])
        .status().unwrap();
    assert!(exit_code.success());
    assert_eq!(runner.cmds(),
               &[&["git", "clone", "https://github.com/torvalds/linux"]]);
}
