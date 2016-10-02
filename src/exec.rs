//! Options which can be passed to `docker-compose exec`.

use compose_yml::v2 as dc;
use std::ffi::{OsStr, OsString};
use std::marker::PhantomData;

use ovr::Override;
use pod::Pod;
use project::Project;
use util::Error;

/// Trait for types which can be converted to command-line arguments.
pub trait ToArgs {
    /// Convert this type to command-line arguments.
    fn to_args(&self) -> Vec<OsString>;
}

/// Command-line flags which can be passed to `docker-compose exec`.
#[derive(Debug, Clone)]
pub struct Options {
    /// Should we execute this command in the background?
    pub detached: bool,

    /// Should we run this command with elevated privileges?
    pub privileged: bool,

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

impl ToArgs for Options {
    /// Convert to arguments suitable for `std::process::Command` or our
    /// `CommandBuilder`.
    fn to_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = vec![];
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
fn options_to_args_returns_empty_for_default_opts() {
    assert_eq!(Options::default().to_args(), Vec::<OsString>::new());
}

#[test]
fn options_to_args_returns_appropriate_flags() {
    let opts = Options {
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

impl Default for Options {
    fn default() -> Options {
        Options {
            detached: false,
            privileged: false,
            user: None,
            allocate_tty: true, // Not false!
            _nonexhaustive: PhantomData,
        }
    }
}

/// The pod and service within which to execute a command.  The lifetime
/// `'a` needs to be longer than the useful lifetime of this `Target`.
#[derive(Debug)]
pub struct Target<'a> {
    /// The override we're using to run this command.
    ovr: &'a Override,
    /// The name of the pod in which to run the command.
    pod: &'a Pod,
    /// The name of the service in which to run the command.
    service_name: &'a str,
    /// The `Service` object for the service where we'll run the command.
    service: dc::Service,
}

impl<'a> Target<'a> {
    /// Create a new `Target`, looking up the underlying pod and service
    /// objects.
    pub fn new(project: &'a Project,
               ovr: &'a Override,
               pod_name: &'a str,
               service_name: &'a str)
               -> Result<Target<'a>, Error> {
        let pod = try!(project.pod(pod_name)
            .ok_or_else(|| err!("Cannot find pod {}", pod_name)));
        let file = try!(pod.merged_file(ovr));
        let service = try!(file.services
            .get(service_name)
            .ok_or_else(|| err!("Cannot find service {}", service_name)));
        Ok(Target {
            ovr: ovr,
            pod: pod,
            service_name: service_name,
            service: service.to_owned(),
        })
    }

    /// The active override for the command we want to run.
    pub fn ovr(&self) -> &Override {
        self.ovr
    }

    /// The pod for this target.
    pub fn pod(&self) -> &Pod {
        self.pod
    }

    /// The service name for this target.
    pub fn service_name(&self) -> &str {
        self.service_name
    }

    /// The `Service` object for this target.
    pub fn service(&self) -> &dc::Service {
        &self.service
    }
}

/// A command which can be executed.
#[derive(Debug)]
pub struct Command {
    /// The command to execute.
    pub command: OsString,
    /// The arguments to pass to the command.
    pub args: Vec<OsString>,
}

impl Command {
    /// Create a new `Command` object.
    pub fn new<S: AsRef<OsStr>>(command: S) -> Command {
        Command {
            command: command.as_ref().to_owned(),
            args: vec![],
        }
    }

    /// Add arguments to a `Command` object.  This is meant to be chained
    /// immediately after `new`, and it consumes `self` and returns it.
    pub fn with_args<S: AsRef<OsStr>>(mut self, args: &[S]) -> Command {
        self.args = args.iter().map(|a| a.as_ref().to_owned()).collect();
        self
    }
}

impl ToArgs for Command {
    fn to_args(&self) -> Vec<OsString> {
        let mut result: Vec<OsString> = vec![];
        result.push(self.command.clone());
        if !self.args.is_empty() {
            result.push(OsStr::new("--").to_owned());
            for arg in &self.args {
                result.push(arg.clone());
            }
        }
        result
    }
}

#[test]
fn command_to_args_converts_to_arguments() {
    assert_eq!(Command::new("foo").to_args(), vec![OsStr::new("foo")]);
    assert_eq!(Command::new("foo").with_args(&["--opt"]).to_args(),
               vec![OsStr::new("foo"), OsStr::new("--"), OsStr::new("--opt")]);
}
