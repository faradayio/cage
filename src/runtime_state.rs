//! Support for fetching runtime state directly from the Docker daemon.

use regex::Regex;
use std::collections::BTreeMap;
use std::net;
use tokio::runtime;

use crate::errors::*;
use crate::pod::Pod;
use crate::project::Project;

/// Everything we know about the running application, based on querying Docker.
#[derive(Debug)]
pub struct RuntimeState {
    /// Map service names to information about associated containers.
    services: BTreeMap<String, Vec<ContainerInfo>>,
}

impl RuntimeState {
    /// Look up the runtime state for the specified project (and its
    /// current target).
    pub fn for_project(project: &Project) -> Result<RuntimeState> {
        // Standardize our error messages since this is going to fail a lot
        // until we debug all the Docker wire formats and undocumented
        // special cases.
        Self::for_project_inner(project)
            .map_err(|e| e.context(Error::CouldNotGetRuntimeState))
    }

    /// The actual implementation of `for_project`.
    fn for_project_inner(project: &Project) -> Result<RuntimeState> {
        debug!("Querying Docker for running containers");

        // Start a local `tokio` runtime for async calls.
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let name = project.compose_name();
        let target = project.current_target().name().to_owned();
        let docker = bollard::Docker::connect_with_local_defaults()
            .map_err(|e| anyhow::anyhow!("failed to connect to Docker: {}", e))?;

        let mut services = BTreeMap::new();
        use bollard::query_parameters::ListContainersOptionsBuilder;
        let opts = ListContainersOptionsBuilder::default().all(true).build();
        let containers = rt
            .block_on(docker.list_containers(Some(opts)))
            .map_err(|e| anyhow::anyhow!("failed to list Docker containers: {}", e))?;
        for container in &containers {
            let container_id = container
                .id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("container missing id"))?;
            let info = rt
                .block_on(docker.inspect_container(
                    container_id,
                    None::<bollard::query_parameters::InspectContainerOptions>,
                ))
                .map_err(|e| {
                    anyhow::anyhow!(
                        "error looking up container {:?}: {}",
                        container_id,
                        e
                    )
                })?;
            let labels = &info
                .config
                .as_ref()
                .and_then(|c| c.labels.as_ref())
                .ok_or_else(|| {
                    anyhow::anyhow!("container missing config or labels")
                })?;
            if labels.get("com.docker.compose.project").map(|s| s.as_str())
                == Some(&name)
                && labels.get("io.fdy.cage.target").map(|s| s.as_str())
                    == Some(&target)
            {
                if let Some(service) = labels.get("com.docker.compose.service") {
                    let our_info = ContainerInfo::new(&info)?;
                    services
                        .entry(service.to_owned())
                        .or_insert_with(Vec::new)
                        .push(our_info);
                }
            }
        }
        Ok(RuntimeState { services })
    }

    /// Is the specified pod running?
    pub fn all_services_in_pod_are_running(&self, pod: &Pod) -> bool {
        for service_name in pod.service_names() {
            let containers = self
                .service_containers(service_name)
                .iter()
                .filter(|c| !c.is_one_off())
                .collect::<Vec<_>>();
            if containers.is_empty() {
                // No containers are associated with this service.
                return false;
            }
            if containers
                .iter()
                .any(|c| c.state() != ContainerStatus::Running)
            {
                // We have at least one container which isn't running.
                return false;
            }
        }
        true
    }

    /// Get the containers associated with a service.  This will return the
    /// empty list if it can't find any containers related to the specified
    /// `service_name`.
    pub fn service_containers(&self, service_name: &str) -> &[ContainerInfo] {
        self.services
            .get(service_name)
            .map_or_else(|| &[] as &[ContainerInfo], |containers| &containers[..])
    }
}

/// Information about a specific container associated with a service.
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    /// The name of this container.
    name: String,

    /// Was this a one-off container?
    is_one_off: bool,

    /// The current state of this container.
    state: ContainerStatus,

    /// An IP address at which we can access this container.
    ip_addr: Option<net::IpAddr>,

    /// The TCP ports this container is listening on (not the corresponding
    /// host ports!).
    container_tcp_ports: Vec<u16>,
}

impl ContainerInfo {
    /// Construct our summary from the raw data returned by Docker.
    fn new(info: &bollard::models::ContainerInspectResponse) -> Result<ContainerInfo> {
        // Was this a one-off container?
        let one_off_label = info
            .config
            .as_ref()
            .and_then(|c| c.labels.as_ref())
            .and_then(|labels| labels.get("com.docker.compose.oneoff"));
        let is_one_off = one_off_label.map(|s| s.as_str()) == Some("True");

        // Get an IP address for this running container.
        let raw_ip_addr = info
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ip_address.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("");
        let ip_addr = if !raw_ip_addr.is_empty() {
            Some(raw_ip_addr.parse().map_err(|e| {
                anyhow::Error::new(e).context(Error::parse("IP address", raw_ip_addr))
            })?)
        } else {
            None
        };

        // Get the listening network ports.
        let mut ports = vec![];
        if let Some(network_settings) = &info.network_settings {
            if let Some(port_map) = &network_settings.ports {
                for port_str in port_map.keys() {
                    lazy_static! {
                        static ref TCP_PORT: Regex =
                            Regex::new(r#"^(\d+)/tcp$"#).unwrap();
                    }
                    if let Some(caps) = TCP_PORT.captures(port_str) {
                        let port =
                            caps.get(1).unwrap().as_str().parse().map_err(|e| {
                                anyhow::Error::new(e).context(Error::parse(
                                    "TCP port",
                                    port_str.clone(),
                                ))
                            })?;
                        ports.push(port);
                    }
                }
            }
        }

        let name = info
            .name
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("container missing name"))?
            .to_owned();

        let state = info
            .state
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("container missing state"))?;

        Ok(ContainerInfo {
            name,
            is_one_off,
            state: ContainerStatus::new(state),
            ip_addr,
            container_tcp_ports: ports,
        })
    }

    /// Is this a one-off container created by `docker-compose run`?
    pub fn is_one_off(&self) -> bool {
        self.is_one_off
    }

    /// The current state of this container.
    pub fn state(&self) -> ContainerStatus {
        self.state
    }

    /// An IP address at which we can access this container.
    pub fn ip_addr(&self) -> Option<net::IpAddr> {
        self.ip_addr
    }

    /// The TCP ports this container is listening on (not the corresponding
    /// host ports!).
    pub fn container_tcp_ports(&self) -> &[u16] {
        &self.container_tcp_ports
    }

    /// The socket addresses this container is listening on.
    pub fn socket_addrs(&self) -> Vec<net::SocketAddr> {
        self.ip_addr()
            .map(|addr| {
                self.container_tcp_ports
                    .iter()
                    .map(|port| net::SocketAddr::new(addr, *port))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Is this container listening to its ports?
    pub fn is_listening_to_ports(&self) -> bool {
        debug!("scanning container '{}'", &self.name);
        for addr in self.socket_addrs() {
            trace!("scanning container '{}' at {}", &self.name, addr);
            if net::TcpListener::bind(addr).is_err() {
                debug!("container '{}': {} is CLOSED", &self.name, addr);
                return false;
            }
        }
        debug!("container '{}' is listening on all ports", &self.name);
        true
    }
}

/// Is a Docker container running? Stopped?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStatus {
    /// The container has been created.
    Created,
    /// The container is currently restarting.
    Restarting,
    /// The container is current running.
    Running,
    /// The container has been paused.
    Paused,
    /// The container is stopped with exit code 0.
    Done,
    /// The container is stopped with a non-zero exit code.
    Exited(i64),
    /// The container is in some other state.
    Other,
}

impl ContainerStatus {
    /// Create a new `ContainerStatus` from Docker data.
    fn new(state: &bollard::models::ContainerState) -> ContainerStatus {
        use bollard::models::ContainerStateStatusEnum;

        let status = state.status.as_ref();
        let exit_code = state.exit_code.unwrap_or(0);

        match status {
            Some(ContainerStateStatusEnum::CREATED) => ContainerStatus::Created,
            Some(ContainerStateStatusEnum::RESTARTING) => ContainerStatus::Restarting,
            Some(ContainerStateStatusEnum::RUNNING) => ContainerStatus::Running,
            Some(ContainerStateStatusEnum::PAUSED) => ContainerStatus::Paused,
            Some(ContainerStateStatusEnum::EXITED) if exit_code == 0 => {
                ContainerStatus::Done
            }
            Some(ContainerStateStatusEnum::EXITED) => {
                ContainerStatus::Exited(exit_code)
            }
            _ => ContainerStatus::Other,
        }
    }
}
