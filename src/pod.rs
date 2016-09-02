//! A single pod in a project.

use std::marker::PhantomData;

/// A pod, specified by `pods/$NAME.yml` and zero or more
/// `pods/overrides/*/*.yml` overrides that we can apply to it.
pub struct Pod {
    /// The name of this pod, based on the file `pods/$NAME.yml`.
    pub name: String,

    /// PRIVATE.  Mark this struct as having unknown fields for future
    /// compatibility.  This prevents direct construction and exhaustive
    /// matching.  This needs to be be public because of
    /// http://stackoverflow.com/q/39277157/12089
    #[doc(hidden)]
    pub _phantom: PhantomData<()>,
}
