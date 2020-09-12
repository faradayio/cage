//! Pass simple commands directly through to `docker-compose`.

use crate::args;
#[cfg(test)]
use crate::command_runner::TestCommandRunner;
use crate::command_runner::{Command, CommandRunner};
use crate::errors::*;
use crate::pod::Pod;
use crate::project::{PodOrService, Project};

/// Pass simple commands directly through to `docker-compose`.
pub trait CommandCompose {
    /// Pass simple commands directly through to `docker-compose`.
    fn compose<CR>(
        &self,
        runner: &CR,
        command: &str,
        act_on: &args::ActOn,
        opts: &dyn args::ToArgs,
    ) -> Result<()>
    where
        CR: CommandRunner;

    /// Run a `docker-compose` command on a single pod.  If the pod is
    /// disabled, this does nothing.
    fn compose_pod<CR>(
        &self,
        runner: &CR,
        command: &str,
        pod: &Pod,
        opts: &dyn args::ToArgs,
    ) -> Result<()>
    where
        CR: CommandRunner;

    /// Run a `docker-compose` command on a single service.  If the pod is
    /// disabled, this does nothing.
    fn compose_service<CR>(
        &self,
        runner: &CR,
        command: &str,
        pod: &Pod,
        service_name: &str,
        opts: &dyn args::ToArgs,
    ) -> Result<()>
    where
        CR: CommandRunner;
}

impl CommandCompose for Project {
    fn compose<CR>(
        &self,
        runner: &CR,
        command: &str,
        act_on: &args::ActOn,
        opts: &dyn args::ToArgs,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        for pod_or_service in act_on.pods_or_services(self) {
            match pod_or_service? {
                PodOrService::Pod(pod) => {
                    self.compose_pod(runner, command, pod, opts)?;
                }
                PodOrService::Service(pod, service_name) => {
                    self.compose_service(runner, command, pod, service_name, opts)?;
                }
            }
        }

        Ok(())
    }

    fn compose_pod<CR>(
        &self,
        runner: &CR,
        command: &str,
        pod: &Pod,
        opts: &dyn args::ToArgs,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        if pod.enabled_in(self.current_target()) {
            runner
                .build("docker-compose")
                .args(&pod.compose_args(self)?)
                .arg(command)
                .args(&opts.to_args())
                .exec()?;
        }
        Ok(())
    }

    fn compose_service<CR>(
        &self,
        runner: &CR,
        command: &str,
        pod: &Pod,
        service_name: &str,
        opts: &dyn args::ToArgs,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        if pod.enabled_in(self.current_target()) {
            runner
                .build("docker-compose")
                .args(&pod.compose_args(self)?)
                .arg(command)
                .args(&opts.to_args())
                .arg(service_name)
                .exec()?;
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_on_all_pods() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("stop").unwrap();

    let opts = args::opts::Empty;
    proj.compose(&runner, "stop", &args::ActOn::All, &opts)
        .unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "railshello",
         "-f",
         proj.output_dir().join("pods").join("db.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "railshello",
         "-f",
         proj.output_dir().join("pods").join("frontend.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "railshello",
         "-f",
         proj.output_dir().join("pods").join("rake.yml"),
         "stop"]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn runs_docker_compose_on_named_pods_and_services() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("stop").unwrap();

    let act_on = args::ActOn::Named(vec!["db".to_owned(), "web".to_owned()]);
    let opts = args::opts::Empty;
    proj.compose(&runner, "stop", &act_on, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "railshello",
         "-f",
         proj.output_dir().join("pods").join("db.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "railshello",
         "-f",
         proj.output_dir().join("pods").join("frontend.yml"),
         "stop",
         "web"]
    });

    proj.remove_test_output().unwrap();
}
