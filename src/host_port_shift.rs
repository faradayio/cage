//! The host-port offset for one checkout.
//!
//! Pod files say which ports a service exposes. Cage adds one offset, taken
//! from the project name, to every host port when it writes output files.
//! The same offset is used for every port in the checkout, so the ports keep
//! their spacing.

use faraday_compose_yml::v2 as dc;
use std::collections::BTreeMap;

use crate::errors::*;
use crate::project::Project;

/// How far to move every host port in this checkout.
#[derive(Debug, Clone, Copy)]
pub struct HostPortShift {
    /// Added to every host port. A multiple of 100, from 100 to 9900, unless
    /// `CAGE_HOST_PORT_OFFSET` set something else (including 0).
    offset: u16,
}

impl HostPortShift {
    /// An offset chosen by the caller, including 0.
    pub(crate) fn from_offset(offset: u16) -> HostPortShift {
        HostPortShift { offset }
    }

    /// The offset cage would assign to a project of this name.
    ///
    /// The hash is FNV-1a, so the result stays the same when Rust is
    /// upgraded. `DefaultHasher` does not.
    pub(crate) fn for_name(project_name: &str) -> HostPortShift {
        let hash = fnv1a(project_name.as_bytes());
        let bucket = (hash % 99) + 1;
        HostPortShift::from_offset(
            u16::try_from(bucket * 100).expect("host port offset is at most 9900"),
        )
    }

    /// The shift for `project`, or `override_offset` when that is set.
    pub fn for_project(
        project: &Project,
        override_offset: Option<u16>,
    ) -> Result<HostPortShift> {
        Ok(match override_offset {
            Some(offset) => HostPortShift::from_offset(offset),
            None => HostPortShift::for_name(project.name()),
        })
    }

    /// The number added to every host port.
    pub fn offset(&self) -> u16 {
        self.offset
    }

    /// Add this checkout's offset to `mapping`'s host port.
    ///
    /// Container ports stay put. A mapping with no host port stays put. A
    /// range moves at both ends. Returns an error when the shifted port
    /// would pass 65535.
    pub fn shift(&self, mapping: &mut dc::PortMapping) -> Result<()> {
        if let Some(host_ports) = mapping.host_ports {
            mapping.host_ports = Some(self.shift_ports(host_ports)?);
        }
        Ok(())
    }

    /// `localhost:3300 -> 3000` for a mapping that binds a host port.
    ///
    /// `None` when the mapping leaves the host port for Docker to pick.
    pub fn host_binding(&self, mapping: &dc::PortMapping) -> Result<Option<String>> {
        if mapping.host_ports.is_none() {
            return Ok(None);
        }
        let mut shifted = mapping.clone();
        self.shift(&mut shifted)?;
        let host_ports = match shifted.host_ports {
            Some(ports) => ports,
            None => return Ok(None),
        };
        let host = match shifted.host_address {
            Some(address) => format!("{}:{}", address, host_ports),
            None => format!("localhost:{}", host_ports),
        };
        let protocol = match shifted.protocol {
            dc::Protocol::Tcp => "",
            dc::Protocol::Udp => "/udp",
        };
        Ok(Some(format!(
            "{}{} -> {}{}",
            host, protocol, shifted.container_ports, protocol
        )))
    }

    /// Environment variables that tell every service the real host ports.
    ///
    /// `CAGE_HOST_PORT_OFFSET` is this checkout's offset.
    /// `CAGE_HOST_PORT_<POD>_<SERVICE>_<CONTAINER_PORT>` is one host port.
    /// The pod name is part of the variable because two pods can use the
    /// same service name. Names are uppercased, and any character that is
    /// not an ASCII letter or digit becomes `_`. Port ranges and mappings
    /// with no host port are left out.
    pub fn environment(&self, project: &Project) -> Result<BTreeMap<String, String>> {
        let mut environment = BTreeMap::new();
        environment
            .insert("CAGE_HOST_PORT_OFFSET".to_owned(), self.offset.to_string());
        for pod in project.pods() {
            let file = pod.merged_file(project.current_target())?;
            for (service_name, service) in &file.services {
                for port in &service.ports {
                    let mapping = port.value()?;
                    let dc::Ports::Port(container_port) = mapping.container_ports
                    else {
                        continue;
                    };
                    let Some(dc::Ports::Port(written_host_port)) = mapping.host_ports
                    else {
                        continue;
                    };
                    let host_port =
                        self.shift_port(written_host_port).map_err(|err| {
                            err.context(format!(
                                "service '{}/{}'",
                                pod.name(),
                                service_name
                            ))
                        })?;
                    environment.insert(
                        format!(
                            "CAGE_HOST_PORT_{}_{}_{}",
                            Self::env_name_part(pod.name()),
                            Self::env_name_part(service_name),
                            container_port,
                        ),
                        host_port.to_string(),
                    );
                }
            }
        }
        Ok(environment)
    }

    fn shift_ports(&self, ports: dc::Ports) -> Result<dc::Ports> {
        match ports {
            dc::Ports::Port(port) => Ok(dc::Ports::Port(self.shift_port(port)?)),
            dc::Ports::Range(first, last) => Ok(dc::Ports::Range(
                self.shift_port(first)?,
                self.shift_port(last)?,
            )),
        }
    }

    fn shift_port(&self, port: u16) -> Result<u16> {
        let shifted = u32::from(port) + u32::from(self.offset);
        u16::try_from(shifted).map_err(|_| {
            err!(
                "host port {} shifted by {} is {}, which is past 65535",
                port,
                self.offset,
                shifted
            )
        })
    }

    fn env_name_part(name: &str) -> String {
        let mut part = String::new();
        for c in name.chars() {
            if c.is_ascii_alphanumeric() {
                part.push(c.to_ascii_uppercase());
            } else {
                part.push('_');
            }
        }
        part
    }
}

/// FNV-1a, 64-bit.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn offset_is_deterministic_for_a_project_name() {
        let proj = Project::from_example("hello").unwrap();
        assert_eq!(proj.host_port_shift().offset(), 9100);
        assert_eq!(
            HostPortShift::for_project(&proj, None).unwrap().offset(),
            9100
        );
        assert_eq!(HostPortShift::for_name("hello").offset(), 9100);
        assert_eq!(HostPortShift::for_name("myapp").offset(), 3000);
        assert_eq!(HostPortShift::for_name("cage").offset(), 4700);
        assert_eq!(HostPortShift::for_name("cage-2").offset(), 2800);

        for name in ["hello", "myapp", "cage", "cage-2", ""] {
            let offset = HostPortShift::for_name(name).offset();
            assert_eq!(offset % 100, 0, "{name}");
            assert!((100..=9900).contains(&offset), "{name} -> {offset}");
        }
    }

    #[test]
    fn set_name_recomputes_the_offset_until_an_override_is_set() {
        let mut proj = Project::from_example("hello").unwrap();
        assert_eq!(proj.host_port_shift().offset(), 9100);
        proj.set_name("cage");
        assert_eq!(proj.host_port_shift().offset(), 4700);
        proj.set_host_port_offset(0);
        proj.set_name("hello");
        assert_eq!(proj.host_port_shift().offset(), 0);
        assert_eq!(
            HostPortShift::for_project(&proj, Some(2500))
                .unwrap()
                .offset(),
            2500
        );
    }

    #[test]
    fn ranges_shift_at_both_ends_and_container_ports_stay() {
        let shift = HostPortShift::from_offset(300);
        let mut mapping = dc::PortMapping::new(
            dc::Ports::Range(8080, 8089),
            dc::Ports::Range(3000, 3009),
        );
        shift.shift(&mut mapping).unwrap();
        assert_eq!(mapping.host_ports, Some(dc::Ports::Range(8380, 8389)));
        assert_eq!(mapping.container_ports, dc::Ports::Range(3000, 3009));

        let mut single = dc::PortMapping::new(3000, 3000);
        single.host_address = Some("127.0.0.1".parse().unwrap());
        shift.shift(&mut single).unwrap();
        assert_eq!(single.host_ports, Some(dc::Ports::Port(3300)));
        assert_eq!(single.container_ports, dc::Ports::Port(3000));
        assert_eq!(single.host_address, Some("127.0.0.1".parse().unwrap()));

        let mut unbound = dc::PortMapping::any_to(3000);
        shift.shift(&mut unbound).unwrap();
        assert_eq!(unbound.host_ports, None);
        assert_eq!(unbound.container_ports, dc::Ports::Port(3000));
    }

    #[test]
    fn a_port_past_65535_is_an_error_and_the_mapping_stays() {
        let shift = HostPortShift::from_offset(100);
        let mut mapping = dc::PortMapping::new(65500, 80);
        let before = mapping.clone();
        let err = shift.shift(&mut mapping).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("65500"), "{message}");
        assert!(message.contains("65600"), "{message}");
        assert_eq!(mapping, before);

        let mut range = dc::PortMapping::new(
            dc::Ports::Range(65530, 65535),
            dc::Ports::Range(1, 6),
        );
        assert!(shift.shift(&mut range).is_err());
    }

    #[test]
    fn environment_names_each_host_port_with_its_pod_and_service() {
        let proj = Project::from_example("hello").unwrap();
        let environment = proj.host_port_shift().environment(&proj).unwrap();
        assert_eq!(
            environment.get("CAGE_HOST_PORT_OFFSET").map(String::as_str),
            Some("9100")
        );
        assert_eq!(
            environment
                .get("CAGE_HOST_PORT_FRONTEND_PROXY_80")
                .map(String::as_str),
            Some("17180")
        );
        assert!(!environment.contains_key("CAGE_HOST_PORT_FRONTEND_WEB_3000"));
        assert_eq!(HostPortShift::env_name_part("my-web.1"), "MY_WEB_1");
    }

    #[test]
    fn host_binding_shows_the_shifted_host_port() {
        let shift = HostPortShift::from_offset(300);
        let mapping = dc::PortMapping::new(3000, 3000);
        assert_eq!(
            shift.host_binding(&mapping).unwrap().as_deref(),
            Some("localhost:3300 -> 3000")
        );

        let range = dc::PortMapping::new(
            dc::Ports::Range(3000, 3009),
            dc::Ports::Range(4000, 4009),
        );
        assert_eq!(
            shift.host_binding(&range).unwrap().as_deref(),
            Some("localhost:3300-3309 -> 4000-4009")
        );

        assert_eq!(
            shift.host_binding(&dc::PortMapping::any_to(3000)).unwrap(),
            None
        );

        let mut addressed = dc::PortMapping::new(3000, 3000);
        addressed.host_address = Some("127.0.0.1".parse().unwrap());
        assert_eq!(
            shift.host_binding(&addressed).unwrap().as_deref(),
            Some("127.0.0.1:3300 -> 3000")
        );

        let mut udp = dc::PortMapping::new(3000, 3000);
        udp.protocol = dc::Protocol::Udp;
        assert_eq!(
            shift.host_binding(&udp).unwrap().as_deref(),
            Some("localhost:3300/udp -> 3000/udp")
        );
    }
}
