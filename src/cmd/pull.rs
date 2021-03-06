//! The `pull` command.

use std::collections::BTreeMap;

use crate::args;
use crate::cmd::CommandCompose;
use crate::command_runner::CommandRunner;
#[cfg(test)]
use crate::command_runner::TestCommandRunner;
use crate::errors::*;
use crate::project::Project;

/// We implement `pull` with a trait so we put it in its own module.
pub trait CommandPull {
    /// Pull all the images associated with a project.
    fn pull<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Pull,
    ) -> Result<()>
    where
        CR: CommandRunner;
}

impl CommandPull for Project {
    fn pull<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Pull,
    ) -> Result<()>
    where
        CR: CommandRunner,
    {
        // Run our hook.
        self.hooks().invoke(runner, "pull", &BTreeMap::new())?;

        // Pass everything else off to `compose`, as usual.
        self.compose(runner, "pull", act_on, opts)
    }
}

#[test]
fn runs_docker_compose_pull_on_all_pods() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("pull").unwrap();

    let opts = args::opts::Pull { quiet: true };
    proj.pull(&runner, &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        [proj.root_dir().join("config").join("hooks").join("pull.d")
             .join("hello.hook")],
        ["docker-compose",
         "-p",
         "hello",
         "-f",
         proj.output_dir().join("pods").join("frontend.yml"),
         "pull",
         "--quiet"]
    });

    proj.remove_test_output().unwrap();
}
