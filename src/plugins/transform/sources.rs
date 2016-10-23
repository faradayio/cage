//! Plugin which transforms `dc::File` to point at local clones of GitHub
//! repositories, and which handles mounting local source trees into
//! containers.

use compose_yml::v2 as dc;
use std::marker::PhantomData;

use errors::*;
use ext::service::ServiceExt;
use plugins;
use plugins::{Operation, PluginNew, PluginTransform};
use project::Project;
use util::ConductorPathExt;

/// Transforms `dc::File` to point at local source trees.
///
/// Note that this is only part of our source tree support—this plugin
/// doesn't load source trees or check them out using git.  (That's the job
/// of our top-level `sources` module.)  Rather, this plugin uses
/// information that we already know about source trees in order to
/// transform a `dc::File` for local development purposes.
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
        "sources"
    }

    fn new(_project: &Project) -> Result<Self> {
        Ok(Plugin { _placeholder: PhantomData })
    }
}

impl PluginTransform for Plugin {
    fn transform(&self,
                 op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<()> {
        // Give up immediately if we're not doing this for local output.
        if op != Operation::Output {
            return Ok(());
        }

        // Update each service to point to our locally cloned sources.
        let project = ctx.project;
        for service in &mut file.services.values_mut() {
            for sources_result in try!(service.sources(project.sources())) {
                let (mount_as, source) = try!(sources_result);
                if source.is_available_locally(project) && source.mounted() {
                    // Build an absolute path to our source's local
                    // directory.
                    let path = try!(source.path(project).to_absolute());

                    // Add a mount point to the container.
                    let mount = dc::VolumeMount::host(&path, mount_as);
                    service.volumes.push(dc::value(mount));

                    // Update the `build` field if it's present and it
                    // corresponds to this `Source`.
                    if let Some(ref mut build) = service.build {
                        if source.context() == try!(build.context.value()) {
                            build.context = dc::value(dc::Context::Dir(path));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
