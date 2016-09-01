//! Parse a docker-compose.yml file and print it to standard output in
//! normalized format.  Try running:
//!
//! ```sh
//! conductor docker-compose.in.yml docker-compose.yml
//! ```

extern crate docker_compose;
extern crate regex;
extern crate walkdir;

use docker_compose::v2 as dc;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use walkdir::{DirEntry, WalkDir};

use ext::service::ServiceExt;

#[macro_use] mod util;
mod ext;

/// Update a `docker-compose.yml` file in place.  `path` is a relative path
/// to this file from the conductor working directory, which we use to
/// resolve things.
fn update(file: &mut dc::File, path: &Path) -> Result<(), dc::Error> {
    // Get the directory from which we read this file, and use it to
    // construct some useful paths.
    let dir = try!(path.parent().ok_or_else(|| {
        err!("Can't get parent of {}", path.display())
    }));
    let env_file = try!(dir.strip_prefix("pods")).join("common.env");

    // Iterate over each name/server pair in the file using `iter_mut`, so
    // we can modify the services.
    for (_name, service) in file.services.iter_mut() {
        // Insert standard `env_file` entry (if the file actually exists).
        if env_file.exists() {
            service.env_files.insert(0, dc::value(env_file.clone()));
        }

        // Figure out where we'll keep the local checkout, if any.
        let build_dir = try!(service.local_build_dir());

        // If we have a local build directory, update the service to use it.
        if let Some(ref dir) = build_dir {
            if dir.exists() {
                // Make build dir path relative to `.output/pods`.
                let rel = Path::new("../../").join(dir);

                // Mount the local build directory as `/app` inside the
                // container.
                let mount = dc::VolumeMount::host(&rel, "/app");
                service.volumes.push(dc::value(mount));

                // Update the `build` field if present.
                if let Some(ref mut build) = service.build {
                    build.context = dc::value(dc::Context::Dir(rel.clone()));
                }
            }
        }
    }
    Ok(())
}

/// Should we copy this directory entry?
fn should_copy(entry: &DirEntry) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }
    entry.file_name()
        .to_str()
        .map(|s| {
            !s.starts_with(".") && (s.ends_with(".yml") || s.ends_with(".env"))
        })
        .unwrap_or(false)
}

/// Our real `main` function.  This is a standard wrapper pattern: we put
/// all the real logic in a function that returns `Result` so that we can
/// use `try!` to handle errors, and we reserve `main` just for error
/// handling.
fn run() -> Result<(), dc::Error> {
    // Parse arguments.
    let args: Vec<_> = env::args().collect();
    if args.len() != 1 {
        return Err(err!("Usage: conductor"));
    }

    // We want to copy from "pods/" to ".conductor/".
    let dotdir = Path::new(".conductor");

    // Clean up our target directory.
    let dotdir_pods = dotdir.join("pods");
    if dotdir_pods.exists() {
        try!(fs::remove_dir_all(dotdir_pods));
    }

    // Walk over "pods/" recursively.
    for entry in WalkDir::new("pods") {
        let entry = try!(entry);
        if should_copy(&entry) {
            // Prefix "pods/" with ".conductor/" to get the output path.
            let in_path = entry.path();
            let out_path = dotdir.join(in_path);
            println!("Copying {} to {}", in_path.display(), out_path.display());

            // Make sure the destination directory exists.  It's OK to use
            // `unwrap` here because we know that `out_path` is not a file
            // system root because of how we constructed it above.
            try!(fs::create_dir_all(out_path.parent().unwrap()));

            // Make sure the destination file does not exist.  This is
            // reasonably safe because we do it under `.conductor`, which
            // is fair game.
            if out_path.exists() {
                try!(fs::remove_file(&out_path));
            }

            // Transform our file.  `unwrap` is safe because we know that
            // we have a real file extension thanks to `should_copy`.
            if in_path.extension().unwrap() == "yml" {
                let mut file = try!(dc::File::read_from_path(in_path));
                try!(update(&mut file, &in_path));
                try!(file.write_to_path(out_path));
            } else {
                try!(fs::copy(in_path, out_path));
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(ref err) = run() {
        // We use `unwrap` here to turn I/O errors into application panics.
        // If we can't print a message to stderr without an I/O error,
        // the situation is hopeless.
        write!(io::stderr(), "Error: {}", err).unwrap();
        process::exit(1);
    }
}
