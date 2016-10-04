//! The `conductor run` command.

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
use exec::{self, ToArgs};
use ext::service::ServiceExt;
use ovr::Override;
use project::Project;
use util::err;

/// We implement `conductor run` with a trait so we put it in its own module.
pub trait CommandRun {
    /// Run a specific pod as a one-shot task.
    fn run<CR>(&self,
               runner: &CR,
               ovr: &Override,
               pod: &str,
               command: Option<&exec::Command>,
               opts: &exec::Options)
               -> Result<()>
        where CR: CommandRunner;

    /// Execute tests inside a fresh container.
    fn test<CR>(&self, runner: &CR, target: &exec::Target) -> Result<()>
        where CR: CommandRunner;
}

impl CommandRun for Project {
    fn run<CR>(&self,
               runner: &CR,
               ovr: &Override,
               pod: &str,
               command: Option<&exec::Command>,
               opts: &exec::Options)
               -> Result<()>
        where CR: CommandRunner
    {
        let pod = try!(self.pod(pod)
            .ok_or_else(|| err!("Cannot find pod {}", pod)));

        // There's no reason docker-compose couldn't support this in the
        // future (it works fine for `exec`), but apparently nobody has
        // gotten around to implementing it.
        if opts.privileged {
            return Err(err("`run` does not currently support `--privileged`"));
        }

        // Get the single service in our pod.
        let file = try!(pod.merged_file(ovr));
        if file.services.len() != 1 {
            return Err(err!("Can only `run` pods with 1 service, {} has {}",
                            pod.name(),
                            file.services.len()));
        }
        let service = file.services.keys().next().expect("should have had a service");

        // Build and run our command.
        let command_args = if let Some(c) = command {
            c.to_args()
        } else {
            vec![]
        };
        let status = try!(runner.build("docker-compose")
            .args(&try!(pod.compose_args(self, ovr)))
            .arg("run")
            .args(&opts.to_args())
            .arg(service)
            .args(&command_args)
            .status());
        if !status.success() {
            return Err(err("Error running docker-compose"));
        }
        Ok(())
    }

    fn test<CR>(&self, runner: &CR, target: &exec::Target) -> Result<()>
        where CR: CommandRunner
    {
        let status = try!(runner.build("docker-compose")
            .args(&try!(target.pod().compose_args(self, target.ovr())))
            .arg("run")
            .arg("--rm")
            .arg("--no-deps")
            .arg(target.service_name())
            .args(&try!(target.service().test_command()))
            .status());
        if !status.success() {
            return Err(err("Error running docker-compose"));
        }

        Ok(())
    }
}

#[test]
fn fails_on_a_multi_service_pod() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    let opts = Default::default();
    assert!(proj.run(&runner, ovr, "frontend", None, &opts).is_err());
}

#[test]
fn runs_a_single_service_pod() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("rails_hello").unwrap();
    let ovr = proj.ovr("development").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    let cmd = exec::Command::new("rake").with_args(&["db:migrate"]);
    let opts = exec::Options { allocate_tty: false, ..Default::default() };
    proj.run(&runner, ovr, "migrate", Some(&cmd), &opts).unwrap();
    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "rails_hello",
         "-f",
         proj.output_dir().join("pods/migrate.yml"),
         "run",
         "-T",
         "migrate",
         "rake",
         "--",
         "db:migrate"]
    });
    proj.remove_test_output().unwrap();
}

#[test]
fn runs_tests() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let ovr = proj.ovr("test").unwrap();
    let runner = TestCommandRunner::new();
    proj.output(ovr).unwrap();
    let target = exec::Target::new(&proj, ovr, "frontend", "proxy").unwrap();

    proj.test(&runner, &target).unwrap();

    assert_ran!(runner, {
        ["docker-compose",
         "-p",
         "hellotest",
         "-f",
         proj.output_pods_dir().join("frontend.yml"),
         "run",
         "--rm",
         "--no-deps",
         "proxy",
         "echo",
         "All tests passed"]
    });

    proj.remove_test_output().unwrap();
}
