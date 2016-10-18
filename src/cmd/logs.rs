//! The `logs` command.

use args;
use cmd::CommandCompose;
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use pod::{Pod, PodType};
use project::Project;

/// We implement `logs` with a trait so we put it in its own module.
pub trait CommandLogs {
    /// Display logs for a given service
    fn logs<CR>(&self,
               runner: &CR,
               act_on: &args::ActOn,
               opts: &args::opts::Logs)
              -> Result<()>
        where CR: CommandRunner;
}

impl CommandLogs for Project {
    fn logs<CR>(&self,
              runner: &CR,
              act_on: &args::ActOn,
              opts: &args::opts::Logs)
              -> Result<()>
        where CR: CommandRunner
    {
        return match *act_on {
            args::ActOn::All => {
                Err("You may only specify a single service or pod".into())
            },
            args::ActOn::Named(ref names) => {
                if names.len() > 1 {
                    Err("You may only specify a single service or pod".into())
                } else {
                    let pred = |p: &Pod| p.pod_type() != PodType::Task;
                    self.compose(runner, "logs", act_on, pred, opts)
                }
            }
        }
    }
}

#[test]
fn runs_docker_compose_logs() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let opts = args::opts::Logs::default();
    proj.logs(
        &runner,
        &args::ActOn::Named(vec!("frontend".to_owned())),
        &opts
    ).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "logs"]
    });

    proj.remove_test_output().unwrap();
}

#[test]
fn errors_when_act_on_specifies_multiple_containers() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let opts = args::opts::Logs::default();
    assert!(proj.logs(&runner, &args::ActOn::All, &opts).is_err());

    proj.remove_test_output().unwrap();
}
