//! A single pod in a project.

use std::path::PathBuf;

/// A pod, specified by `pods/$NAME.yml` and zero or more
/// `pods/overrides/*/*.yml` overrides that we can apply to it.
#[derive(Debug)]
pub struct Pod {
    /// All paths in any associated `dc::File` should be intepreted
    /// relative to this base, including paths in overlay files.
    base_dir: PathBuf,

    /// The name of this pod, based on the file `pods/$NAME.yml`.
    name: String,
}

impl Pod {
    /// Create a new pod, specifying the base directory from which we'll load
    /// pod definitions and the name of the pod.
    #[doc(hidden)]
    pub fn new<P, S>(base_dir: P, name: S) -> Pod
        where P: Into<PathBuf>, S: Into<String>
    {
        Pod {
            base_dir: base_dir.into(),
            name: name.into(),
        }
    }

    /// Get the name of this pod.
    pub fn name(&self) -> &str {
        &self.name
    }
}
