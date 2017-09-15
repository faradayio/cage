//! Plugin which loads secrets from `config/secrets.yml` and adds them to a
//! project.

use compose_yml::v2 as dc;
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::result;

use errors::*;
use plugins;
use plugins::{Operation, PluginGenerate, PluginNew, PluginTransform};
use project::Project;
#[cfg(test)]
use subcommand::Subcommand;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_helpers::load_yaml;

#[cfg(feature = "serde_derive")]
include!(concat!("secrets_config.in.rs"));
#[cfg(feature = "serde_codegen")]
include!(concat!(env!("OUT_DIR"), "/plugins/transform/secrets_config.rs"));

/// Loads a `config/secrets.yml` file and merges in into a project.
#[derive(Debug)]
pub struct Plugin {
    /// Our `config/secrets.yml` YAML file, parsed and read into memory.
    /// Optional because if we're being run as a `PluginGenerate`, we won't
    /// have it (but it's guaranteed otherwise).
    config: Option<Config>,
}

impl Plugin {
    /// Get the path to this plugin's config file.
    fn config_path(project: &Project) -> PathBuf {
        project.root_dir().join("config").join("secrets.yml")
    }
}

impl plugins::Plugin for Plugin {
    fn name(&self) -> &'static str {
        Self::plugin_name()
    }
}

impl PluginNew for Plugin {
    fn plugin_name() -> &'static str {
        "secrets"
    }

    fn is_configured_for(project: &Project) -> Result<bool> {
        let path = Self::config_path(project);
        Ok(path.exists())
    }

    fn new(project: &Project) -> Result<Self> {
        let path = Self::config_path(project);
        let config = if path.exists() {
            Some(load_yaml(&path)?)
        } else {
            None
        };
        Ok(Plugin { config: config })
    }
}

impl PluginGenerate for Plugin {
    fn generator_description(&self) -> &'static str {
        "Store passwords & other secrets in a local file"
    }
}

impl PluginTransform for Plugin {
    fn transform(&self,
                 _op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<()> {

        let config = self.config
            .as_ref()
            .expect("config should always be present for transform");

        let append_service =
            |service: &mut dc::Service, pods: &BTreeMap<_, PodSecrets>, name| {
                let opt_env = pods.get(ctx.pod.name()).and_then(|p| p.get(name));
                if let Some(env) = opt_env {
                    service.environment.append(&mut env.to_compose_env());
                }
            };

        for (name, mut service) in &mut file.services {
            service.environment.append(&mut config.common.to_compose_env());
            append_service(&mut service, &config.pods, name);
            let target_name = ctx.project.current_target().name();
            if let Some(target) = config.targets.get(target_name) {
                service.environment.append(&mut target.common.to_compose_env());
                append_service(&mut service, &target.pods, name);
            }
        }
        Ok(())
    }
}

#[test]
fn enabled_for_projects_with_config_file() {
    use env_logger;
    let _ = env_logger::init();
    let proj1 = Project::from_example("hello").unwrap();
    assert!(!Plugin::is_configured_for(&proj1).unwrap());
    let proj2 = Project::from_example("rails_hello").unwrap();
    assert!(Plugin::is_configured_for(&proj2).unwrap());
}

#[test]
fn injects_secrets_into_services() {
    use env_logger;
    let _ = env_logger::init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    proj.set_current_target_name("production").unwrap();
    let plugin = Plugin::new(&proj).unwrap();

    let target = proj.current_target();
    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, frontend, Subcommand::Up);
    let mut file = frontend.merged_file(target).unwrap();
    plugin.transform(Operation::Output, &ctx, &mut file).unwrap();
    let web = file.services.get("web").unwrap();
    let global_password =
        web.environment.get("GLOBAL_PASSWORD").expect("has GLOBAL_PASSWORD");
    assert_eq!(global_password.value().unwrap(),
               "more magic");
    let some_password =
        web.environment.get("SOME_PASSWORD").expect("has SOME_PASSWORD");
    assert_eq!(some_password.value().unwrap(),
               "production secret");
}
