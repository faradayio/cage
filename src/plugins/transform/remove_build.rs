//! Plugin which removes the `build` field in a in a `dc::File`.

use faraday_compose_yml::v2 as dc;
use std::marker::PhantomData;

use crate::errors::*;
use crate::plugins;
use crate::plugins::{Operation, PluginNew, PluginTransform};
use crate::project::Project;

/// Updates the `labels` in a `dc::File`.
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
        "remove_build"
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
        _op: Operation,
        ctx: &plugins::Context<'_>,
        file: &mut dc::File,
    ) -> Result<()> {
        if ctx.subcommand != "build" {
            for service in &mut file.services.values_mut() {
                service.build = None;
            }
        }
        Ok(())
    }
}

#[test]
fn removes_build_for_most_commands() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let plugin = Plugin::new(&proj).unwrap();

    let target = proj.current_target();
    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, frontend, "up");
    let mut file = frontend.merged_file(target).unwrap();

    plugin
        .transform(Operation::Output, &ctx, &mut file)
        .unwrap();

    let web = file.services.get("web").unwrap();
    assert_eq!(web.build, None);
}

#[test]
fn leaves_build_in_when_building() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let plugin = Plugin::new(&proj).unwrap();

    let target = proj.current_target();
    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, frontend, "build");
    let mut file = frontend.merged_file(target).unwrap();

    plugin
        .transform(Operation::Output, &ctx, &mut file)
        .unwrap();

    let web = file.services.get("web").unwrap();
    assert!(web.build != None);
}
