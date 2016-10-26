//! Pass simple commands directly through to `docker-compose`.

use args;
use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use pod::Pod;
use project::{PodOrService, Project};

/// Pass simple commands directly through to `docker-compose`.
pub trait CommandCompose {
    /// Pass simple commands directly through to `docker-compose`.
    fn compose<CR>(&self,
                   runner: &CR,
                   command: &str,
                   act_on: &args::ActOn,
                   opts: &args::ToArgs)
                   -> Result<()>
        where CR: CommandRunner;

    /// Run a `docker-compose` command on a single pod.  If the pod is
    /// disabled, this does nothing.
    fn compose_pod<CR>(&self,
                       runner: &CR,
                       command: &str,
                       pod: &Pod,
                       opts: &args::ToArgs)
                       -> Result<()>
        where CR: CommandRunner;

    /// Run a `docker-compose` command on a single service.  If the pod is
    /// disabled, this does nothing.
    fn compose_service<CR>(&self,
                           runner: &CR,
                           command: &str,
                           pod: &Pod,
                           service_name: &str,
                           opts: &args::ToArgs)
                           -> Result<()>
        where CR: CommandRunner;
}

impl CommandCompose for Project {
    fn compose<CR>(&self,
                   runner: &CR,
                   command: &str,
                   act_on: &args::ActOn,
                   opts: &args::ToArgs)
                   -> Result<()>
        where CR: CommandRunner
    {
        for pod_or_service in act_on.pods_or_services(self) {
            match try!(pod_or_service) {
                PodOrService::Pod(pod) => {
                    try!(self.compose_pod(runner, command, pod, opts));
                }
                PodOrService::Service(pod, service_name) => {
                    try!(self.compose_service(runner,
                                              command,
                                              pod,
                                              service_name,
                                              opts));
                }
            }
        }

        Ok(())
    }

    fn compose_pod<CR>(&self,
                       runner: &CR,
                       command: &str,
                       pod: &Pod,
                       opts: &args::ToArgs)
                       -> Result<()>
        where CR: CommandRunner
    {
        let target = self.current_target();
        if pod.enabled_in(target) {
            try!(runner.build("docker-compose")
                .args(&try!(pod.compose_args(self, target)))
                .arg(command)
                .args(&opts.to_args())
                .exec());
        }
        Ok(())
    }

    fn compose_service<CR>(&self,
                           runner: &CR,
                           command: &str,
                           pod: &Pod,
                           service_name: &str,
                           opts: &args::ToArgs)
                           -> Result<()>
        where CR: CommandRunner
    {
        let target = self.current_target();
        if pod.enabled_in(target) {
            try!(runner.build("docker-compose")
                .args(&try!(pod.compose_args(self, target)))
                .arg(command)
                .args(&opts.to_args())
                .arg(service_name)
                .exec());
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let opts = args::opts::Empty;
    proj.compose(&runner, "stop", &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods").join("db.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods").join("frontend.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods").join("rake.yml"),
         "stop"]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn runs_docker_compose_on_named_pods_and_services() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let act_on = args::ActOn::Named(vec!("db".to_owned(), "web".to_owned()));
    let opts = args::opts::Empty;
    proj.compose(&runner, "stop", &act_on, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods").join("db.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods").join("frontend.yml"),
         "stop",
         "web"]
    });

    proj.remove_test_output().unwrap();
}
