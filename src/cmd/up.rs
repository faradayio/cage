//! The `up` command.

use std::thread;
use std::time;

use faraday_compose_yml::v2 as dc;

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
        let mut started = Vec::new();
        for pod_or_service in pods_or_services {
            let pod_or_service = pod_or_service?;
            // `compose_*` returns Ok without starting a pod this target disables.
            let enabled = match &pod_or_service {
                PodOrService::Pod(pod) | PodOrService::Service(pod, _) => {
                    pod.enabled_in(self.current_target())
                }
            };
            match &pod_or_service {
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
            if enabled {
                started.push(pod_or_service);
            }
        }
        self.print_host_ports(&started)?;
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

        println!("Waiting for pod '{}' to be running", pod.name());
        loop {
            let state: RuntimeState = RuntimeState::for_project(self)?;
            if state.all_services_in_pod_are_running(pod) {
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

impl Project {
    /// Print one line per service that binds a host port.
    fn print_host_ports(&self, started: &[PodOrService<'_>]) -> Result<()> {
        let mut lines = Vec::new();
        for pod_or_service in started {
            lines.extend(self.host_port_lines(pod_or_service)?);
        }
        if lines.is_empty() {
            return Ok(());
        }
        println!();
        for line in &lines {
            println!("{}", line);
        }
        Ok(())
    }

    fn host_port_lines(
        &self,
        pod_or_service: &PodOrService<'_>,
    ) -> Result<Vec<String>> {
        match *pod_or_service {
            PodOrService::Pod(pod) => {
                let file = pod.merged_file(self.current_target())?;
                let mut lines = Vec::new();
                for (service_name, service) in &file.services {
                    if let Some(line) =
                        self.service_host_port_line(pod.name(), service_name, service)?
                    {
                        lines.push(line);
                    }
                }
                Ok(lines)
            }
            PodOrService::Service(pod, service_name) => {
                let service =
                    pod.service_or_err(self.current_target(), service_name)?;
                match self.service_host_port_line(
                    pod.name(),
                    service_name,
                    &service,
                )? {
                    Some(line) => Ok(vec![line]),
                    None => Ok(Vec::new()),
                }
            }
        }
    }

    fn service_host_port_line(
        &self,
        pod_name: &str,
        service_name: &str,
        service: &dc::Service,
    ) -> Result<Option<String>> {
        let mut bindings = Vec::new();
        for port in &service.ports {
            let binding =
                self.host_port_shift()
                    .host_binding(port.value()?)
                    .map_err(|err| {
                        err.context(format!("service '{}/{}'", pod_name, service_name))
                    })?;
            if let Some(binding) = binding {
                bindings.push(binding);
            }
        }
        if bindings.is_empty() {
            Ok(None)
        } else {
            Ok(Some(format!(
                "{}/{}  {}",
                pod_name,
                service_name,
                bindings.join(", ")
            )))
        }
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
fn up_lists_shifted_host_ports() {
    let proj = Project::from_example("hello").unwrap();
    let pod = proj.pod("frontend").unwrap();
    let lines = proj.host_port_lines(&PodOrService::Pod(pod)).unwrap();
    assert_eq!(
        lines,
        vec!["frontend/proxy  localhost:17180 -> 80".to_owned()]
    );
}
