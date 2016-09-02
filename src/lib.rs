extern crate docker_compose;
extern crate glob;
extern crate regex;

use docker_compose::v2 as dc;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ext::service::ServiceExt;

#[macro_use] mod util;
mod ext;

/// Update a `docker-compose.yml` file in place.  `path` is a relative path
/// to this file from the conductor working directory, which we use to
/// resolve things.
pub fn update(file: &mut dc::File, path: &Path) -> Result<(), dc::Error> {
    // Get the directory from which we read this file, and use it to
    // construct some useful paths.
    let dir = try!(path.parent().ok_or_else(|| {
        err!("Can't get parent of {}", path.display())
    }));
    let env_file = dir.join("common.env");

    // Iterate over each name/server pair in the file using `iter_mut`, so
    // we can modify the services.
    for (_name, service) in file.services.iter_mut() {
        // Insert standard `env_file` entry (if the file actually exists).
        if env_file.exists() {
            // Make our env file path relative to our top-level
            // `docker-compose.yml` file by removing `pods`, so
            // `docker-compose` doesn't get confused.
            let dc_rel = try!(env_file.strip_prefix("pods")).to_owned();
            service.env_files.insert(0, dc::value(dc_rel));
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

/// Generate `.compose/pods` from `pods`, transforming `*.yml` files as
/// necessary and copying `*.env` files.
pub fn generate() -> Result<(), dc::Error> {
    // We want to copy from "pods/" to ".conductor/".
    let dotdir = Path::new(".conductor");

    // Clean up our target directory.
    let dotdir_pods = dotdir.join("pods");
    if dotdir_pods.exists() {
        try!(fs::remove_dir_all(dotdir_pods));
    }

    // Get the output directory corresponding `in_dir`, and make sure that
    // the containing directory exists.  We use a closure instead of a
    // local function here so that we can capture variables.
    let get_out_path = |in_path: &Path| -> Result<PathBuf, dc::Error> {
        let out_path = dotdir.join(in_path);
        try!(fs::create_dir_all(try!(out_path.parent().ok_or_else(|| {
            err!("can't find parent of {}", out_path.display())
        }))));
        Ok(out_path)
    };

    // Set up some standard glob options we'll use repeatedly.
    let glob_opts = glob::MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: true,
    };

    // Copy over all our simple files by walking pods/ recursively.
    for glob_result in try!(glob::glob_with("pods/**/*.env", &glob_opts)) {
        let in_path = try!(glob_result);
        let out_path = try!(get_out_path(&in_path));
        try!(fs::copy(in_path, out_path));
    }

    // For *.yaml files, we need to do this in several tiers, because we
    // need to figure out which services a given pod is supposed to
    // contain, and make sure that those services appear in all override
    // pods.
    for glob_result in try!(glob::glob_with("pods/*.yml", &glob_opts)) {
        let in_path = try!(glob_result);
        let out_path = try!(get_out_path(&in_path));

        // Munge our top-level file.
        let mut file = try!(dc::File::read_from_path(&in_path));
        try!(update(&mut file, &in_path));
        try!(file.write_to_path(out_path));

        // Extract the service names from our top-level file, and get the
        // filename as string.
        let service_names = file.services.keys().cloned().collect::<Vec<_>>();
        let filename = try!(in_path.file_name().and_then(|s| {
            s.to_str()
        }).ok_or_else(|| {
            err!("can't get file name for {}", in_path.display())
        }));

        // Find all overrides matching this top-level file.
        let overrides = format!("pods/overrides/*/{}", filename);
        for glob_result in try!(glob::glob_with(&overrides, &glob_opts)) {
            let in_path = try!(glob_result);
            let out_path = try!(get_out_path(&in_path));

            let mut file = try!(dc::File::read_from_path(&in_path));
            for name in &service_names {
                // If this services does exist, create it so that we can
                // set `env_file` on it.
                file.services.entry(name.to_owned()).or_insert_with(Default::default);
            }
            try!(update(&mut file, &in_path));
            try!(file.write_to_path(out_path));
        }
    }

    Ok(())
}
