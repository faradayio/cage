//! The `conductor up` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ovr::Override;
use pod::PodType;
use project::Project;
use util::err;

/// We implement `conductor up` with a trait so we put it in its own module.
pub trait CommandUp {
    /// Up all the images associated with a project.
    fn up_all<CR>(&self, runner: &CR, ovr: &Override) -> Result<()>
        where CR: CommandRunner;

    /// Up all the images in the specified pods.
    fn up<CR>(&self,
              runner: &CR,
              ovr: &Override,
              pod_names: &[&str])
              -> Result<()>
        where CR: CommandRunner;
}

impl CommandUp for Project {
    fn up_all<CR>(&self, runner: &CR, ovr: &Override) -> Result<()>
        where CR: CommandRunner
    {
        let up_by_pod_type = |ty: PodType| -> Result<()> {
            let pod_names: Vec<_> = self.pods()
                .filter(|p| p.pod_type() == ty)
                .map(|p| p.name())
                .collect();
            self.up(runner, ovr, &pod_names)
        };
        try!(up_by_pod_type(PodType::Placeholder));
        up_by_pod_type(PodType::Service)
    }

    fn up<CR>(&self,
              runner: &CR,
              ovr: &Override,
              pods_names: &[&str])
              -> Result<()>
        where CR: CommandRunner
    {
        for pod_name in pods_names {
            let pod = try!(self.pod(pod_name)
                .ok_or_else(|| err!("Cannot find pod {}", pod_name)));
            if pod.enabled_in(ovr) {
                // We pass `-d` because we need to detach from each pod to
                // launch the next.  To avoid this, we'd need to use
                // multiple parallel threads and maybe some intelligent
                // output buffering.
                let status = try!(runner.build("docker-compose")
                    .args(&try!(pod.compose_args(self, ovr)))
                    .arg("up")
                    .arg("-d")
                    .status());
                if !status.success() {
                    return Err(err("Error running docker-compose"));
                }
            }
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_up_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    proj.up_all(&runner, ovr).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "up",
         "-d"]
    });
    proj.remove_test_output().unwrap();
}

#[test]
fn runs_docker_compose_up_on_specified_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    proj.up(&runner, ovr, &["db"]).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/db.yml"),
         "up",
         "-d"]
    });
    proj.remove_test_output().unwrap();
}

#[test]
fn runs_docker_compose_up_honors_start_in_overrides() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("production").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    proj.up_all(&runner, ovr).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "up",
         "-d"]
    });
    proj.remove_test_output().unwrap();
}
