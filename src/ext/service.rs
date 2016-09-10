//! Extension methods for `docker_compose::v2::Service`.

use docker_compose::v2 as dc;
use shlex;

use ext::context::ContextExt;
use project::Project;
use util::{ConductorPathExt, Error};

/// These methods will appear as regular methods on `Service` in any module
/// which includes `ServiceExt`.
pub trait ServiceExt {
    /// The URL for the the git repository associated with this service.
    fn git_url(&self) -> Result<Option<&dc::GitUrl>, Error>;

    /// Get the default shell associated with this service.  Used for
    /// getting interactive access to a container.
    fn shell(&self) -> Result<String, Error>;

    /// Get the test command associated with this service.
    fn test_command(&self) -> Result<Vec<String>, Error>;

    /// Make any local updates to this service we want to make before
    /// outputting it for `Project::output`.
    fn update_for_output(&mut self, project: &Project) -> Result<(), Error>;
}

impl ServiceExt for dc::Service {
    fn git_url(&self) -> Result<Option<&dc::GitUrl>, Error> {
        if let Some(ref build) = self.build {
            Ok(try!(build.context.value()).git_url())
        } else {
            Ok(None)
        }
    }

    fn shell(&self) -> Result<String, Error> {
        Ok(self.labels.get("io.fdy.conductor.shell").cloned().unwrap_or_else(|| {
            "sh".to_owned()
        }))
    }

    fn test_command(&self) -> Result<Vec<String>, Error> {
        let raw = try!(self.labels.get("io.fdy.conductor.test").ok_or_else(|| {
            err!("specify a value for the label io.fdy.conductor.test to run tests")
        }));
        let mut lexer = shlex::Shlex::new(raw);
        let result: Vec<String> =
            lexer.by_ref().map(|w| w.to_owned()).collect();
        if lexer.had_error {
            Err(err!("cannot parse <{}> into shell words", raw))
        } else {
            Ok(result)
        }
    }

    fn update_for_output(&mut self, project: &Project) -> Result<(), Error> {
        // Handle locally cloned repositories.
        if let Some(git_url) = try!(self.git_url()).cloned() {
            if let Some(repo) = project.repos().find_by_git_url(&git_url) {
                if repo.is_cloned(project) {
                    // Build an absolute path to our repo's clone directory.
                    let path = try!(repo.path(project).to_absolute());

                    // Mount the local build directory as `/app` inside the
                    // container.
                    let mount = dc::VolumeMount::host(&path, "/app");
                    self.volumes.push(dc::value(mount));

                    // Update the `build` field if present.
                    if let Some(ref mut build) = self.build {
                        build.context = dc::value(dc::Context::Dir(path));
                    }
                }
            }
        }

        // Handle image version defaulting.
        if let Some(default_tags) = project.default_tags() {
            // Clone `self.image` to make life easy for the borrow checker,
            // so that it remains my friend.
            if let Some(image) = self.image.to_owned() {
                let default = default_tags.default_for(try!(image.value()));
                self.image = Some(dc::value(default));
            }
        }

        // TODO LOW: Remove `io.fdy.conductor.` labels?

        Ok(())
    }
}

#[test]
fn shell_returns_preferred_shell_for_this_service() {
    let proj: Project = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(&ovr).unwrap();

    // Default value.
    let web = merged.services.get("web").unwrap();
    assert_eq!(web.shell().unwrap(), "sh");

    // Custom value.
    let proxy = merged.services.get("proxy").unwrap();
    assert_eq!(proxy.shell().unwrap(), "/bin/sh");
}

// update_for_output is tested in ext::file::FileExt.
