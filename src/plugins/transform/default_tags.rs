//! Plugin which applies `DefaultTags` to `dc::File`.

use compose_yml::v2 as dc;
use std::marker::PhantomData;

use plugins;
use plugins::{Operation, PluginNew, PluginTransform};
use project::Project;
use util::Error;

/// Applies `DefaultTags` to `dc::File`.
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
        "default_tags"
    }

    fn new(_project: &Project) -> Result<Self, Error> {
        Ok(Plugin { _placeholder: PhantomData })
    }
}

impl PluginTransform for Plugin {
    fn transform(&self,
                 _op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<(), Error> {
        // Do we have any default tags specified for this project?
        if let Some(tags) = ctx.project.default_tags() {
            // Apply the tags to each service.
            for service in &mut file.services.values_mut() {
                // Clone `self.image` to make life easy for the borrow checker,
                // so that it remains my friend.
                if let Some(image) = service.image.to_owned() {
                    let default = tags.default_for(try!(image.value()));
                    service.image = Some(dc::value(default));
                }
            }
        }
        Ok(())
    }
}
