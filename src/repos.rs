//! APIs for working with the git repositories associated with a `Project`.

use compose_yml::v2 as dc;
use serde_yaml;
use std::collections::BTreeMap;
use std::collections::btree_map;
use std::fs;
use std::path::{Path, PathBuf};

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ext::git_url::GitUrlExt;
use ext::service::ServiceExt;
use project::Project;
use pod::Pod;
use util::ConductorPathExt;

/// All the git repositories associated with a project.
#[derive(Debug)]
pub struct Repos {
    /// Our repositories, indexed by their local alias.
    repos: BTreeMap<String, Repo>,

    /// A map from keys in `config/libraries.yml` to repository aliases.
    lib_keys: BTreeMap<String, String>,
}

impl Repos {
    /// Add a repository to a map, keyed by its alias.  Returns the alias.
    fn add_repo(repos: &mut BTreeMap<String, Repo>,
                git_url: &dc::GitUrl)
                -> Result<String> {
        // Figure out what alias we want to use.
        let alias = try!(git_url.human_alias());

        // Build our repository.
        let repo = Repo {
            alias: alias.clone(),
            git_url: git_url.clone(),
        };

        // Insert our repository our map, checking for alias
        // clashes.
        match repos.entry(repo.alias.clone()) {
            btree_map::Entry::Vacant(vacant) => {
                vacant.insert(repo);
            }
            btree_map::Entry::Occupied(occupied) => {
                if &repo.git_url != &occupied.get().git_url {
                    return Err(err!("{} and {} would both alias to \
                                     {}",
                                    &occupied.get().git_url,
                                    &repo.git_url,
                                    &repo.alias));
                }
            }
        }
        Ok(alias)
    }

    /// Create a collection of repositories based on a list of pods.
    #[doc(hidden)]
    pub fn new(root_dir: &Path, pods: &[Pod]) -> Result<Repos> {
        let mut repos: BTreeMap<String, Repo> = BTreeMap::new();
        let mut lib_keys: BTreeMap<String, String> = BTreeMap::new();

        // Scan our pods for repositories.
        for pod in pods {
            for file in pod.all_files() {
                for service in file.services.values() {
                    if let Some(git_url) = try!(service.git_url()) {
                        try!(Self::add_repo(&mut repos, git_url));
                    }
                }
            }
        }

        // Scan our config files for repositories.
        let path = root_dir.join("config/libraries.yml");
        if path.exists() {
            let f = try!(fs::File::open(&path)
                .map_err(|e| err!("Error opening {}: {}", &path.display(), e)));
            let libs: BTreeMap<String, String> = try!(serde_yaml::from_reader(f)
                .map_err(|e| err!("Error reading {}: {}", &path.display(), e)));
            for (lib_key, lib_src) in &libs {
                let git_url: dc::GitUrl = try!(dc::GitUrl::new(&lib_src[..]));
                let alias = try!(Self::add_repo(&mut repos, &git_url));
                lib_keys.insert(lib_key.clone(), alias);
            }
        }

        Ok(Repos {
            repos: repos,
            lib_keys: lib_keys,
        })
    }

    /// Iterate over all repositories associated with this project.
    pub fn iter(&self) -> Iter {
        Iter { iter: self.repos.iter() }
    }

    /// Look up a repository using the short-form local alias.
    pub fn find_by_alias(&self, alias: &str) -> Option<&Repo> {
        self.repos.get(alias)
    }

    /// Look up a repository given a git URL.
    pub fn find_by_git_url(&self, url: &dc::GitUrl) -> Option<&Repo> {
        self.repos.values().find(|r| r.git_url() == url)
    }

    /// Look up a repository using a "lib key", which is key used in
    /// `config/libraries.yml` and with service labels of the form
    /// `io.fdy.conductor.lib.<KEY>`.
    pub fn find_by_lib_key(&self, lib_key: &str) -> Option<&Repo> {
        self.lib_keys
            .get(lib_key)
            .and_then(|alias| self.find_by_alias(alias))
    }
}

/// An iterator over all repositories associated with this project.
#[derive(Clone)]
#[allow(missing_debug_implementations)]
pub struct Iter<'a> {
    /// Our wrapped iterator.  We wrap this in our own struct to make the
    /// underlying type opaque.
    iter: btree_map::Iter<'a, String, Repo>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Repo;

    fn next(&mut self) -> Option<&'a Repo> {
        self.iter.next().map(|(_alias, repo)| repo)
    }
}

/// A single repository.
#[derive(Debug)]
pub struct Repo {
    /// A short name for this repository.
    alias: String,
    /// The remote location from which we can clone this repository.
    git_url: dc::GitUrl,
}

impl Repo {
    /// A short local name for this git repository, suitable for use as
    /// a directory name or command-line argument.
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// The remote git URL from which we can clone this repository.
    pub fn git_url(&self) -> &dc::GitUrl {
        &self.git_url
    }

    /// The path to which we would check out this repository, relative to
    /// `Project::src_dir`.
    pub fn rel_path(&self) -> PathBuf {
        Path::new(self.alias()).to_owned()
    }

    /// The full path to which we would check out this repository.  The
    /// `project` argument is mandatory because we can't store a pointer
    /// to it without creating a circular reference loop.
    pub fn path(&self, project: &Project) -> PathBuf {
        project.src_dir().join(self.rel_path())
    }

    /// Has this project been cloned locally?
    pub fn is_cloned(&self, project: &Project) -> bool {
        self.path(project).exists()
    }

    /// Clone the source code of this repository using git.
    pub fn clone_source<CR>(&self, runner: &CR, project: &Project) -> Result<()>
        where CR: CommandRunner
    {
        let dest = try!(self.path(project).with_guaranteed_parent());
        let status = try!(runner.build("git")
            .arg("clone")
            .args(&try!(self.git_url().clone_args()))
            .arg(&dest)
            .status());
        if !status.success() {
            return Err(err!("Error cloning {} to {}", &self.git_url, dest.display()));
        }
        Ok(())
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
    let repos = proj.repos();
    assert_eq!(repos.iter().count(), 2);
    let hello = repos.find_by_alias("dockercloud-hello-world")
        .expect("repos should include dockercloud-hello-world");
    assert_eq!(hello.alias(), "dockercloud-hello-world");
    assert_eq!(hello.git_url().as_ref() as &str,
               "https://github.com/docker/dockercloud-hello-world.git");
    assert_eq!(hello.rel_path(), Path::new("dockercloud-hello-world"));
}

#[test]
fn are_loaded_from_config_libraries() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let repos = proj.repos();
    let lib = repos.find_by_lib_key("coffee_rails")
        .expect("libs should include coffee_rails");
    assert_eq!(lib.alias(), "coffee-rails");
    assert_eq!(lib.git_url().as_ref() as &str,
               "https://github.com/rails/coffee-rails.git");
    assert_eq!(lib.rel_path(), Path::new("coffee-rails"));
}

#[test]
fn can_be_cloned() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let repo = proj.repos().find_by_alias("dockercloud-hello-world").unwrap();
    let runner = TestCommandRunner::new();
    repo.clone_source(&runner, &proj).unwrap();
    assert_ran!(runner, {
        ["git", "clone", repo.git_url(), proj.src_dir().join(repo.rel_path())]
    });
    proj.remove_test_output().unwrap();
}

#[test]
fn can_be_checked_to_see_if_cloned() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let repo = proj.repos().find_by_alias("dockercloud-hello-world").unwrap();
    assert!(!repo.is_cloned(&proj));
    repo.fake_clone_source(&proj).unwrap();
    assert!(repo.is_cloned(&proj));
    proj.remove_test_output().unwrap();
}
