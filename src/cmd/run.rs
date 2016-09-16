//! The `conductor run` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use ovr::Override;
use project::Project;
use util::Error;

/// We implement `conductor run` with a trait so we put it in its own module.
pub trait CommandRun {
    /// Run a specific pod as a one-shot task.
    fn run<CR>(&self, runner: &CR, ovr: &Override, pod: &str) ->
        Result<(), Error>
        where CR: CommandRunner;
}

impl CommandRun for Project {
    fn run<CR>(&self, runner: &CR, ovr: &Override, pod: &str) ->
        Result<(), Error>
        where CR: CommandRunner
    {
        let pod = try!(self.pod(pod).ok_or_else(|| {
            err!("Cannot find pod {}", pod)
        }));
        let status = try!(runner.build("docker-compose")
            .args(&try!(pod.compose_args(self, ovr)))
            .arg("up")
            .status());
        if !status.success() {
            return Err(err!("Error running docker-compose"));
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_up_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();
    proj.run(&runner, &ovr, "migrate").unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p", "rails_hello",
         "-f", proj.output_dir().join("pods/migrate.yml"),
         "-f", proj.output_dir().join("pods/overrides/development/migrate.yml"),
         "up"]
    });
    proj.remove_test_output().unwrap();
}
