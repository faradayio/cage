//! Hooks that are run during cage execution.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use command_runner::{Command, CommandRunner};
#[cfg(test)]
use command_runner::TestCommandRunner;
use errors::*;
#[cfg(test)]
use project::Project;
use util::ToStrOrErr;

/// Keeps track of hook scripts and invokes them at appropriate times.
#[derive(Debug)]
pub struct HookManager {
    /// A directory containing subdirectories for each hook.
    hooks_dir: PathBuf,

    /// The root directory of our project.
    root_dir: PathBuf,
}

impl HookManager {
    /// Create a new hook manager that runs hooks from the specified
    /// directory.
    pub fn new<P>(root_dir: P) -> Result<HookManager>
        where P: Into<PathBuf>
    {
        let root_dir: PathBuf = root_dir.into();
        Ok(HookManager {
            hooks_dir: root_dir.join("config").join("hooks"),
            root_dir: root_dir,
        })
    }

    /// Invoke all scripts available for the specified hook, passing
    /// `args` as environment variables.
    pub fn invoke<CR>(&self,
                      runner: &CR,
                      hook_name: &str,
                      env: &BTreeMap<String, String>)
                      -> Result<()>
        where CR: CommandRunner
    {

        let d_dir = self.hooks_dir.join(format!("{}.d", hook_name));
        if !d_dir.exists() {
            // Bail early if we don't have a hooks dir.
            debug!("No hooks for '{}' because {} does not exist",
                   hook_name,
                   &d_dir.display());
            return Ok(());
        }

        let mkerr = || ErrorKind::CouldNotReadDirectory(d_dir.clone());

        // Find all our hook scripts and alphabetize them.
        let mut scripts = vec![];
        for entry in fs::read_dir(&d_dir).chain_err(&mkerr)? {
            let entry = entry.chain_err(&mkerr)?;
            let path = entry.path();
            trace!("Checking {} to see if it's a hook", path.display());
            let ty = entry.file_type()
                .chain_err(|| ErrorKind::CouldNotReadFile(path.clone()))?;
            let os_name = entry.file_name();
            let name = os_name.to_str_or_err()?;
            if ty.is_file() && !name.starts_with('.') && name.ends_with(".hook") {
                trace!("Found hook {}", path.display());
                scripts.push(path)
            }
        }
        scripts.sort();

        // Run all our hook scripts.
        for script in scripts {
            let mut cmd = runner.build(&script);
            cmd.current_dir(&self.root_dir);
            for (name, val) in env {
                cmd.env(name, val);
            }
            cmd.exec()?;
        }

        Ok(())
    }
}

#[test]
fn runs_requested_hook_scripts() {
    use env_logger;
    let _ = env_logger::init();
    let proj = Project::from_example("hello").unwrap();
    let runner = TestCommandRunner::new();
    proj.output("pull").unwrap();

    proj.hooks().invoke(&runner, "pull", &BTreeMap::default()).unwrap();
    assert_ran!(runner, {
        [proj.root_dir()
             .join("config")
             .join("hooks")
             .join("pull.d")
             .join("hello.hook")]
    });

    proj.remove_test_output().unwrap();
}
