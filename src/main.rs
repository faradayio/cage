//! Our main CLI tool.

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#![deny(warnings)]

#[macro_use]
extern crate cage;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rustc_serialize;

use docopt::Docopt;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

use cage::command_runner::{Command, CommandRunner, OsCommandRunner};
use cage::cmd::*;
use cage::Result;

/// Our version number, set by Cargo at compile time.
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// Our help string.
const USAGE: &'static str = "
cage: Manage large, multi-pod docker-compose apps

Usage:
  cage [options] new <name>
  cage [options] build
  cage [options] pull
  cage [options] up [<pods>..]
  cage [options] stop
  cage [options] run [exec options] <pod> [<command> [--] [<args>...]]
  cage [options] exec [exec options] <pod> <service> <command> [--] [<args>..]
  cage [options] shell [exec options] <pod> <service>
  cage [options] test <pod> <service>
  cage [options] repo list
  cage [options] repo clone <repo>
  cage [options] generate list
  cage [options] generate <generator>
  cage [options] export <dir>
  cage (--help | --version | --all-versions)

Commands:
  new               Create a directory containing a new sample project
  build             Build images for the containers associated with this project
  pull              Pull Docker images used by project
  up                Run project
  stop              Stop all containers associated with project
  run               Run a specific pod as a one-shot task
  exec              Run a command inside a container
  shell             Run an interactive shell inside a running container
  test              Run the tests associated with a service, if any
  repo list         List all git repository aliases and URLs
  repo clone        Clone a git repository using its short alias and mount it
                    into the containers that use it
  generate list     List all available generators
  generate          Run the specified generator
  export            Export to the named directory as flattened *.yml files

Arguments:
  <dir>             The name of a directory
  <name>            The name of the project directory to create
  <pod>, <pods>     The name of a pod specified in `pods/`
  <repo>            Short alias for a repo (see `repo list`)
  <service>         The name of a service in a pod
  <generator>       The name of a generator

Exec options:
  -d                Run command detached in background
  --privileged      Run a command with elevated privileges
  --user <user>     User as which to run a command
  -T                Do not allocate a TTY when running a command

General options:
  -h, --help        Show this message
  --version         Show the version of cage
  --all-versions    Show the version of cage and supporting tools
  -p, --project-name <project_name>
                    The name of this project.  Defaults to the current
                    directory name.
  --override=<override>
                    Use overrides from the specified subdirectory of
                    `pods/overrides`.  Defaults to `development` unless
                    running tests.
  --default-tags=<tag_file>
                    A list of tagged image names, one per line, to
                    be used as defaults for images

Run `cage` in a directory containing a `pods` subdirectory.  For more
information, see https://github.com/faradayio/cage.
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
    cmd_run: bool,
    cmd_exec: bool,
    cmd_shell: bool,
    cmd_test: bool,
    cmd_repo: bool,
    cmd_list: bool,
    cmd_clone: bool,
    cmd_new: bool,
    cmd_generate: bool,
    cmd_export: bool,

    arg_args: Option<Vec<String>>,
    arg_command: Option<String>,
    arg_dir: Option<String>,
    arg_generator: Option<String>,
    arg_name: Option<String>,
    arg_pod: Option<String>,
    arg_pods: Vec<String>,
    arg_repo: Option<String>,
    arg_service: Option<String>,

    // Exec options.
    flag_d: bool,
    flag_privileged: bool,
    flag_user: Option<String>,
    flag_T: bool,

    // General options.
    flag_version: bool,
    flag_all_versions: bool,
    flag_default_tags: Option<String>,
    flag_override: Option<String>,
    flag_project_name: Option<String>,
}

impl Args {
    /// Do we need to generate `.cage/pods`?  This will probably be
    /// refactored in the future.
    fn should_output_project(&self) -> bool {
        !self.cmd_export
    }

    /// Get either the specified override name, or a reasonable default.
    fn override_name(&self) -> &str {
        self.flag_override
            .as_ref()
            .map_or_else(|| { if self.cmd_test { "test" } else { "development" } },
                         |s| &s[..])
    }

    /// Extract `exec::Options` from our command-line arguments.
    fn to_exec_options(&self) -> cage::exec::Options {
        cage::exec::Options {
            detached: self.flag_d,
            privileged: self.flag_privileged,
            user: self.flag_user.clone(),
            allocate_tty: !self.flag_T,
            ..Default::default()
        }
    }

    /// Extract `exec::Target` from our command-line arguments.
    fn to_exec_target<'a>(&'a self,
                          project: &'a cage::Project,
                          ovr: &'a cage::Override)
                          -> Result<Option<cage::exec::Target<'a>>> {
        match (&self.arg_pod, &self.arg_service) {
            (&Some(ref pod), &Some(ref service)) => {
                Ok(Some(try!(cage::exec::Target::new(project, ovr, pod, service))))
            }
            _ => Ok(None),
        }
    }

    /// Extract `exec::Command` from our command-line arguments.
    fn to_exec_command(&self) -> Option<cage::exec::Command> {
        // We have an `Option<Vec<String>>` and we want a `&[String]`,
        // so do a little munging.
        let args = self.arg_args
            .as_ref()
            .map_or(&[] as &[_], |v| (v as &[_]));
        if let Some(ref command) = self.arg_command {
            Some(cage::exec::Command::new(command).with_args(args))
        } else {
            None
        }
    }
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `try!` to handle errors.
fn run(args: &Args) -> Result<()> {

    // Handle any flags or arguments we can handle without a project
    // directory.
    if args.flag_all_versions {
        try!(all_versions());
        return Ok(());
    } else if args.flag_version {
        version();
        return Ok(());
    } else if args.cmd_new {
        try!(cage::Project::generate_new(&try!(env::current_dir()),
                                         args.arg_name.as_ref().unwrap()));
        return Ok(());
    }

    let mut proj = try!(cage::Project::from_current_dir());
    if let Some(ref project_name) = args.flag_project_name {
        proj.set_name(project_name);
    }
    if let Some(ref default_tags_path) = args.flag_default_tags {
        let file = try!(fs::File::open(default_tags_path));
        proj.set_default_tags(try!(cage::DefaultTags::read(file)));
    }
    let override_name = args.override_name();
    let ovr = try!(proj.ovr(override_name)
        .ok_or_else(|| err!("override {} is not defined", override_name)));

    // Output our `*.yml` files if requested.
    if args.should_output_project() {
        try!(proj.output(ovr));
    }

    let runner = OsCommandRunner::new();
    if args.cmd_pull {
        try!(proj.pull(&runner, &ovr));
    } else if args.cmd_build {
        try!(proj.build(&runner, &ovr));
    } else if args.cmd_up {
        if args.arg_pods.is_empty() {
            try!(proj.up_all(&runner, &ovr));
        } else {
            let pods: Vec<&str> = args.arg_pods
                .iter()
                .map(|p| &p[..])
                .collect();
            try!(proj.up(&runner, &ovr, &pods));
        }
    } else if args.cmd_stop {
        try!(proj.stop(&runner, &ovr));
    } else if args.cmd_run {
        let opts = args.to_exec_options();
        let cmd = args.to_exec_command();
        try!(proj.run(&runner,
                      &ovr,
                      args.arg_pod.as_ref().unwrap(),
                      cmd.as_ref(),
                      &opts));
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
        let test_ovr = try!(proj.ovr("test")
            .ok_or_else(|| cage::err("override test is required to run tests")));
        let target = try!(args.to_exec_target(&proj, &test_ovr)).unwrap();
        try!(proj.test(&runner, &target));
    } else if args.cmd_repo && args.cmd_list {
        try!(proj.repo_list(&runner));
    } else if args.cmd_repo && args.cmd_clone {
        try!(proj.repo_clone(&runner, args.arg_repo.as_ref().unwrap()));
        // Regenerate our output now that we've cloned.
        try!(proj.output(ovr));
    } else if args.cmd_generate && args.cmd_list {
        try!(proj.generate_list())
    } else if args.cmd_generate {
        try!(proj.generate(&args.arg_generator.as_ref().unwrap()))
    } else if args.cmd_export {
        try!(proj.export(&ovr, &Path::new(args.arg_dir.as_ref().unwrap())));
    } else {
        // The above cases should be exhaustive.
        unreachable!()
    }

    Ok(())
}

/// Print the version of this executable.
fn version() {
    println!("cage {}", VERSION);
}

/// Print the version of this executable and also the versions of several
/// tools we use.
fn all_versions() -> Result<()> {
    version();

    let runner = OsCommandRunner::new();
    for tool in &["docker", "docker-compose", "git"] {
        try!(runner.build(tool)
            .arg("--version")
            .exec());
    }
    Ok(())
}

/// Our main entry point.
fn main() {
    // Initialize logging with some custom options, mostly so we can see
    // our own warnings.
    let mut builder = env_logger::LogBuilder::new();
    builder.filter(Some("compose_yml"), log::LogLevelFilter::Warn);
    builder.filter(Some("cage"), log::LogLevelFilter::Warn);
    if let Ok(config) = env::var("RUST_LOG") {
        builder.parse(&config);
    }
    builder.init().unwrap();

    // Parse our args using docopt.rs.
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
    debug!("Arguments: {:?}", &args);

    // Defer all our real work to `run`, and handle any errors.  This is a
    // standard Rust pattern to make error-handling in `main` nicer.
    if let Err(ref err) = run(&args) {
        // We use `unwrap` here to turn I/O errors into application panics.
        // If we can't print a message to stderr without an I/O error,
        // the situation is hopeless.
        write!(io::stderr(), "Error: ").unwrap();
        for e in err.iter() {
            write!(io::stderr(), "{}\n", e).unwrap();
        }
        if let Some(backtrace) = err.backtrace() {
            write!(io::stderr(), "{:?}\n", backtrace).unwrap();
        }
        process::exit(1);
    }
}
