//! The `status` command.

use colored::*;
use compose_yml::v2 as dc;

use crate::args;
use crate::command_runner::CommandRunner;
use crate::errors::*;
use crate::ext::port_mapping::PortMappingExt;
use crate::ext::service::ServiceExt;
use crate::pod::Pod;
use crate::project::{PodOrService, Project};
use crate::runtime_state::{ContainerStatus, RuntimeState};
use crate::sources::Source;

/// We implement `status` with a trait so we can put it in its own
/// module.
pub trait CommandStatus {
    /// Get the current status of the project.  This is eventually intended
    /// to include a fair bit of detail.
    fn status<CR>(&self, runner: &CR, act_on: &args::ActOn) -> Result<()>
    where
        CR: CommandRunner;
}

impl CommandStatus for Project {
    fn status<CR>(&self, _runner: &CR, act_on: &args::ActOn) -> Result<()>
    where
        CR: CommandRunner,
    {
        let state = RuntimeState::for_project(self)?;
        for pod_or_service in act_on.pods_or_services(self) {
            match pod_or_service? {
                PodOrService::Pod(pod) => self.pod_status(&state, pod)?,
                PodOrService::Service(pod, service_name) => {
                    self.pod_header(pod)?;
                    let service =
                        pod.service_or_err(self.current_target(), service_name)?;
                    self.service_status(&state, pod, service_name, &service, true)?;
                }
            }
        }
        Ok(())
    }
}

impl Project {
    /// Display information about a pod, but not any of its services.
    fn pod_header(&self, pod: &Pod) -> Result<()> {
        let enabled = if pod.enabled_in(self.current_target()) {
            "enabled".normal()
        } else {
            "disabled".red().bold()
        };
        println!(
            "{:15} {} type:{}",
            pod.name().blue().bold(),
            enabled,
            pod.pod_type()
        );
        Ok(())
    }

    /// Display information about a pod and its services.
    fn pod_status(&self, state: &RuntimeState, pod: &Pod) -> Result<()> {
        self.pod_header(pod)?;
        let file = pod.merged_file(self.current_target())?;
        for (i, (service_name, service)) in file.services.iter().enumerate() {
            self.service_status(
                state,
                pod,
                service_name,
                service,
                i + 1 == file.services.len(),
            )?;
        }
        Ok(())
    }

    /// Display information about a service.
    fn service_status(
        &self,
        state: &RuntimeState,
        _pod: &Pod,
        service_name: &str,
        service: &dc::Service,
        last: bool,
    ) -> Result<()> {
        if last {
            print!("└─ {:12}", service_name.blue().bold());
        } else {
            print!("├─ {:12}", service_name.blue().bold());
        }

        // Print out our runtime status.
        for container in state.service_containers(service_name) {
            let text = match container.state() {
                ContainerStatus::Running => "RUNNING".green().bold(),
                ContainerStatus::Done => "DONE".green(),
                ContainerStatus::Exited(_) => "EXITED".red().bold(),
                _ => "OTHER".yellow(),
            };
            print!(" {}", text);
        }

        // Print out ports with known host bindings.
        let ports: Vec<String> = service
            .ports
            .iter()
            .map(|port| Ok(port.value()?.host_string()))
            .filter_map(|result| match result {
                Ok(Some(val)) => Some(Ok(val)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            })
            .collect::<Result<_>>()?;
        if !ports.is_empty() {
            print!(" ports:{}", ports.join(","));
        }

        // Print out mounted source code.
        let sources: Vec<&Source> = service
            .sources(self.sources())?
            .map(|source_mount| Ok(source_mount.source))
            .collect::<Result<_>>()?;
        let sources_dirs = self.sources_dirs();
        let source_names: Vec<&str> = sources
            .iter()
            .filter(|s| s.is_available_locally(&sources_dirs) && s.mounted())
            .map(|s| s.alias())
            .collect();
        if !source_names.is_empty() {
            print!(" mounted:{}", source_names.join(","));
        }

        println!();
        Ok(())
    }
}
