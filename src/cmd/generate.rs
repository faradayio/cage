//! The `new` and `generate` commands.

use rustc_serialize::json::{Json, ToJson};
use std::collections::BTreeMap;
#[cfg(test)]
use std::env;
#[cfg(test)]
use std::fs;
use std::io;
use std::path::{PathBuf, Path};

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
        let mut proj_tmpl = try!(Template::new("new"));
        let proj_info = ProjectInfo {
            name: name,
            cage_version: &version().to_string(),
        };
        try!(proj_tmpl.generate(&proj_dir, &proj_info, &mut io::stdout()));

        // Generate a sample secrets.yml file.
        let mut secrets_tmpl = try!(Template::new("secrets"));
        try!(secrets_tmpl.generate(&proj_dir, &proj_info, &mut io::stdout()));

        // Generate files for each target that uses our defaults.
        let mut target_tmpl = try!(Template::new("new/pods/targets/_default"));
        let targets_dir = proj_dir.join("pods").join("targets");
        for target in DEFAULT_TARGETS {
            let target_info = TargetInfo {
                project: &proj_info,
                name: target,
            };
            let dir = targets_dir.join(target);
            try!(target_tmpl.generate(&dir, &target_info, &mut io::stdout()));
        }

        Ok(proj_dir)
    }

    fn generate_list(&self) -> Result<()> {
        for generator in self.plugins().generators() {
            println!("{:19} {}",
                     generator.name(),
                     generator.generator_description());
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
    assert!(proj_dir.join("config/secrets.yml").exists());
    assert!(proj_dir.join("pods/common.env").exists());
    assert!(proj_dir.join("pods/frontend.yml").exists());
    assert!(proj_dir.join("pods/db.yml").exists());
    assert!(proj_dir.join("pods/targets/development/common.env").exists());
    assert!(proj_dir.join("pods/targets/production/common.env").exists());
    assert!(proj_dir.join("pods/targets/test/common.env").exists());

    fs::remove_dir_all(&proj_dir.as_path()).unwrap();
}


/// Information about the project we're generating.  This will be passed to
/// our templates.
#[derive(Debug)]
struct ProjectInfo<'a> {
    /// The current version of cage.
    cage_version: &'a str,

    /// The name of this project.
    name: &'a str,
}

// Convert to JSON for use in a Handlebars template.  We could get
// Handlebars and serde to convert to JSON automatically, but it's less
// work to define it by hand.
impl<'a> ToJson for ProjectInfo<'a> {
    fn to_json(&self) -> Json {
        let mut info: BTreeMap<String, Json> = BTreeMap::new();
        info.insert("cage_version".to_string(),
                    self.cage_version.to_string().to_json());
        info.insert("name".to_string(), self.name.to_json());
        info.to_json()
    }
}

/// Information about the target we're generating.  This will be passed
/// to our templates.
#[derive(Debug)]
struct TargetInfo<'a> {
    /// The project in which this target appears.
    project: &'a ProjectInfo<'a>,
    /// The name of this target.
    name: &'a str,
}

impl<'a> ToJson for TargetInfo<'a> {
    fn to_json(&self) -> Json {
        let mut info: BTreeMap<String, Json> = BTreeMap::new();
        info.insert("project".to_string(), self.project.to_json());
        info.insert("name".to_string(), self.name.to_json());
        info.to_json()
    }
}
