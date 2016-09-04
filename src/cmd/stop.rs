//! The `conductor stop` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use ovr::Override;
use project::Project;
use util::Error;

/// We implement `conductor stop` with a trait so we put it in its own module.
pub trait CommandStop {
    /// Stop all the images associated with a project.
    fn stop<CR>(&self, runner: &CR, ovr: &Override) -> Result<(), Error>
        where CR: CommandRunner;
}

impl CommandStop for Project {
    fn stop<CR>(&self, runner: &CR, ovr: &Override) -> Result<(), Error>
        where CR: CommandRunner
    {
        for pod in self.pods() {
            let ovr_rel_path = try!(pod.override_rel_path(ovr));
            let status = try!(runner.build("docker-compose")
                .arg("-p").arg(pod.name())
                .arg("-f").arg(self.output_pods_dir().join(pod.rel_path()))
                .arg("-f").arg(self.output_pods_dir().join(ovr_rel_path))
                .arg("stop")
                .status());
            if !status.success() {
                return Err(err!("Error running docker-compose"));
            }
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_stop_on_all_pods() {
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();
    proj.stop(&runner, &ovr).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p", "frontend",
         "-f", proj.output_dir().join("pods/frontend.yml"),
         "-f", proj.output_dir().join("pods/overrides/development/frontend.yml"),
         "stop"]
    });
}
