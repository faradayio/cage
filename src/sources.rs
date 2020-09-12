//! APIs for working with the source code associated with a `Project`'s
//! Docker images.

use compose_yml::v2 as dc;
use std::collections::btree_map;
use std::collections::BTreeMap;
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
use crate::command_runner::TestCommandRunner;
use crate::command_runner::{Command, CommandRunner};
use crate::errors::*;
use crate::ext::context::ContextExt;
use crate::ext::git_url::GitUrlExt;
use crate::ext::service::ServiceExt;
use crate::pod::Pod;
#[cfg(test)]
use crate::project::Project;
use crate::serde_helpers::{dump_yaml, load_yaml};
use crate::util::ConductorPathExt;

/// The file where we define extra source trees for libraries used by our
/// services.
const SOURCES_YML: &str = "config/sources.yml";

/// The file where we store our `mounted` state.
const MOUNTED_YML: &str = "mounted.yml";

/// Configuration for an individual source tree.
#[derive(Debug, Clone, Deserialize)]
struct SourceConfig {
    /// The local or remote `context` for this source tree.  We don't
    /// really want to use `dc::RawOr` here, but it's the easiest way to
    /// get this to work with serde, because that's how it works in
    /// `docker-compose.yml` files, and that's what our `compose_yml`
    /// library supports.
    context: dc::RawOr<dc::Context>,
}

/// Project-related directories needed by `sources`.
///
/// We break these out into their own struct so that `Sources` doesn't need to
/// depend on `Project`, which makes the borrow-checker much happier.
#[derive(Debug)]
#[non_exhaustive]
pub struct SourcesDirs {
    /// The `src/` directory associated with the project, for resolving paths to
    /// sources stored in external repositories.
    pub src_dir: PathBuf,
    /// The `pods/` directory associated with the project, for resolving paths
    /// to sources stored in the base repo.
    pub pods_dir: PathBuf,
}

/// All the source trees associated with a project's Docker images.
#[derive(Debug)]
pub struct Sources {
    /// Our source trees, indexed by their local alias.
    sources: BTreeMap<String, Source>,

    /// A map from keys in `config/sources.yml` to source tree
    /// aliases.
    lib_keys: BTreeMap<String, String>,
}

impl Sources {
    /// Add a source tree to a map, keyed by its alias.  Returns the alias.
    fn add_source(
        sources: &mut BTreeMap<String, Source>,
        mounted_sources: &BTreeMap<String, bool>,
        context: &dc::Context,
    ) -> Result<String> {
        // Figure out what alias we want to use.
        let alias = context.human_alias()?;

        // Look up whether we've mounted this container or not.
        let mounted = mounted_sources.get(&alias).cloned().unwrap_or(false);

        // Build our Source object. If two services share a git repo but
        // use different subdirectories, we only create a single Source
        // object, which we ensure by stripping the subdirectory part of
        // any git URL when creating the Source. This also prevents us
        // from trying to clone an invalid git URL containing a subdir.
        let source = Source {
            alias: alias.clone(),
            context: context.without_repository_subdirectory(),
            mounted,
        };

        // Insert our Source object into our map, checking for alias
        // clashes.
        match sources.entry(source.alias.clone()) {
            btree_map::Entry::Vacant(vacant) => {
                vacant.insert(source);
            }
            btree_map::Entry::Occupied(occupied) => {
                if source.context != occupied.get().context {
                    return Err(err!(
                        "{} and {} would both alias to \
                         {}",
                        &occupied.get().context,
                        &source.context,
                        &source.alias
                    ));
                }
            }
        }
        Ok(alias)
    }

    /// Create a collection of source trees based on a list of pods and our
    /// configuration files.
    #[doc(hidden)]
    pub fn new(root_dir: &Path, output_dir: &Path, pods: &[Pod]) -> Result<Sources> {
        let mut sources: BTreeMap<String, Source> = BTreeMap::new();
        let mut lib_keys: BTreeMap<String, String> = BTreeMap::new();

        // Load our `mounted` state, if we've saved it previously.
        let mounted_path = output_dir.join(MOUNTED_YML);
        let mounted: BTreeMap<String, bool> = if mounted_path.exists() {
            load_yaml(&mounted_path)?
        } else {
            Default::default()
        };

        // Scan our pods for dc::Context objects.
        for pod in pods {
            for file in pod.all_files() {
                for service in file.services.values() {
                    if let Some(context) = service.context()? {
                        Self::add_source(&mut sources, &mounted, context)?;
                    }
                }
            }
        }

        // Scan our config files for more source trees.
        let path = root_dir.join(SOURCES_YML);
        if path.exists() {
            let libs: BTreeMap<String, SourceConfig> = load_yaml(&path)?;
            for (lib_key, lib_info) in &libs {
                let context = lib_info.context.value()?;
                if *context != context.without_repository_subdirectory() {
                    // We might actually be able to handle this case, but lib sources
                    // are already awkward enough without adding more features.
                    return Err(
                        ErrorKind::LibHasRepoSubdirectory(lib_key.clone()).into()
                    );
                }
                let alias = Self::add_source(&mut sources, &mounted, context)?;
                lib_keys.insert(lib_key.clone(), alias);
            }
        }

        Ok(Sources { sources, lib_keys })
    }

    /// Iterate over all source trees associated with this project.
    ///
    /// TODO LOW: Replace with IntoIterator.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            iter: self.sources.iter(),
        }
    }

    /// Iterate over all source trees associated with this project.
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut {
            iter: self.sources.iter_mut(),
        }
    }

    /// Look up a source tree using the short-form local alias.
    pub fn find_by_alias(&self, alias: &str) -> Option<&Source> {
        self.sources.get(alias)
    }

    /// Look up a source tree mutably using the short-form local alias.
    pub fn find_by_alias_mut(&mut self, alias: &str) -> Option<&mut Source> {
        self.sources.get_mut(alias)
    }

    /// Look up a source tree given a git URL.
    pub fn find_by_context(&self, context: &dc::Context) -> Option<&Source> {
        self.sources.values().find(|r| r.context() == context)
    }

    /// Look up a source tree using a "lib key", which is key used in
    /// `config/sources.yml` and with service labels of the form
    /// `io.fdy.cage.lib.<KEY>`.
    pub fn find_by_lib_key(&self, lib_key: &str) -> Option<&Source> {
        match self.lib_keys.get(lib_key) {
            Some(alias) => self.find_by_alias(&alias),
            None => None,
        }
    }

    /// Look up a source tree using a "lib key", which is key used in
    /// `config/sources.yml` and with service labels of the form
    /// `io.fdy.cage.lib.<KEY>`.
    pub fn find_by_lib_key_mut(&mut self, lib_key: &str) -> Option<&mut Source> {
        match self.lib_keys.get(lib_key) {
            Some(alias) => {
                let alias = alias.to_owned();
                self.find_by_alias_mut(&alias)
            }
            None => None,
        }
    }

    /// Save any state that we want to persist until the next run.
    pub fn save_settings(&self, out_dir: &Path) -> Result<()> {
        let mut mounted = BTreeMap::new();
        for source in self.iter() {
            // Only record non-default mount values.
            if source.mounted() {
                mounted.insert(source.alias(), source.mounted());
            }
        }
        dump_yaml(&out_dir.join(MOUNTED_YML), &mounted)?;

        Ok(())
    }
}

/// An iterator over all source trees associated with this project.
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct Iter<'a> {
    /// Our wrapped iterator.  We wrap this in our own struct to make the
    /// underlying type opaque.
    iter: btree_map::Iter<'a, String, Source>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Source;

    fn next(&mut self) -> Option<&'a Source> {
        self.iter.next().map(|(_alias, source)| source)
    }
}

/// A mutable iterator over all source trees associated with this project.
#[allow(missing_debug_implementations)]
pub struct IterMut<'a> {
    /// Our wrapped iterator.  We wrap this in our own struct to make the
    /// underlying type opaque.
    iter: btree_map::IterMut<'a, String, Source>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = &'a mut Source;

    fn next(&mut self) -> Option<&'a mut Source> {
        self.iter.next().map(|(_alias, source)| source)
    }
}

/// A single source tree.
#[derive(Debug)]
pub struct Source {
    /// A short name for this source tree.
    alias: String,
    /// The remote location from which we can clone this source tree, or
    /// the local directory where we can find it.
    context: dc::Context,
    /// Should this source tree be mounted into all of the containers that
    /// use it?
    mounted: bool,
}

impl Source {
    /// A short local name for this source tree, suitable for use as
    /// a directory name or command-line argument.
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// The remote git URL from which we can clone this source tree.
    pub fn context(&self) -> &dc::Context {
        &self.context
    }

    /// Should this source tree be mounted into the appropriate containers?
    pub fn mounted(&self) -> bool {
        self.mounted
    }

    /// Set whether this source tree should be mounted into the appropriate
    /// containers?
    pub fn set_mounted(&mut self, mounted: bool) {
        self.mounted = mounted;
    }

    /// The full path to where we expect any local copies of this code to
    /// live.  This will either be the location where we will check out a
    /// git repository, or the path to the actual source tree, depending on
    /// what type of `Context` object we're dealing with.
    ///
    /// The `project` argument is mandatory because we can't store a pointer
    /// to it without creating a circular reference loop.
    pub fn path(&self, dirs: &SourcesDirs) -> PathBuf {
        match self.context {
            dc::Context::GitUrl(_) => dirs.src_dir.join(Path::new(self.alias())),
            dc::Context::Dir(ref path) => dirs.pods_dir.join(path),
        }
    }

    /// Has this project been cloned locally?
    pub fn is_available_locally(&self, dirs: &SourcesDirs) -> bool {
        self.path(dirs).exists()
    }

    /// Clone the source code of this repository using git.
    pub fn clone_source<CR>(&mut self, runner: &CR, dirs: &SourcesDirs) -> Result<()>
    where
        CR: CommandRunner,
    {
        if let dc::Context::GitUrl(ref git_url) = self.context {
            let dest = self.path(dirs).with_guaranteed_parent()?;
            runner
                .build("git")
                .arg("clone")
                .args(&git_url.clone_args()?)
                .arg(&dest)
                .exec()?;
            self.set_mounted(true);
            Ok(())
        } else {
            Err(format!("'{}' is not a git repository", &self.context).into())
        }
    }

    /// (Test mode only.) Pretend to clone the source code for this
    /// repository by creating an empty directory in the right place.
    #[cfg(test)]
    pub fn fake_clone_source(&mut self, dirs: &SourcesDirs) -> Result<()> {
        fs::create_dir_all(self.path(dirs))?;
        self.set_mounted(true);
        Ok(())
    }
}

#[test]
fn are_loaded_with_projects() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("hello").unwrap();
    let sources = proj.sources();
    assert_eq!(sources.iter().count(), 2);
    let hello = sources
        .find_by_alias("dockercloud-hello-world")
        .expect("sources should include dockercloud-hello-world");
    assert_eq!(hello.alias(), "dockercloud-hello-world");
    let url = "https://github.com/docker/dockercloud-hello-world.git";
    assert_eq!(hello.context(), &dc::Context::new(url));
    assert_eq!(
        hello.path(&proj.sources_dirs()),
        proj.src_dir().join("dockercloud-hello-world")
    );
}

#[test]
fn are_loaded_from_config_sources_yml() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let sources = proj.sources();
    let lib = sources
        .find_by_lib_key("coffee_rails")
        .expect("libs should include coffee_rails");
    assert_eq!(lib.alias(), "coffee-rails");
    assert_eq!(
        lib.context(),
        &dc::Context::new("https://github.com/rails/coffee-rails.git")
    );
    assert_eq!(
        lib.path(&proj.sources_dirs()),
        proj.src_dir().join("coffee-rails")
    );
}

#[test]
fn rejects_libs_with_subdirectories() {
    let _ = env_logger::try_init();
    assert!(Project::from_fixture("with_lib_subdir").is_err())
}

#[test]
fn can_be_cloned() {
    let _ = env_logger::try_init();
    let mut proj = Project::from_example("hello").unwrap();
    let sources_dirs = proj.sources_dirs();
    let source = proj
        .sources_mut()
        .find_by_alias_mut("dockercloud-hello-world")
        .unwrap();
    let runner = TestCommandRunner::new();
    source.clone_source(&runner, &sources_dirs).unwrap();
    let url = "https://github.com/docker/dockercloud-hello-world.git";
    assert_ran!(runner, {
        ["git", "clone", url, source.path(&sources_dirs)]
    });
    proj.remove_test_output().unwrap();
}

#[test]
fn can_be_checked_to_see_if_cloned() {
    let _ = env_logger::try_init();
    let mut proj = Project::from_example("hello").unwrap();
    let sources_dirs = proj.sources_dirs();
    let source = proj
        .sources_mut()
        .find_by_alias_mut("dockercloud-hello-world")
        .unwrap();
    assert!(!source.is_available_locally(&sources_dirs));
    source.fake_clone_source(&sources_dirs).unwrap();
    assert!(source.is_available_locally(&sources_dirs));
    proj.remove_test_output().unwrap();
}

#[test]
fn dir_context_is_always_available_locally() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("node_hello").unwrap();
    let source = proj.sources().find_by_alias("node_hello").unwrap();
    assert!(source.is_available_locally(&proj.sources_dirs()));
    proj.remove_test_output().unwrap();
}

#[test]
fn mounted_state_is_saved_between_runs() {
    let _ = env_logger::try_init();
    use rand::random;
    let id: u16 = random();

    // Load the project and update a `mounted` flag.
    {
        let mut proj = Project::from_example_and_random_id("node_hello", id).unwrap();
        {
            let sources = proj.sources_mut();
            let source = sources.find_by_alias_mut("node_hello").unwrap();
            assert_eq!(source.mounted(), false);
            source.set_mounted(true);
            assert_eq!(source.mounted(), true);
        }
        proj.save_settings().unwrap();
    }

    // Reload the project and make sure the value was saved.
    let proj = Project::from_example_and_random_id("node_hello", id).unwrap();
    let source = proj.sources().find_by_alias("node_hello").unwrap();
    assert_eq!(source.mounted(), true);
    proj.remove_test_output().unwrap();
}
