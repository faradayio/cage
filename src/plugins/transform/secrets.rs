//! Plugin which loads secrets from `config/secrets.yml` and adds them to a
//! project.

use compose_yml::v2 as dc;
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::result;

use crate::errors::*;
use crate::plugins;
use crate::plugins::{Operation, PluginGenerate, PluginNew, PluginTransform};
use crate::project::Project;
use crate::serde_helpers::load_yaml;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The secrets for a single service.  We implement this as a very thin
/// wrapper around `BTreeMap` so that we can add methods.
#[derive(Default, Debug, PartialEq, Eq)]
struct ServiceSecrets {
    secrets: BTreeMap<String, String>,
}

impl ServiceSecrets {
    fn to_compose_env(&self) -> BTreeMap<String, dc::RawOr<String>> {
        let mut env = BTreeMap::new();
        for (var, val) in &self.secrets {
            let val = dc::escape(val).expect("escape string should never fail");
            env.insert(var.to_owned(), val);
        }
        env
    }
}

impl<'de> Deserialize<'de> for ServiceSecrets {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secrets = Deserialize::deserialize(deserializer)?;
        Ok(ServiceSecrets { secrets })
    }
}

impl Serialize for ServiceSecrets {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.secrets.serialize(serializer)
    }
}

/// The secrets for a pod.
type PodSecrets = BTreeMap<String, ServiceSecrets>;

/// The secrets for an target.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetSecrets {
    /// Shared between all services in this target.
    #[serde(default)]
    common: ServiceSecrets,
    /// Secrets for each of our pods.
    #[serde(default)]
    pods: BTreeMap<String, PodSecrets>,
}

/// The deserialized form of `secrets.yml`.  This is basically
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Shared between all services in this pod.
    #[serde(default)]
    common: ServiceSecrets,
    /// Secrets for each of our pods.
    #[serde(default)]
    pods: BTreeMap<String, PodSecrets>,
    /// Secrets for each of our targets.
    #[serde(default)]
    targets: BTreeMap<String, TargetSecrets>,
}

#[test]
fn can_deserialize_config() {
    let path = Path::new("examples/rails_hello/config/secrets.yml");
    let config: Config = load_yaml(path).unwrap();
    assert_eq!(
        config.common.secrets.get("GLOBAL_PASSWORD").unwrap(),
        "magic"
    );
}

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
        Ok(Plugin { config })
    }
}

impl PluginGenerate for Plugin {
    fn generator_description(&self) -> &'static str {
        "Store passwords & other secrets in a local file"
    }
}

impl PluginTransform for Plugin {
    fn transform(
        &self,
        _op: Operation,
        ctx: &plugins::Context<'_>,
        file: &mut dc::File,
    ) -> Result<()> {
        let config = self
            .config
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
            service
                .environment
                .append(&mut config.common.to_compose_env());
            append_service(&mut service, &config.pods, name);
            let target_name = ctx.project.current_target().name();
            if let Some(target) = config.targets.get(target_name) {
                service
                    .environment
                    .append(&mut target.common.to_compose_env());
                append_service(&mut service, &target.pods, name);
            }
        }
        Ok(())
    }
}

#[test]
fn enabled_for_projects_with_config_file() {
    let _ = env_logger::try_init();
    let proj1 = Project::from_example("hello").unwrap();
    assert!(!Plugin::is_configured_for(&proj1).unwrap());
    let proj2 = Project::from_example("rails_hello").unwrap();
    assert!(Plugin::is_configured_for(&proj2).unwrap());
}

#[test]
fn injects_secrets_into_services() {
    let _ = env_logger::try_init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    proj.set_current_target_name("production").unwrap();
    let plugin = Plugin::new(&proj).unwrap();

    let target = proj.current_target();
    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, frontend, "up");
    let mut file = frontend.merged_file(target).unwrap();
    plugin
        .transform(Operation::Output, &ctx, &mut file)
        .unwrap();
    let web = file.services.get("web").unwrap();
    let global_password = web
        .environment
        .get("GLOBAL_PASSWORD")
        .expect("has GLOBAL_PASSWORD");
    assert_eq!(global_password.value().unwrap(), "more magic");
    let some_password = web
        .environment
        .get("SOME_PASSWORD")
        .expect("has SOME_PASSWORD");
    assert_eq!(some_password.value().unwrap(), "production secret");
}
