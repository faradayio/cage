//! The `exec` command.

use args::{self, ToArgs};
use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ext::service::ServiceExt;
use project::Project;
use util::err;

/// We implement `exec` with a trait so we can put it in its own
/// module.
pub trait CommandExec {
    /// Exectute a command inside a running container.  Even though we
    /// package up most of our arguments into structs, we still have a
    /// ridiculous number of arguments.
    fn exec<CR>(&self,
                runner: &CR,
                target: &args::Target,
                command: &args::Command,
                opts: &args::opts::Exec)
                -> Result<()>
        where CR: CommandRunner;

    /// Execute an interactive shell inside a running container.
    fn shell<CR>(&self,
                 runner: &CR,
                 target: &args::Target,
                 opts: &args::opts::Exec)
                 -> Result<()>
        where CR: CommandRunner;
}

impl CommandExec for Project {
    fn exec<CR>(&self,
                runner: &CR,
                target: &args::Target,
                command: &args::Command,
                opts: &args::opts::Exec)
                -> Result<()>
        where CR: CommandRunner
    {

        runner.build("docker-compose")
            .args(&try!(target.pod().compose_args(self, target.ovr())))
            .arg("exec")
            .args(&opts.to_args())
            .arg(target.service_name())
            .args(&command.to_args())
            .exec()
    }

    fn shell<CR>(&self,
                 runner: &CR,
                 target: &args::Target,
                 opts: &args::opts::Exec)
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
        self.exec(runner, target, &args::Command::new(shell), opts)
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
    let target = args::Target::new(&proj, ovr, "frontend", "web").unwrap();

    let command = args::Command::new("true");
    let mut opts = args::opts::Exec::default();
    opts.allocate_tty = false;
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
    let target = args::Target::new(&proj, ovr, "frontend", "web").unwrap();

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
