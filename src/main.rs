//! Our main CLI tool.

#[macro_use]
extern crate conductor;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rustc_serialize;

use docopt::Docopt;
use std::env;
use std::fs;
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
  conductor [options] new <name>
  conductor [options] build
  conductor [options] pull
  conductor [options] up
  conductor [options] stop
  conductor [options] exec [exec options] <pod> <service> <command> [--] [<args>..]
  conductor [options] shell [exec options] <pod> <service>
  conductor [options] test <pod> <service>
  conductor [options] repo list
  conductor [options] repo clone <repo>
  conductor (--help | --version)

Commands:
  new               Create a directory containing a new sample project
  build             Build images for the containers associated with this project
  pull              Pull Docker images used by project
  up                Run project
  stop              Stop all containers associated with project
  exec              Run a command inside a container
  shell             Run an interactive shell inside a running container
  test              Run the tests associated with a service, if any
  repo list         List all git repository aliases and URLs
  repo clone        Clone a git repository using its short alias and mount it
                    into the containers that use it

Arguments:
  <name>            The name of the project directory to create
  <repo>            Short alias for a repo (see `repo list`)
  <pod>             The name of a pod specified in `pods/`
  <service>         The name of a service in a pod

Exec options:
  -d                Run command detached in background
  --privileged      Run a command with elevated privileges
  --user <user>     User as which to run a command
  -T                Do not allocate a TTY when running a command

General options:
  -h, --help        Show this message
  --version         Show the version of conductor
  -p, --project-name <project_name>
                    The name of this project.  Defaults to the current
                    directory name.
  --override=<override>
                    Use overrides from the specified subdirectory of
                    `pods/overrides` [default: development]
  --default-tags=<tag_file>
                    A list of tagged image names, one per line, to
                    be used as defaults for images

Run conductor in a directory containing a `pods` subdirectory.  For more
information, see https://github.com/faradayio/conductor.
";

/// Our parsed command-line arguments.  See [docopt.rs][] for an
/// explanation of how this works.
///
/// [docopt.rs]: https://github.com/docopt/docopt.rs
#[derive(Debug, RustcDecodable)]
#[allow(non_snake_case)] // Allow uppercase options without warnings.
struct Args {
    cmd_build: bool,
    cmd_pull: bool,
    cmd_up: bool,
    cmd_stop: bool,
    cmd_exec: bool,
    cmd_shell: bool,
    cmd_test: bool,
    cmd_repo: bool,
    cmd_list: bool,
    cmd_clone: bool,
    cmd_new: bool,

    arg_args: Option<Vec<String>>,
    arg_command: Option<String>,
    arg_name: Option<String>,
    arg_pod: Option<String>,
    arg_repo: Option<String>,
    arg_service: Option<String>,

    // Exec options.
    flag_d: bool,
    flag_privileged: bool,
    flag_user: Option<String>,
    flag_T: bool,

    // General options.
    flag_version: bool,
    flag_default_tags: Option<String>,
    flag_override: String,
    flag_project_name: Option<String>,
}

impl Args {
    /// Extract `exec::Options` from our command-line arguments.
    fn to_exec_options(&self) -> conductor::exec::Options {
        conductor::exec::Options {
            detached: self.flag_d,
            privileged: self.flag_privileged,
            user: self.flag_user.clone(),
            allocate_tty: !self.flag_T,
            ..Default::default()
        }
    }

    /// Extract `exec::Target` from our command-line arguments.
    fn to_exec_target<'a>(&'a self, project: &'a conductor::Project,
                          ovr: &'a conductor::Override) ->
        Result<Option<conductor::exec::Target<'a>>, Error>
    {
        match (&self.arg_pod, &self.arg_service) {
            (&Some(ref pod), &Some(ref service)) =>
                Ok(Some(try!(conductor::exec::Target::new(project, ovr, pod,
                                                          service)))),
            _ => Ok(None),
        }
    }

    /// Extract `exec::Command` from our command-line arguments.
    fn to_exec_command(&self) -> Option<conductor::exec::Command> {
        // We have an `Option<Vec<String>>` and we want a `&[String]`,
        // so do a little munging.
        let args = self.arg_args.as_ref()
            .map(|v| (v as &[_])).unwrap_or(&[]);
        if let &Some(ref command) = &self.arg_command {
            Some(conductor::exec::Command::new(command).with_args(&args))
        } else {
            None
        }
    }
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `try!` to handle errors.
fn run(args: &Args) -> Result<(), Error> {

    if args.cmd_new {
        try!(conductor::Project::generate(&try!(env::current_dir()),
                                          args.arg_name.as_ref().unwrap()));
        return Ok(());
    }

    let mut proj = try!(conductor::Project::from_current_dir());
    if let Some(ref project_name) = args.flag_project_name {
        proj.set_name(project_name);
    }
    if let Some(ref default_tags_path) = args.flag_default_tags {
        let file = try!(fs::File::open(default_tags_path));
        proj.set_default_tags(try!(conductor::DefaultTags::read(file)));
    }
    let ovr = try!(proj.ovr(&args.flag_override).ok_or_else(|| {
        err!("override {} is not defined", &args.flag_override)
    }));
    try!(proj.output());
    let runner = OsCommandRunner;

    if args.cmd_pull {
        try!(proj.pull(&runner, &ovr));
    } else if args.cmd_build {
        try!(proj.build(&runner, &ovr));
    } else if args.cmd_up {
        try!(proj.up(&runner, &ovr));
    } else if args.cmd_stop {
        try!(proj.stop(&runner, &ovr));
    } else if args.cmd_exec {
        let target = try!(args.to_exec_target(&proj, &ovr)).unwrap();
        let opts = args.to_exec_options();
        let cmd = args.to_exec_command().unwrap();
        try!(proj.exec(&runner, &target, &cmd, &opts));
    } else if args.cmd_shell {
        let target = try!(args.to_exec_target(&proj, &ovr)).unwrap();
        let opts = args.to_exec_options();
        try!(proj.shell(&runner, &target, &opts));
    } else if args.cmd_test {
        let test_ovr = try!(proj.ovr("test").ok_or_else(|| {
            err!("override test is required to run tests")
        }));
        let target = try!(args.to_exec_target(&proj, &test_ovr)).unwrap();
        try!(proj.test(&runner, &target));
    } else if args.cmd_repo && args.cmd_list {
        try!(proj.repo_list(&runner));
    } else if args.cmd_repo && args.cmd_clone {
        try!(proj.repo_clone(&runner, args.arg_repo.as_ref().unwrap()));
        // Regenerate our output now that we've cloned.
        try!(proj.output());
    }

    Ok(())
}

/// Our main entry point.
fn main() {
    // Initialize logging with some custom options, mostly so we can see
    // our own warnings.
    let mut builder = env_logger::LogBuilder::new();
    builder.filter(Some("docker_compose"), log::LogLevelFilter::Warn);
    builder.filter(Some("conductor"), log::LogLevelFilter::Warn);
    if let Ok(config) = env::var("RUST_LOG") {
        builder.parse(&config);
    }
    builder.init().unwrap();

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
        write!(io::stderr(), "Error: {}\n", err).unwrap();
        process::exit(1);
    }
}
