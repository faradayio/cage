//! Options which can be passed to `docker-compose exec`.

use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;

/// Command-line flags which can be passed to `docker-compose exec`.
#[derive(Debug, Clone)]
pub struct ExecOptions {
    /// Should we execute this command in the background?
    pub detached: bool,

    /// Should we run this command with elevated privileges?
    pub privileged: bool,

    /// An optional user as whom we should run the command.
    ///
    /// TODO LOW: Is this technically "user[:group]"?  If so, we need
    /// support for that type in docker_compose-rs and use it here.
    pub user: Option<String>,

    /// Should we allocate a TTY when executing the command?
    /// Defaults to true for `docker-compose`.
    pub allocate_tty: bool,

    /// PRIVATE: This field is a stand-in for future options.
    /// See http://stackoverflow.com/q/39277157/12089
    #[doc(hidden)]
    pub _nonexhaustive: PhantomData<()>,
}

impl ExecOptions {
    /// Convert to arguments suitable for `std::process::Command` or our
    /// `CommandBuilder`.
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec!();
        if self.detached {
            args.push(OsStr::new("-d").to_owned());
        }
        if self.privileged {
            args.push(OsStr::new("--privileged").to_owned());
        }
        if let Some(ref user) = self.user {
            args.push(OsStr::new("--user").to_owned());
            args.push(user.into());
        }
        if !self.allocate_tty {
            args.push(OsStr::new("-T").to_owned());
        }
        args
    }
}

#[test]
fn to_args_returns_empty_for_default_opts() {
    assert_eq!(ExecOptions::default().to_args(), Vec::<OsString>::new());
}

#[test]
fn to_args_returns_appropriate_flags() {
    let opts = ExecOptions {
        detached: true,
        privileged: true,
        user: Some("root".to_owned()),
        allocate_tty: false,
        ..Default::default()
    };
    let raw_expected = &["-d", "--privileged", "--user", "root", "-T"];
    let expected: Vec<OsString> = raw_expected.iter()
        .map(|s| OsStr::new(s).to_owned())
        .collect();
    assert_eq!(opts.to_args(), expected);
}

impl Default for ExecOptions {
    fn default() -> ExecOptions {
        ExecOptions {
            detached: false,
            privileged: false,
            user: None,
            allocate_tty: true, // Not false!
            _nonexhaustive: PhantomData,
        }
    }
}
