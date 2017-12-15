//! The `run-script` command.

use args;
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use project::{PodOrService, Project};

/// Included into project in order to run named scripts on one ore more services
pub trait CommandRunScript {
    /// Run a named script on all matching services
    fn run_script<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        script_name: &str,
        opts: &args::opts::Run,
    ) -> Result<()>
    where
        CR: CommandRunner;
}

impl CommandRunScript for Project {
    fn run_script<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        script_name: &str,
        opts: &args::opts::Run,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        let target = self.current_target();

        for pod_or_service in act_on.pods_or_services(self) {
            match pod_or_service? {
                PodOrService::Pod(pod) => {
                    // Ignore any pods that aren't enabled in the current target
                    if pod.enabled_in(&target) {
                        for service_name in pod.service_names() {
                            pod.run_script(
                                runner,
                                &self,
                                &service_name,
                                &script_name,
                                &opts,
                            )?;
                        }
                    }
                }
                PodOrService::Service(pod, service_name) => {
                    // Don't run this on any service whose pod isn't enabled in
                    // the current target
                    if pod.enabled_in(&target) {
                        pod.run_script(
                            runner,
                            &self,
                            &service_name,
                            &script_name,
                            &opts,
                        )?;
                    }
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
    let opts = args::opts::Run::default();
    proj.output("run-script").unwrap();

    proj.run_script(&runner, &args::ActOn::All, "routes", &opts)
        .unwrap();
    assert_ran!(runner, {
        [
            "docker-compose",
            "-p",
            "railshello",
            "-f",
            proj.output_dir().join("pods").join("rake.yml"),
            "run",
            "rake",
            "rake",
            "routes"
        ]
    });

    proj.remove_test_output().unwrap();
}
