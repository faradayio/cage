// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// Indicates whether a pod is a regular service or a one-shot task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PodType {
    /// A placeholder represents an externally-managed service, and it is
    /// generally only present in development mode.  This is mostly treated
    /// as though it were a service, but with different defaults in several
    /// places.
    #[serde(rename = "placeholder")]
    Placeholder,

    /// A service is normally started up and left running.
    #[serde(rename = "service")]
    Service,

    /// A task is run once and expected to exit.
    #[serde(rename = "task")]
    Task,
}

/// In addition to serde serialization, also provide basic formatting.
impl fmt::Display for PodType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PodType::Placeholder => write!(f, "placeholder"),
            PodType::Service => write!(f, "service"),
            PodType::Task => write!(f, "task"),
        }
    }
}

/// Configuration information about a pod.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Only use this pod in the specified targets.  If this field is
    /// omitted, we apply the plguin in all targets.
    enable_in_targets: Option<Vec<String>>,

    /// What kind of pod is this?
    pod_type: Option<PodType>,

    /// A list of commands to invoke with `cage run` when this pod is
    /// initialized.
    #[serde(default, skip_serializing_if="Vec::is_empty")]
    run_on_init: Vec<Vec<String>>,

    /// List of per-service configurations for the pod.
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    services: BTreeMap<String, ServiceConfig>,
}

impl Config {

    /// Run a named script for the specified service in this pod
    pub fn run_script<CR>(&self, runner: &CR, project: &Project, service_name: &str, script_name: &str) -> Result<()> 
        where CR: CommandRunner
    {
        match self.services.get(service_name) {
            Some(service_config) => {
                service_config.run_script(runner, &project, &service_name, &script_name)?
            },
            None => {}
        }
        Ok(())
    }
    
}

/// Individual, per-service configurations.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceConfig {
    /// List of scripts that can be executed via `cage run-script <name>`.
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    scripts: BTreeMap<String, Script>,
}

impl ServiceConfig {

    /// Run a named script for the given service
    pub fn run_script<CR>(&self, runner: &CR, project: &Project, service_name: &str, script_name: &str) -> Result<()>
        where CR: CommandRunner
    {
        if let Some(script) = self.scripts.get(script_name) {
            script.run(runner, &project, service_name)?;
        }
        Ok(())
    }

}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Script(Vec<Vec<String>>);

impl Script {

    /// Execute each command defined for the named script
    pub fn run<CR>(&self, runner: &CR, project: &Project, service_name: &str) -> Result<()> 
        where CR: CommandRunner
    {
        for cmd in &self.0 {
            if cmd.len() < 1 {
                return Err("all items in script must have at least one value"
                    .into());
            }
            let cmd = if cmd.len() >= 2 {
                Some(args::Command::new(&cmd[0]).with_args(&cmd[1..]))
            } else {
                None
            };
            let opts = args::opts::Run::default();
            project.run(runner, service_name, cmd.as_ref(), &opts)?;
        }
        Ok(())
    }

}
