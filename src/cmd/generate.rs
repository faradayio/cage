//! The `new` and `generate` commands.

#[cfg(test)]
use std::env;
#[cfg(test)]
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use errors::*;
use project::Project;
use template::Template;
use version;

/// A list of standard targets to generate.  We handle `production`
/// manually.
const DEFAULT_TARGETS: &'static [&'static str] = &["development", "test"];

/// Interface to various file-generation commands.
pub trait CommandGenerate {
    /// Create a new project skeleton, returning the path of the generated
    /// project.
    ///
    /// ```text
    /// <name>
    /// └── pods
    ///   ├── common.env
    ///   ├── frontend.yml
    ///   └── targets
    ///       ├── development
    ///       │   └── common.env
    ///       ├── production
    ///       │   ├── common.env
    ///       └── test
    ///           └── common.env
    /// ```
    fn generate_new(parent_dir: &Path, name: &str) -> Result<PathBuf>;

    /// Print our all available generators (excluding the `generate_new`
    /// generator).
    fn generate_list(&self) -> Result<()>;

    /// Run the specified generator.
    fn generate(&self, name: &str) -> Result<()>;
}

impl CommandGenerate for Project {
    fn generate_new(parent_dir: &Path, name: &str) -> Result<PathBuf> {
        let proj_dir = parent_dir.join(name);

        // Generate our top-level files.
        let mut proj_tmpl = Template::new("new")?;
        let proj_info = ProjectInfo {
            name: name,
            cage_version: &version().to_string(),
        };
        proj_tmpl.generate(&proj_dir, &proj_info, &mut io::stdout())?;

        // Generate a sample secrets.yml file.
        let mut secrets_tmpl = Template::new("secrets")?;
        secrets_tmpl.generate(&proj_dir, &proj_info, &mut io::stdout())?;

        // Generate files for each target that uses our defaults.
        let mut target_tmpl = Template::new("new/pods/targets/_default")?;
        let targets_dir = proj_dir.join("pods").join("targets");
        for target in DEFAULT_TARGETS {
            let target_info = TargetInfo {
                project: &proj_info,
                name: target,
            };
            let dir = targets_dir.join(target);
            target_tmpl.generate(&dir, &target_info, &mut io::stdout())?;
        }

        Ok(proj_dir)
    }

    fn generate_list(&self) -> Result<()> {
        for generator in self.plugins().generators() {
            println!(
                "{:19} {}",
                generator.name(),
                generator.generator_description()
            );
        }
        Ok(())
    }

    fn generate(&self, name: &str) -> Result<()> {
        self.plugins().generate(self, name, &mut io::stdout())
    }
}

#[test]
fn generate_new_creates_a_project() {
    let cwd = env::current_dir().unwrap();
    Project::generate_new(&cwd, "test_project").unwrap();
    let proj_dir = env::current_dir().unwrap().join("test_project");

    assert!(proj_dir.exists());
    assert!(proj_dir.join("config").join("secrets.yml").exists());
    assert!(proj_dir.join("pods").join("common.env").exists());
    assert!(proj_dir.join("pods").join("frontend.yml").exists());
    assert!(proj_dir.join("pods").join("db.yml").exists());
    let targets = proj_dir.join("pods").join("targets");
    assert!(targets.join("development").join("common.env").exists());
    assert!(targets.join("production").join("common.env").exists());
    assert!(targets.join("test").join("common.env").exists());

    fs::remove_dir_all(&proj_dir.as_path()).unwrap();
}

/// Information about the project we're generating.  This will be passed to
/// our templates.
#[derive(Debug, Serialize)]
struct ProjectInfo<'a> {
    /// The current version of cage.
    cage_version: &'a str,

    /// The name of this project.
    name: &'a str,
}

/// Information about the target we're generating.  This will be passed
/// to our templates.
#[derive(Debug, Serialize)]
struct TargetInfo<'a> {
    /// The project in which this target appears.
    project: &'a ProjectInfo<'a>,
    /// The name of this target.
    name: &'a str,
}
