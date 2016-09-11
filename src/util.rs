//! Miscellaneous utility macros and functions.

use docker_compose::v2 as dc;
use glob;
use retry::retry;
use std::env;
use std::error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// We use the same `Error` type as `docker_compose` for simplicity.
pub type Error = dc::Error;

/// Create an error using a format string and arguments.
#[macro_export]
macro_rules! err {
    ($( $e:expr ),*) => ($crate::Error::from(format!($( $e ),*)));
}

pub trait ToStrOrErr {
    /// Convert to a Rust string as per `OsStr::to_str`, or return an
    /// error;
    fn to_str_or_err(&self) -> Result<&str, Error>;
}

impl ToStrOrErr for OsStr {
    fn to_str_or_err(&self) -> Result<&str, Error> {
        self.to_str().ok_or_else(|| {
            err!("the string {:?} contains non-Unicode characters",
                 self)
        })
    }
}

impl ToStrOrErr for Path {
    fn to_str_or_err(&self) -> Result<&str, Error> {
        self.to_str().ok_or_else(|| {
            err!("the path {} contains non-Unicode characters",
                 self.display())
        })
    }
}

/// Custom methods which we add to `Path` to support common operations.
pub trait ConductorPathExt: ToStrOrErr {
    /// Glob relative to this path.
    fn glob(&self, pattern: &str) -> Result<glob::Paths, Error>;

    /// Ensure the directory containing this path exists.  Returns the path
    /// itself so we can chain function calls, which might be too cute.
    /// (And copy it to a fully-owned `PathBuf` to avoid borrow-checker
    /// issues.)
    fn with_guaranteed_parent(&self) -> Result<PathBuf, Error>;

    /// Convert this path to a relative path, interpreting relative to the
    /// current working directory.
    fn to_absolute(&self) -> Result<PathBuf, Error>;
}

impl ConductorPathExt for Path {
    fn glob(&self, pattern: &str) -> Result<glob::Paths, Error> {
        // We always use the same options.
        let opts = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: true,
        };

        // Construct a full glob and run it.
        let pat = format!("{}/{}", try!(self.to_str_or_err()), pattern);
        Ok(try!(glob::glob_with(&pat, &opts)))
    }

    fn with_guaranteed_parent(&self) -> Result<PathBuf, Error> {
        let parent = try!(self.parent().ok_or_else(|| {
            err!("can't find parent path of {}", self.display())
        }));

        // Take an error message and elaborate a bit.  We use a trait
        // pointer here so we can use this for multiple error types,
        // because Rust closures don't seem to support type parameters.
        let wrap_err = |err: &error::Error| -> Error {
            err!("error creating parent directories for {}: {}",
                 parent.display(), err)
        };

        // On certain file systems, `create_dir_all` is not terribly thread
        // safe, and it may fail if another thread is trying to create the
        // same directories.  This seems to happen inside Docker containers
        // on Travis CI, for example.  For a possibly related issue, see
        // https://github.com/jpetazzo/dind/issues/73.  So we're going to
        // retry this function if it fails, in case it failed at some
        // intermediate step in the directory hierarchy.
        let retry_result = retry(5, 50, || {
            // The function to re-try.
            fs::create_dir_all(&parent)
        }, |result| {
            // Return true if we're done retrying.
            match result {
                &Err(ref err) if err.kind() == io::ErrorKind::AlreadyExists =>
                    false,
                _ => true
            }
        });
        // Unwrap twice: Outer error is a possible retry failure, inner
        // error is a filesystem error.
        try!(try!(retry_result.map_err(|e| wrap_err(&e))).map_err(|e| wrap_err(&e)));
        Ok(self.to_owned())
    }

    fn to_absolute(&self) -> Result<PathBuf, Error> {
        let path = try!(env::current_dir()).join(self);
        assert!(path.is_absolute());
        Ok(path)
    }
}

#[test]
fn path_glob_uses_path_as_base() {
    let base = Path::new("examples/hello/pods/overrides");
    let paths: Vec<_> = base.glob("test/*.env").unwrap()
        .map(|p| p.unwrap().strip_prefix(base).unwrap().to_owned())
        .collect();
    assert_eq!(paths, vec!(Path::new("test/common.env")));
}
