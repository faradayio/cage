//! Extension methods for `compose_yml::v2::PortMapping`.

use compose_yml::v2 as dc;

/// These methods will appear as regular methods on `PortMapping` in any
/// module which includes `PortMappingExt`.
pub trait PortMappingExt {
    /// Return a string representation of the host portion of this port
    /// mapping, if any.  For now, we have no support for mappings assigned
    /// by Docker at runtime.
    fn host_string(&self) -> Option<String>;
}

impl PortMappingExt for dc::PortMapping {
    fn host_string(&self) -> Option<String> {
        match (self.host_address, self.host_ports) {
            (Some(ref addr), Some(ref ports)) => Some(format!("{}:{}", addr, ports)),
            (None, Some(ref ports)) => Some(format!("{}", ports)),
            _ => None,
        }
    }
}
