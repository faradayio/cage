//! The `conductor logs` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use exec::{self, ToArgs};
use project::Project;
use util::{Error, err};

/// We implement `conductor logs` with a trait so we can put it in its own
/// module.
pub trait CommandLogs {
    /// Display logs for a service
    fn logs<CR>(&self,
               runner: &CR,
               target: &exec::Target,
               opts: &[CR])
               -> Result<(), Error>
        where CR: CommandRunner;
}

impl CommandLogs for Project {
    fn logs<CR>(&self,
               runner: &CR,
               target: &exec::Target,
               opts: &[CR])
               -> Result<(), Error>
        where CR: CommandRunner
    {

        let status = try!(runner.build("docker-compose")
            .args(&try!(target.pod().compose_args(self, target.ovr())))
            .arg("logs")
            .arg(target.service_name())
            .args(opts)
            .status());
        if !status.success() {
            return Err(err("Error running docker-compose"));
        }

        Ok(())
    }
}

#[test]
fn invokes_docker_logs() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    let target = exec::Target::new(&proj, ovr, "frontend", "web").unwrap();

    let opts = ["-f"];
    proj.logs(&runner, &target, &opts).unwrap();

    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "logs",
         "-f",
         "web"]
    });

    proj.remove_test_output().unwrap();
}
