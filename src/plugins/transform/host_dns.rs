//! Plugin which sets up "host.docker.internal" on Linux.
//!
//! This allows containers to talk to the host. It's set up automatically on
//! MacOS, but not yet on Linux.

use compose_yml::v2 as dc;
use std::{
    marker::PhantomData,
    net::IpAddr,
    process::{Command, Stdio},
};

use crate::errors::*;
use crate::plugins::{self, Operation, PluginNew, PluginTransform};
use crate::project::Project;

/// Adds `extra_hosts` with `"host.docker.internal"` on Linux.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Plugin {
    /// Placeholder field for future hidden fields, to keep this from being
    /// directly constructable.
    _placeholder: PhantomData<()>,
}

impl plugins::Plugin for Plugin {
    fn name(&self) -> &'static str {
        Self::plugin_name()
    }
}

impl PluginNew for Plugin {
    fn plugin_name() -> &'static str {
        "host_dns"
    }

    fn new(_project: &Project) -> Result<Self> {
        Ok(Plugin {
            _placeholder: PhantomData,
        })
    }
}

impl PluginTransform for Plugin {
    fn transform(
        &self,
        op: Operation,
        _ctx: &plugins::Context<'_>,
        file: &mut dc::File,
    ) -> Result<()> {
        // Only do this on Linux, and in `Output` mode.
        if cfg!(target_os = "linux") && op == Operation::Output {
            // Look up the IP address associated with docker0. If we can't find it,
            // report very detailed warnings.
            let addr = match InterfaceInfo::find("docker0") {
                Ok(Some(iface)) => {
                    if let Some(addr) = iface.ipv4_address() {
                        addr
                    } else {
                        warn!("omitting host.docker.internal (interface docker0 has no IPv4 address)");
                        return Ok(());
                    }
                }
                Ok(None) => {
                    warn!("omitting host.docker.internal (could not find interface docker0)");
                    return Ok(());
                }
                Err(err) => {
                    warn!("omitting host.docker.internal ({})", err);
                    return Ok(());
                }
            };
            let addr = addr
                .parse::<IpAddr>()
                .chain_err(|| err!("invalid IP address {:?}", addr))?;
            trace!("mapping host.docker.internal to {}", addr);

            // Add an extra host to each service.
            for service in &mut file.services.values_mut() {
                service.extra_hosts.push(dc::value(dc::HostMapping::new(
                    "host.docker.internal",
                    &addr,
                )));
            }
        }
        Ok(())
    }
}

/// Information about a network interface.
#[derive(Clone, Debug, Deserialize)]
struct InterfaceInfo {
    /// The name of the interface, if any.
    ifname: Option<String>,

    /// The addresses asssociated with the interface.
    #[serde(default)]
    addr_info: Vec<AddressInfo>,
}

impl InterfaceInfo {
    /// Look up the interface with the specified name.
    fn find(ifname: &str) -> Result<Option<InterfaceInfo>> {
        // TODO: Should we use our `command_runner` for this so that we can mock
        // it during tests?
        let output = Command::new("ip")
            .args(&["-j", "address", "show", ifname])
            .stderr(Stdio::inherit())
            .output()
            .chain_err(|| "error running `ip address show`")?;
        if output.status.success() {
            let data = output.stdout;
            let interfaces = serde_json::from_slice::<Vec<InterfaceInfo>>(&data)
                .chain_err(|| "error parsing `ip address show` output")?;
            for i in &interfaces {
                if let Some(found_ifname) = &i.ifname {
                    if found_ifname == ifname {
                        return Ok(Some(i.to_owned()));
                    }
                }
            }
            Ok(None)
        } else {
            Err(err!("`ip address show` exited with {}", output.status))
        }
    }

    /// Get the first IPv4 address of this interface.
    fn ipv4_address(&self) -> Option<String> {
        for addr in &self.addr_info {
            if let Some(family) = &addr.family {
                if family == "inet" {
                    if let Some(local) = &addr.local {
                        return Some(local.to_owned());
                    }
                }
            }
        }
        None
    }
}

/// An address associated with a network interface.
#[derive(Clone, Debug, Deserialize)]
struct AddressInfo {
    /// The IP address family. Values include `"inet"` or `"inet6"`.
    family: Option<String>,

    /// The network address, as a string.
    local: Option<String>,
}
