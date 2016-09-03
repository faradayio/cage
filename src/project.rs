//! A conductor project.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use dir;
use overrides::Override;
use pod::Pod;
use util::{ConductorPathExt, Error, ToStrOrErr};

/// A `conductor` project, which is represented as a directory containing a
/// `pods` subdirectory.
#[derive(Debug)]
pub struct Project {
    /// The directory which contains our `project`.  Must have a
    /// subdirectory named `pods`.
    root_dir: PathBuf,

    /// The directory to which we'll write our transformed pods.  Defaults
    /// to `root_dir.join(".conductor")`.
    output_dir: PathBuf,

    /// All the pods associated with this project.
    pods: Vec<Pod>,

    /// All the overrides associated with this project.
    overrides: Vec<Override>,
}

impl Project {
    /// Create a `Project` using the current directory as input and the
    /// `.conductor` subdirectory as output.
    ///
    /// ```
    /// use conductor::Project;
    /// use std::env;
    ///
    /// let saved = env::current_dir().unwrap();
    /// env::set_current_dir("examples/hello/pods").unwrap();
    ///
    /// let proj = Project::from_current_dir().unwrap();
    /// assert_eq!(proj.root_dir(), saved.join("examples/hello"));
    /// assert_eq!(proj.output_dir(), saved.join("examples/hello/.conductor"));
    ///
    /// env::set_current_dir(saved).unwrap();
    /// ```
    pub fn from_current_dir() -> Result<Project, Error> {
        // (We can only test this using a doc test because testing it
        // requires messing with `set_current_dir`, which isn't thread safe
        // and will break parallel tests.)
        let current = try!(env::current_dir());
        let root_dir = try!(dir::find_project(&current));
        let overrides = try!(Project::find_overrides(&root_dir));
        Ok(Project {
            root_dir: root_dir.clone(),
            output_dir: root_dir.join(".conductor"),
            pods: try!(Project::find_pods(&root_dir, &overrides)),
            overrides: overrides,
        })
    }

    /// (Tests only.) Create a `Project` from a subirectory of `examples`,
    /// with an output directory under `target/test_output/$NAME`.
    #[cfg(test)]
    pub fn from_example(name: &str) -> Result<Project, Error> {
        use rand::random;
        let example_dir = Path::new("examples").join(name);
        let root_dir = try!(dir::find_project(&example_dir));
        let rand_name = format!("{}-{}", name, random::<u16>());
        let overrides = try!(Project::find_overrides(&root_dir));
        Ok(Project {
            root_dir: root_dir.clone(),
            output_dir: Path::new("target/test_output").join(&rand_name),
            pods: try!(Project::find_pods(&root_dir, &overrides)),
            overrides: overrides,
        })
    }

    /// (Tests only.) Remove our output directory after a test.
    #[cfg(test)]
    pub fn remove_test_output(&self) -> Result<(), Error> {
        if self.output_dir.exists() {
            try!(fs::remove_dir_all(&self.output_dir));
        }
        Ok(())
    }

    /// Find all the overrides defined in this project.
    fn find_overrides(root_dir: &Path) -> Result<Vec<Override>, Error> {
        let overrides_dir = root_dir.join("pods/overrides");
        let mut overrides = vec!();
        for glob_result in try!(overrides_dir.glob("*")) {
            let path = try!(glob_result);
            if path.is_dir() {
                // It's safe to unwrap file_name because we know it matched
                // our glob.
                let name =
                    try!(path.file_name().unwrap().to_str_or_err()).to_owned();
                overrides.push(Override::new(name));
            }
        }
        Ok(overrides)
    }
    
    /// Find all the pods defined in this project.
    fn find_pods(root_dir: &Path, overrides: &[Override]) ->
        Result<Vec<Pod>, Error>
    {
        let pods_dir = root_dir.join("pods");
        let mut pods = vec!();
        for glob_result in try!(pods_dir.glob("*.yml")) {
            let path = try!(glob_result);
            // It's safe to unwrap the file_stem because we know it matched
            // our glob.
            let name =
                try!(path.file_stem().unwrap().to_str_or_err()).to_owned();
            pods.push(try!(Pod::new(pods_dir.clone(), name, overrides)));
        }
        Ok(pods)
    }

    /// The root directory of this project.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// The output directory of this project.  Normally `.conductor` inside
    /// the `root_dir`, but it may be overriden.
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Delete our existing output and replace it with a processed and
    /// expanded version of our pod definitions.
    pub fn output(&self) -> Result<(), Error> {
        // Get a path to our input pods directory.
        let in_pods = self.root_dir.join("pods");

        // Get a path to our output pods directory (and delete it if it
        // exists).
        let out_pods = self.output_dir.join("pods");
        if out_pods.exists() {
            try!(fs::remove_dir_all(&out_pods));
        }

        // Iterate over our *.env files recursively.
        for glob_result in try!(in_pods.glob("**/*.env")) {
            let rel = try!(try!(glob_result).strip_prefix(&in_pods)).to_owned();
            let in_path = in_pods.join(&rel);
            let out_path = try!(out_pods.join(&rel).with_guaranteed_parent());
            debug!("Copy {} to {}", in_path.display(), out_path.display());
            try!(fs::copy(in_path, out_path));
        }

        // Copy over our top-level pods.
        for pod in &self.pods {
            let rel = pod.rel_path();
            let out_path = try!(out_pods.join(&rel).with_guaranteed_parent());
            debug!("Generating {}", out_path.display());

            let file = pod.file();
            try!(file.write_to_path(out_path));

            // Copy over any override pods, too.
            for ovr in &self.overrides {
                let rel = try!(pod.override_rel_path(ovr));
                let out_path = try!(out_pods.join(&rel).with_guaranteed_parent());
                debug!("Generating {}", out_path.display());

                let file = try!(pod.override_file(ovr));
                try!(file.write_to_path(out_path));
            }
        }

        Ok(())
    }
}

#[test]
fn new_from_example_uses_example_and_target() {
    let proj = Project::from_example("hello").unwrap();
    assert_eq!(proj.root_dir, Path::new("examples/hello"));
    let output_dir = proj.output_dir.to_str_or_err().unwrap();
    assert!(output_dir.starts_with("target/test_output/hello-"));
    proj.remove_test_output().unwrap();
}

#[test]
fn output_copies_env_files() {
    let proj = Project::from_example("hello").unwrap();
    proj.output().unwrap();
    assert!(proj.output_dir.join("pods/common.env").exists());
    assert!(proj.output_dir.join("pods/overrides/test/common.env").exists());
    proj.remove_test_output().unwrap();
}

#[test]
fn output_processes_pods_and_overrides() {
    //use docker_compose::v2 as dc;

    let proj = Project::from_example("hello").unwrap();
    proj.output().unwrap();
    assert!(proj.output_dir.join("pods/frontend.yml").exists());
    assert!(proj.output_dir.join("pods/overrides/production/frontend.yml").exists());
    assert!(proj.output_dir.join("pods/overrides/test/frontend.yml").exists());

    //dc::File::

    proj.remove_test_output().unwrap();
}

#[test]
fn pods_are_loaded() {
    let proj = Project::from_example("hello").unwrap();
    let names: Vec<_> = proj.pods.iter().map(|pod| pod.name()).collect();
    assert_eq!(names, ["frontend"]);
    proj.remove_test_output().unwrap();
}

#[test]
fn overrides_are_loaded() {
    let proj = Project::from_example("hello").unwrap();
    let names: Vec<_> = proj.overrides.iter().map(|o| o.name()).collect();
    assert_eq!(names, ["development", "production", "test"]);
    proj.remove_test_output().unwrap();
}
