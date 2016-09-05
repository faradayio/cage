use std::env;
use std::fs::{create_dir};
use std::io;
use std::path::PathBuf;
use util::{Error};

// weird type error
// src/create_project.rs:13:5: 13:7 error: unable to infer enough type information about `_`; type
// annotations or generic parameter binding required [E0282]
// src/create_project.rs:13     Ok(cwd);
//
//pub fn create_project(name: &str) -> Result<PathBuf, Error> {
    //let cwd = try!(env::current_dir());
    //cwd.push(name);

    ////fs::create_dir(project_directory(name));

    //Ok(cwd);
//}

pub fn create_project(name: &str) {
    let mut cwd = env::current_dir().unwrap();
    cwd.push(name);

    //fs::create_dir(project_directory(name));
}


#[test]
fn create_project_default() {
    create_project("test");
}
