//! Plugin which loads secrets from `config/secrets.yml` and adds them to a
//! project.

use docker_compose::v2 as dc;
use serde_yaml;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use plugins;
use plugins::{Operation, PluginGenerate, PluginNew, PluginTransform};
use project::Project;
use util::Error;

#[cfg(feature = "serde_macros")]
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
        project.root_dir().join("config/secrets.yml")
    }
}

impl plugins::Plugin for Plugin {
    fn name(&self) -> &'static str {
        Self::plugin_name()
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
                 -> Result<(), Error> {

        let config = self.config
            .as_ref()
            .expect("config should always be present for transform");

        let append_service =
            |service: &mut dc::Service, pods: &BTreeMap<_, PodSecrets>, name| {
                let opt_env = pods.get(ctx.pod.name()).and_then(|p| p.get(name));
                if let Some(env) = opt_env {
                    service.environment.append(&mut env.clone());
                }
            };

        for (name, mut service) in &mut file.services {
            service.environment.append(&mut config.common.clone());
            append_service(&mut service, &config.pods, name);
            if let Some(ovr) = config.overrides.get(ctx.ovr.name()) {
                service.environment.append(&mut ovr.common.clone());
                append_service(&mut service, &ovr.pods, name);
            }
        }
        Ok(())
    }
}

impl PluginNew for Plugin {
    fn plugin_name() -> &'static str {
        "secrets"
    }

    fn is_configured_for(project: &Project) -> Result<bool, Error> {
        let path = Self::config_path(project);
        Ok(path.exists())
    }

    fn new(project: &Project) -> Result<Self, Error> {
        let path = Self::config_path(project);
        let config = if path.exists() {
            let f = try!(fs::File::open(&path));
            Some(try!(serde_yaml::from_reader(f)
                .map_err(|e| err!("Error reading {}: {}", path.display(), e))))
        } else {
            None
        };
        Ok(Plugin { config: config })
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
    let proj = Project::from_example("rails_hello").unwrap();
    let plugin = Plugin::new(&proj).unwrap();

    let ovr = proj.ovr("production").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, ovr, frontend);
    let mut file = frontend.merged_file(ovr).unwrap();
    plugin.transform(Operation::Output, &ctx, &mut file).unwrap();
    let web = file.services.get("web").unwrap();
    assert_eq!(web.environment.get("GLOBAL_PASSWORD").expect("has GLOBAL_PASSWORD"),
               "more magic");
    assert_eq!(web.environment.get("SOME_PASSWORD").expect("has SOME_PASSWORD"),
               "production secret");
}
