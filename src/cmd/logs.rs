//! The `logs` command.

use crate::args;
use crate::cmd::CommandCompose;
use crate::command_runner::CommandRunner;
#[cfg(test)]
use crate::command_runner::TestCommandRunner;
use crate::errors::*;
use crate::project::Project;

/// We implement `logs` with a trait so we put it in its own module.
pub trait CommandLogs {
    /// Display logs for a given service
    fn logs<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Logs,
    ) -> Result<()>
    where
        CR: CommandRunner;
}

impl CommandLogs for Project {
    fn logs<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Logs,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        match *act_on {
            args::ActOn::Named(ref names) if names.len() == 1 => {
                self.compose(runner, "logs", act_on, opts)
            }
            _ => Err("You may only specify a single service or pod".into()),
        }
    }
}

#[test]
fn runs_docker_compose_logs() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("logs").unwrap();

    let opts = args::opts::Logs::default();
    proj.logs(
        &runner,
        &args::ActOn::Named(vec!["frontend".to_owned()]),
        &opts,
    )
    .unwrap();
    assert_ran!(runner, {
        [
            "docker-compose",
            "-p",
            "railshello",
            "-f",
            proj.output_dir().join("pods").join("frontend.yml"),
            "logs",
        ]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn errors_when_act_on_specifies_multiple_containers() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("logs").unwrap();

    let opts = args::opts::Logs::default();
    assert!(proj.logs(&runner, &args::ActOn::All, &opts).is_err());

    proj.remove_test_output().unwrap();
}
