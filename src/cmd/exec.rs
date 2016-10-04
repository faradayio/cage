//! The `conductor exec` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use exec::{self, ToArgs};
use ext::service::ServiceExt;
use project::Project;
use util::err;

/// We implement `conductor exec` with a trait so we can put it in its own
/// module.
pub trait CommandExec {
    /// Exectute a command inside a running container.  Even though we
    /// package up most of our arguments into structs, we still have a
    /// ridiculous number of arguments.
    fn exec<CR>(&self,
                runner: &CR,
                target: &exec::Target,
                command: &exec::Command,
                opts: &exec::Options)
                -> Result<()>
        where CR: CommandRunner;

    /// Execute an interactive shell inside a running container.
    fn shell<CR>(&self,
                 runner: &CR,
                 target: &exec::Target,
                 opts: &exec::Options)
                 -> Result<()>
        where CR: CommandRunner;
}

impl CommandExec for Project {
    fn exec<CR>(&self,
                runner: &CR,
                target: &exec::Target,
                command: &exec::Command,
                opts: &exec::Options)
                -> Result<()>
        where CR: CommandRunner
    {

        let status = try!(runner.build("docker-compose")
            .args(&try!(target.pod().compose_args(self, target.ovr())))
            .arg("exec")
            .args(&opts.to_args())
            .arg(target.service_name())
            .args(&command.to_args())
            .status());
        if !status.success() {
            return Err(err("Error running docker-compose"));
        }

        Ok(())
    }

    fn shell<CR>(&self,
                 runner: &CR,
                 target: &exec::Target,
                 opts: &exec::Options)
                 -> Result<()>
        where CR: CommandRunner
    {
        // Sanity-check our arguments.
        if opts.detached {
            return Err(err("Can't run shell in detached mode"));
        }
        if !opts.allocate_tty {
            return Err(err("Can't run shell without a TTY"));
        }

        let shell = try!(target.service().shell());
        self.exec(runner, target, &exec::Command::new(shell), opts)
    }
}

#[test]
fn invokes_docker_exec() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    let target = exec::Target::new(&proj, ovr, "frontend", "web").unwrap();

    let command = exec::Command::new("true");
    let opts = exec::Options { allocate_tty: false, ..Default::default() };
    proj.exec(&runner, &target, &command, &opts).unwrap();

    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "exec",
         "-T",
         "web",
         "true"]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn runs_shells() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    let target = exec::Target::new(&proj, ovr, "frontend", "web").unwrap();

    proj.shell(&runner, &target, &Default::default()).unwrap();

    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "exec",
         "web",
         "sh"]
    });

    proj.remove_test_output().unwrap();
}
