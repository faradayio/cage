//! Pass simple commands directly through to `docker-compose`.

use std::ops::Deref;

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
    fn compose<CR, F>(&self,
                      runner: &CR,
                      command: &str,
                      act_on: &args::ActOn,
                      matching: F,
                      opts: &args::ToArgs)
                      -> Result<()>
        where CR: CommandRunner,
              F: Fn(&Pod) -> bool;
}

impl CommandCompose for Project {
    fn compose<CR, F>(&self,
                      runner: &CR,
                      command: &str,
                      act_on: &args::ActOn,
                      matching: F,
                      opts: &args::ToArgs)
                      -> Result<()>
        where CR: CommandRunner,
              F: Fn(&Pod) -> bool
    {

        let names = match *act_on {
            args::ActOn::Named(ref names) => names.to_owned(),
            args::ActOn::All => {
                let mut pods: Vec<_> = self.pods().collect();
                // Sort so that placeholders come before other pod types,
                // which is important for the `up` command.
                pods.sort_by_key(|p| (p.pod_type(), p.name()));
                pods.iter().map(|p| p.name().to_owned()).collect()
            }
        };

        for name in names.deref() {
            let target = self.current_target();
            match try!(self.pod_or_service_or_err(name)) {
                PodOrService::Pod(pod) => {
                    if pod.enabled_in(target) && matching(pod) {
                        try!(runner.build("docker-compose")
                            .args(&try!(pod.compose_args(self, target)))
                            .arg(command)
                            .args(&opts.to_args())
                            .exec());
                    }
                }
                PodOrService::Service(pod, service_name) => {
                    if pod.enabled_in(target) && matching(pod) {
                        try!(runner.build("docker-compose")
                            .args(&try!(pod.compose_args(self, target)))
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
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let opts = args::opts::Empty;
    proj.compose(&runner, "stop", &args::ActOn::All, |_| true, &opts).unwrap();
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
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let act_on = args::ActOn::Named(vec!("db".to_owned(), "web".to_owned()));
    let opts = args::opts::Empty;
    proj.compose(&runner, "stop", &act_on, |_| true, &opts).unwrap();
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
