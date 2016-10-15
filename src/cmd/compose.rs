//! Pass simple commands directly through to `docker-compose`.

use std::ops::Deref;

use args;
use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ovr::Override;
use project::{PodOrService, Project};

/// Pass simple commands directly through to `docker-compose`.
pub trait CommandCompose {
    /// Pass simple commands directly through to `docker-compose`.
    fn compose<CR>(&self,
                   runner: &CR,
                   ovr: &Override,
                   command: &str,
                   act_on: &args::ActOn,
                   opts: &args::ToArgs)
                   -> Result<()>
        where CR: CommandRunner;
}

impl CommandCompose for Project {
    fn compose<CR>(&self,
                   runner: &CR,
                   ovr: &Override,
                   command: &str,
                   act_on: &args::ActOn,
                   opts: &args::ToArgs)
                   -> Result<()>
        where CR: CommandRunner {

        let names = match *act_on {
            args::ActOn::Named(ref names) => names.to_owned(),
            args::ActOn::All => {
                self.pods().map(|p| p.name().to_owned()).collect()
            }
        };

        for name in names.deref() {
            match try!(self.pod_or_service_or_err(name)) {
                PodOrService::Pod(pod) => {
                    if pod.enabled_in(ovr) {
                        try!(runner.build("docker-compose")
                             .args(&try!(pod.compose_args(self, ovr)))
                             .arg(command)
                             .args(&opts.to_args())
                             .exec());
                    }
                }
                PodOrService::Service(pod, service_name) => {
                    if pod.enabled_in(ovr) {
                        try!(runner.build("docker-compose")
                             .args(&try!(pod.compose_args(self, ovr)))
                             .arg(command)
                             .args(&opts.to_args())
                             .arg(service_name)
                             .exec());
                    }
                }
            }
        }

        Ok(())
    }
}

#[test]
fn runs_docker_compose_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();

    let opts = args::opts::Empty;
    proj.compose(&runner, ovr, "stop", &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/db.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/migrate.yml"),
         "stop"]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn runs_docker_compose_on_named_pods_and_services() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();

    let act_on = args::ActOn::Named(vec!("db".to_owned(), "web".to_owned()));
    let opts = args::opts::Empty;
    proj.compose(&runner, ovr, "stop", &act_on, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/db.yml"),
         "stop"],
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "stop",
         "web"]
    });

    proj.remove_test_output().unwrap();
}
