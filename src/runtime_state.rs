//! Support for fetching runtime state directly from the Docker daemon.

use boondock;
use regex::Regex;
use std::collections::BTreeMap;
use std::net;

use errors::*;
use project::Project;

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
            .chain_err(|| ErrorKind::CouldNotGetRuntimeState)
    }

    /// The actual implementation of `for_project`.
    fn for_project_inner(project: &Project) -> Result<RuntimeState> {
        let name = project.compose_name();
        let target = project.current_target().name().to_owned();
        let docker = try!(boondock::Docker::connect_with_defaults());

        let mut services = BTreeMap::new();
        let containers = try!(docker.get_containers(true));
        for container in &containers {
            let info = try!(docker.get_container_info(&container));
            let labels = &info.Config.Labels;
            if labels.get("com.docker.compose.project") == Some(&name) &&
               labels.get("io.fdy.cage.target") == Some(&target) {
                if let Some(service) = labels.get("com.docker.compose.service") {
                    let our_info = try!(ContainerInfo::new(&info));
                    services.entry(service.to_owned())
                        .or_insert_with(Vec::new)
                        .push(our_info);
                }
            }
        }
        Ok(RuntimeState { services: services })
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

    /// The current state of this container.
    state: ContainerState,

    /// An IP address at which we can access this container.
    ip_addr: Option<net::IpAddr>,

    /// The TCP ports this container is listening on (not the corresponding
    /// host ports!).
    container_tcp_ports: Vec<u16>,
}

impl ContainerInfo {
    /// Construct our summary from the raw data returned by Docker.
    fn new(info: &boondock::container::ContainerInfo) -> Result<ContainerInfo> {
        // Get an IP address for this running container.
        let raw_ip_addr = &info.NetworkSettings.IPAddress[..];
        let ip_addr = if raw_ip_addr != "" {
            Some(try!(raw_ip_addr.parse()
                .chain_err(|| ErrorKind::parse("IP address", raw_ip_addr))))
        } else {
            None
        };

        // Get the listening network ports.
        let mut ports = vec![];
        for port_str in info.NetworkSettings.Ports.keys() {
            lazy_static! {
                static ref TCP_PORT: Regex = Regex::new(r#"^(\d+)/tcp$"#).unwrap();
            }
            if let Some(caps) = TCP_PORT.captures(port_str) {
                let port = try!(caps.at(1)
                    .unwrap()
                    .parse()
                    .chain_err(|| ErrorKind::parse("TCP port", port_str.clone())));
                ports.push(port);
            }
        }

        Ok(ContainerInfo {
            name: info.Name.to_owned(),
            state: ContainerState::new(&info.State),
            ip_addr: ip_addr,
            container_tcp_ports: ports,
        })
    }

    /// The current state of this container.
    pub fn state(&self) -> ContainerState {
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
            .unwrap_or_else(Vec::new)
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
#[derive(Debug, Clone, Copy)]
pub enum ContainerState {
    /// The container is current running.
    Running,
    /// The container is stopped with exit code 0.
    Done,
    /// The container is stopped with a non-zero exit code.
    Error(i64),
    /// The container is in some other state.
    Other,
}

impl ContainerState {
    /// Create a new `ContainerState` from Docker data.
    fn new(state: &boondock::container::State) -> ContainerState {
        if state.Running {
            ContainerState::Running
        } else if state.Dead && state.ExitCode == 0 {
            ContainerState::Done
        } else if state.Dead {
            ContainerState::Error(state.ExitCode)
        } else {
            ContainerState::Other
        }
    }
}
