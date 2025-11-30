//! The `up` command.

use std::thread;
use std::time;

use crate::args;
use crate::cmd::{CommandCompose, CommandRun};
use crate::command_runner::CommandRunner;
#[cfg(test)]
use crate::command_runner::TestCommandRunner;
use crate::errors::*;
use crate::pod::{Pod, PodType};
use crate::project::{PodOrService, Project};
use crate::runtime_state::RuntimeState;

/// We implement `up` with a trait so we put it in its own module.
pub trait CommandUp {
    /// Up all the images in the specified pods.
    fn up<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Up,
    ) -> Result<()>
    where
        CR: CommandRunner;

    /// Run the initialization functions for the specified pod.
    fn init_pod<CR>(&self, runner: &CR, pod: &Pod) -> Result<()>
    where
        CR: CommandRunner;
}

impl CommandUp for Project {
    fn up<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Up,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        let pods_or_services = act_on
            .pods_or_services(self)
            // TODO LOW: Refactor this into a `filter_result` helper?
            .filter(|v| match *v {
                Ok(ref p_s) => p_s.pod_type() != PodType::Task,
                Err(_) => true,
            });
        for pod_or_service in pods_or_services {
            match pod_or_service? {
                PodOrService::Pod(pod) => {
                    self.compose_pod(runner, "up", pod, opts)?;
                    if opts.init {
                        self.init_pod(runner, pod)?;
                    }
                }
                PodOrService::Service(pod, service_name) => {
                    self.compose_service(runner, "up", pod, service_name, opts)?;
                }
            }
        }
        Ok(())
    }

    fn init_pod<CR>(&self, runner: &CR, pod: &Pod) -> Result<()>
    where
        CR: CommandRunner,
    {
        // Skip initialization for this pod if there's nothing to do.
        if pod.run_on_init().is_empty() {
            return Ok(());
        }

        // Wait for the pod's ports to be open.
        println!(
            "Waiting for pod '{}' to be listening on all ports",
            pod.name()
        );
        loop {
            let state: RuntimeState = RuntimeState::for_project(self)?;
            let listening = pod.service_names().iter().all(|service_name| {
                debug!("scanning service '{}'", service_name);
                let containers = state.service_containers(service_name);
                if containers.is_empty() {
                    // No containers visible yet; give Docker time.
                    debug!("no containers for service '{}' yet", service_name);
                    false
                } else {
                    // If we have at least one container, scan it.
                    containers
                        .iter()
                        .all(|container| container.is_listening_to_ports())
                }
            });
            if listening {
                break;
            }
            thread::sleep(time::Duration::from_millis(250));
        }

        // Run our initialization commands.
        println!("Initializing pod '{}'", pod.name());
        for cmd in pod.run_on_init() {
            if cmd.is_empty() {
                return Err(anyhow::anyhow!(
                    "all `run_on_init` items for '{}' must have at least one value",
                    pod.name()
                ));
            }
            let service = &cmd[0];
            let cmd = if cmd.len() >= 2 {
                Some(args::Command::new(&cmd[1]).with_args(&cmd[2..]))
            } else {
                None
            };
            let opts = args::opts::Run::default();
            self.run(runner, service, cmd.as_ref(), &opts)?;
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_up_honors_enable_in_targets() {
    let _ = env_logger::try_init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    proj.set_current_target_name("production").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("up").unwrap();

    let opts = args::opts::Up::default();
    proj.up(&runner, &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        [
            "docker-compose",
            "-p",
            "railshello",
            "-f",
            proj.output_dir().join("pods").join("frontend.yml"),
            "up",
            "-d",
        ]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn runs_docker_compose_up_with_quiet_mode() {
    let _ = env_logger::try_init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    proj.set_quiet(true);
    proj.set_current_target_name("production").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("up").unwrap();

    let opts = args::opts::Up::default();
    proj.up(&runner, &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        [
            "docker-compose",
            "-p",
            "railshello",
            "-f",
            proj.output_dir().join("pods").join("frontend.yml"),
            "--progress",
            "quiet",
            "up",
            "-d",
        ]
    });

    proj.remove_test_output().unwrap();
}
