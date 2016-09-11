//! The `conductor pull` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use ovr::Override;
use project::Project;
use util::Error;

/// We implement `conductor pull` with a trait so we put it in its own module.
pub trait CommandPull {
    /// Pull all the images associated with a project.
    fn pull<CR>(&self, runner: &CR, ovr: &Override) -> Result<(), Error>
        where CR: CommandRunner;
}

impl CommandPull for Project {
    fn pull<CR>(&self, runner: &CR, ovr: &Override) -> Result<(), Error>
        where CR: CommandRunner
    {
        for pod in self.pods() {
            let status = try!(runner.build("docker-compose")
                .args(&try!(pod.compose_args(self, ovr)))
                .arg("pull")
                .status());
            if !status.success() {
                return Err(err!("Error running docker-compose"));
            }
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_pull_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();
    proj.pull(&runner, &ovr).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p", "frontend",
         "-f", proj.output_dir().join("pods/frontend.yml"),
         "-f", proj.output_dir().join("pods/overrides/development/frontend.yml"),
         "pull"]
    });
    proj.remove_test_output().unwrap();
}
