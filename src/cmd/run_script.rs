//! The `run-script` command.

use std::collections::BTreeMap;

use args;
use cmd::CommandCompose;
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use project::{Project, PodOrService};

pub trait CommandRunScript {
    /// Run a named script on all matching services
    fn run_script<CR>(&self, runner: &CR, act_on: &args::ActOn, script_name: &str) -> Result<()>
        where CR: CommandRunner;
}

impl CommandRunScript for Project {
    fn run_script<CR>(&self, runner: &CR, act_on: &args::ActOn, script_name: &str) -> Result<()>
        where CR: CommandRunner
    {
        let opts = args::opts::Empty;

        for pod_or_service in act_on.pods_or_services(self) {
            match pod_or_service? {
                PodOrService::Pod(pod) => {
                    for service_name in pod.service_names() {
                        pod.run_script(runner, &self, &service_name, &script_name)?;
                    }
                }
                PodOrService::Service(pod, service_name) => {
                    pod.run_script(runner, &self, &service_name, &script_name)?;
                }
            }
        }
        Ok(())
    }
}

#[test]
fn runs_scripts_on_all_services() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    proj.run_script(&runner, &args::ActOn::All, "routes").unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "railshello",
         "-f",
         proj.output_dir().join("pods").join("rake.yml"),
         "run",
         "rake",
         "rake",
         "routes"]
    });

    proj.remove_test_output().unwrap();
}
