//! The `conductor new` command, and any other file generators.

use rustc_serialize::json::{Json, ToJson};
use std::collections::BTreeMap;
#[cfg(test)]
use std::env;
#[cfg(test)]
use std::fs;
use std::path::{PathBuf, Path};

use project::Project;
use template::Template;
use util::Error;

/// A list of standard overrides to generate.
const OVERRIDES: &'static [ &'static str ] = &["development", "production", "test"];

/// Interface to various file-generation commands.
pub trait CommandGenerate {
    /// Create a new conductor project skeleton, returning the path of the
    /// generated project.
    ///
    /// ```text
    /// <name>
    /// └── pods
    ///   ├── common.env
    ///   ├── frontend.yml
    ///   └── overrides
    ///       ├── development
    ///       │   └── common.env
    ///       ├── production
    ///       │   ├── common.env
    ///       └── test
    ///           └── common.env
    /// ```
    fn generate(parent_dir: &Path, name: &str) -> Result<PathBuf, Error>;
}

impl CommandGenerate for Project {
    fn generate(parent_dir: &Path, name: &str) -> Result<PathBuf, Error> {
        let proj_dir = parent_dir.join(name);

        // Generate our top-level files.
        let mut proj_tmpl = try!(Template::new("new"));
        let proj_info = ProjectInfo { name: name };
        try!(proj_tmpl.generate(&proj_dir, &proj_info));

        // Generate files for each override.
        let mut ovr_tmpl = try!(Template::new("new/pods/_overrides/_default"));
        let overrides_dir = proj_dir.join("pods").join("overrides");
        for ovr in OVERRIDES {
            let ovr_info = OverrideInfo {
                project: &proj_info,
                name: ovr,
            };
            let dir = overrides_dir.join(ovr);
            try!(ovr_tmpl.generate(&dir, &ovr_info));
        }

        Ok(proj_dir)
    }
}

#[test]
fn create_project_default() {
    let cwd = env::current_dir().unwrap();
    Project::generate(&cwd, "test_project").unwrap();
    let proj_dir = env::current_dir().unwrap().join("test_project");

    assert!(proj_dir.exists());
    assert!(proj_dir.join("pods/common.env").exists());
    assert!(proj_dir.join("pods/frontend.yml").exists());
    assert!(proj_dir.join("pods/db.yml").exists());
    assert!(proj_dir.join("pods/overrides/development/common.env").exists());
    assert!(proj_dir.join("pods/overrides/production/common.env").exists());
    assert!(proj_dir.join("pods/overrides/test/common.env").exists());

    fs::remove_dir_all(&proj_dir.as_path()).unwrap();
}


/// Information about the project we're generating.  This will be passed to
/// our templates.
#[derive(Debug)]
struct ProjectInfo<'a> {
    /// The name of this project.
    name: &'a str,
}

// Convert to JSON for use in a Handlebars template.  We could get
// Handlebars and serde to convert to JSON automatically, but it's less
// work to define it by hand.
impl<'a> ToJson for ProjectInfo<'a> {
    fn to_json(&self) -> Json {
        let mut info: BTreeMap<String, Json> = BTreeMap::new();
        info.insert("name".to_string(), self.name.to_json());
        info.to_json()
    }
}

/// Information about the override we're generating.  This will be passed
/// to our templates.
#[derive(Debug)]
struct OverrideInfo<'a> {
    /// The project in which this override appears.
    project: &'a ProjectInfo<'a>,
    /// The name of this override.
    name: &'a str,
}

impl<'a> ToJson for OverrideInfo<'a> {
    fn to_json(&self) -> Json {
        let mut info: BTreeMap<String, Json> = BTreeMap::new();
        info.insert("project".to_string(), self.project.to_json());
        info.insert("name".to_string(), self.name.to_json());
        info.to_json()
    }
}
