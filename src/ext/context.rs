//! Extension methods for `docker_compose::v2::Service`.

use docker_compose::v2 as dc;

use git_url::GitUrl;
use util::Error;

/// These methods will appear as regular methods on `Context` in any module
/// which includes `ContextExt`.
pub trait ContextExt {
    /// The URL for the the git repository associated with this context,
    /// if there is one.
    fn git_url(&self) -> Result<Option<GitUrl>, Error>;
}

impl ContextExt for dc::Context {
    fn git_url(&self) -> Result<Option<GitUrl>, Error> {
        match self {
            &dc::Context::GitUrl(ref url) =>
                Ok(Some(try!(GitUrl::new(url.as_ref())))),
            &dc::Context::Dir(_) => Ok(None),
        }
    }
}
