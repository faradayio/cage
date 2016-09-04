//! The `conductor up` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use ovr::Override;
use project::Project;
use util::Error;

/// We implement `conductor up` with a trait so we put it in its own module.
pub trait CommandUp {
    /// Up all the images associated with a project.
    fn up<CR>(&self, runner: &CR, ovr: &Override) -> Result<(), Error>
        where CR: CommandRunner;
}

impl CommandUp for Project {
    fn up<CR>(&self, runner: &CR, ovr: &Override) -> Result<(), Error>
        where CR: CommandRunner
    {
        for pod in self.pods() {
            // We pass `-d` because we need to detach from each pod to
            // launch the next.  To avoid this, we'd need to use multiple
            // parallel threads and maybe some intelligent output
            // buffering.
            let ovr_rel_path = try!(pod.override_rel_path(ovr));
            let status = try!(runner.build("docker-compose")
                .arg("-p").arg(pod.name())
                .arg("-f").arg(self.output_pods_dir().join(pod.rel_path()))
                .arg("-f").arg(self.output_pods_dir().join(ovr_rel_path))
                .arg("up").arg("-d")
                .status());
            if !status.success() {
                return Err(err!("Error running docker-compose"));
            }
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_up_on_all_pods() {
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();
    proj.up(&runner, &ovr).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p", "frontend",
         "-f", proj.output_dir().join("pods/frontend.yml"),
         "-f", proj.output_dir().join("pods/overrides/development/frontend.yml"),
         "up", "-d"]
    });
}
