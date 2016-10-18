//! The `logs` command.

use args; ///::{self, ToArgs};
use cmd::CommandCompose;
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ovr::Override;
use pod::{Pod, PodType};
use project::Project;

/// We implement `logs` with a trait so we put it in its own module.
pub trait CommandLogs {
    /// Display logs for a given service
    fn logs<CR>(&self,
               runner: &CR,
               ovr: &Override,
               act_on: &args::ActOn,
               opts: &args::opts::Logs)
              -> Result<()>
        where CR: CommandRunner;
}

impl CommandLogs for Project {
    fn logs<CR>(&self,
              runner: &CR,
              ovr: &Override,
              act_on: &args::ActOn,
              opts: &args::opts::Logs)
              -> Result<()>
        where CR: CommandRunner
    {
        let pred = |p: &Pod| p.pod_type() != PodType::Task;
        self.compose(runner, ovr, "logs", act_on, pred, opts)
    }
}

#[test]
fn runs_docker_compose_logs_honors_enable_in_overrides() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("production").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();

    let opts = args::opts::Logs::default();
    proj.logs(&runner, ovr, &args::ActOn::All, &opts).unwrap();
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
