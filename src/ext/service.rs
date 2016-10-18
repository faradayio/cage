//! Extension methods for `compose_yml::v2::Service`.

use compose_yml::v2 as dc;
use shlex;
use std::path::{Path, PathBuf};

use errors::*;
#[cfg(test)]
use project::Project;
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
