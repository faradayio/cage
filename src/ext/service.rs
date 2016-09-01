//! Extension methods for `docker_compose::v2::Service`.

use docker_compose::v2 as dc;
use std::path::{PathBuf};

use ext::context::ContextExt;

/// These methods will appear as regular methods on `Service` in any module
/// which includes `ServiceExt`.
pub trait ServiceExt {
    /// Get the local build directory that we'll use for a service.
    /// Normally this will be based on its GitHub URL if one if provided in
    /// the `build.context` field.
    fn local_build_dir(&self) -> Result<Option<PathBuf>, dc::Error>;
}

impl ServiceExt for dc::Service {
    fn local_build_dir(&self) -> Result<Option<PathBuf>, dc::Error>
    {
        if let Some(ref build) = self.build {
            let ctx = try!(build.context.value());
            Ok(Some(try!(ctx.local())))
        } else {
            Ok(None)
        }
    }
}
