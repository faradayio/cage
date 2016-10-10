//! Our main CLI tool.

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#![deny(warnings)]

#[macro_use]
extern crate cage;
#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate yaml_rust;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use yaml_rust::yaml;

use cage::command_runner::{Command, CommandRunner, OsCommandRunner};
use cage::cmd::*;
use cage::Result;

/// Load our command-line interface definitions from an external `clap`
/// YAML file.  We could create these using code, but at the cost of more
/// verbosity.
fn cli(yaml: &yaml::Yaml) -> clap::App {
    clap::App::from_yaml(yaml)
        .version(crate_version!())
}

//cli.gen_completions_to("cage", Shell::Fish, &mut std::io::stdout());

/// Custom methods we want to add to `clap::App`.
trait ArgMatchesExt {
    /// Do we need to generate `.cage/pods`?  This will probably be
    /// refactored in the future.
    fn should_output_project(&self) -> bool;

    /// Get either the specified override name, or a reasonable default.
    fn override_name(&self) -> &str;

    /// Extract options shared by `exec` and `run` from our command-line
    /// arguments.
    fn to_common_options(&self) -> cage::exec::CommonOptions;

    /// Extract `exec` options from our command-line arguments.
    fn to_exec_options(&self) -> cage::exec::ExecOptions;

    /// Extract `run` options from our command-line arguments.
    fn to_run_options(&self) -> cage::exec::RunOptions;

    /// Extract `exec::Target` from our command-line arguments.
    fn to_exec_target<'a>(&'a self,
                          project: &'a cage::Project,
                          ovr: &'a cage::Override)
                          -> Result<Option<cage::exec::Target<'a>>>;

    /// Extract `exec::Command` from our command-line arguments.
    fn to_exec_command(&self) -> Option<cage::exec::Command>;
}

impl<'a> ArgMatchesExt for clap::ArgMatches<'a> {
    fn should_output_project(&self) -> bool {
        self.subcommand_name() != Some("export")
    }

    fn override_name(&self) -> &str {
        self.value_of("override")
            .unwrap_or_else(|| {
                if self.subcommand_name() == Some("test") {
                    "test"
                } else {
                    "development"
                }
            })
    }

    fn to_common_options(&self) -> cage::exec::CommonOptions {
        let mut opts = cage::exec::CommonOptions::default();
        opts.detached = self.is_present("detached");
        opts.user = self.value_of("user").map(|v| v.to_owned());
        opts.allocate_tty = !self.is_present("no-allocate-tty");
        opts
    }

    fn to_exec_options(&self) -> cage::exec::ExecOptions {
        let mut opts = cage::exec::ExecOptions::default();
        opts.common = self.to_common_options();
        opts.privileged = self.is_present("privileged");
        opts
    }

    fn to_run_options(&self) -> cage::exec::RunOptions {
        let mut opts = cage::exec::RunOptions::default();
        opts.common = self.to_common_options();
        opts.entrypoint = self.value_of("entrypoint").map(|v| v.to_owned());
        if let Some(environment) = self.values_of("environment") {
            let environment: Vec<&str> = environment.collect();
            for env_val in environment.chunks(2) {
                if env_val.len() != 2 {
                    // Clap should prevent this.
                    panic!("Environment binding '{}' has no value", env_val[0]);
                }
                opts.environment.insert(env_val[0].to_owned(),
                                        env_val[1].to_owned());
            }
        }
        opts
    }

    fn to_exec_target<'b>(&'b self,
                          project: &'b cage::Project,
                          ovr: &'b cage::Override)
                          -> Result<Option<cage::exec::Target<'b>>> {
        match (self.value_of("POD"), self.value_of("SERVICE")) {
            (Some(pod), Some(service)) => {
                Ok(Some(try!(cage::exec::Target::new(project, ovr, pod, service))))
            }
            _ => Ok(None),
        }
    }

    fn to_exec_command(&self) -> Option<cage::exec::Command> {
        if self.is_present("COMMAND") {
            let values: Vec<&str> = self.values_of("COMMAND").unwrap().collect();
            assert!(values.len() >= 1, "too few values from CLI parser");
            Some(cage::exec::Command::new(values[0]).with_args(&values[1..]))
        } else {
            None
        }
    }
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `try!` to handle errors.
fn run(matches: &clap::ArgMatches) -> Result<()> {
    // We know that we always have a subcommand because our `cli.yml`
    // requires this and `clap` is supposed to enforce it.
    let sc_name = matches.subcommand_name().unwrap();
    let sc_matches: &clap::ArgMatches = matches.subcommand_matches(sc_name).unwrap();

    // Handle any subcommands that we can handle without a project
    // directory.
    match sc_name {
        "sysinfo" => {
            try!(all_versions());
            return Ok(());
        }
        "new" => {
            try!(cage::Project::generate_new(&try!(env::current_dir()),
                                             sc_matches.value_of("NAME").unwrap()));
            return Ok(());
        }
        _ => {},
    }

    // Handle our standard arguments that apply to all subcommands.
    let mut proj = try!(cage::Project::from_current_dir());
    if let Some(project_name) = matches.value_of("project-name") {
        proj.set_name(project_name);
    }
    if let Some(default_tags_path) = matches.value_of("default-tags") {
        let file = try!(fs::File::open(default_tags_path));
        proj.set_default_tags(try!(cage::DefaultTags::read(file)));
    }
    let override_name = matches.override_name();
    let ovr = try!(proj.ovr(override_name)
        .ok_or_else(|| err!("override {} is not defined", override_name)));

    // Output our project's `*.yml` files for `docker-compose` if we'll
    // need it.
    if matches.should_output_project() {
        try!(proj.output(ovr));
    }

    // Handle our subcommands that require a `Project`.
    let runner = OsCommandRunner::new();
    match sc_name {
        "pull" => try!(proj.pull(&runner, &ovr)),
        "build" => try!(proj.build(&runner, &ovr)),
        "up" => {
            let pods: Vec<&str> = sc_matches.values_of("POD")
                .map_or_else(|| vec![], |p| p.collect());
            if pods.is_empty() {
                try!(proj.up_all(&runner, &ovr));
            } else {
                try!(proj.up(&runner, &ovr, &pods));
            }
        }
        "stop" => try!(proj.stop(&runner, &ovr)),
        "run" => {
            let opts = sc_matches.to_run_options();
            let cmd = sc_matches.to_exec_command();
            let pod = sc_matches.value_of("POD").unwrap();
            try!(proj.run(&runner, &ovr, pod, cmd.as_ref(), &opts));
        }
        "exec" => {
            let target = try!(sc_matches.to_exec_target(&proj, &ovr)).unwrap();
            let opts = sc_matches.to_exec_options();
            let cmd = sc_matches.to_exec_command().unwrap();
            try!(proj.exec(&runner, &target, &cmd, &opts));
        }
        "shell" => {
            let target = try!(sc_matches.to_exec_target(&proj, &ovr)).unwrap();
            let opts = sc_matches.to_exec_options();
            try!(proj.shell(&runner, &target, &opts));
        }
        "test" => {
            let target = try!(sc_matches.to_exec_target(&proj, &ovr)).unwrap();
            let cmd = sc_matches.to_exec_command();
            try!(proj.test(&runner, &target, cmd.as_ref()));
        }
        "repo" => try!(run_repo(&runner, &proj, &ovr, sc_matches)),
        "generate" => try!(run_generate(&runner, &proj, &ovr, sc_matches)),
        "export" => {
            let dir = sc_matches.value_of("DIR").unwrap();
            try!(proj.export(&ovr, &Path::new(dir)));
        }
        unknown => unreachable!("Unexpected subcommand '{}'", unknown),
    }

    Ok(())
}

/// Our `repo` subcommand.
fn run_repo<R>(runner: &R,
               proj: &cage::Project,
               ovr: &cage::Override,
               matches: &clap::ArgMatches)
               -> Result<()>
    where R: CommandRunner
{
    // We know that we always have a subcommand because our `cli.yml`
    // requires this and `clap` is supposed to enforce it.
    let sc_name = matches.subcommand_name().unwrap();
    let sc_matches: &clap::ArgMatches = matches.subcommand_matches(sc_name).unwrap();

    // Dispatch our subcommand.
    match sc_name {
        "list" => try!(proj.repo_list(runner)),
        "clone" => {
            let alias = sc_matches.value_of("ALIAS").unwrap();
            try!(proj.repo_clone(runner, alias));
            // Regenerate our output now that we've cloned.
            try!(proj.output(ovr));
        }
        unknown => unreachable!("Unexpected subcommand '{}'", unknown),
    }
    Ok(())
}

/// Our `generate` subcommand.
fn run_generate<R>(_runner: &R,
                   proj: &cage::Project,
                   _ovr: &cage::Override,
                   matches: &clap::ArgMatches)
                   -> Result<()>
    where R: CommandRunner
{
    // We know that we always have a subcommand because our `cli.yml`
    // requires this and `clap` is supposed to enforce it.
    let sc_name = matches.subcommand_name().unwrap();
    let sc_matches: &clap::ArgMatches = matches.subcommand_matches(sc_name).unwrap();

    match sc_name {
        // TODO LOW: Allow running this without a project?
        "completion" => {
            let shell = match sc_matches.value_of("SHELL").unwrap() {
                "bash" => clap::Shell::Bash,
                "fish" => clap::Shell::Fish,
                unknown => unreachable!("Unknown shell '{}'", unknown),
            };
            let cli_yaml = load_yaml!("cli.yml");
            cli(cli_yaml).gen_completions("cage", shell, proj.root_dir());
        }
        other => try!(proj.generate(other)),
    }
    Ok(())
}

/// Print the version of this executable.
fn version() {
    println!("cage {}", cage::version());
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

    // Parse our command-line arguments.
    let cli_yaml = load_yaml!("cli.yml");
    let matches: clap::ArgMatches = cli(cli_yaml).get_matches();
    debug!("Arguments: {:?}", &matches);

    // Defer all our real work to `run`, and handle any errors.  This is a
    // standard Rust pattern to make error-handling in `main` nicer.
    if let Err(ref err) = run(&matches) {
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
