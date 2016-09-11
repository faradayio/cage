//! Create a new conductor project

use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{PathBuf, Path};
use util::{Error};

/// Create a new conductor project skeleton
///
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
pub fn create_project(name: &str) -> Result<PathBuf, Error> {
    let mut cwd = try!(env::current_dir());
    cwd.push(name);

    try!(create_dir(&cwd));

    let pods: PathBuf = cwd.join("pods");
    try!(create_dir(&pods));

    try!(write_file(&COMMON_ENV, &pods.join("common.env").as_path()));
    try!(write_file(&MAIN_YML, &pods.join(format!("{}.yml", name)).as_path()));

    let overrides: PathBuf = pods.join("overrides");
    try!(create_dir(&overrides));

    for env in ENVIRONMENTS {
        let dir = overrides.join(env);
        try!(create_dir(&dir));

        try!(write_file(
            &format!("DATABASE_URL=postgres://postgres@db:5432/{}_{}", name, env),
            &dir.join("common.env").as_path()
        ));
    }

    Ok(cwd)
}

#[test]
fn create_project_default() {
    create_project("test_project").unwrap();

    let mut cwd = env::current_dir().unwrap();
    cwd.push("test_project");

    assert!(cwd.exists());
    assert!(cwd.join("pods/common.env").exists());
    assert!(cwd.join("pods/test_project.yml").exists());
    assert!(cwd.join("pods/overrides/development/common.env").exists());
    assert!(cwd.join("pods/overrides/production/common.env").exists());
    assert!(cwd.join("pods/overrides/test/common.env").exists());

    use std::fs::remove_dir_all;
    remove_dir_all(&cwd.as_path()).unwrap();
}

fn create_dir(cwd: &PathBuf) -> Result<(), Error> {
    println!("Creating directory {:?}", cwd.to_str());

    match fs::create_dir(&cwd) {
        Err(e) => {
            return Err(err!("Unable to create directory {}: {:?}", cwd.to_str().unwrap(), e.kind()));
        },
        Ok(_) => {} 
    }

    Ok(())
}

fn write_file(content: &str, path: &Path) -> Result<(), Error> {
    println!("Writing file {:?}", path.to_str());

    let mut f = match File::create(path) {
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

const ENVIRONMENTS: &'static [ &'static str ] = &["development","production","test"];

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
