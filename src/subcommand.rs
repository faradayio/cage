//! A cage subcommand

use std::str::FromStr;

/// A cage subcommand
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Subcommand {

    /// The `cage build` subcommand
    Build,

    /// The `cage exec` subcommand
    Exec,

    /// The `cage export` subcommand
    Export,

    /// The `cage generate` subcommand
    Generate,

    /// The `cage logs` subcommand
    Logs,

    /// The `cage new` subcommand
    New,

    /// The `cage pull` subcommand
    Pull,

    /// The `cage restart` subcommand
    Restart,

    /// The `cage rm` subcommand
    Rm,

    /// The `cage run` subcommand
    Run,

    /// The `cage run-script` subcommand
    RunScript,

    /// The `cage shell` subcommand
    Shell,

    /// The `cage source` subcommand
    Source,

    /// The `cage status` subcommand
    Status,

    /// The `cage stop` subcommand
    Stop,

    /// The `cage sysinfo` subcommand
    Sysinfo,

    /// The `cage test` subcommand
    Test,

    /// The `cage up` subcommand
    Up,

    /// The `cage source clone` subcommand
    SourceClone,

    /// The `cage source ls` subcommand
    SourceLs,

    /// The `cage source mount` subcommand
    SourceMount,

    /// The `cage source unmount` subcommand
    SourceUnmount,

    /// The `cage generate completion` subcommand
    GenerateCompletion,

    /// The `cage generate secrets` subcommand
    GenerateSecrets,

    /// The `cage generate vault` subcommand
    GenerateVault,
}

impl FromStr for Subcommand {
    type Err = ();

    fn from_str(s: &str) -> Result<Subcommand, ()> {
        match s {
            "build" => Ok(Subcommand::Build),
            "exec" => Ok(Subcommand::Exec),
            "export" => Ok(Subcommand::Export),
            "generate" => Ok(Subcommand::Generate),
            "logs" => Ok(Subcommand::Logs),
            "new" => Ok(Subcommand::New),
            "pull" => Ok(Subcommand::Pull),
            "restart" => Ok(Subcommand::Restart),
            "rm" => Ok(Subcommand::Rm),
            "run" => Ok(Subcommand::Run),
            "run-script" => Ok(Subcommand::RunScript),
            "shell" => Ok(Subcommand::Shell),
            "source" => Ok(Subcommand::Source),
            "status" => Ok(Subcommand::Status),
            "stop" => Ok(Subcommand::Stop),
            "sysinfo" => Ok(Subcommand::Sysinfo),
            "test" => Ok(Subcommand::Test),
            "up" => Ok(Subcommand::Up),

            "clone" => Ok(Subcommand::SourceClone),
            "ls" => Ok(Subcommand::SourceLs),
            "mount" => Ok(Subcommand::SourceMount),
            "unmount" => Ok(Subcommand::SourceUnmount),

            "completion" => Ok(Subcommand::GenerateCompletion),
            "secrets" => Ok(Subcommand::GenerateSecrets),
            "vault" => Ok(Subcommand::GenerateVault),

            _ => Err(()),
        }
    }
}
