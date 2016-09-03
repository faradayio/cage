//! A single pod in a project.

use docker_compose::v2 as dc;
use std::path::{Path, PathBuf};

use overrides::Override;
use util::Error;

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

    /// The path to this pod, relative to the `base_dir` specified at
    /// creation time.
    pub fn rel_path(&self) -> PathBuf {
        Path::new(&format!("{}.yml", self.name)).to_owned()
    }

    /// The path to the specificied override file for this pod.
    pub fn override_rel_path(&self, ovr: &Override) -> PathBuf {
        let name = format!("overrides/{}/{}.yml", ovr.name(), self.name);
        Path::new(&name).to_owned()
    }

    /// Read the `dc::File` object associated with this pod.
    pub fn read(&self) -> Result<dc::File, Error> {
        let path = self.base_dir.join(&self.rel_path());
        let file = try!(dc::File::read_from_path(&path));
        Ok(file)
    }

    /// Read the `dc::File` object associated with the specified override
    /// for this pod.  This will automatically be created if necessary, and any
    /// services which appear in the main pod will also appear in the override.
    pub fn read_override(&self, ovr: &Override) -> Result<dc::File, Error> {
        let path = self.base_dir.join(&self.override_rel_path(ovr));
        let file =
            if path.exists() {
                try!(dc::File::read_from_path(&path))
            } else {
                Default::default()
            };
        Ok(file)
    }
}
