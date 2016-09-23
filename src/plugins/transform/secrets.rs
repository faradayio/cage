//! Plugin which loads secrets from `config/secrets.yml` and adds them to a
//! project.

use docker_compose::v2 as dc;
use serde_yaml;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use plugins;
use plugins::transform::Operation;
use plugins::transform::{Plugin as TransformPlugin, PluginNew};
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
    config: Config,
}

impl Plugin {
    /// Get the path to this plugin's config file.
    fn config_path(project: &Project) -> PathBuf {
        project.root_dir().join("config/secrets.yml")
    }
}

impl TransformPlugin for Plugin {
    fn name(&self) -> &'static str {
        "secrets"
    }

    fn transform(&self,
                 _op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<(), Error> {

        let append_container =
            |service: &mut dc::Service, pods: &BTreeMap<_, PodSecrets>, name| {
                let opt_env = pods.get(ctx.pod.name()).and_then(|p| p.get(name));
                if let Some(env) = opt_env {
                    service.environment.append(&mut env.clone());
                }
            };

        for (name, mut service) in &mut file.services {
            service.environment.append(&mut self.config.common.clone());
            append_container(&mut service, &self.config.pods, name);
            if let Some(ovr) = self.config.overrides.get(ctx.ovr.name()) {
                service.environment.append(&mut ovr.common.clone());
                append_container(&mut service, &ovr.pods, name);
            }
        }
        Ok(())
    }
}

impl PluginNew for Plugin {
    /// Should we enable this plugin for this project?
    fn should_enable_for(project: &Project) -> Result<bool, Error> {
        let path = Self::config_path(project);
        Ok(path.exists())
    }

    /// Create a new plugin.
    fn new(project: &Project) -> Result<Self, Error> {
        let f = try!(fs::File::open(&Self::config_path(project)));
        let config = try!(serde_yaml::from_reader(f));
        Ok(Plugin { config: config })
    }
}

#[test]
fn enabled_for_projects_with_config_file() {
    use env_logger;
    let _ = env_logger::init();
    let proj1 = Project::from_example("hello").unwrap();
    assert!(!Plugin::should_enable_for(&proj1).unwrap());
    let proj2 = Project::from_example("rails_hello").unwrap();
    assert!(Plugin::should_enable_for(&proj2).unwrap());
}

#[test]
fn injects_secrets_into_containers() {
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
