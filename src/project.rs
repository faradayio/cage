//! A cage project.

#[cfg(test)]
use compose_yml::v2 as dc;
use semver;
use serde_yaml;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::io::Read;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::slice;
use std::str;

use default_tags::DefaultTags;
use dir;
use errors::*;
use hook::HookManager;
use target::Target;
use plugins::{self, Operation};
use pod::{Pod, PodType};
use sources::Sources;
use rayon::prelude::*;
use rustc_serialize::json::{Json, ToJson};
use serde_helpers::deserialize_parsable_opt;
use service_locations::ServiceLocations;
use util::{ConductorPathExt, ToStrOrErr};
use version;

// Include some source code containing data structures we need to run
// through serde.
#[cfg(feature = "serde_derive")]
include!(concat!("project_config.in.rs"));
#[cfg(feature = "serde_codegen")]
include!(concat!(env!("OUT_DIR"), "/project_config.rs"));

/// Represents either a `Pod` object or a `Service` object.
#[derive(Debug)]
pub enum PodOrService<'a> {
    /// A `Pod`.
    Pod(&'a Pod),
    /// A `Pod` and the name of one of its `Service` objects.
    Service(&'a Pod, &'a str),
}

/// A `cage` project, which is represented as a directory containing a
/// `pods` subdirectory.
#[derive(Debug)]
pub struct Project {
    /// The name of this project.  This defaults to the name of the
    /// directory containing the project, but it can be targetn, just
    /// like with `docker-compose`.
    name: String,

    /// The directory which contains our `project`.  Must have a
    /// subdirectory named `pods`.
    root_dir: PathBuf,

    /// Where we keep cloned git repositories.
    src_dir: PathBuf,

    /// The directory to which we'll write our transformed pods.  Defaults
    /// to `root_dir.join(".cage")`.
    output_dir: PathBuf,

    /// All the pods associated with this project.
    pods: Vec<Pod>,

    /// Mappings from user-visible service names to `(pod, service)` pairs.
    service_locations: ServiceLocations,

    /// All the targets associated with this project.
    targets: Vec<Target>,

    /// The target that we're currently using.  Applies to most
    /// operations.
    current_target: Target,

    /// All the source trees associated with this project.
    sources: Sources,

    /// User-specific hooks that we can call before or after certain actions.
    hooks: HookManager,

    /// The main configuration for this project.
    config: ProjectConfig,

    /// Docker image tags to use for images that don't have them.
    /// Typically used to lock down versions supplied by a CI system.
    default_tags: Option<DefaultTags>,

    /// The plugins associated with this project.  Guaranteed to never be
    /// `None` after returning from `from_dirs`.
    plugins: Option<plugins::Manager>,
}

impl Project {
    /// Create a `Project`, specifying what directories to use.
    fn from_dirs(root_dir: &Path,
                 src_dir: &Path,
                 output_dir: &Path)
                 -> Result<Project> {
        let targets = try!(Project::find_targets(root_dir));
        let current_target = try!(targets.iter()
                .find(|target| target.name() == "development")
                .ok_or_else(|| ErrorKind::UnknownTarget("development".into())))
            .to_owned();
        let pods = try!(Project::find_pods(root_dir, &targets));
        let service_locations = ServiceLocations::new(&pods);
        let sources = try!(Sources::new(&root_dir, &output_dir, &pods));
        let config_path = root_dir.join(PROJECT_CONFIG_PATH.deref());
        let config = try!(ProjectConfig::new(&config_path));
        let absolute_root = try!(root_dir.to_absolute());
        let name = try!(absolute_root.file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    err!("Can't find directory name for {}", root_dir.display())
                }));
        let mut proj = Project {
            name: name.to_owned(),
            root_dir: root_dir.to_owned(),
            src_dir: src_dir.to_owned(),
            output_dir: output_dir.to_owned(),
            pods: pods,
            service_locations: service_locations,
            targets: targets,
            current_target: current_target,
            sources: sources,
            hooks: try!(HookManager::new(root_dir.join("config").join("hooks"))),
            config: config,
            default_tags: None,
            plugins: None,
        };
        let plugins = try!(plugins::Manager::new(&proj));
        proj.plugins = Some(plugins);
        Ok(proj)
    }

    /// Create a `Project` using the pre-existing project files in the
    /// current directory as input and the `.cage` subdirectory as
    /// output.
    ///
    /// ```
    /// use cage::Project;
    /// use std::env;
    ///
    /// let saved = env::current_dir().unwrap();
    /// env::set_current_dir("examples/hello/pods").unwrap();
    ///
    /// let proj = Project::from_current_dir().unwrap();
    /// assert_eq!(proj.root_dir(), saved.join("examples").join("hello"));
    /// assert_eq!(proj.src_dir(),
    ///            saved.join("examples").join("hello").join("src"));
    /// assert_eq!(proj.output_dir(),
    ///            saved.join("examples").join("hello").join(".cage"));
    ///
    /// env::set_current_dir(saved).unwrap();
    /// ```
    pub fn from_current_dir() -> Result<Project> {
        // (We can only test this using a doc test because testing it
        // requires messing with `set_current_dir`, which isn't thread safe
        // and will break parallel tests.)
        let current = try!(env::current_dir());
        let root_dir = try!(dir::find_project(&current));
        Project::from_dirs(&root_dir, &root_dir.join("src"), &root_dir.join(".cage"))
    }

    /// (Tests only.) Create a `Project` from a subirectory of `examples`,
    /// with an output directory under `target/test_output/$NAME`.
    #[cfg(test)]
    pub fn from_example(name: &str) -> Result<Project> {
        use rand::random;
        Project::from_example_and_random_id(name, random())
    }

    /// (Tests only.) Create a `Project` from a subirectory of `examples`
    /// and a random ID, with an output directory under
    /// `target/test_output/$NAME`.
    #[cfg(test)]
    pub fn from_example_and_random_id(name: &str, id: u16) -> Result<Project> {
        let root_dir = Path::new("examples").join(name);
        let rand_name = format!("{}-{}", name, id);
        let test_output = Path::new("target/test_output").join(&rand_name);
        Project::from_dirs(&root_dir, &test_output.join("src"), &test_output)
    }

    /// (Tests only.) Remove our output directory after a test.
    #[cfg(test)]
    pub fn remove_test_output(&self) -> Result<()> {
        if self.output_dir.exists() {
            try!(fs::remove_dir_all(&self.output_dir));
        }
        Ok(())
    }

    /// Find all the targets defined in this project.
    fn find_targets(root_dir: &Path) -> Result<Vec<Target>> {
        let targets_dir = root_dir.join("pods").join("targets");
        let mut targets = vec![];
        for glob_result in try!(targets_dir.glob("*")) {
            let path = try!(glob_result);
            if path.is_dir() {
                // It's safe to unwrap file_name because we know it matched
                // our glob.
                let name = try!(path.file_name().unwrap().to_str_or_err()).to_owned();
                targets.push(Target::new(name));
            }
        }
        Ok(targets)
    }

    /// Find all the pods defined in this project.
    fn find_pods(root_dir: &Path, targets: &[Target]) -> Result<Vec<Pod>> {
        let pods_dir = root_dir.join("pods");
        let mut pods = vec![];
        for glob_result in try!(pods_dir.glob("*.yml")) {
            let path = try!(glob_result);
            // It's safe to unwrap the file_stem because we know it matched
            // our glob.
            let name = try!(path.file_stem().unwrap().to_str_or_err()).to_owned();
            if !name.ends_with(".metadata") {
                pods.push(try!(Pod::new(pods_dir.clone(), name, targets)));
            }
        }
        Ok(pods)
    }

    /// The name of this project.  This defaults to the name of the current
    /// directory.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name of this project.  This should be done before calling
    /// `output` or any methods in `cmd`.
    pub fn set_name(&mut self, name: &str) -> &mut Project {
        self.name = name.to_owned();
        self
    }

    /// The root directory of this project.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// The source directory of this project, where we can put cloned git
    /// repositories, and where our local source trees are typically found.
    pub fn src_dir(&self) -> &Path {
        &self.src_dir
    }

    /// The output directory of this project.  Normally `.cage` inside
    /// the `root_dir`, but it may be targetn.
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// The directory in which are pods are defined, and relative to which
    /// all `docker-compose.yml` paths should be interpreted.
    pub fn pods_dir(&self) -> PathBuf {
        self.root_dir.join("pods")
    }

    /// The path relative to which our pods will be output.  This can be
    /// joined with `Pod::rel_path` to get an output path for a specific pod.
    pub fn output_pods_dir(&self) -> PathBuf {
        self.output_dir.join("pods")
    }

    /// Iterate over all pods in this project.
    pub fn pods(&self) -> Pods {
        Pods { iter: self.pods.iter() }
    }

    /// Look up the named pod.
    pub fn pod(&self, name: &str) -> Option<&Pod> {
        // TODO LOW: Do we want to store pods in a BTreeMap by name?
        self.pods().find(|pod| pod.name() == name)
    }

    /// Look up the named service.  Returns the pod containing the service
    /// and the name of the service within that pod.
    pub fn service<'a>(&self, name: &'a str) -> Option<(&Pod, &str)> {
        if let Some((pod_name, service_name)) = self.service_locations.find(name) {
            let pod = self.pod(pod_name).expect("pod should exist");
            Some((pod, service_name))
        } else {
            None
        }
    }

    /// Like `service`, but returns an error if the service is unknown.
    pub fn service_or_err<'a>(&self, name: &'a str) -> Result<(&Pod, &str)> {
        self.service(name)
            .ok_or_else(|| ErrorKind::UnknownService(name.to_owned()).into())
    }

    /// Look for a name as a pod first, and if that fails, look for it as a
    /// service.
    pub fn pod_or_service<'a, 'b>(&'a self,
                                  name: &'b str)
                                  -> Option<PodOrService<'a>> {
        if let Some(pod) = self.pod(name) {
            Some(PodOrService::Pod(pod))
        } else if let Some((pod, service_name)) = self.service(name) {
            Some(PodOrService::Service(pod, service_name))
        } else {
            None
        }
    }

    /// Like `pod_or_service`, but returns an error if no pod or service of
    /// that name can be found.
    pub fn pod_or_service_or_err<'a, 'b>(&'a self,
                                         name: &'b str)
                                         -> Result<PodOrService<'a>> {
        self.pod_or_service(name)
            .ok_or_else(|| ErrorKind::UnknownPodOrService(name.to_owned()).into())
    }

    /// Iterate over all targets in this project.
    pub fn targets(&self) -> Targets {
        Targets { iter: self.targets.iter() }
    }

    /// Look up the named target.  We name this function `target` instead of
    /// `target` to avoid a keyword clash.
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets().find(|target| target.name() == name)
    }

    /// Like `target`, but returns an error if no each target is found.
    pub fn target_or_err(&self, name: &str) -> Result<&Target> {
        self.target(name).ok_or_else(|| ErrorKind::UnknownTarget(name.into()).into())
    }

    /// Get the current target that we're using with this project.
    pub fn current_target(&self) -> &Target {
        &self.current_target
    }

    /// Set the name of the target to use.  This must be done before
    /// calling `output` or `export`.
    pub fn set_current_target_name(&mut self, name: &str) -> Result<()> {
        self.current_target = try!(self.target_or_err(name)).to_owned();
        Ok(())
    }

    /// Return the collection of source trees associated with this project,
    /// including both extern git repositories and local source trees.
    pub fn sources(&self) -> &Sources {
        &self.sources
    }

    /// Return the collection of source trees associated with this project
    /// in mutable form.
    pub fn sources_mut(&mut self) -> &mut Sources {
        &mut self.sources
    }

    /// Get our available hooks.
    pub fn hooks(&self) -> &HookManager {
        &self.hooks
    }

    /// Get the default tags associated with this project, if any.
    pub fn default_tags(&self) -> Option<&DefaultTags> {
        self.default_tags.as_ref()
    }

    /// Set the default tags associated with this project.
    pub fn set_default_tags(&mut self, tags: DefaultTags) -> &mut Project {
        self.default_tags = Some(tags);
        self
    }

    /// Our plugin manager.
    pub fn plugins(&self) -> &plugins::Manager {
        self.plugins
            .as_ref()
            .expect("plugins should always be set at Project init")
    }

    /// Save persistent project settings to disk.
    pub fn save_settings(&mut self) -> Result<()> {
        self.sources.save_settings(&self.output_dir)
    }

    /// Process our pods, flattening and transforming them using our
    /// plugins, and output them to the specified directory.
    fn output_helper(&self, op: Operation, export_dir: &Path) -> Result<()> {
        // Output each pod.  This isn't especially slow (except maybe the
        // Vault plugin), but parallelizing things is easy.
        self.pods.par_iter()
            // Don't export pods which aren't enabled.
            //
            // TODO MED: Should we exclude these at load time instead?
            .filter(|pod| pod.enabled_in(&self.current_target))
            // Process each pod in parallel.
            .map(|pod| -> Result<()> {
                // Figure out where to put our pod.
                let file_name = format!("{}.yml", pod.name());
                let rel_path = match (op, pod.pod_type()) {
                    (Operation::Export, PodType::Task) => {
                        Path::new("tasks").join(file_name)
                    }
                    _ => Path::new(&file_name).to_owned(),
                };
                let out_path = try!(export_dir.join(&rel_path)
                    .with_guaranteed_parent());
                debug!("Outputting {}", out_path.display());

                // Combine targets, make it standalone, tweak as needed, and
                // output.
                let mut file = try!(pod.merged_file(&self.current_target));
                try!(file.make_standalone(&self.pods_dir()));
                let ctx = plugins::Context::new(self, pod);
                try!(self.plugins().transform(op, &ctx, &mut file));
                try!(file.write_to_path(out_path));
                Ok(())
            })
            // If more than one parallel branch fails, just return one error.
            .reduce_with(|result1, result2| result1.and(result2).and(Ok(())))
            .unwrap_or(Ok(()))
    }

    /// Delete our existing output and replace it with a processed and
    /// expanded version of our pod definitions.
    pub fn output(&self) -> Result<()> {
        // Get a path to our output pods directory (and delete it if it
        // exists).
        let out_pods = self.output_pods_dir();
        if out_pods.exists() {
            try!(fs::remove_dir_all(&out_pods)
                .map_err(|e| err!("Cannot delete {}: {}", out_pods.display(), e)));
        }

        self.output_helper(Operation::Output, &out_pods)
    }

    /// Export this project (with the specified target applied) as a set
    /// of standalone `*.yml` files with no environment variable
    /// interpolations and no external dependencies.
    pub fn export(&self, export_dir: &Path) -> Result<()> {
        // Don't clobber an existing directory.
        if export_dir.exists() {
            return Err(err!("The directory {} already exists", export_dir.display()));
        }

        // You should really supply default tags if you're going to export.
        if self.default_tags().is_none() {
            warn!("Exporting project without --default-tags");
        }

        self.output_helper(Operation::Export, export_dir)
    }
}

/// Convert to JSON for use in generator templates.
impl<'a> ToJson for Project {
    fn to_json(&self) -> Json {
        let mut info: BTreeMap<String, Json> = BTreeMap::new();
        info.insert("name".to_string(), self.name().to_json());
        info.to_json()
    }
}

/// An iterator over the pods in a project.
#[derive(Debug, Clone)]
pub struct Pods<'a> {
    /// Our wrapped iterator.  We wrap this in our own struct to make the
    /// underlying type opaque.
    iter: slice::Iter<'a, Pod>,
}

impl<'a> Iterator for Pods<'a> {
    type Item = &'a Pod;

    fn next(&mut self) -> Option<&'a Pod> {
        self.iter.next()
    }
}

/// An iterator over the targets in a project.
#[derive(Debug, Clone)]
pub struct Targets<'a> {
    /// Our wrapped iterator.  We wrap this in our own struct to make the
    /// underlying type opaque.
    iter: slice::Iter<'a, Target>,
}

impl<'a> Iterator for Targets<'a> {
    type Item = &'a Target;

    fn next(&mut self) -> Option<&'a Target> {
        self.iter.next()
    }
}

#[test]
fn new_from_example_uses_example_and_target() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    assert_eq!(proj.root_dir, Path::new("examples/hello"));
    let output_dir = proj.output_dir.to_str_or_err().unwrap();
    assert!(output_dir.starts_with("target/test_output/hello-") ||
            output_dir.starts_with("target/test_output\\hello-"));
    let src_dir = proj.src_dir.to_str_or_err().unwrap();
    assert!(src_dir.starts_with("target/test_output/hello-") ||
            src_dir.starts_with("target/test_output\\hello-"));
}

#[test]
fn name_defaults_to_project_dir_but_can_be_overridden() {
    use env_logger;
    let _ = env_logger::init();
    let mut proj = Project::from_example("hello").unwrap();
    assert_eq!(proj.name(), "hello");
    proj.set_name("hi");
    assert_eq!(proj.name(), "hi");
}

#[test]
fn pod_or_service_finds_either() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();

    match proj.pod_or_service("frontend").unwrap() {
        PodOrService::Pod(pod) => assert_eq!(pod.name(), "frontend"),
        _ => panic!("Did not find pod 'frontend'"),
    }
    match proj.pod_or_service("frontend/web").unwrap() {
        PodOrService::Service(pod, "web") => assert_eq!(pod.name(), "frontend"),
        _ => panic!("Did not find service 'frontend/web'"),
    }
    match proj.pod_or_service("web").unwrap() {
        PodOrService::Service(pod, "web") => assert_eq!(pod.name(), "frontend"),
        _ => panic!("Did not find service 'web'"),
    }
}

#[test]
fn pods_are_loaded() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let names: Vec<_> = proj.pods.iter().map(|pod| pod.name()).collect();
    assert_eq!(names, ["frontend"]);
}

#[test]
fn targets_are_loaded() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let names: Vec<_> = proj.targets.iter().map(|o| o.name()).collect();
    assert_eq!(names, ["development", "production", "test"]);
}

#[test]
fn output_creates_a_directory_of_flat_yml_files() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    proj.output().unwrap();
    assert!(proj.output_dir.join("pods").join("frontend.yml").exists());
    assert!(proj.output_dir.join("pods").join("db.yml").exists());
    assert!(proj.output_dir.join("pods").join("rake.yml").exists());
    proj.remove_test_output().unwrap();
}

#[test]
fn output_applies_expected_transforms() {
    use env_logger;
    let _ = env_logger::init();

    let cursor = io::Cursor::new("dockercloud/hello-world:staging\n");
    let default_tags = DefaultTags::read(cursor).unwrap();

    let mut proj = Project::from_example("hello").unwrap();
    proj.set_default_tags(default_tags);
    let source = proj.sources().find_by_alias("dockercloud-hello-world").unwrap();
    source.fake_clone_source(&proj).unwrap();
    proj.output().unwrap();

    // Load the generated file and look at the `web` service we cloned.
    let frontend_file = proj.output_dir().join("pods").join("frontend.yml");
    let file = dc::File::read_from_path(frontend_file).unwrap();
    let web = file.services.get("web").unwrap();
    let src_path = source.path(&proj).to_absolute().unwrap();

    // Make sure our `build` entry has been pointed at the local source
    // directory.
    assert_eq!(web.build.as_ref().unwrap().context.value().unwrap(),
               &dc::Context::new(src_path.to_str().unwrap()));

    // Make sure the local source directory is being mounted into the
    // container.
    let mount = web.volumes
        .last()
        .expect("expected web service to have volumes")
        .value()
        .unwrap();
    assert_eq!(mount.host, Some(dc::HostVolume::Path(src_path)));
    assert_eq!(mount.container, Path::new("/app"));

    // Make sure that our image versions were correctly defaulted.
    assert_eq!(web.image.as_ref().unwrap().value().unwrap(),
               &dc::Image::new("dockercloud/hello-world:staging").unwrap());

    proj.remove_test_output().unwrap();
}

#[test]
fn output_mounts_cloned_libraries() {
    use env_logger;
    let _ = env_logger::init();

    let proj = Project::from_example("rails_hello").unwrap();
    let source = proj.sources()
        .find_by_lib_key("coffee_rails")
        .expect("should define lib coffee_rails");
    source.fake_clone_source(&proj).unwrap();
    proj.output().unwrap();

    // Load the generated file and look at the `web` service we cloned.
    let frontend_file = proj.output_dir().join("pods").join("frontend.yml");
    let file = dc::File::read_from_path(frontend_file).unwrap();
    let web = file.services.get("web").unwrap();
    let src_path = source.path(&proj).to_absolute().unwrap();

    // Make sure the local source directory is being mounted into the
    // container.
    let mount = web.volumes
        .last()
        .expect("expected web service to have volumes")
        .value()
        .unwrap();
    assert_eq!(mount.host, Some(dc::HostVolume::Path(src_path)));
    assert_eq!(mount.container,
               Path::new("/usr/src/app/vendor/coffee-rails"));
}

#[test]
fn output_supports_in_tree_source_code() {
    let proj = Project::from_example("node_hello").unwrap();
    proj.output().unwrap();

    // Load the generated file and look at the `web` service we cloned.
    let frontend_file = proj.output_dir().join("pods").join("frontend.yml");
    let file = dc::File::read_from_path(frontend_file).unwrap();
    let web = file.services.get("web").unwrap();

    let abs_src = proj.root_dir()
        .join("pods")
        .join("..")
        .join("src")
        .join("node_hello")
        .to_absolute()
        .unwrap();
    assert_eq!(web.build.as_ref().unwrap().context.value().unwrap(),
               &dc::Context::Dir(abs_src));
}

#[test]
fn export_creates_a_directory_of_flat_yml_files() {
    use env_logger;
    let _ = env_logger::init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    let export_dir = proj.output_dir.join("hello_export");
    proj.set_current_target_name("production").unwrap();
    proj.export(&export_dir).unwrap();
    assert!(export_dir.join("frontend.yml").exists());
    assert!(!export_dir.join("db.yml").exists());
    assert!(export_dir.join("tasks").join("rake.yml").exists());
    proj.remove_test_output().unwrap();
}

#[test]
fn export_applies_expected_transforms() {
    use env_logger;
    let _ = env_logger::init();

    // We only test the ways in which `export`'s transforms differ from
    // `output`.

    let proj = Project::from_example("hello").unwrap();
    let source = proj.sources().find_by_alias("dockercloud-hello-world").unwrap();
    source.fake_clone_source(&proj).unwrap();
    let export_dir = proj.output_dir.join("hello_export");
    proj.export(&export_dir).unwrap();

    // Load the generated file and look at the `web` service we cloned.
    let frontend_file = export_dir.join("frontend.yml");
    let file = dc::File::read_from_path(frontend_file).unwrap();
    let web = file.services.get("web").unwrap();

    // Make sure our `build` entry has not been pointed at the local source
    // directory.
    let url = "https://github.com/docker/dockercloud-hello-world.git";
    assert_eq!(web.build.as_ref().unwrap().context.value().unwrap(),
               &dc::Context::new(dc::GitUrl::new(url).unwrap()));

    // Make sure we've added our custom labels.
    assert_eq!(web.labels.get("io.fdy.cage.target"),
               Some(&"development".to_owned()));
    assert_eq!(web.labels.get("io.fdy.cage.pod"),
               Some(&"frontend".to_owned()));
}
