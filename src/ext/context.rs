//! Extension methods for `compose_yml::v2::Service`.

use compose_yml::v2 as dc;

/// These methods will appear as regular methods on `Context` in any module
/// which includes `ContextExt`.
pub trait ContextExt {
    /// The URL for the the git repository associated with this context,
    /// if there is one.
    fn git_url(&self) -> Option<&dc::GitUrl>;
}

impl ContextExt for dc::Context {
    fn git_url(&self) -> Option<&dc::GitUrl> {
        match *self {
            dc::Context::GitUrl(ref url) => Some(url),
            dc::Context::Dir(_) => None,
        }
    }
}
