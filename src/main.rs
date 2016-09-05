//! Our main CLI tool.

#[macro_use]
extern crate conductor;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rustc_serialize;

use docopt::Docopt;
use std::io::{self, Write};
use std::process;

use conductor::command_runner::OsCommandRunner;
use conductor::cmd::*;
use conductor::Error;

/// Our version number, set by Cargo at compile time.
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// Our help string.
const USAGE: &'static str = "
conductor: Manage large, multi-pod docker-compose apps

Usage:
  conductor [options]
  conductor [options] pull
  conductor [options] up
  conductor [options] stop
  conductor [options] repo list
  conductor [options] repo clone <repo>
  conductor (--help | --version)

Options:
    -h, --help             Show this message
    --version              Show the version of conductor
    --override=<override>  Use overrides from the specified subdirectory
                           of `pods/overrides` [default: development]

Run conductor in a directory containing a `pods` subdirectory.  For more
information, see https://github.com/faradayio/conductor.
";

/// Our parsed command-line arguments.  See [docopt.rs][] for an
/// explanation of how this works.
///
/// [docopt.rs]: https://github.com/docopt/docopt.rs
#[derive(Debug, RustcDecodable)]
struct Args {
    cmd_pull: bool,
    cmd_up: bool,
    cmd_stop: bool,
    cmd_repo: bool,
    cmd_list: bool,
    cmd_clone: bool,

    arg_repo: Option<String>,

    flag_version: bool,
    flag_override: String,
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `try!` to handle errors.
fn run(args: &Args) -> Result<(), Error> {
    let proj = try!(conductor::Project::from_current_dir());
    let ovr = try!(proj.ovr(&args.flag_override).ok_or_else(|| {
        err!("override {} is not defined", &args.flag_override)
    }));
    try!(proj.output());
    let runner = OsCommandRunner;

    if args.cmd_pull {
        try!(proj.pull(&runner, &ovr));
    } else if args.cmd_up {
        try!(proj.up(&runner, &ovr));
    } else if args.cmd_stop {
        try!(proj.stop(&runner, &ovr));
    } else if args.cmd_repo && args.cmd_list {
        try!(proj.repo_list(&runner));
    } else if args.cmd_repo && args.cmd_clone {
        try!(proj.repo_clone(&runner, args.arg_repo.as_ref().unwrap()));
    }

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
