//! Extension methods for `docker_compose::v2::Service`.

use docker_compose::v2 as dc;

use ext::context::ContextExt;
use git_url::GitUrl;
use util::Error;

/// These methods will appear as regular methods on `Service` in any module
/// which includes `ServiceExt`.
pub trait ServiceExt {
    /// The URL for the the git repository associated with this service.
    fn git_url(&self) -> Result<Option<GitUrl>, Error>;
}

impl ServiceExt for dc::Service {
    fn git_url(&self) -> Result<Option<GitUrl>, Error> {
        if let Some(ref build) = self.build {
            try!(build.context.value()).git_url()
        } else {
            Ok(None)
        }
    }
}
