//! A single pod in a project.

use docker_compose::v2 as dc;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use overrides::Override;
use util::Error;

/// Information about a `docker-compose.yml` file, including its path
/// relative to `base_dir` (`base_dir` is normally `$PROJECT/pods`), and
/// the normalized version of its contents:
///
/// 1. Any missing services will be explicitly added to an override file.
/// 2. The `env_file` list will be updated to contain the appropriate
///    `common.yml` file.
///
/// If this file doesn't actually exist on disk, we'll still fill in the
/// default contents as above.
///
/// If you need to process this further, clone the `File` and work on the
/// clone.  This is the master copy.
#[derive(Debug)]
struct FileInfo {
    rel_path: PathBuf,
    file: dc::File,
}

impl FileInfo {
    /// Create a `FileInfo` by either loading `base_dir.join(rel_path)` or
    /// by creating an empty `dc::File` in its place.  Do not perform
    /// normalization.
    fn unnormalized(base_dir: &Path, rel_path: &Path) -> Result<FileInfo, Error> {
        let path = base_dir.join(rel_path);
        Ok(FileInfo {
            rel_path: rel_path.to_owned(),
            file: if path.exists() {
                try!(dc::File::read_from_path(&path))
            } else {
                Default::default()
            },
        })
    }

    /// Make sure that all services from `base` are also present in this
    /// file.  If you're going tp call this, it must be called after
    /// `finish_normalization`
    fn ensure_all_services_from(&mut self, base: &dc::File) {
        for (name, _service) in base.services.iter() {
            self.file.services.entry(name.to_owned())
                .or_insert_with(Default::default);
        }
    }

    /// Finish normalizing this file by inserting things like `env_file`
    /// entries.
    fn finish_normalization(&mut self) {
        // It's safe to call `unwrap` here because we know `rel_path` should
        // have a parent directory.
        let env_path = self.rel_path.parent().unwrap().join("common.env");
        for (_name, service) in self.file.services.iter_mut() {
            service.env_files.insert(0, dc::value(env_path.clone()));
        }
    }
}

/// A pod, specified by `pods/$NAME.yml` and zero or more
/// `pods/overrides/*/*.yml` overrides that we can apply to it.
#[derive(Debug)]
pub struct Pod {
    /// All paths in any associated `dc::File` should be intepreted
    /// relative to this base, including paths in overlay files.
    base_dir: PathBuf,

    /// The name of this pod, based on the file `pods/$NAME.yml`.
    name: String,

    /// The top-level file defining this pod.
    file_info: FileInfo,

    /// The individual override files for this pod.  There will always be a
    /// sensible value here for each pod, even if the file doesn't exist on
    /// disk.
    override_file_infos: BTreeMap<String, FileInfo>,
}

impl Pod {
    /// Create a new pod, specifying the base directory from which we'll load
    /// pod definitions and the name of the pod.
    #[doc(hidden)]
    pub fn new<P, S>(base_dir: P, name: S, overrides: &[Override]) ->
        Result<Pod, Error>
        where P: Into<PathBuf>, S: Into<String>
    {
        let base_dir = base_dir.into();
        let name = name.into();

        // Load our main `*.yml` file.
        let rel_path = Path::new(&format!("{}.yml", &name)).to_owned();
        let mut file_info = try!(FileInfo::unnormalized(&base_dir, &rel_path));
        file_info.finish_normalization();

        // Load our override `*.yml` files.
        let mut ovr_infos = BTreeMap::new();
        for ovr in overrides {
            let ovr_rel_path =
                Path::new(&format!("overrides/{}/{}.yml", ovr.name(), &name))
                    .to_owned();
            let mut ovr_info =
                try!(FileInfo::unnormalized(&base_dir, &ovr_rel_path));
            ovr_info.ensure_all_services_from(&file_info.file);
            ovr_info.finish_normalization();
            ovr_infos.insert(ovr.name().to_owned(), ovr_info);
        }

        Ok(Pod {
            base_dir: base_dir,
            name: name,
            file_info: file_info,
            override_file_infos: ovr_infos,
        })
    }

    /// Get the name of this pod.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The path to the top-level file defining this pod, relative to the
    /// `base_dir` specified at creation time.
    pub fn rel_path(&self) -> &Path {
        &self.file_info.rel_path
    }

    /// The top-level file defining this pod.  This is normalized to
    /// include the appropriate `env_file` entries, but if you want to do
    /// more complicated transformations, you'll need to clone it with
    /// `to_owned()` first.
    pub fn file(&self) -> &dc::File {
        &self.file_info.file
    }

    /// Look up the file info associated with an override, or return an
    /// error if this override was not specified for this `Pod` at creation
    /// time.
    fn override_file_info(&self, ovr: &Override) -> Result<&FileInfo, Error> {
        self.override_file_infos.get(ovr.name()).ok_or_else(|| {
            err!("The override {} is not defined", ovr.name())
        })
    }

    /// The path to the specificied override file for this pod.
    pub fn override_rel_path(&self, ovr: &Override) -> Result<&Path, Error> {
        Ok(&(try!(self.override_file_info(ovr)).rel_path))
    }

    pub fn override_file(&self, ovr: &Override) -> Result<&dc::File, Error> {
        Ok(&(try!(self.override_file_info(ovr)).file))
    }
}

#[test]
fn pods_are_normalized_on_load() {
    use project::Project;

    let proj = Project::from_example("hello").unwrap();
    let frontend = proj.pod("frontend").unwrap();

    let web = frontend.file().services.get("web").unwrap();
    assert_eq!(web.env_files.len(), 1);
    assert_eq!(web.env_files[0].value().unwrap(),
               Path::new("common.env"));

    // This test assumes that there's no `web` entry in the `production`
    // override, so we have to create everything from scratch.
    let production = proj.ovr("production").unwrap();
    let web_ovr = frontend.override_file(production).unwrap()
        .services.get("web").unwrap();
    assert_eq!(web_ovr.env_files.len(), 1);
    assert_eq!(web_ovr.env_files[0].value().unwrap(),
               Path::new("overrides/production/common.env"));
}
