//! Extension methods for `compose_yml::v2::Service`.

use compose_yml::v2 as dc;
use shlex;
use std::path::{Path, PathBuf};
use std::vec;

use errors::*;
#[cfg(test)]
use project::Project;
use sources::{self, Source};
use util::err;

/// These methods will appear as regular methods on `Service` in any module
/// which includes `ServiceExt`.
pub trait ServiceExt {
    /// The build context associated with this service (either a git
    /// repository URL or a local directory).
    fn context(&self) -> Result<Option<&dc::Context>>;

    /// The directory in which to mount our source code if it's checked
    /// out.
    fn source_mount_dir(&self) -> Result<PathBuf>;

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
    fn sources<'a, 'b>(&'a self, sources: &'b sources::Sources)
                       -> Result<Sources<'b>>;
}

impl ServiceExt for dc::Service {
    fn context(&self) -> Result<Option<&dc::Context>> {
        if let Some(ref build) = self.build {
            Ok(Some(try!(build.context.value())))
        } else {
            Ok(None)
        }
    }

    fn source_mount_dir(&self) -> Result<PathBuf> {
        Ok(Path::new(self.labels
                .get("io.fdy.cage.srcdir")
                .map_or_else(|| "/app", |v| v as &str))
            .to_owned())
    }

    fn shell(&self) -> Result<String> {
        Ok(self.labels
            .get("io.fdy.cage.shell")
            .cloned()
            .unwrap_or_else(|| "sh".to_owned()))
    }

    fn test_command(&self) -> Result<Vec<String>> {
        let raw = try!(self.labels.get("io.fdy.cage.test").ok_or_else(|| {
            err("specify a value for the label io.fdy.cage.test to run tests")
        }));
        let mut lexer = shlex::Shlex::new(raw);
        let result: Vec<String> = lexer.by_ref().map(|w| w.to_owned()).collect();
        if lexer.had_error {
            Err(err!("cannot parse <{}> into shell words", raw))
        } else {
            Ok(result)
        }
    }

    fn sources<'a, 'b>(&'a self, sources: &'b sources::Sources)
                       -> Result<Sources<'b>> {
        // Get our `context`, if any.
        let source_mount_dir = try!(self.source_mount_dir());
        let context = try!(self.context())
            .map(|ctx| (source_mount_dir, ctx.clone()));

        // Get our library keys and mount points.
        let mut libs = vec![];
        for (label, mount_as) in &self.labels {
            let prefix = "io.fdy.cage.lib.";
            if label.starts_with(prefix) {
                libs.push((Path::new(mount_as).to_owned(),
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
    context: Option<(PathBuf, dc::Context)>,
    /// Libraries
    libs: vec::IntoIter<(PathBuf, String)>,
}

impl<'a> Iterator for Sources<'a> {
    type Item = Result<(PathBuf, &'a Source)>;

    fn next(&mut self) -> Option<Self::Item> {
        // Check for a `dc::Context` using `take`, which moves data out of
        // an `Option` value and leaves `None` in its place,
        // simulataneously updating our internal state and keeping the
        // borrow checker happy.
        if let Some((path_buf, context)) = self.context.take() {
            if let Some(source) = self.sources.find_by_context(&context) {
                // We have a `context` and a `source`, so return them.
                Some(Ok((path_buf, source)))
            } else {
                // We have a `context` but it doesn't correspond to a known
                // `Source`, so move on the next step of the iteration.
                self.next()
            }
        } else {
            // Iterate over any "libs"-style mounts.
            self.libs.next().map(|(path_buf, name)| {
                match self.sources.find_by_lib_key(&name) {
                    None => Err(ErrorKind::UnknownLibKey(name).into()),
                    Some(source) => Ok((path_buf, source)),
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
    assert_eq!(db.source_mount_dir().unwrap(), Path::new("/app"));

    // Custom value.
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(target).unwrap();
    let proxy = merged.services.get("web").unwrap();
    assert_eq!(proxy.source_mount_dir().unwrap(), Path::new("/usr/src/app"));
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
