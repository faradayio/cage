//! Utilities for running and testing shell commands.

use std::cell::{Ref, RefCell};
use std::ffi::{OsStr, OsString};
use std::io;
use std::process;
use std::rc::Rc;

/// A factory that produces objects conforming to our `Command` wrapper
/// trait.  During tests, we'll use this to mock out the underlying system
/// and record all commands executed.
pub trait CommandRunner {
    /// The type of the commands we build.  Must implement our custom
    /// `Command` trait.
    type Command: Command;

    /// Build a new command.
    fn build<S: AsRef<OsStr>>(&self, program: S) -> Self::Command;
}

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

/// Support for running operating system commands.
pub struct OsCommandRunner;

impl CommandRunner for OsCommandRunner {
    type Command = process::Command;

    fn build<S: AsRef<OsStr>>(&self, program: S) -> Self::Command {
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
    let runner = OsCommandRunner;
    assert!(runner.build("true").status().unwrap().success());
    assert!(!runner.build("false").status().unwrap().success());
}

/// Support for running commands in test mode.
pub struct TestCommandRunner {
    /// The commands that have been executed.  Because we want to avoid
    /// borrow checker hell, we use `Rc<RefCell<_>>` to implement a shared,
    /// mutable value.
    cmds: Rc<RefCell<Vec<Vec<OsString>>>>,
}

impl TestCommandRunner {
    /// Create a new `TestCommandRunner`.
    pub fn new() -> TestCommandRunner {
        TestCommandRunner { cmds: Rc::new(RefCell::new(vec!())) }
    }

    /// Access the list of commands run.
    pub fn cmds(&self) -> Ref<Vec<Vec<OsString>>> {
        self.cmds.borrow()
    }
}

impl CommandRunner for TestCommandRunner {
    type Command = TestCommand;

    fn build<S: AsRef<OsStr>>(&self, program: S) -> Self::Command {
        TestCommand {
            cmd: vec!(program.as_ref().to_owned()),
            cmds: self.cmds.clone(),
        }
    }
}

/// A fake command that gets logged to a `TestCommandRunner` instead of
/// actually getting run.
pub struct TestCommand {
    /// The command we're building.
    cmd: Vec<OsString>,
    /// The list of commands we share with our `TestCommandRunner`, into which
    /// we'll insert `self.cmd` just before running.
    cmds: Rc<RefCell<Vec<Vec<OsString>>>>,
}

impl TestCommand {
    /// Record the execution of this command.
    fn record_execution(&self) {
        self.cmds.borrow_mut().push(self.cmd.clone());
    }
}

impl Command for TestCommand {
    /// Record the command arguments in our `TestCommandRunner`.
    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.cmd.push(arg.as_ref().to_owned());
        self
    }

    /// Always returns success.
    fn status(&mut self) -> io::Result<process::ExitStatus> {
        self.record_execution();

        // There's no portable way to build an `ExitStatus` in portable
        // Rust without actually running a command, so just choose an
        // inoffensive one with the result we want.
        process::Command::new("true").status()
    }
}

/// A macro for comparing the commands that were run with the commands we
/// hoped were run.  This is a bit trickier than you'd expect, because we
/// need to handle two complications:
///
/// 1. `TestCommandRunner::cmds` returns a `Ref<_>` type, which we
///    need to explicitly `deref()` before trying to compare, so that
///    we get a real `&`-style reference.
/// 2. We want to allow this macro to be passed a mix of `&'static str`
///    and `Path` objects so that it's easier to use, and internally
///    convert everything to an `OsString`.  So we need an internal `coerce`
///    helper that converts from `AsRef<OsStr>` (a very general trait
///    interface) to an actual `OsString`.
macro_rules! assert_ran {
    ($runner:expr, { $( [ $($arg:expr),+ ] ),* }) => {
        use std::ops::Deref;
        fn coerce<S: AsRef<$crate::std::ffi::OsStr>>(s: S) ->
            $crate::std::ffi::OsString
        {
            s.as_ref().to_owned()
        }
        let expected = vec!( $( vec!( $( coerce($arg) ),+ ) ),* );
        assert_eq!($runner.cmds().deref(), &expected);
    }
}

#[test]
pub fn test_command_runner_logs_commands() {
    let runner = TestCommandRunner::new();

    let exit_code = runner.build("git")
        .args(&["clone", "https://github.com/torvalds/linux"])
        .status().unwrap();
    assert!(exit_code.success());

    runner.build("echo").arg("a").arg("b").status().unwrap();

    assert_ran!(runner, {
        ["git", "clone", "https://github.com/torvalds/linux"],
        ["echo", "a", "b"]
    });
}
