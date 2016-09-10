//! A single pod in a project.

use docker_compose::v2 as dc;
use docker_compose::v2::MergeOverride;
use std::collections::BTreeMap;
use std::collections::btree_map;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ovr::Override;
use project::Project;
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
    override_file_infos: BTreeMap<Override, FileInfo>,
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
            ovr_infos.insert(ovr.to_owned(), ovr_info);
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

    /// The base directory for our relative paths.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
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
        self.override_file_infos.get(ovr).ok_or_else(|| {
            err!("The override {} is not defined", ovr.name())
        })
    }

    /// The path to the specificied override file for this pod.
    pub fn override_rel_path(&self, ovr: &Override) -> Result<&Path, Error> {
        Ok(&(try!(self.override_file_info(ovr)).rel_path))
    }

    /// The `dc::File` for this override.
    pub fn override_file(&self, ovr: &Override) -> Result<&dc::File, Error> {
        Ok(&(try!(self.override_file_info(ovr)).file))
    }

    /// Return the base file and the override file merged into a single
    /// `docker-compose.yml` file.
    pub fn merged_file(&self, ovr: &Override) -> Result<dc::File, Error> {
        // This is expensive so log it.
        debug!("Merging pod {} with override {}", self.name(), ovr.name());
        Ok(self.file().merge_override(try!(self.override_file(ovr))))
    }

    /// All the overrides associated with this pod.
    pub fn override_files(&self) -> OverrideFiles {
        OverrideFiles { iter: self.override_file_infos.iter() }
    }

    /// Iterate over all `dc::File` objects associated with this pod, including
    /// both the main `file()` and all the files in `override_files()`.
    pub fn all_files(&self) -> AllFiles {
        // Defer all the hard work to our iterator type.
        AllFiles {
            pod: self,
            state: AllFilesState::TopLevelFile,
        }
    }

    /// Command-line `-p` and `-f` arguments that we'll pass to
    /// `docker-compose` to describe this file.
    pub fn compose_args(&self, proj: &Project, ovr: &Override) ->
        Result<Vec<OsString>, Error>
    {
        let ovr_rel_path = try!(self.override_rel_path(ovr));
        let compose_project_name = ovr.compose_project_name(self);
        Ok(vec!("-p".into(), compose_project_name.into(),
                "-f".into(), proj.output_pods_dir().join(self.rel_path()).into(),
                "-f".into(), proj.output_pods_dir().join(ovr_rel_path).into()))
    }
}

/// An iterator over this pods overrides and their associated files.
pub struct OverrideFiles<'a> {
    iter: btree_map::Iter<'a, Override, FileInfo>,
}

impl<'a> Iterator for OverrideFiles<'a> {
    type Item = (&'a Override, &'a dc::File);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(ovr, file_info)| (ovr, &file_info.file))
    }
}

/// What should we yield next from our AllFiles iterator?
enum AllFilesState<'a> {
    /// Yield the top-level `file()` next.
    TopLevelFile,
    /// Yield an item from this iterator next.
    OverrideFiles(OverrideFiles<'a>),
}

/// An iterator over all the `dc::File` objects associated with a pod, in
/// all overlays.
pub struct AllFiles<'a> {
    /// The pod whose files we're iterating over.
    pod: &'a Pod,
    /// Our current iteration state.
    state: AllFilesState<'a>,
}

impl<'a> Iterator for AllFiles<'a> {
    type Item = &'a dc::File;

    fn next(&mut self) -> Option<Self::Item> {
        // We could try to implement this by calling:
        //
        // ```
        // iter::once(pod.file())
        //     .chain(pod.override_files().map(|(_, file)| file))
        // ```
        //
        // ...and storing the result in our object, but the type of that
        // expression is exquisitely hideous and we'd go mad.
        match self.state {
            AllFilesState::TopLevelFile => {
                self.state = AllFilesState::OverrideFiles(self.pod.override_files());
                Some(self.pod.file())
            }
            AllFilesState::OverrideFiles(ref mut iter) => {
                iter.next().map(|(_, file)|  file)
            }
        }
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

#[test]
fn can_merge_base_file_and_override() {
    let proj: Project = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(&ovr).unwrap();
    let proxy = merged.services.get("proxy").unwrap();
    assert_eq!(proxy.env_files.len(), 2);
}
