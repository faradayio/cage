//! Plugin which transforms `dc::File` to point at local clones of GitHub
//! repositories, and which handles mounting local source trees into
//! containers.

use compose_yml::v2 as dc;
use std::marker::PhantomData;
#[cfg(test)]
use std::path::Path;

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
        Ok(Plugin {
            _placeholder: PhantomData,
        })
    }
}

impl PluginTransform for Plugin {
    fn transform(
        &self,
        op: Operation,
        ctx: &plugins::Context,
        file: &mut dc::File,
    ) -> Result<()> {
        // Give up immediately if we're not doing this for local output.
        if op != Operation::Output {
            return Ok(());
        }

        // Update each service to point to our locally cloned sources.
        let project = ctx.project;
        for service in &mut file.services.values_mut() {
            for source_mount in service.sources(project.sources())? {
                let source = source_mount.source;
                if source.is_available_locally(project) && source.mounted() {
                    // Build an absolute path to our source's local directory.
                    let source_subdirectory =
                        source_mount.source_subdirectory.unwrap_or("".to_string());
                    let path = source
                        .path(project)
                        .join(source_subdirectory)
                        .to_absolute()?;

                    // Add a mount point to the container.
                    let mount =
                        dc::VolumeMount::host(&path, source_mount.container_path);
                    service.volumes.push(dc::value(mount));

                    // Update the `build` field if it's present and it
                    // corresponds to this `Source`.
                    if let Some(ref mut build) = service.build {
                        if source.context() == build.context.value()? {
                            build.context = dc::value(dc::Context::Dir(path));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[test]
fn adds_a_volume_with_a_subdirectory() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_fixture("with_repo_subdir").unwrap();
    let plugin = Plugin::new(&proj).unwrap();

    let source = proj.sources().find_by_alias("rails_hello").unwrap();
    source.fake_clone_source(&proj).unwrap();

    let target = proj.current_target();
    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, frontend, "up");
    let mut file = frontend.merged_file(target).unwrap();

    plugin
        .transform(Operation::Output, &ctx, &mut file)
        .unwrap();

    let web = file.services.get("web").unwrap();
    let src_volume = web.volumes[0].value().unwrap();
    let host_path = match src_volume.clone().host.unwrap() {
        dc::HostVolume::Path(ref path_buf) => path_buf.clone(),
        _ => unreachable!(),
    };

    assert!(host_path.ends_with(Path::new("src/rails_hello/myfolder")));
    assert_eq!(src_volume.container, "/usr/src/app");
}
