//! The `up` command.

use args;
use cmd::CommandCompose;
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use pod::{Pod, PodType};
use project::Project;

/// We implement `up` with a trait so we put it in its own module.
pub trait CommandUp {
    /// Up all the images in the specified pods.
    fn up<CR>(&self,
              runner: &CR,
              act_on: &args::ActOn,
              opts: &args::opts::Up)
              -> Result<()>
        where CR: CommandRunner;
}

impl CommandUp for Project {
    fn up<CR>(&self,
              runner: &CR,
              act_on: &args::ActOn,
              opts: &args::opts::Up)
              -> Result<()>
        where CR: CommandRunner
    {
        let pred = |p: &Pod| p.pod_type() != PodType::Task;
        self.compose(runner, "up", act_on, &pred, opts)
    }
}

#[test]
fn runs_docker_compose_up_honors_enable_in_overrides() {
    use env_logger;
    let _ = env_logger::init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    proj.set_current_override_name("production").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let opts = args::opts::Up::default();
    proj.up(&runner, &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "up",
         "-d"]
    });

    proj.remove_test_output().unwrap();
}
