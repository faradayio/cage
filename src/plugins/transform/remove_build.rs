//! Plugin which removes the `build` field in a in a `dc::File`.

use compose_yml::v2 as dc;
use std::marker::PhantomData;

use errors::*;
use plugins;
use plugins::{Operation, PluginNew, PluginTransform};
use project::Project;

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
        Ok(Plugin { _placeholder: PhantomData })
    }
}

impl PluginTransform for Plugin {
    fn transform(&self,
                 _op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<()> {

        // TODO: Test this
        if ctx.subcommand != "build" {
            for service in &mut file.services.values_mut() {
                service.build = None;
            }
        }
        Ok(())
    }
}
