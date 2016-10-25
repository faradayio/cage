//! The `up` command.

use args;
use cmd::{CommandCompose, CommandRun};
use command_runner::CommandRunner;
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use pod::{Pod, PodType};
use project::{PodOrService, Project};

/// We implement `up` with a trait so we put it in its own module.
pub trait CommandUp {
    /// Up all the images in the specified pods.
    fn up<CR>(&self,
              runner: &CR,
              act_on: &args::ActOn,
              opts: &args::opts::Up)
              -> Result<()>
        where CR: CommandRunner;

    /// Run the initialization functions for the specified pod.
    fn init_pod<CR>(&self, runner: &CR, pod: &Pod) -> Result<()>
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
        let pods_or_services = act_on.pods_or_services(self)
            // TODO LOW: Refactor this into a `filter_result` helper?
            .filter(|v| {
                match *v {
                    Ok(ref p_s) => p_s.pod_type() != PodType::Task,
                    Err(_) => true,
                }
            });
        for pod_or_service in pods_or_services {
            match try!(pod_or_service) {
                PodOrService::Pod(pod) => {
                    try!(self.compose_pod(runner, "up", pod, opts));
                    if opts.init {
                        try!(self.init_pod(runner, pod));
                    }
                }
                PodOrService::Service(pod, service_name) => {
                    try!(self.compose_service(runner,
                                              "up",
                                              pod,
                                              service_name,
                                              opts));
                }
            }
        }
        Ok(())
    }

    fn init_pod<CR>(&self, runner: &CR, pod: &Pod) -> Result<()>
        where CR: CommandRunner
    {
        for cmd in pod.run_on_init() {
            if cmd.len() < 1 {
                return Err("all `run_on_init` items for '{}' \
                            must have at least one value"
                    .into());
            }
            let pod_name = &cmd[0];
            let cmd = if cmd.len() >= 2 {
                Some(args::Command::new(&cmd[1]).with_args(&cmd[2..]))
            } else {
                None
            };
            let opts = args::opts::Run::default();
            try!(self.run(runner, pod_name, cmd.as_ref(), &opts));
        }
        Ok(())
    }
}

#[test]
fn runs_docker_compose_up_honors_enable_in_targets() {
    use env_logger;
    let _ = env_logger::init();
    let mut proj = Project::from_example("rails_hello").unwrap();
    proj.set_current_target_name("production").unwrap();
    let runner = TestCommandRunner::new();
    proj.output().unwrap();

    let opts = args::opts::Up::default();
    proj.up(&runner, &args::ActOn::All, &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods").join("frontend.yml"),
         "up",
         "-d"]
    });

    proj.remove_test_output().unwrap();
}
