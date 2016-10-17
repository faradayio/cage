//! APIs for working with the source code associated with a `Project`'s
//! Docker images.

use compose_yml::v2 as dc;
use std::collections::BTreeMap;
use std::collections::btree_map;
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ext::context::ContextExt;
use ext::git_url::GitUrlExt;
use ext::service::ServiceExt;
use project::Project;
use pod::Pod;
use serde_helpers::load_yaml;
use util::ConductorPathExt;

/// All the source trees associated with a project's Docker images.
#[derive(Debug)]
pub struct Sources {
    /// Our source trees, indexed by their local alias.
    sources: BTreeMap<String, Source>,

    /// A map from keys in `config/libraries.yml` to source tree
    /// aliases.
    lib_keys: BTreeMap<String, String>,
}

impl Sources {
    /// Add a source tree to a map, keyed by its alias.  Returns the alias.
    fn add_source(sources: &mut BTreeMap<String, Source>,
                context: &dc::Context)
                -> Result<String> {
        // Figure out what alias we want to use.
        let alias = try!(context.human_alias());

        // Build our Source object.
        let source = Source {
            alias: alias.clone(),
            context: context.clone(),
        };

        // Insert our Source object into our map, checking for alias
        // clashes.
        match sources.entry(source.alias.clone()) {
            btree_map::Entry::Vacant(vacant) => {
                vacant.insert(source);
            }
            btree_map::Entry::Occupied(occupied) => {
                if &source.context != &occupied.get().context {
                    return Err(err!("{} and {} would both alias to \
                                     {}",
                                    &occupied.get().context,
                                    &source.context,
                                    &source.alias));
                }
            }
        }
        Ok(alias)
    }

    /// Create a collection of source trees based on a list of pods and our
    /// configuration files.
    #[doc(hidden)]
    pub fn new(root_dir: &Path, pods: &[Pod]) -> Result<Sources> {
        let mut sources: BTreeMap<String, Source> = BTreeMap::new();
        let mut lib_keys: BTreeMap<String, String> = BTreeMap::new();

        // Scan our pods for dc::Context objects.
        for pod in pods {
            for file in pod.all_files() {
                for service in file.services.values() {
                    if let Some(context) = try!(service.context()) {
                        try!(Self::add_source(&mut sources, context));
                    }
                }
            }
        }

        // Scan our config files for more source trees.
        let path = root_dir.join("config/libraries.yml");
        if path.exists() {
            let libs: BTreeMap<String, String> = try!(load_yaml(&path));
            for (lib_key, lib_src) in &libs {
                let context = dc::Context::new(&lib_src[..]);
                let alias = try!(Self::add_source(&mut sources, &context));
                lib_keys.insert(lib_key.clone(), alias);
            }
        }

        Ok(Sources {
            sources: sources,
            lib_keys: lib_keys,
        })
    }

    /// Iterate over all source trees associated with this project.
    pub fn iter(&self) -> Iter {
        Iter { iter: self.sources.iter() }
    }

    /// Look up a source tree using the short-form local alias.
    pub fn find_by_alias(&self, alias: &str) -> Option<&Source> {
        self.sources.get(alias)
    }

    /// Look up a source tree given a git URL.
    pub fn find_by_context(&self, context: &dc::Context) -> Option<&Source> {
        self.sources.values().find(|r| r.context() == context)
    }

    /// Look up a source tree using a "lib key", which is key used in
    /// `config/libraries.yml` and with service labels of the form
    /// `io.fdy.cage.lib.<KEY>`.
    pub fn find_by_lib_key(&self, lib_key: &str) -> Option<&Source> {
        self.lib_keys
            .get(lib_key)
            .and_then(|alias| self.find_by_alias(alias))
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

/// A single source tree.
#[derive(Debug)]
pub struct Source {
    /// A short name for this source tree.
    alias: String,
    /// The remote location from which we can clone this source tree, or
    /// the local directory where we can find it.
    context: dc::Context,
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

    /// The full path to where we expect any local copies of this code to
    /// live.  This will either be the location where we will check out a
    /// git repository, or the path to the actual source tree, depending on
    /// what type of `Context` object we're dealing with.
    ///
    /// The `project` argument is mandatory because we can't store a pointer
    /// to it without creating a circular reference loop.
    pub fn path(&self, project: &Project) -> PathBuf {
        match self.context {
            dc::Context::GitUrl(_) => project.src_dir().join(Path::new(self.alias())),
            dc::Context::Dir(ref path) => project.pods_dir().join(path),
        }
    }

    /// Has this project been cloned locally?
    pub fn is_available_locally(&self, project: &Project) -> bool {
        self.path(project).exists()
    }

    /// Clone the source code of this repository using git.
    pub fn clone_source<CR>(&self, runner: &CR, project: &Project) -> Result<()>
        where CR: CommandRunner
    {
        if let dc::Context::GitUrl(ref git_url) = self.context {
            let dest = try!(self.path(project).with_guaranteed_parent());
            runner.build("git")
                .arg("clone")
                .args(&try!(git_url.clone_args()))
                .arg(&dest)
                .exec()
        } else {
            Err(format!("'{}' is not a git repository", &self.context).into())
        }
    }

    /// (Test mode only.) Pretend to clone the source code for this
    /// repository by creating an empty directory in the right place.
    #[cfg(test)]
    pub fn fake_clone_source(&self, project: &Project) -> Result<()> {
        try!(fs::create_dir_all(self.path(project)));
        Ok(())
    }
}

#[test]
fn are_loaded_with_projects() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let sources = proj.sources();
    assert_eq!(sources.iter().count(), 2);
    let hello = sources.find_by_alias("dockercloud-hello-world")
        .expect("sources should include dockercloud-hello-world");
    assert_eq!(hello.alias(), "dockercloud-hello-world");
    let url = "https://github.com/docker/dockercloud-hello-world.git";
    assert_eq!(hello.context(), &dc::Context::new(url));
    assert_eq!(hello.path(&proj),
               proj.src_dir().join("dockercloud-hello-world"));
}

#[test]
fn are_loaded_from_config_libraries() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let sources = proj.sources();
    let lib = sources.find_by_lib_key("coffee_rails")
        .expect("libs should include coffee_rails");
    assert_eq!(lib.alias(), "coffee-rails");
    assert_eq!(lib.context(),
               &dc::Context::new("https://github.com/rails/coffee-rails.git"));
    assert_eq!(lib.path(&proj), proj.src_dir().join("coffee-rails"));
}

#[test]
fn can_be_cloned() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let source = proj.sources().find_by_alias("dockercloud-hello-world").unwrap();
    let runner = TestCommandRunner::new();
    source.clone_source(&runner, &proj).unwrap();
    let url = "https://github.com/docker/dockercloud-hello-world.git";
    assert_ran!(runner, {
        ["git", "clone", url, source.path(&proj)]
    });
    proj.remove_test_output().unwrap();
}

#[test]
fn can_be_checked_to_see_if_cloned() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let source = proj.sources().find_by_alias("dockercloud-hello-world").unwrap();
    assert!(!source.is_available_locally(&proj));
    source.fake_clone_source(&proj).unwrap();
    assert!(source.is_available_locally(&proj));
    proj.remove_test_output().unwrap();
}

#[test]
fn dir_context_is_always_available_locally() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("node_hello").unwrap();
    let source = proj.sources().find_by_alias("node_hello").unwrap();
    assert!(source.is_available_locally(&proj));
    proj.remove_test_output().unwrap();
}
