//! Utilities for finding and working with `conductor` project directories.

use std::path::{Path, PathBuf};

use util::Error;

/// Walk up the directory tree until we find a directory that looks like a
/// `conductor` project.
pub fn find_project(start_dir: &Path) -> Result<PathBuf, Error> {
    // Do this as a loop, not recusively, so we can use `start_dir` in
    // error messages.
    let mut dir = start_dir;
    loop {
        if dir.join("pods").exists() {
            return Ok(dir.to_owned());
        } else if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            let err = err!("could not find conductor project in {} or any directory \
                            above it",
                           start_dir.display());
            return Err(err);
        }
    }
}

#[test]
fn find_project_walks_up_directory_tree() {
    assert_eq!(find_project(Path::new("examples/hello")).unwrap(),
               Path::new("examples/hello"));
    assert_eq!(find_project(Path::new("examples/hello/pods")).unwrap(),
               Path::new("examples/hello"));
    assert!(find_project(Path::new("examples")).is_err());
}
