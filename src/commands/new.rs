use std::fs;
use std::io::Error;
use std::path::Path;
use std::result::Result;

pub fn default_dir(name: &String) -> String {
    let path = &Path::new(env!("PWD"));

    match path.join(name).to_str() {
        None => panic!("Unable to create project path"),
        Some(s) => return s.to_string()
    }
}

pub fn run(name: &String) -> Result<bool, Error> {
    let path = default_dir(&name);

    println!("Initializing {}", name.to_string());

    println!("     Creating {}", path);
    match fs::create_dir(path) {
        Err(e) => return Err(e),
        Ok(_) => return Ok(true)
    }
}
