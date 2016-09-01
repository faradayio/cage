//! Parse a docker-compose.yml file and print it to standard output in
//! normalized format.  Try running:
//!
//! ```sh
//! conductor docker-compose.in.yml docker-compose.yml
//! ```

extern crate docker_compose;
extern crate regex;

use docker_compose::v2 as dc;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process;

use ext::service::ServiceExt;

#[macro_use] mod util;
mod ext;

/// Update a `docker-compose.yml` file in place.
fn update(file: &mut dc::File) -> Result<(), dc::Error> {
    // Iterate over each name/server pair in the file using `iter_mut`, so
    // we can modify the services.
    for (_name, service) in file.services.iter_mut() {
        // Insert standard env_file entries.
        service.env_files.insert(0, try!(dc::escape("pods/common.env")));
        service.env_files.insert(1, try!(dc::raw("pods/overrides/$ENV/common.env")));

        // Figure out where we'll keep the local checkout, if any.
        let build_dir = try!(service.local_build_dir());

        // If we have a local build directory, update the service to use it.
        if let Some(ref dir) = build_dir {
            // Mount the local build directory as `/app` inside the container.
            service.volumes.push(dc::value(dc::VolumeMount {
                host: Some(dc::HostVolume::Path(dir.clone())),
                container: Path::new("/app").to_owned(),
                permissions: Default::default(),
            }));
            // Update the `build` field if present.
            if let Some(ref mut build) = service.build {
                build.context = dc::value(dc::Context::Dir(dir.clone()));
            }
        }
    }
    Ok(())
}

/// Our real `main` function.  This is a standard wrapper pattern: we put
/// all the real logic in a function that returns `Result` so that we can
/// use `try!` to handle errors, and we reserve `main` just for error
/// handling.
fn run() -> Result<(), dc::Error> {
    // Parse arguments.
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err(err!("Usage: miniconductor <infile> <outfile>"));
    }
    let in_path = Path::new(&args[1]);
    let out_path = Path::new(&args[2]);

    // Transform our file.
    let mut file = try!(dc::File::read_from_path(in_path));
    try!(update(&mut file));
    try!(file.write_to_path(out_path));

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
