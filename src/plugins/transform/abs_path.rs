//! Plugin which converts all paths in a `dc::File` to absolute.

use compose_yml::v2 as dc;
use std::marker::PhantomData;

use crate::errors::*;
use crate::plugins;
use crate::plugins::{Operation, PluginNew, PluginTransform};
use crate::project::Project;
use crate::util::{err, ConductorPathExt};

/// Plugin which converts all paths in a `dc::File` to absolute.
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
        "abs_path"
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
        ctx: &plugins::Context<'_>,
        file: &mut dc::File,
    ) -> Result<()> {
        // Give up immediately if we're not doing this for local output.
        // It's not yet clear what we should do with relative paths in
        // exported output, anyway.
        if op != Operation::Output {
            return Ok(());
        }

        for service in file.services.values_mut() {
            // Handle relative paths in `build:`.
            if let Some(ref mut build) = service.build {
                let context: &mut _ = build.context.value_mut()?;
                if let dc::Context::Dir(ref mut path) = *context {
                    let new_path = ctx.project.pods_dir().join(&path).to_absolute()?;
                    *path = new_path;
                }
            }

            for volume in &mut service.volumes {
                let volume = volume.value_mut()?;
                // TODO LOW: Move to `dc` library.
                let new_host = match volume.host {
                    Some(dc::HostVolume::Path(ref path)) if path.is_relative() => {
                        let new_path = ctx.project.pods_dir().join(path);
                        Some(dc::HostVolume::Path(new_path.to_absolute()?))
                    }
                    Some(dc::HostVolume::UserRelativePath(ref path))
                        if path.is_relative() =>
                    {
                        let home = dirs::home_dir()
                            .ok_or_else(|| err("Cannot find HOME directory"))?;
                        let new_path = home.join(path);
                        Some(dc::HostVolume::Path(new_path.to_absolute()?))
                    }
                    ref other => other.to_owned(),
                };
                volume.host = new_host;
            }
        }

        Ok(())
    }
}

// TODO MED: This is no longer a recommended configuration, so we need to
// figure out a good way to test it without using it in one of our main
// example programs.
//
// #[test]
// fn converts_relative_paths_to_absolute() {
//     let _ = env_logger::try_init();
//     let proj = Project::from_example("rails_hello").unwrap();
//     proj.output().unwrap();
//
//     // Load the generated file and look at the `db` service we cloned.
//     let db_file = proj.output_dir().join("pods").join("db.yml");
//     let file = dc::File::read_from_path(db_file).unwrap();
//     let db = file.services.get("db").unwrap();
//
//     assert_eq!(db.volumes.len(), 1);
//     let expected = proj.pods_dir().join("..").join("data").join("db")
//         .to_absolute().unwrap();
//     assert_eq!(db.volumes[0].value().unwrap().host.as_ref().unwrap(),
//                &dc::HostVolume::Path(expected));
// }
