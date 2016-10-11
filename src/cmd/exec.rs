//! The `exec` command.

use args::{self, ToArgs};
use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ext::service::ServiceExt;
use ovr::Override;
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
                ovr: &Override,
                service_name: &str,
                command: &args::Command,
                opts: &args::opts::Exec)
                -> Result<()>
        where CR: CommandRunner;

    /// Execute an interactive shell inside a running container.
    fn shell<CR>(&self,
                 runner: &CR,
                 ovr: &Override,
                 service_name: &str,
                 opts: &args::opts::Exec)
                 -> Result<()>
        where CR: CommandRunner;
}

impl CommandExec for Project {
    fn exec<CR>(&self,
                runner: &CR,
                ovr: &Override,
                service_name: &str,
                command: &args::Command,
                opts: &args::opts::Exec)
                -> Result<()>
        where CR: CommandRunner
    {
        let (pod, service_name) = try!(self.service_or_err(service_name));
        runner.build("docker-compose")
            .args(&try!(pod.compose_args(self, ovr)))
            .arg("exec")
            .args(&opts.to_args())
            .arg(service_name)
            .args(&command.to_args())
            .exec()
    }

    fn shell<CR>(&self,
                 runner: &CR,
                 ovr: &Override,
                 service_name: &str,
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

        let (pod, service_name) = try!(self.service_or_err(service_name));
        let service = try!(pod.service_or_err(ovr, service_name));
        let shell = try!(service.shell());
        self.exec(runner, ovr, service_name, &args::Command::new(shell), opts)
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

    let command = args::Command::new("true");
    let mut opts = args::opts::Exec::default();
    opts.allocate_tty = false;
    proj.exec(&runner, ovr, "web", &command, &opts).unwrap();

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

    proj.shell(&runner, ovr, "web", &Default::default()).unwrap();

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
