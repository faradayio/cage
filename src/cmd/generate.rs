//! The `conductor new` command, and any other file generators.

use handlebars as hb;
use rustc_serialize::json::{Json, ToJson};
use std::collections::BTreeMap;
#[cfg(test)]
use std::env;
use std::fmt::Debug;
use std::fs;
use std::path::{PathBuf, Path};

use project::Project;
use util::{ConductorPathExt, Error};

/// Interface to various file-generation commands.
pub trait CommandGenerate {
    /// Create a new conductor project skeleton, returning the path of the
    /// generated project.
    ///
    /// ```text
    /// <name>
    /// └── pods
    ///   ├── common.env
    ///   ├── app.yml
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
        let pods_dir = proj_dir.join("pods");

        // Create a template engine to generate our files.
        let mut renderer: hb::Handlebars = hb::Handlebars::new();
        renderer.register_escape_fn(escape_double_quotes);

        // Generate our top-level files.
        let proj_info = ProjectInfo { name: name };
        try!(generate(&mut renderer, &COMMON_ENV, &proj_info,
                      &pods_dir.join("common.env")));
        try!(generate(&mut renderer, &MAIN_YML, &proj_info,
                      &pods_dir.join("app.yml")));

        // Generate files for each override.
        let overrides_dir: PathBuf = pods_dir.join("overrides");
        for ovr in OVERRIDES {
            let ovr_info = OverrideInfo {
                project: &proj_info,
                name: ovr,
            };
            let dir = overrides_dir.join(ovr);
            try!(generate(&mut renderer, &OVERRIDE_ENV, &ovr_info,
                          &dir.join("common.env")));
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
    assert!(proj_dir.join("pods/app.yml").exists());
    assert!(proj_dir.join("pods/overrides/development/common.env").exists());
    assert!(proj_dir.join("pods/overrides/production/common.env").exists());
    assert!(proj_dir.join("pods/overrides/test/common.env").exists());

    fs::remove_dir_all(&proj_dir.as_path()).unwrap();
}

/// Escape double quotes in a string that we're rendering, which should
/// work well more-or-less well enough for all the formats we're generating.
fn escape_double_quotes(data: &str) -> String {
    data.replace(r#"""#, r#"\""#)
}

/// Generate a template and write it to the specified file.
fn generate<T>(renderer: &mut hb::Handlebars,
               template: &str,
               data: &T,
               path: &Path) ->
    Result<(), Error>
    where T: ToJson + Debug
{
    debug!("Generating {} with {:?}", path.display(), data);
    println!("Generating {}", path.display());

    // Make sure our parent directory exists.
    try!(path.with_guaranteed_parent());

    // Create our output file and copy data into it.
    let mut out = try!(fs::File::create(path).map_err(|e| {
        err!("Unable to create file {}: {}", path.display(), &e)
    }));

    // Render our template to the file.
    let ctx = hb::Context::wraps(data);
    renderer.template_renderw(template.trim_left(), &ctx, &mut out).map_err(|e| {
        err!("Unable to generate {}: {}", path.display(), &e)
    })
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

//fn write_to_file<F, W>(path: &Path,   ->
//    Result<(), Error>
//    where W: Write
//{
//    println!("Writing file {:?}", path.to_str());
//
//    // Make sure our parent directory exists.
//    try!(path.with_guaranteed_parent());
//
//    // Create our output file and copy data into it.
//    let mut out = try!(fs::File::create(path).map_err(|e| {
//        err!("Unable to create file {}: {}", path.display(), &e)
//    }));
//    try!(io::copy(content, out).map_err(|e| {
//        err!("Unable to write to file {}: {}", path.display(), &e)
//    }));
//    Ok(())
//}

// Standard overrides that we'll use for new projects.
const OVERRIDES: &'static [ &'static str ] =
    &["development", "production", "test"];

static MAIN_YML: &'static str = r#"
db:
  image: postgres
web:
  image: rails
  links:
    - db:db
  ports:
    - 3000:3000
"#;

static COMMON_ENV: &'static str = r#"
# Define environment variables here to make them visible in all containers.
# For example:
PROJECT_NAME="{{name}}"
"#;

static OVERRIDE_ENV: &'static str = r#"
# Define environment variables here to make them visible to all containers
# when this overlay is being used.  For example:
RAILS_ENV="{{name}}"
RACK_ENV="{{name}}"
NODE_ENV="{{name}}"
DATABASE_URL="postgres://postgres@db:5432/{{project.name}}_{{name}}"
"#;
