//! A single pod in a project.

use docker_compose::v2 as dc;
use docker_compose::v2::MergeOverride;
use serde_yaml;
use std::collections::BTreeMap;
use std::collections::btree_map;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ovr::Override;
use project::Project;
use util::Error;

// Include some source code containing data structures we need to run
// through serde.
#[cfg(feature = "serde_macros")]
include!(concat!("pod_config.in.rs"));
#[cfg(feature = "serde_codegen")]
include!(concat!(env!("OUT_DIR"), "/pod_config.rs"));

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
    /// The path to this file relative to `pods/`.
    rel_path: PathBuf,
    /// Either the data we loaded from file, or default data if the file
    /// didn't exist.
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
                debug!("Parsing {}", path.display());
                try!(dc::File::read_from_path(&path).map_err(|e| {
                    // Make sure we tie parse errors to a specific file, for
                    // the sake of sanity.
                    err!("Error parsing {}: {}", path.display(), e)
                }))
            } else {
                Default::default()
            },
        })
    }

    /// Make sure that all services from `base` are also present in this
    /// file.  If you're going tp call this, it must be called after
    /// `finish_normalization`
    fn ensure_all_services_from(&mut self, base: &dc::File) {
        for name in base.services.keys() {
            self.file
                .services
                .entry(name.to_owned())
                .or_insert_with(Default::default);
        }
    }

    /// Finish normalizing this file by inserting things like `env_file`
    /// entries.
    fn finish_normalization(&mut self) {
        // It's safe to call `unwrap` here because we know `rel_path` should
        // have a parent directory.
        let env_path = self.rel_path.parent().unwrap().join("common.env");
        for service in self.file.services.values_mut() {
            service.env_files.insert(0, dc::value(env_path.clone()));
        }
    }
}

impl FromStr for PodType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "service" => Ok(PodType::Service),
            "task" => Ok(PodType::Task),
            _ => Err(err!("Unknown pod type: <{}>", s)),
        }
    }
}

/// A pod, specified by `pods/$NAME.yml` and zero or more
/// `pods/overrides/*/*.yml` overrides that we can apply to it.
#[derive(Debug)]
pub struct Pod {
    /// All paths in any associated `dc::File` should be intepreted
    /// relative to this base, including paths in override files.
    base_dir: PathBuf,

    /// The name of this pod, based on the file `pods/$NAME.yml`.
    name: String,

    /// The top-level file defining this pod.
    file_info: FileInfo,

    /// The individual override files for this pod.  There will always be a
    /// sensible value here for each pod, even if the file doesn't exist on
    /// disk.
    override_file_infos: BTreeMap<Override, FileInfo>,

    /// Per-pod configuration.
    config: Config,
}

impl Pod {
    /// Create a new pod, specifying the base directory from which we'll load
    /// pod definitions and the name of the pod.
    #[doc(hidden)]
    pub fn new<P, S>(base_dir: P,
                     name: S,
                     overrides: &[Override])
                     -> Result<Pod, Error>
        where P: Into<PathBuf>,
              S: Into<String>
    {
        let base_dir = base_dir.into();
        let name = name.into();

        // Load our `*.metadata.yml` file, if any.
        let config_path = base_dir.join(&format!("{}.metadata.yml", &name));
        let config: Config = if config_path.exists() {
            let f = try!(fs::File::open(&config_path)
                .map_err(|e| err!("Error opening {}: {}", &config_path.display(), e)));
            try!(serde_yaml::from_reader(f)
                .map_err(|e| err!("Error reading {}: {}", &config_path.display(), e)))
        } else {
            Config::default()
        };

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
            let mut ovr_info = try!(FileInfo::unnormalized(&base_dir, &ovr_rel_path));
            ovr_info.ensure_all_services_from(&file_info.file);
            ovr_info.finish_normalization();
            ovr_infos.insert(ovr.to_owned(), ovr_info);
        }


        Ok(Pod {
            base_dir: base_dir,
            name: name,
            file_info: file_info,
            override_file_infos: ovr_infos,
            config: config,
        })
    }

    /// Get the name of this pod.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the type of this pod.
    pub fn pod_type(&self) -> PodType {
        self.config.pod_type.unwrap_or(PodType::Service)
    }

    /// Is this pod enabled in the specified override?
    pub fn enabled_in(&self, ovr: &Override) -> bool {
        ovr.included_by(&self.config.only_in_overrides)
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
        self.override_file_infos
            .get(ovr)
            .ok_or_else(|| err!("The override {} is not defined", ovr.name()))
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
    pub fn compose_args(&self,
                        proj: &Project,
                        ovr: &Override)
                        -> Result<Vec<OsString>, Error> {
        let compose_project_name = ovr.compose_project_name(proj);
        Ok(vec!["-p".into(),
                compose_project_name.into(),
                "-f".into(),
                proj.output_pods_dir().join(self.rel_path()).into()])
    }
}

/// An iterator over this pods overrides and their associated files.
#[allow(missing_debug_implementations)]
pub struct OverrideFiles<'a> {
    /// Our wrapped iterator.
    iter: btree_map::Iter<'a, Override, FileInfo>,
}

impl<'a> Iterator for OverrideFiles<'a> {
    type Item = (&'a Override, &'a dc::File);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(ovr, file_info)| (ovr, &file_info.file))
    }
}

/// What should we yield next from our `AllFiles` iterator?
#[allow(missing_debug_implementations)]
enum AllFilesState<'a> {
    /// Yield the top-level `file()` next.
    TopLevelFile,
    /// Yield an item from this iterator next.
    OverrideFiles(OverrideFiles<'a>),
}

/// An iterator over all the `dc::File` objects associated with a pod, in
/// all overrides.
#[allow(missing_debug_implementations)]
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
                iter.next().map(|(_, file)| file)
            }
        }
    }
}

#[test]
fn pods_are_normalized_on_load() {
    use env_logger;
    use project::Project;
    let _ = env_logger::init();

    let proj = Project::from_example("hello").unwrap();
    let frontend = proj.pod("frontend").unwrap();

    let web = frontend.file().services.get("web").unwrap();
    assert_eq!(web.env_files.len(), 1);
    assert_eq!(web.env_files[0].value().unwrap(), Path::new("common.env"));

    // This test assumes that there's no `web` entry in the `production`
    // override, so we have to create everything from scratch.
    let production = proj.ovr("production").unwrap();
    let web_ovr = frontend.override_file(production)
        .unwrap()
        .services
        .get("web")
        .unwrap();
    assert_eq!(web_ovr.env_files.len(), 1);
    assert_eq!(web_ovr.env_files[0].value().unwrap(),
               Path::new("overrides/production/common.env"));
}

#[test]
fn can_merge_base_file_and_override() {
    use env_logger;
    let _ = env_logger::init();
    let proj: Project = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(ovr).unwrap();
    let proxy = merged.services.get("proxy").unwrap();
    assert_eq!(proxy.env_files.len(), 2);
}

#[test]
fn pod_type_returns_type_of_pod() {
    use env_logger;
    let _ = env_logger::init();
    let proj: Project = Project::from_example("rails_hello").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    assert_eq!(frontend.pod_type(), PodType::Service);
    let migrate = proj.pod("migrate").unwrap();
    assert_eq!(migrate.pod_type(), PodType::Task);
}
