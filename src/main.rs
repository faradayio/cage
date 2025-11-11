//! Our main CLI tool.

#![allow(clippy::field_reassign_with_default)]

use cage::{
    cmd::*,
    command_runner::{Command, CommandRunner, OsCommandRunner},
    Error, Project, Result,
};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    path::Path,
    process,
};

#[macro_use]
extern crate log;

const AFTER_HELP: &str = r#"To create a new project:

    cage new myproj

From inside a project directory:

    cage pull                      # Download images for the a project
    cage up --init                 # Start the app, initializing the database
    cage status                    # Get an overview of the project

Access your application at http://localhost:3000/.  To download and edit
the source code for your application, run:

    cage source ls                 # List available service source code
    cage source mount rails_hello  # Clone source and configure mounts
    cage up                        # Restart any affected services
    cage status                    # See how things have changed

Now create `src/rails_hello/public/index.html` and reload in your browser.

Cage is copyright 2016 by Faraday, Inc., and distributed under either the
Apache 2.0 or MIT license. For more information, see
https://github.com/faradayio/cage."#;

#[derive(Parser, Debug)]
#[command(name = "cage", version, about = "Develop complex projects with lots of Docker services", after_help = AFTER_HELP)]
struct Cli {
    #[arg(
        short = 'p',
        long = "project-name",
        value_name = "PROJECT_NAME",
        help = "The name of this project.  Defaults to the current directory name."
    )]
    project_name: Option<String>,

    #[arg(
        long = "target",
        value_name = "TARGET",
        help = "Override settings with values from the specified subdirectory of `pods/targets`.  Defaults to `development` unless running tests."
    )]
    target: Option<String>,

    #[arg(
        long = "default-tags",
        value_name = "TAG_FILE",
        help = "A list of tagged image names, one per line, to be used as defaults for images."
    )]
    default_tags: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Print information about the system", hide = true)]
    Sysinfo,

    #[command(about = "Create a directory containing a new project")]
    New {
        #[arg(value_name = "NAME", help = "The name of the new project")]
        name: String,
    },

    #[command(about = "Print out the status of the current project")]
    Status {
        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(about = "Build images for the containers associated with this project")]
    Build {
        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(about = "Build images for the containers associated with this project")]
    Pull {
        #[arg(short = 'q', long = "quiet", help = "Don't show download progress")]
        quiet: bool,

        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(about = "Run project")]
    Up {
        #[arg(
            long = "init",
            help = "Run any pod initialization commands (for first startup)"
        )]
        init: bool,

        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(about = "Restart all services associated with this project")]
    Restart {
        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(about = "Stop all containers associated with this project")]
    Stop {
        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(about = "Remove the containers associated with a pod or service")]
    Rm {
        #[arg(short = 'f', long = "force", help = "Remove without confirming first")]
        force: bool,

        #[arg(
            short = 'v',
            help = "Remove anonymous volumes associated with containers"
        )]
        remove_volumes: bool,

        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(
        about = "Run a specific pod as a one-shot task",
        trailing_var_arg = true
    )]
    Run {
        #[arg(short = 'd', help = "Run command detached in background")]
        detached: bool,

        #[arg(
            long = "user",
            value_name = "USER",
            help = "User as which to run a command"
        )]
        user: Option<String>,

        #[arg(short = 'T', help = "Do not allocate a TTY when running a command")]
        no_allocate_tty: bool,

        #[arg(
            long = "entrypoint",
            value_name = "ENTRYPOINT",
            help = "Override the entrypoint of the service"
        )]
        entrypoint: Option<String>,

        #[arg(short = 'e', value_names = &["KEY", "VAL"], num_args = 2, value_delimiter = '=', help = "Set an environment variable in the container")]
        environment: Vec<String>,

        #[arg(
            value_name = "SERVICE",
            help = "The name of the service, either as `pod/service`, or as just `service` if unique"
        )]
        service: String,

        #[arg(
            value_name = "COMMAND",
            help = "The command to run, with any arguments"
        )]
        command: Vec<String>,
    },

    #[command(
        about = "Run a named script defined in metadata for specified pods or services"
    )]
    RunScript {
        #[arg(
            long = "no-deps",
            help = "Do not start linked services when running scripts"
        )]
        no_deps: bool,

        #[arg(value_name = "SCRIPT_NAME", help = "The named script to run")]
        script_name: String,

        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(
        about = "Run a command inside an existing container",
        trailing_var_arg = true
    )]
    Exec {
        #[arg(short = 'd', help = "Run command detached in background")]
        detached: bool,

        #[arg(
            long = "user",
            value_name = "USER",
            help = "User as which to run a command"
        )]
        user: Option<String>,

        #[arg(short = 'T', help = "Do not allocate a TTY when running a command")]
        no_allocate_tty: bool,

        #[arg(long = "privileged", help = "Run a command with elevated privileges")]
        privileged: bool,

        #[arg(
            value_name = "SERVICE",
            help = "The name of the service, either as `pod/service`, or as just `service` if unique"
        )]
        service: String,

        #[arg(
            value_name = "COMMAND",
            help = "The command to run, with any arguments"
        )]
        command: Vec<String>,
    },

    #[command(about = "Run an interactive shell inside a running container")]
    Shell {
        #[arg(short = 'd', help = "Run command detached in background")]
        detached: bool,

        #[arg(
            long = "user",
            value_name = "USER",
            help = "User as which to run a command"
        )]
        user: Option<String>,

        #[arg(short = 'T', help = "Do not allocate a TTY when running a command")]
        no_allocate_tty: bool,

        #[arg(long = "privileged", help = "Run a command with elevated privileges")]
        privileged: bool,

        #[arg(
            value_name = "SERVICE",
            help = "The name of the service, either as `pod/service`, or as just `service` if unique"
        )]
        service: String,
    },

    #[command(
        about = "Run the tests associated with a service, if any",
        trailing_var_arg = true,
        after_help = r#"To enable tests for a service, add a label with the test command.
Assuming your service uses rspec, this might look like:

    myservice:
      labels:
        io.fdy.cage.test: "rspec"

Run this test command using:

    cage test myservice

To run only a subset of your tests, you can also pass a custom test
command:

    cage test myservice rspec spec/my_new_feature_spec.rb
"#
    )]
    Test {
        #[arg(
            long = "export-test-output",
            help = "Copy container's $WORKDIR/test_output to $PROJECT_DIR/test_output"
        )]
        export_test_output: bool,

        #[arg(
            value_name = "SERVICE",
            help = "The name of the service, either as `pod/service`, or as just `service` if unique"
        )]
        service: String,

        #[arg(
            value_name = "COMMAND",
            help = "The command to run, with any arguments"
        )]
        command: Vec<String>,
    },

    #[command(about = "Display logs for a service")]
    Logs {
        #[arg(short = 'f', help = "Follow log output")]
        follow: bool,

        #[arg(
            long = "tail",
            value_name = "NUMBER",
            help = "Number of lines from end of output to display"
        )]
        number: Option<String>,

        #[arg(
            value_name = "POD_OR_SERVICE",
            help = "Pod or service names.  Defaults to all."
        )]
        pod_or_service: Vec<String>,
    },

    #[command(
        about = "Commands for working with git repositories and local source trees"
    )]
    Source {
        #[command(subcommand)]
        command: SourceCommands,
    },

    #[command(about = "Commands for generating new source files")]
    Generate {
        #[command(subcommand)]
        command: GenerateCommands,
    },

    #[command(about = "Export project as flattened *.yml files")]
    Export {
        #[arg(value_name = "DIR", help = "The name of the directory to create")]
        dir: String,
    },
}

#[derive(Subcommand, Debug)]
enum SourceCommands {
    #[command(about = "List all known source tree aliases and URLs")]
    Ls,

    #[command(
        about = "Clone a git repository using its short alias and mount it into the containers that use it"
    )]
    Clone {
        #[arg(
            value_name = "ALIAS",
            help = "The short alias of the repo to clone (see `source list`)"
        )]
        alias: String,
    },

    #[command(about = "Mount a source tree into the containers that use it")]
    Mount {
        #[arg(
            value_name = "ALIASES",
            help = "The short aliases of the source trees to operate on (see `source list`)"
        )]
        aliases: Vec<String>,

        #[arg(
            short = 'a',
            long = "all",
            help = "Operate on all source trees",
            conflicts_with = "aliases"
        )]
        all: bool,
    },

    #[command(about = "Unmount a local source tree from all containers")]
    Unmount {
        #[arg(
            value_name = "ALIASES",
            help = "The short aliases of the source trees to operate on (see `source list`)"
        )]
        aliases: Vec<String>,

        #[arg(
            short = 'a',
            long = "all",
            help = "Operate on all source trees",
            conflicts_with = "aliases"
        )]
        all: bool,
    },
}

#[derive(Subcommand, Debug)]
enum GenerateCommands {
    #[command(
        about = "Generate shell autocompletion support",
        after_help = r#"To set up shell auto-completion for bash:

    cage generate completion bash
    source cage.bash-completion

And set up your ~/.profile or ~/.bash_profile to source this file on
each login.

To set up shell auto-completion for fish:

    cage generate completion fish
    source cage.fish
    mkdir -p ~/.config/fish/completions
    mv cage.fish ~/.config/fish/completions
"#
    )]
    Completion {
        #[arg(
            value_enum,
            value_name = "SHELL",
            help = "The name of shell for which to generate an autocompletion script"
        )]
        shell: Shell,
    },

    #[command(about = "Generate config/secrets.yml for local secret storage")]
    Secrets,

    #[command(about = "Generate config/vault.yml for fetching secrets from vault")]
    Vault,
}

#[derive(Debug, Clone, ValueEnum)]
enum Shell {
    Bash,
    Fish,
}

impl Cli {
    fn should_output_project(&self) -> bool {
        !matches!(self.command, Commands::Export { .. })
    }

    fn target_name(&self) -> &str {
        self.target.as_deref().unwrap_or({
            if matches!(self.command, Commands::Test { .. }) {
                "test"
            } else {
                "development"
            }
        })
    }
}

fn to_acts_on(pod_or_service: &[String], include_tasks: bool) -> cage::args::ActOn {
    if pod_or_service.is_empty() {
        if include_tasks {
            cage::args::ActOn::All
        } else {
            cage::args::ActOn::AllExceptTasks
        }
    } else {
        cage::args::ActOn::Named(pod_or_service.to_vec())
    }
}

fn to_acts_on_sources(
    aliases: &[String],
    all: bool,
    proj: &Project,
) -> Result<cage::args::ActOnSources> {
    if all {
        Ok(cage::args::ActOnSources::All)
    } else if !aliases.is_empty() {
        let validated_aliases = aliases
            .iter()
            .map(|a| -> Result<String> {
                if proj.sources().find_by_alias(a).is_none() {
                    Err(Error::UnknownSource(a.to_owned()).into())
                } else {
                    Ok(a.to_owned())
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(cage::args::ActOnSources::Named(validated_aliases))
    } else {
        panic!("clap source always require --all or a list of sources");
    }
}

fn to_process_options(
    detached: bool,
    user: &Option<String>,
    no_allocate_tty: bool,
) -> cage::args::opts::Process {
    let mut opts = cage::args::opts::Process::default();
    opts.detached = detached;
    opts.user = user.clone();
    opts.allocate_tty = !no_allocate_tty;
    opts
}

fn to_exec_options(
    detached: bool,
    user: &Option<String>,
    no_allocate_tty: bool,
    privileged: bool,
) -> cage::args::opts::Exec {
    let mut opts = cage::args::opts::Exec::default();
    opts.process = to_process_options(detached, user, no_allocate_tty);
    opts.privileged = privileged;
    opts
}

fn to_run_options(
    detached: bool,
    user: &Option<String>,
    no_allocate_tty: bool,
    entrypoint: &Option<String>,
    environment: &[String],
    no_deps: bool,
) -> cage::args::opts::Run {
    let mut opts = cage::args::opts::Run::default();
    opts.process = to_process_options(detached, user, no_allocate_tty);
    opts.entrypoint = entrypoint.clone();

    let mut env_map = BTreeMap::new();
    for chunk in environment.chunks(2) {
        if chunk.len() == 2 {
            env_map.insert(chunk[0].to_owned(), chunk[1].to_owned());
        }
    }
    opts.environment = env_map;
    opts.no_deps = no_deps;
    opts
}

fn to_test_options(export_test_output: bool) -> cage::args::opts::Test {
    let mut opts = cage::args::opts::Test::default();
    opts.export_test_output = export_test_output;
    opts
}

fn to_logs_options(follow: bool, number: &Option<String>) -> cage::args::opts::Logs {
    let mut opts = cage::args::opts::Logs::default();
    opts.follow = follow;
    opts.number = number.clone();
    opts
}

fn to_rm_options(force: bool, remove_volumes: bool) -> cage::args::opts::Rm {
    let mut opts = cage::args::opts::Rm::default();
    opts.force = force;
    opts.remove_volumes = remove_volumes;
    opts
}

fn to_exec_command(command: &[String]) -> Option<cage::args::Command> {
    if command.is_empty() {
        None
    } else {
        Some(cage::args::Command::new(&command[0]).with_args(&command[1..]))
    }
}

/// Display a warning if we think some of our services should be running but
/// they're not.
fn warn_if_pods_are_enabled_but_not_running(project: &cage::Project) -> Result<()> {
    let pods = project.enabled_pods_that_are_not_running()?;
    if !pods.is_empty() {
        let pod_names = pods.iter().map(|p| p.name());
        warn!(
            "You might want to start the following pods first: {0} (see \
             `cage --target={1} up` or `cage --target={1} status`)",
            pod_names.format(", "),
            project.current_target().name()
        );
    }
    Ok(())
}

/// The function which does the real work.  Unlike `main`, we have a return
/// type of `Result` and may therefore use `?` to handle errors.
fn run(cli: &Cli) -> Result<()> {
    // Handle any subcommands that we can handle without a project directory.
    match &cli.command {
        Commands::Sysinfo => {
            all_versions()?;
            return Ok(());
        }
        Commands::New { name } => {
            cage::Project::generate_new(&env::current_dir()?, name)?;
            return Ok(());
        }
        _ => {}
    }

    // Handle our standard arguments that apply to all subcommands.
    let mut proj = cage::Project::from_current_dir()?;
    if let Some(project_name) = &cli.project_name {
        proj.set_name(project_name);
    }
    if let Some(default_tags_path) = &cli.default_tags {
        let f = fs::File::open(default_tags_path)?;
        let reader = io::BufReader::new(f);
        proj.set_default_tags(cage::DefaultTags::read(reader)?);
    }
    proj.set_current_target_name(cli.target_name())?;

    // Output our project's `*.yml` files for `docker-compose` if we'll need it.
    let subcommand_name = match &cli.command {
        Commands::Sysinfo => "sysinfo",
        Commands::New { .. } => "new",
        Commands::Status { .. } => "status",
        Commands::Build { .. } => "build",
        Commands::Pull { .. } => "pull",
        Commands::Up { .. } => "up",
        Commands::Restart { .. } => "restart",
        Commands::Stop { .. } => "stop",
        Commands::Rm { .. } => "rm",
        Commands::Run { .. } => "run",
        Commands::RunScript { .. } => "run-script",
        Commands::Exec { .. } => "exec",
        Commands::Shell { .. } => "shell",
        Commands::Test { .. } => "test",
        Commands::Source { .. } => "source",
        Commands::Generate { .. } => "generate",
        Commands::Logs { .. } => "logs",
        Commands::Export { .. } => "export",
    };

    if cli.should_output_project() {
        proj.output(subcommand_name)?;
    }

    // Handle our subcommands that require a `Project`.
    let runner = OsCommandRunner::new();
    match &cli.command {
        Commands::Status { pod_or_service } => {
            let acts_on = to_acts_on(pod_or_service, true);
            proj.status(&runner, &acts_on)?;
        }
        Commands::Pull {
            quiet,
            pod_or_service,
        } => {
            let acts_on = to_acts_on(pod_or_service, true);
            let mut opts = cage::args::opts::Pull::default();
            opts.quiet = *quiet;
            proj.pull(&runner, &acts_on, &opts)?;
        }
        Commands::Build { pod_or_service } => {
            let acts_on = to_acts_on(pod_or_service, true);
            let opts = cage::args::opts::Empty;
            proj.compose(&runner, "build", &acts_on, &opts)?;
        }
        Commands::Up {
            init,
            pod_or_service,
        } => {
            let acts_on = to_acts_on(pod_or_service, false);
            let opts = cage::args::opts::Up::new(*init);
            proj.up(&runner, &acts_on, &opts)?;
        }
        Commands::Restart { pod_or_service } => {
            let acts_on = to_acts_on(pod_or_service, false);
            let opts = cage::args::opts::Empty;
            proj.compose(&runner, "restart", &acts_on, &opts)?;
        }
        Commands::Stop { pod_or_service } => {
            let acts_on = to_acts_on(pod_or_service, false);
            let opts = cage::args::opts::Empty;
            proj.compose(&runner, "stop", &acts_on, &opts)?;
        }
        Commands::Rm {
            force,
            remove_volumes,
            pod_or_service,
        } => {
            let acts_on = to_acts_on(pod_or_service, true);
            let opts = to_rm_options(*force, *remove_volumes);
            proj.compose(&runner, "rm", &acts_on, &opts)?;
        }
        Commands::Run {
            detached,
            user,
            no_allocate_tty,
            entrypoint,
            environment,
            service,
            command,
        } => {
            warn_if_pods_are_enabled_but_not_running(&proj)?;
            let opts = to_run_options(
                *detached,
                user,
                *no_allocate_tty,
                entrypoint,
                environment,
                false,
            );
            let cmd = to_exec_command(command);
            proj.run(&runner, service, cmd.as_ref(), &opts)?;
        }
        Commands::RunScript {
            no_deps,
            script_name,
            pod_or_service,
        } => {
            warn_if_pods_are_enabled_but_not_running(&proj)?;
            let opts = to_run_options(false, &None, false, &None, &[], *no_deps);
            let acts_on = to_acts_on(pod_or_service, true);
            proj.run_script(&runner, &acts_on, script_name.as_ref(), &opts)?;
        }
        Commands::Exec {
            detached,
            user,
            no_allocate_tty,
            privileged,
            service,
            command,
        } => {
            warn_if_pods_are_enabled_but_not_running(&proj)?;
            let opts = to_exec_options(*detached, user, *no_allocate_tty, *privileged);
            let cmd = to_exec_command(command).unwrap();
            proj.exec(&runner, service, &cmd, &opts)?;
        }
        Commands::Shell {
            detached,
            user,
            no_allocate_tty,
            privileged,
            service,
        } => {
            warn_if_pods_are_enabled_but_not_running(&proj)?;
            let opts = to_exec_options(*detached, user, *no_allocate_tty, *privileged);
            proj.shell(&runner, service, &opts)?;
        }
        Commands::Test {
            export_test_output,
            service,
            command,
        } => {
            warn_if_pods_are_enabled_but_not_running(&proj)?;
            let opts = to_test_options(*export_test_output);
            let cmd = to_exec_command(command);
            proj.test(&runner, service, cmd.as_ref(), &opts)?;
        }
        Commands::Source { command } => run_source(&runner, &mut proj, command)?,
        Commands::Generate { command } => run_generate(&runner, &proj, command)?,
        Commands::Logs {
            follow,
            number,
            pod_or_service,
        } => {
            let acts_on = to_acts_on(pod_or_service, true);
            let opts = to_logs_options(*follow, number);
            proj.logs(&runner, &acts_on, &opts)?;
        }
        Commands::Export { dir } => {
            proj.export(Path::new(dir))?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Our `source` subcommand.
fn run_source<R>(
    runner: &R,
    proj: &mut cage::Project,
    command: &SourceCommands,
) -> Result<()>
where
    R: CommandRunner,
{
    let subcommand_name = match command {
        SourceCommands::Ls => "ls",
        SourceCommands::Clone { .. } => "clone",
        SourceCommands::Mount { .. } => "mount",
        SourceCommands::Unmount { .. } => "unmount",
    };

    // Dispatch our subcommand.
    let mut re_output = true;
    match command {
        SourceCommands::Ls => {
            re_output = false;
            proj.source_list(runner)?;
        }
        SourceCommands::Clone { alias } => {
            proj.source_clone(runner, alias)?;
        }
        SourceCommands::Mount { aliases, all } => {
            let act_on_sources = to_acts_on_sources(aliases, *all, proj)?;
            proj.source_set_mounted(runner, act_on_sources, true)?;
        }
        SourceCommands::Unmount { aliases, all } => {
            let act_on_sources = to_acts_on_sources(aliases, *all, proj)?;
            proj.source_set_mounted(runner, act_on_sources, false)?;
        }
    }

    // Regenerate our output if it might have changed.
    if re_output {
        proj.output(subcommand_name)?;
    }

    Ok(())
}

/// Our `generate` subcommand.
fn run_generate<R>(
    _runner: &R,
    proj: &cage::Project,
    command: &GenerateCommands,
) -> Result<()>
where
    R: CommandRunner,
{
    match command {
        GenerateCommands::Completion { shell } => {
            use clap::CommandFactory;
            use clap_complete::{generate, shells};

            let mut cmd = Cli::command();
            match shell {
                Shell::Bash => {
                    generate(shells::Bash, &mut cmd, "cage", &mut io::stdout())
                }
                Shell::Fish => {
                    generate(shells::Fish, &mut cmd, "cage", &mut io::stdout())
                }
            }
        }
        GenerateCommands::Secrets => proj.generate("secrets")?,
        GenerateCommands::Vault => proj.generate("vault")?,
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
        runner.build(tool).arg("--version").exec()?;
    }
    Ok(())
}

fn log_level_label(level: log::Level) -> colored::ColoredString {
    match level {
        log::Level::Error => "ERROR:".red().bold(),
        log::Level::Warn => "WARNING:".yellow().bold(),
        log::Level::Info => "INFO:".bold(),
        log::Level::Debug => "DEBUG:".normal(),
        log::Level::Trace => "TRACE:".normal(),
    }
}

/// Our main entry point.
fn main() {
    // Initialize logging with some custom options, mostly so we can see
    // our own warnings.
    let mut builder = env_logger::Builder::new();
    builder.filter(Some("faraday_compose_yml"), log::LevelFilter::Warn);
    builder.filter(
        Some("faraday_compose_yml::v2::validate"),
        log::LevelFilter::Error,
    );
    builder.filter(Some("cage"), log::LevelFilter::Warn);
    builder.format(
        |f: &mut env_logger::fmt::Formatter, record: &log::Record<'_>| {
            let msg = format!(
                "{} {} (from {})",
                log_level_label(record.level()),
                record.args(),
                record.target()
            );
            if record.level() > log::Level::Info {
                writeln!(f, "{}", msg.dimmed())
            } else {
                writeln!(f, "{}", msg)
            }
        },
    );
    if let Ok(config) = env::var("RUST_LOG") {
        builder.parse_filters(&config);
    }
    builder.init();

    // Parse our command-line arguments.
    let cli = Cli::parse();
    debug!("Arguments: {:?}", &cli);

    // Defer all our real work to `run`, and handle any errors.  This is a
    // standard Rust pattern to make error-handling in `main` nicer.
    if let Err(ref err) = run(&cli) {
        // We use `unwrap` here to turn I/O errors into application panics.
        // If we can't print a message to stderr without an I/O error,
        // the situation is hopeless.
        eprintln!("Error: {:#}", err);
        process::exit(1);
    }
}
