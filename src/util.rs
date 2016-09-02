//! Miscellaneous utility macros and functions.

use docker_compose::v2 as dc;

/// We use the same `Error` type as `docker_compose` for simplicity.
pub type Error = dc::Error;

/// Create an error using a format string and arguments.
macro_rules! err {
    ($( $e:expr ),*) => ($crate::Error::from(format!($( $e ),*)));
}
