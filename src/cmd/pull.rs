//! The `pull` command.

use std::collections::BTreeMap;

use colored::Colorize;
use rayon::prelude::*;

use crate::args::{self, ToArgs};
#[cfg(test)]
use crate::command_runner::TestCommandRunner;
use crate::command_runner::{Command, CommandRunner};
use crate::errors::*;
use crate::pod::Pod;
use crate::project::{PodOrService, Project};

/// We implement `pull` with a trait so we put it in its own module.
pub trait CommandPull {
    /// Pull all the images associated with a project, in parallel across
    /// pods/services.
    ///
    /// Each per-pod `docker-compose pull` invocation runs in its own
    /// thread; output is buffered per pod and only printed on failure.
    /// On success, only a brief progress line is shown for each pod.
    fn pull<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Pull,
    ) -> Result<()>
    where
        CR: CommandRunner + Sync;
}

impl CommandPull for Project {
    fn pull<CR>(
        &self,
        runner: &CR,
        act_on: &args::ActOn,
        opts: &args::opts::Pull,
    ) -> Result<()>
    where
        CR: CommandRunner + Sync,
    {
        self.hooks().invoke(runner, "pull", &BTreeMap::new())?;

        let target = self.current_target();
        let work: Vec<PodOrService<'_>> = act_on
            .pods_or_services(self)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|pos| match pos {
                PodOrService::Pod(pod) | PodOrService::Service(pod, _) => {
                    pod.enabled_in(target)
                }
            })
            .collect();

        if work.is_empty() {
            return Ok(());
        }

        if !opts.quiet {
            println!("{} {} target(s) in parallel", "Pulling".bold(), work.len(),);
        }

        let results: Vec<Result<()>> = work
            .par_iter()
            .map(|pos| pull_one(runner, self, pos, opts))
            .collect();

        let mut failures = 0usize;
        for r in results {
            if let Err(e) = r {
                failures += 1;
                eprintln!("{} {:#}", "[error]".red().bold(), e);
            }
        }

        if failures > 0 {
            Err(anyhow::anyhow!("{} pull(s) failed", failures))
        } else {
            if !opts.quiet {
                println!("{}", "Pull complete.".bold());
            }
            Ok(())
        }
    }
}

fn pull_one<CR>(
    runner: &CR,
    project: &Project,
    pos: &PodOrService<'_>,
    opts: &args::opts::Pull,
) -> Result<()>
where
    CR: CommandRunner,
{
    match *pos {
        PodOrService::Pod(pod) => pull_pod(runner, project, pod, None, opts),
        PodOrService::Service(pod, service_name) => {
            pull_pod(runner, project, pod, Some(service_name), opts)
        }
    }
}

fn pull_pod<CR>(
    runner: &CR,
    project: &Project,
    pod: &Pod,
    service_name: Option<&str>,
    opts: &args::opts::Pull,
) -> Result<()>
where
    CR: CommandRunner,
{
    let label = match service_name {
        Some(svc) => format!("{}/{}", pod.name(), svc),
        None => pod.name().to_owned(),
    };

    let mut cmd = runner.build("docker-compose");
    cmd.args(&pod.compose_args(project)?)
        .arg("pull")
        .args(&opts.to_args());
    if let Some(svc) = service_name {
        cmd.arg(svc);
    }

    let result = cmd.exec_capturing(&label);
    if result.is_ok() && !opts.quiet {
        println!("  {} {}", "[ok]".green().bold(), label);
    }
    result
}

#[test]
fn runs_docker_compose_pull_on_all_pods() {
    let _ = env_logger::try_init();
    let proj = Project::from_example("hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("pull").unwrap();

    let opts = args::opts::Pull { quiet: true };
    proj.pull(&runner, &args::ActOn::All, &opts).unwrap();

    let cmds = runner.cmds();
    let hook_path = proj
        .root_dir()
        .join("config")
        .join("hooks")
        .join("pull.d")
        .join("hello.hook");

    assert!(
        cmds.iter()
            .any(|c| c.len() == 1 && c[0] == hook_path.as_os_str()),
        "expected pull hook to have been invoked, got {:?}",
        cmds
    );

    let pull_cmds: Vec<&Vec<std::ffi::OsString>> = cmds
        .iter()
        .filter(|c| c.first().is_some_and(|a| a == "docker-compose"))
        .collect();
    assert_eq!(
        pull_cmds.len(),
        1,
        "expected one docker-compose pull, got {:?}",
        pull_cmds
    );
    let pull = pull_cmds[0];
    let expected = [
        "docker-compose".into(),
        "-p".into(),
        std::ffi::OsString::from("hello"),
        "-f".into(),
        proj.output_dir()
            .join("pods")
            .join("frontend.yml")
            .into_os_string(),
        "pull".into(),
        "--quiet".into(),
    ];
    assert_eq!(pull, &expected.to_vec());

    proj.remove_test_output().unwrap();
}
