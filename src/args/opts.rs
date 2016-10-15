//! Command-line options which can be passed to `docker-compose`.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use args::ToArgs;

/// An empty set of options, used for `docker-compose` subcommands for
/// which don't need any.
#[derive(Debug, Clone, Copy)]
pub struct Empty;

impl ToArgs for Empty {
    fn to_args(&self) -> Vec<OsString> {
        vec![]
    }
}

/// Command-line flags which can be passed to `docker-compose exec`.
#[derive(Debug, Clone)]
pub struct Process {
    /// Should we execute this command in the background?
    pub detached: bool,

    /// An optional user as whom we should run the command.
    ///
    /// TODO LOW: Is this technically "user[:group]"?  If so, we need
    /// support for that type in `compose_yml` and use it here.
    pub user: Option<String>,

    /// Should we allocate a TTY when executing the command?
    /// Defaults to true for `docker-compose`.
    pub allocate_tty: bool,

    /// PRIVATE: This field is a stand-in for future options.
    /// See http://stackoverflow.com/q/39277157/12089
    #[doc(hidden)]
    pub _nonexhaustive: PhantomData<()>,
}

impl ToArgs for Process {
    fn to_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec![];
        if self.detached {
            args.push(OsStr::new("-d").to_owned());
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
fn process_options_to_args_returns_empty_for_default_opts() {
    assert_eq!(Process::default().to_args(), Vec::<OsString>::new());
}

#[test]
fn process_options_to_args_returns_appropriate_flags() {
    let opts = Process {
        detached: true,
        user: Some("root".to_owned()),
        allocate_tty: false,
        ..Default::default()
    };
    let raw_expected = &["-d", "--user", "root", "-T"];
    let expected: Vec<OsString> = raw_expected.iter()
        .map(|s| OsStr::new(s).to_owned())
        .collect();
    assert_eq!(opts.to_args(), expected);
}

impl Default for Process {
    fn default() -> Process {
        Process {
            detached: false,
            user: None,
            allocate_tty: true, // Not false!
            _nonexhaustive: PhantomData,
        }
    }
}

/// Options for `docker_compose exec`.
#[derive(Debug, Clone, Default)]
pub struct Exec {
    /// Our "superclass", faked using `Deref`.
    pub process: Process,

    /// Should we run this command with elevated privileges?
    pub privileged: bool,

    /// PRIVATE: This field is a stand-in for future options.
    /// See http://stackoverflow.com/q/39277157/12089
    #[doc(hidden)]
    pub _nonexhaustive: PhantomData<()>,
}

impl Deref for Exec {
    type Target = Process;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

impl DerefMut for Exec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}

impl ToArgs for Exec {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = self.process.to_args();
        if self.privileged {
            args.push(OsStr::new("--privileged").to_owned());
        }
        args
    }
}

#[test]
fn exec_options_to_args_returns_appropriate_flags() {
    let mut opts = Exec::default();
    opts.detached = true;
    opts.privileged = true;
    let raw_expected = &["-d", "--privileged"];
    let expected: Vec<OsString> = raw_expected.iter()
        .map(|s| OsStr::new(s).to_owned())
        .collect();
    assert_eq!(opts.to_args(), expected);
}

/// Options for `docker_compose exec`.
#[derive(Debug, Clone, Default)]
pub struct Run {
    /// Our "superclass", faked using `Deref`.
    pub process: Process,

    /// Extra environment variables to pass in.
    pub environment: BTreeMap<String, String>,

    /// Override the container's entrypoint.  Specify `Some("".to_owned())`
    /// to reset the entrypoint to the default.
    pub entrypoint: Option<String>,

    /// PRIVATE: This field is a stand-in for future options.
    /// See http://stackoverflow.com/q/39277157/12089
    #[doc(hidden)]
    pub _nonexhaustive: PhantomData<()>,
}

impl Deref for Run {
    type Target = Process;

    fn deref(&self) -> &Self::Target {
        &self.process
    }
}

impl DerefMut for Run {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.process
    }
}

impl ToArgs for Run {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = self.process.to_args();
        for (var, val) in &self.environment {
            args.push(OsStr::new("-e").to_owned());
            args.push(format!("{}={}", var, val).into());
        }
        if let Some(ref entrypoint) = self.entrypoint {
            args.push(OsStr::new("--entrypoint").to_owned());
            args.push(entrypoint.into());
        }
        args
    }
}

#[test]
fn run_options_to_args_returns_appropriate_flags() {
    let mut opts = Run::default();
    opts.detached = true;
    opts.environment.insert("FOO".to_owned(), "foo".to_owned());
    opts.entrypoint = Some("/helper.sh".to_owned());
    let raw_expected = &["-d", "-e", "FOO=foo", "--entrypoint", "/helper.sh"];
    let expected: Vec<OsString> = raw_expected.iter()
        .map(|s| OsStr::new(s).to_owned())
        .collect();
    assert_eq!(opts.to_args(), expected);
}
