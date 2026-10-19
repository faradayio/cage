//! Plugin which shifts host ports and publishes them as environment variables.
//!
//! This runs for `Operation::Output` only. `cage export` keeps the ports
//! written in the pod files.

use faraday_compose_yml::v2 as dc;
use std::marker::PhantomData;

use crate::errors::*;
use crate::plugins::{self, Operation, PluginNew, PluginTransform};
use crate::project::Project;

/// Shifts host ports and injects `CAGE_HOST_PORT_*` into every service.
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
        "host_ports"
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
        ctx: &plugins::Context<'_>,
        file: &mut dc::File,
    ) -> Result<()> {
        if op != Operation::Output {
            return Ok(());
        }

        let shift = *ctx.project.host_port_shift();
        for (service_name, service) in &mut file.services {
            for port in &mut service.ports {
                shift.shift(port.value_mut()?).map_err(|err| {
                    err.context(format!(
                        "service '{}/{}'",
                        ctx.pod.name(),
                        service_name
                    ))
                })?;
            }
        }

        let environment = shift.environment(ctx.project)?;
        for service in file.services.values_mut() {
            for (name, value) in &environment {
                service
                    .environment
                    .insert(name.to_owned(), dc::escape(value)?);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{self, Operation, PluginNew};
    use crate::project::Project;

    #[test]
    fn output_shifts_host_ports_and_export_does_not() {
        let proj = Project::from_example("hello").unwrap();
        proj.output("up").unwrap();

        let file = dc::File::read_from_path(
            proj.output_dir().join("pods").join("frontend.yml"),
        )
        .unwrap();
        let proxy = file.services.get("proxy").unwrap();
        assert_eq!(
            proxy.ports[0].value().unwrap().host_ports,
            Some(dc::Ports::Port(17180))
        );
        assert_eq!(
            proxy.ports[0].value().unwrap().container_ports,
            dc::Ports::Port(80)
        );
        let web = file.services.get("web").unwrap();
        assert!(web.ports[0].value().unwrap().host_ports.is_none());
        assert_eq!(
            proxy
                .environment
                .get("CAGE_HOST_PORT_OFFSET")
                .unwrap()
                .value()
                .unwrap(),
            "9100"
        );
        assert_eq!(
            web.environment
                .get("CAGE_HOST_PORT_FRONTEND_PROXY_80")
                .unwrap()
                .value()
                .unwrap(),
            "17180"
        );

        let export_dir = proj.output_dir().join("hello_export");
        proj.export(&export_dir).unwrap();
        let exported =
            dc::File::read_from_path(export_dir.join("frontend.yml")).unwrap();
        let exported_proxy = exported.services.get("proxy").unwrap();
        assert_eq!(
            exported_proxy.ports[0].value().unwrap().host_ports,
            Some(dc::Ports::Port(8080))
        );
        assert!(!exported_proxy
            .environment
            .contains_key("CAGE_HOST_PORT_OFFSET"));

        proj.remove_test_output().unwrap();
    }

    #[test]
    fn a_shifted_port_past_65535_names_the_service() {
        let mut proj = Project::from_example("hello").unwrap();
        proj.set_host_port_offset(100);
        let plugin = Plugin::new(&proj).unwrap();
        let frontend = proj.pod("frontend").unwrap();
        let ctx = plugins::Context::new(&proj, frontend, "up");
        let mut file = frontend.merged_file(proj.current_target()).unwrap();
        let proxy = file.services.get_mut("proxy").unwrap();
        proxy.ports[0] = dc::value(dc::PortMapping::new(65500, 80));
        let err = plugin
            .transform(Operation::Output, &ctx, &mut file)
            .unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("frontend/proxy"), "{message}");
        assert!(message.contains("65500"), "{message}");
    }
}
