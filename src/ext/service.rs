//! Extension methods for `compose_yml::v2::Service`.

use compose_yml::v2 as dc;
use shlex;
use std::vec;

use errors::*;
#[cfg(test)]
use project::Project;
use sources::{self, Source};
use ext::context::ContextExt;
use util::err;

/// These methods will appear as regular methods on `Service` in any module
/// which includes `ServiceExt`.
pub trait ServiceExt {
    /// The build context associated with this service (either a git
    /// repository URL or a local directory).
    fn context(&self) -> Result<Option<&dc::Context>>;

    /// The directory in which to mount our source code if it's checked
    /// out.
    fn source_mount_dir(&self) -> Result<String>;

    /// The subdirectory inside the git repository where the source code
    /// for this service is located.
    fn repo_subdir(&self) -> Result<Option<String>>;

    /// Get the default shell associated with this service.  Used for
    /// getting interactive access to a container.
    fn shell(&self) -> Result<String>;

    /// Get the test command associated with this service.
    fn test_command(&self) -> Result<Vec<String>>;

    /// All the `Source` trees which can be mounted into this `Service`.
    /// Note that this iterator does not hold any references to this
    /// `Service` object, so you can use it to decide how you want to
    /// update other fields of this object without running afoul of the
    /// borrow checker.
    fn sources<'a, 'b>(&'a self,
                       sources: &'b sources::Sources)
                       -> Result<Sources<'b>>;
}

impl ServiceExt for dc::Service {
    fn context(&self) -> Result<Option<&dc::Context>> {
        if let Some(ref build) = self.build {
            Ok(Some(build.context.value()?))
        } else {
            Ok(None)
        }
    }

    fn source_mount_dir(&self) -> Result<String> {
        let default = dc::escape("/app")?;
        let srcdir = self.labels
            .get("io.fdy.cage.srcdir")
            .unwrap_or_else(|| &default);
        Ok(srcdir.value()?.to_owned())
    }

    fn repo_subdir(&self) -> Result<Option<String>> {
        if let Some(context) = self.context()? {
            match *context {
                dc::Context::Dir(_) => (),
                dc::Context::GitUrl(ref git_url) => {
                    return Ok(git_url.subdirectory())
                },
            }
        }
        Ok(None)
    }

    fn shell(&self) -> Result<String> {
        let default = dc::escape("sh")?;
        let shell = self.labels
            .get("io.fdy.cage.shell")
            .unwrap_or_else(|| &default);
        Ok(shell.value()?.to_owned())
    }

    fn test_command(&self) -> Result<Vec<String>> {
        let raw = self.labels
            .get("io.fdy.cage.test")
            .ok_or_else(|| {
                err("specify a value for the label io.fdy.cage.test to run tests")
            })?;
        let mut lexer = shlex::Shlex::new(raw.value()?);
        let result: Vec<String> = lexer.by_ref().map(|w| w.to_owned()).collect();
        if lexer.had_error {
            Err(err!("cannot parse <{}> into shell words", raw))
        } else {
            Ok(result)
        }
    }

    fn sources<'a, 'b>(&'a self,
                       sources: &'b sources::Sources)
                       -> Result<Sources<'b>> {
        // Get our `context`, if any.
        let source_mount_dir = self.source_mount_dir()?;
        let repo_subdir = self.repo_subdir()?;
        let context = self.context()?.map(|ctx| (source_mount_dir, repo_subdir, ctx.clone()));

        // Get our library keys and mount points.
        let mut libs = vec![];
        for (label, mount_as) in &self.labels {
            let prefix = "io.fdy.cage.lib.";
            if label.starts_with(prefix) {
                libs.push((mount_as.value()?.to_owned(),
                           (&label[prefix.len()..]).to_owned()));
            }
        }

        Ok(Sources {
            sources: sources,
            context: context,
            libs: libs.into_iter(),
        })
    }
}

/// Iterator over all the `Source` trees which can be mounted into this
/// `Service`.
pub struct Sources<'a> {
    /// All `Source` trees available for this repository.
    sources: &'a sources::Sources,
    /// Do we need to iterate over our `context` field?
    context: Option<(String, Option<String>, dc::Context)>,
    /// Libraries
    libs: vec::IntoIter<(String, String)>,
}

impl<'a> Iterator for Sources<'a> {
    type Item = Result<(String, Option<String>, &'a Source)>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check for a `dc::Context` using `take`, which moves data out of
        // an `Option` value and leaves `None` in its place,
        // simulataneously updating our internal state and keeping the
        // borrow checker happy.
        if let Some((container_path, repo_subdir, context)) = self.context.take() {
            if let Some(source) = self.sources.find_by_alias(&context.human_alias().unwrap()) {
                // We have a `context` and a `source`, so return them.
                Some(Ok((container_path, repo_subdir, source)))
            } else {
                // We have a `context` but it doesn't correspond to a known
                // `Source`, so move on the next step of the iteration.
                self.next()
            }
        } else {
            // Iterate over any "libs"-style mounts.
            self.libs.next().map(|(container_path, name)| {
                match self.sources.find_by_lib_key(&name) {
                    None => Err(ErrorKind::UnknownLibKey(name).into()),
                    Some(source) => Ok((container_path, None, source)),
                }
            })
        }
    }
}

#[test]
fn src_dir_returns_the_source_directory_for_this_service() {
    use env_logger;
    let _ = env_logger::init();
    let proj: Project = Project::from_example("rails_hello").unwrap();
    let target = proj.target("development").unwrap();

    // Default value.
    let db = proj.pod("db").unwrap();
    let merged = db.merged_file(target).unwrap();
    let db = merged.services.get("db").unwrap();
    assert_eq!(db.source_mount_dir().unwrap(), "/app");

    // Custom value.
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(target).unwrap();
    let proxy = merged.services.get("web").unwrap();
    assert_eq!(proxy.source_mount_dir().unwrap(), "/usr/src/app");
}

#[test]
fn build_context_can_specify_a_subdirectory() {
    use env_logger;
    let _ = env_logger::init();
    let proj: Project = Project::from_example("rails_hello_with_subdir").unwrap();
    let target = proj.target("development").unwrap();

    // Default value.
    let db = proj.pod("db").unwrap();
    let merged = db.merged_file(target).unwrap();
    let db = merged.services.get("db").unwrap();
    assert_eq!(db.repo_subdir().unwrap(), None);

    // Custom value.
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(target).unwrap();
    let web = merged.services.get("web").unwrap();
    assert_eq!(web.repo_subdir().unwrap(), Some("myfolder".to_string()));
}

#[test]
fn shell_returns_preferred_shell_for_this_service() {
    use env_logger;
    let _ = env_logger::init();
    let proj: Project = Project::from_example("hello").unwrap();
    let target = proj.target("development").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(target).unwrap();

    // Default value.
    let web = merged.services.get("web").unwrap();
    assert_eq!(web.shell().unwrap(), "sh");

    // Custom value.
    let proxy = merged.services.get("proxy").unwrap();
    assert_eq!(proxy.shell().unwrap(), "/bin/sh");
}