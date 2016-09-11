//! The `conductor new` command, and any other file generators.

#[cfg(test)]
use std::env;
use std::fs;
use std::io::Write;
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
    ///   ├── <name>.yml
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

        try!(write_file(&COMMON_ENV, &pods_dir.join("common.env").as_path()));
        try!(write_file(&MAIN_YML, &pods_dir.join(format!("{}.yml", name)).as_path()));

        let overrides_dir: PathBuf = pods_dir.join("overrides");
        for ovr in OVERRIDES {
            let dir = overrides_dir.join(ovr);
            try!(write_file(
                &format!("DATABASE_URL=postgres://postgres@db:5432/{}_{}", name, ovr),
                &dir.join("common.env").as_path()
            ));
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
    assert!(proj_dir.join("pods/test_project.yml").exists());
    assert!(proj_dir.join("pods/overrides/development/common.env").exists());
    assert!(proj_dir.join("pods/overrides/production/common.env").exists());
    assert!(proj_dir.join("pods/overrides/test/common.env").exists());

    fs::remove_dir_all(&proj_dir.as_path()).unwrap();
}

fn write_file(content: &str, path: &Path) -> Result<(), Error> {
    println!("Writing file {:?}", path.to_str());

    // Make sure our parent directory exists.
    try!(path.with_guaranteed_parent());

    let mut f = match fs::File::create(path) {
        Err(e) => {
            return Err(err!("Unable to create file {}: {:?}", path.to_str().unwrap(), e.kind()))
        },
        Ok(f) => f
    };

    match f.write_all(content.as_bytes()) {
        Err(e) => {
            return Err(err!("Unable to write to file {}: {:?}", path.to_str().unwrap(), e.kind()))
        }
        Ok(_) => {}
    }

    Ok(())
}

const OVERRIDES: &'static [ &'static str ] = &["development","production","test"];

static COMMON_ENV: &'static str = r#"
FOO=bar
"#;

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
