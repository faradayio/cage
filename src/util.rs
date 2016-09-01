//! Miscellaneous utility macros and functions.

/// Create an error using a format string and arguments.
macro_rules! err {
    ($( $e:expr ),*) => (dc::Error::from(format!($( $e ),*)));
}
