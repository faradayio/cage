//! Plugin which applies `DefaultTags` to `dc::File`.

use docker_compose::v2 as dc;
use std::marker::PhantomData;

use ext::service::ServiceExt;
use plugins;
use plugins::{Operation, PluginNew, PluginTransform};
use project::Project;
use util::{ConductorPathExt, Error};

/// Loads a `config/secrets.yml` file and merges in into a project.
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Plugin {
    /// Placeholder field for future hidden fields, to keep this from being
    /// directly constructable.
    _placeholder: PhantomData<()>
}

impl plugins::Plugin for Plugin {
    fn name(&self) -> &'static str {
        Self::plugin_name()
    }
}

impl PluginNew for Plugin {
    fn plugin_name() -> &'static str {
        "repos"
    }

    fn new(_project: &Project) -> Result<Self, Error> {
        Ok(Plugin { _placeholder: PhantomData })
    }
}

impl PluginTransform for Plugin {
    fn transform(&self,
                 op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<(), Error> {
        // Give up immediately if we're not doing this for local output.
        if op != Operation::Output {
            return Ok(())
        }

        // Update each service to point to our locally cloned repos.
        let project = ctx.project;
        for service in &mut file.services.values_mut() {

            if let Some(git_url) = try!(service.git_url()).cloned() {
                if let Some(repo) = project.repos().find_by_git_url(&git_url) {
                    if repo.is_cloned(project) {
                        // Build an absolute path to our repo's clone directory.
                        let path = try!(repo.path(project).to_absolute());

                        // Mount the local build directory inside the
                        // container.
                        let srcdir = try!(service.source_mount_dir());
                        let mount = dc::VolumeMount::host(&path, &srcdir);
                        service.volumes.push(dc::value(mount));

                        // Update the `build` field if present.
                        if let Some(ref mut build) = service.build {
                            build.context = dc::value(dc::Context::Dir(path));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
