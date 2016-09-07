//! The `conductor exec` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use exec::{self, ToArgs};
use ovr::Override;
use project::Project;
use util::Error;

/// We implement `conductor exec` with a trait so we can put it in its own
/// module.
pub trait CommandExec {
    /// Exectute a command inside a running container.  Even though we
    /// package up most of our arguments into structs, we still have a
    /// ridiculous number of arguments.
    fn exec<CR>(&self, runner: &CR, ovr: &Override,
                target: &exec::Target,
                command: &exec::Command,
                opts: &exec::Options) ->
        Result<(), Error>
        where CR: CommandRunner;
}

impl CommandExec for Project {
    fn exec<CR>(&self, runner: &CR, ovr: &Override,
                target: &exec::Target,
                command: &exec::Command,
                opts: &exec::Options) ->
        Result<(), Error>
        where CR: CommandRunner
    {
        let pod = try!(self.pod(&target.pod).ok_or_else(|| {
            err!("Cannot find pod {}", &target.pod)
        }));
        // Just check to see whether our service exists.
        let _ = try!(pod.file().services.get(&target.service).ok_or_else(|| {
            err!("Cannot find service {} in pod {}", &target.service, &target.pod)
        }));

        let status = try!(runner.build("docker-compose")
            .args(&try!(pod.compose_args(self, ovr)))
            .arg("exec")
            .args(&opts.to_args())
            .arg(&target.service)
            .args(&command.to_args())
            .status());
        if !status.success() {
            return Err(err!("Error running docker-compose"));
        }

        Ok(())
    }
}

#[test]
fn invokes_docker_exec() {
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();
    let target = exec::Target::new("frontend", "web");
    let command = exec::Command::new("true");
    let opts = exec::Options { allocate_tty: false, ..Default::default() };
    proj.exec(&runner, &ovr, &target, &command, &opts).unwrap();

    assert_ran!(runner, {
        ["docker-compose",
         "-p", "frontend",
         "-f", proj.output_dir().join("pods/frontend.yml"),
         "-f", proj.output_dir().join("pods/overrides/development/frontend.yml"),
         "exec", "-T", "web", "true"]
    });

    proj.remove_test_output().unwrap();
}
