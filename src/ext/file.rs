//! Extension methods for `docker_compose::v2::File`.

use docker_compose::v2 as dc;
#[cfg(test)] use std::path::Path;

use ext::service::ServiceExt;
use project::Project;
use util::{ConductorPathExt, Error};

/// These methods will appear as regular methods on `dc::File` in any module
/// which includes `FileExt`.
pub trait FileExt {
    /// Make any local updates to this file we want to make before
    /// outputting it for `Project::output`.
    fn update_for_output(&mut self, project: &Project) -> Result<(), Error>;
}

impl FileExt for dc::File {
    fn update_for_output(&mut self, project: &Project) -> Result<(), Error> {
        for (_name, mut service) in self.services.iter_mut() {
            if let Some(git_url) = try!(service.git_url()).cloned() {
                if let Some(repo) = project.repos().find_by_git_url(&git_url) {
                    if repo.is_cloned(project) {
                        // Build an absolute path to our repo's clone directory.
                        let path = try!(repo.path(project).to_absolute());

                        // Mount the local build directory as `/app` inside the
                        // container.
                        let mount = dc::VolumeMount::host(&path, "/app");
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

#[test]
fn update_for_output_mounts_cloned_source() {
    use docker_compose::v2 as dc;

    let proj = Project::from_example("hello").unwrap();
    let repo = proj.repos().find_by_alias("dockercloud-hello-world").unwrap();
    repo.fake_clone_source(&proj).unwrap();
    proj.output().unwrap();

    // Load the generated file and look at the `web` service we cloned.
    let frontend_file = proj.output_dir().join("pods/frontend.yml");
    let file = dc::File::read_from_path(frontend_file).unwrap();
    let web = file.services.get("web").unwrap();
    let src_path = repo.path(&proj).to_absolute().unwrap();

    // Make sure our `build` entry has been pointed at the local source
    // directory.
    assert_eq!(web.build.as_ref().unwrap().context.value().unwrap(),
               &dc::Context::new(src_path.to_str().unwrap()));

    // Make sure the local source directory is being mounted into the
    // container.
    let mount = web.volumes.last()
        .expect("expected web service to have volumes")
        .value().unwrap();
    assert_eq!(mount.host, Some(dc::HostVolume::Path(src_path)));
    assert_eq!(mount.container, Path::new("/app"));

    proj.remove_test_output().unwrap();
}
