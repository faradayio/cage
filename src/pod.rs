//! A single pod in a project.

use compose_yml::v2 as dc;
use compose_yml::v2::MergeOverride;
use std::collections::{BTreeMap, BTreeSet};
use std::collections::btree_map;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use args;
use cmd::CommandRun;
use command_runner::CommandRunner;
use errors::*;
use target::Target;
use project::Project;
use serde_helpers::load_yaml;

// TODO: This old-style serde `include!` should be inline or a module.
include!("pod_config.in.rs");

/// Information about a `docker-compose.yml` file, including its path
/// relative to `base_dir` (`base_dir` is normally `$PROJECT/pods`), and
/// the normalized version of its contents:
///
/// 1. Any missing services will be explicitly added to an target file.
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
    fn unnormalized(base_dir: &Path, rel_path: &Path) -> Result<FileInfo> {
        let path = base_dir.join(rel_path);
        Ok(FileInfo {
            rel_path: rel_path.to_owned(),
            file: if path.exists() {
                debug!("Parsing {}", path.display());
                dc::File::read_from_path(&path).chain_err(|| {
                        // Make sure we tie parse errors to a specific file, for
                        // the sake of sanity.
                        ErrorKind::CouldNotReadFile(path.clone())
                    })?
            } else {
                Default::default()
            },
        })
    }

    /// Make sure that all services from `base` are also present in this
    /// file.  If you're going tp call this, it must be called after
    /// `finish_normalization`
    fn ensure_same_services(&mut self,
                            base_file: &Path,
                            service_names: &BTreeSet<String>)
                            -> Result<()> {
        // Check for any newly-introduced services.  These are problematic
        // because (1) in our previous experience, they lead to really
        // confusing and unmaintanable targets, and (2) the rest of this
        // program's design assumes it doesn't have to worry about them.
        let ours: BTreeSet<String> = self.file.services.keys().cloned().collect();
        let introduced: Vec<String> =
            ours.difference(service_names).cloned().collect();
        if !introduced.is_empty() {
            return Err(ErrorKind::ServicesAddedInTarget(base_file.to_owned(),
                                                        self.rel_path.clone(),
                                                        introduced)
                .into());
        }

        // Add any missing services.
        for name in service_names {
            self.file
                .services
                .entry(name.to_owned())
                .or_insert_with(Default::default);
        }
        Ok(())
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

/// A pod, specified by `pods/$NAME.yml` and zero or more
/// `pods/targets/*/*.yml` targets that we can apply to it.
#[derive(Debug)]
pub struct Pod {
    /// All paths in any associated `dc::File` should be intepreted
    /// relative to this base, including paths in target files.
    base_dir: PathBuf,

    /// The name of this pod, based on the file `pods/$NAME.yml`.
    name: String,

    /// The top-level file defining this pod.
    file_info: FileInfo,

    /// The individual target files for this pod.  There will always be a
    /// sensible value here for each pod, even if the file doesn't exist on
    /// disk.
    target_file_infos: BTreeMap<Target, FileInfo>,

    /// Per-pod configuration.
    config: Config,

    /// The names of all the services in this pod.
    service_names: BTreeSet<String>,
}

impl Pod {
    /// Create a new pod, specifying the base directory from which we'll load
    /// pod definitions and the name of the pod.
    #[doc(hidden)]
    pub fn new<P, S>(base_dir: P, name: S, targets: &[Target]) -> Result<Pod>
        where P: Into<PathBuf>,
              S: Into<String>
    {
        let base_dir = base_dir.into();
        let name = name.into();

        // Load our `*.metadata.yml` file, if any.
        let config_path = base_dir.join(&format!("{}.metadata.yml", &name));
        let config: Config = if config_path.exists() {
            load_yaml(&config_path)?
        } else {
            Config::default()
        };

        // Load our main `*.yml` file.
        let rel_path = Path::new(&format!("{}.yml", &name)).to_owned();
        let mut file_info = FileInfo::unnormalized(&base_dir, &rel_path)?;
        file_info.finish_normalization();
        let service_names = file_info.file.services.keys().cloned().collect();

        // Load our target `*.yml` files.
        let mut target_infos = BTreeMap::new();
        for target in targets {
            let target_rel_path =
                Path::new(&format!("targets/{}/{}.yml", target.name(), &name))
                    .to_owned();
            let mut target_info = FileInfo::unnormalized(&base_dir, &target_rel_path)?;
            target_info.ensure_same_services(&rel_path, &service_names)?;
            target_info.finish_normalization();
            target_infos.insert(target.to_owned(), target_info);
        }

        Ok(Pod {
            base_dir: base_dir,
            name: name,
            file_info: file_info,
            target_file_infos: target_infos,
            config: config,
            service_names: service_names,
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

    /// Get the names of the services declared in this pod.
    pub fn service_names(&self) -> &BTreeSet<String> {
        &self.service_names
    }

    /// Is this pod enabled in the specified target?
    pub fn enabled_in(&self, target: &Target) -> bool {
        target.is_enabled_by(&self.config.enable_in_targets)
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

    /// Look up the file info associated with an target, or return an
    /// error if this target was not specified for this `Pod` at creation
    /// time.
    fn target_file_info(&self, target: &Target) -> Result<&FileInfo> {
        self.target_file_infos
            .get(target)
            .ok_or_else(|| err!("The target {} is not defined", target.name()))
    }

    /// The path to the specificied target file for this pod.
    pub fn target_rel_path(&self, target: &Target) -> Result<&Path> {
        Ok(&(self.target_file_info(target)?.rel_path))
    }

    /// The `dc::File` for this target.
    pub fn target_file(&self, target: &Target) -> Result<&dc::File> {
        Ok(&(self.target_file_info(target)?.file))
    }

    /// Return the base file and the target file merged into a single
    /// `docker-compose.yml` file.
    pub fn merged_file(&self, target: &Target) -> Result<dc::File> {
        // This is expensive so log it.
        debug!("Merging pod {} with target {}", self.name(), target.name());
        Ok(self.file().merge_override(self.target_file(target)?))
    }

    /// All the targets associated with this pod.
    pub fn target_files(&self) -> TargetFiles {
        TargetFiles { iter: self.target_file_infos.iter() }
    }

    /// Iterate over all `dc::File` objects associated with this pod, including
    /// both the main `file()` and all the files in `target_files()`.
    pub fn all_files(&self) -> AllFiles {
        // Defer all the hard work to our iterator type.
        AllFiles {
            pod: self,
            state: AllFilesState::TopLevelFile,
        }
    }

    /// Look up a service by name.
    pub fn service(&self, target: &Target, name: &str) -> Result<Option<dc::Service>> {
        let file = self.merged_file(target)?;
        Ok(file.services.get(name).cloned())
    }

    /// Like `service`, but returns an error if the service can't be found.
    pub fn service_or_err(&self, target: &Target, name: &str) -> Result<dc::Service> {
        self.service(target, name)?
            .ok_or_else(|| ErrorKind::UnknownService(name.to_owned()).into())
    }

    /// Command-line `-p` and `-f` arguments that we'll pass to
    /// `docker-compose` to describe this file.
    pub fn compose_args(&self, proj: &Project) -> Result<Vec<OsString>> {
        Ok(vec!["-p".into(),
                proj.compose_name().into(),
                "-f".into(),
                proj.output_pods_dir().join(self.rel_path()).into()])
    }

    /// The commands we should run to initialize this pod.
    pub fn run_on_init(&self) -> &[Vec<String>] {
        &self.config.run_on_init
    }

    /// Run a named script for specified service name
    pub fn run_script<CR>(&self,
                          runner: &CR,
                          project: &Project,
                          service_name: &str,
                          script_name: &str,
                          opts: &args::opts::Run
                          ) -> Result<()>
        where CR: CommandRunner
    {
        self.config.run_script(runner, &project, &service_name, &script_name, &opts)
    }
}

/// An iterator over this pods targets and their associated files.
#[allow(missing_debug_implementations)]
pub struct TargetFiles<'a> {
    /// Our wrapped iterator.
    iter: btree_map::Iter<'a, Target, FileInfo>,
}

impl<'a> Iterator for TargetFiles<'a> {
    type Item = (&'a Target, &'a dc::File);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(target, file_info)| (target, &file_info.file))
    }
}

/// What should we yield next from our `AllFiles` iterator?
#[allow(missing_debug_implementations)]
enum AllFilesState<'a> {
    /// Yield the top-level `file()` next.
    TopLevelFile,
    /// Yield an item from this iterator next.
    TargetFiles(TargetFiles<'a>),
}

/// An iterator over all the `dc::File` objects associated with a pod, in
/// all targets.
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
        //     .chain(pod.target_files().map(|(_, file)| file))
        // ```
        //
        // ...and storing the result in our object, but the type of that
        // expression is exquisitely hideous and we'd go mad.
        match self.state {
            AllFilesState::TopLevelFile => {
                self.state = AllFilesState::TargetFiles(self.pod.target_files());
                Some(self.pod.file())
            }
            AllFilesState::TargetFiles(ref mut iter) => {
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
    // target, so we have to create everything from scratch.
    let production = proj.target("production").unwrap();
    let web_target = frontend.target_file(production)
        .unwrap()
        .services
        .get("web")
        .unwrap();
    assert_eq!(web_target.env_files.len(), 1);
    assert_eq!(web_target.env_files[0].value().unwrap(),
               Path::new("targets/production/common.env"));
}

#[test]
fn can_merge_base_file_and_target() {
    use env_logger;
    let _ = env_logger::init();
    let proj: Project = Project::from_example("hello").unwrap();
    let target = proj.target("development").unwrap();
    let frontend = proj.pod("frontend").unwrap();
    let merged = frontend.merged_file(target).unwrap();
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
    let rake = proj.pod("rake").unwrap();
    assert_eq!(rake.pod_type(), PodType::Task);
}
