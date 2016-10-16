//! The `pull` command.

use std::collections::BTreeMap;

use args;
use cmd::CommandCompose;
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use ovr::Override;
use project::Project;

/// We implement `pull` with a trait so we put it in its own module.
pub trait CommandPull {
    /// Pull all the images associated with a project.
    fn pull<CR>(&self,
                runner: &CR,
                ovr: &Override,
                act_on: &args::ActOn)
                -> Result<()>
        where CR: CommandRunner;
}

impl CommandPull for Project {
    fn pull<CR>(&self,
                runner: &CR,
                ovr: &Override,
                act_on: &args::ActOn)
                -> Result<()>
        where CR: CommandRunner
    {
        // Run our hook.
        try!(self.hooks().invoke(runner, "pull", &BTreeMap::new()));

        // Pass everything else off to `compose`, as usual.
        let opts = args::opts::Empty;
        self.compose(runner, ovr, "pull", act_on, |_| true, &opts)
    }
}

#[test]
fn runs_docker_compose_pull_on_all_pods() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();

    proj.pull(&runner, ovr, &args::ActOn::All).unwrap();
    assert_ran!(runner, {
        [proj.root_dir().join("config/hooks/pull.d/hello.hook")],
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods/frontend.yml"),
         "pull"]
    });

    proj.remove_test_output().unwrap();
}
