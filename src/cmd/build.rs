//! The `conductor build` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ovr::Override;
use project::Project;

/// We implement `conductor build` with a trait so we put it in its own
/// module.
pub trait CommandBuild {
    /// Build all the images associated with this project.
    fn build<CR>(&self, runner: &CR, ovr: &Override) -> Result<()>
        where CR: CommandRunner;
}

impl CommandBuild for Project {
    fn build<CR>(&self, runner: &CR, ovr: &Override) -> Result<()>
        where CR: CommandRunner
    {
        for pod in self.pods() {
            try!(runner.build("docker-compose")
                .args(&try!(pod.compose_args(self, ovr)))
                .arg("build")
                .exec());
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_build_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    proj.build(&runner, ovr).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "build"]
    });
    proj.remove_test_output().unwrap();
}
