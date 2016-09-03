//! Our main CLI tool.

extern crate conductor;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rustc_serialize;

use docopt::Docopt;
use std::io::{self, Write};
use std::process;

use conductor::Error;

/// Our version number, set by Cargo at compile time.
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// Our help string.
const USAGE: &'static str = "
conductor: Manage large, multi-pod docker-compose apps

Usage:
  conductor
  conductor (--help | --version)

Options:
    -h, --help         Show this message
    --version          Show the version of conductor

Run conductor in a directory containing a `pods` subdirectory.  For more
information, see https://github.com/faradayio/conductor.
";

/// Our parsed command-line arguments.  See [docopt.rs][] for an
/// explanation of how this works.
///
/// [docopt.rs]: https://github.com/docopt/docopt.rs
#[derive(Debug, RustcDecodable)]
struct Args {
    flag_version: bool,
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `try!` to handle errors.
fn run(_: &Args) -> Result<(), Error> {
    let proj = try!(conductor::Project::from_current_dir());
    try!(proj.output());
    Ok(())
}

/// Our main entry point.
fn main() {
    // Boot up logging.
    env_logger::init().unwrap();

    // Parse our args using docopt.rs.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    debug!("Arguments: {:?}", &args);

    // Display our version if we were asked to do so.
    if args.flag_version {
        println!("conductor {}", VERSION);
        process::exit(0);
    }

    // Defer all our real work to `run`, and handle any errors.  This is a
    // standard Rust pattern to make error-handling in `main` nicer.
    if let Err(ref err) = run(&args) {
        // We use `unwrap` here to turn I/O errors into application panics.
        // If we can't print a message to stderr without an I/O error,
        // the situation is hopeless.
        write!(io::stderr(), "Error: {}", err).unwrap();
        process::exit(1);
    }
}
