//! The `status` command.

use colored::*;
use compose_yml::v2 as dc;

use args;
use command_runner::CommandRunner;
use errors::*;
use ext::port_mapping::PortMappingExt;
use ext::service::ServiceExt;
use pod::Pod;
use project::{PodOrService, Project};
use sources::Source;

/// We implement `status` with a trait so we can put it in its own
/// module.
pub trait CommandStatus {
    /// Get the current status of the project.  This is eventually intended
    /// to include a fair bit of detail.
    fn status<CR>(&self, runner: &CR, act_on: &args::ActOn) -> Result<()>
        where CR: CommandRunner;
}

impl CommandStatus for Project {
    fn status<CR>(&self, _runner: &CR, act_on: &args::ActOn) -> Result<()>
        where CR: CommandRunner
    {
        for pod_or_service in act_on.pods_or_services(self) {
            match try!(pod_or_service) {
                PodOrService::Pod(pod) => try!(self.pod_status(pod)),
                PodOrService::Service(pod, service_name) => {
                    try!(self.pod_header(pod));
                    let service = try!(pod.service_or_err(self.current_target(),
                                                          service_name));
                    try!(self.service_status(pod, service_name, &service, true));
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
            "enabled".green().bold()
        } else {
            "disabled".red().bold()
        };
        println!("{} {} type:{}",
                 pod.name().blue().bold(),
                 enabled,
                 pod.pod_type());
        Ok(())
    }

    /// Display information about a pod and its services.
    fn pod_status(&self, pod: &Pod) -> Result<()> {
        try!(self.pod_header(pod));
        let file = try!(pod.merged_file(self.current_target()));
        for (i, (service_name, service)) in file.services.iter().enumerate() {
            try!(self.service_status(pod,
                                     service_name,
                                     service,
                                     i + 1 == file.services.len()));
        }
        Ok(())
    }

    /// Display information about a service.
    fn service_status(&self,
                      _pod: &Pod,
                      service_name: &str,
                      service: &dc::Service,
                      last: bool)
                      -> Result<()> {
        if last {
            print!("└─ {}", service_name.blue());
        } else {
            print!("├─ {}", service_name.blue());
        }

        // Print out ports with known host bindings.
        let ports: Vec<String> = try!(service.ports
            .iter()
            .map(|port| Ok(try!(port.value()).host_string()))
            .filter_map(|result| {
                match result {
                    Ok(Some(val)) => Some(Ok(val)),
                    Ok(None) => None,
                    Err(err) => Some(Err(err)),
                }
            })
            .collect::<Result<_>>());
        if !ports.is_empty() {
            print!(" ports:{}", ports.join(","));
        }

        // Print out mounted source code.
        let sources: Vec<&Source> = try!(try!(service.sources(self.sources()))
                .map(|source_result| {
                    let (_, source) = try!(source_result);
                    Ok(source)
                })
                .collect::<Result<_>>());
        let source_names: Vec<&str> = sources.iter()
            .filter(|s| s.is_available_locally(self) && s.mounted())
            .map(|s| s.alias())
            .collect();
        if !source_names.is_empty() {
            print!(" mounted:{}", source_names.join(","));
        }

        println!("");
        Ok(())
    }
}
