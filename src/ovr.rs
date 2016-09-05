//! Overrides modify a pod for use in a specific environment.
//!
//! This module has a plural name to avoid clashing with a keyword.

/// An `Override` is a collection of extensions to a project's basic pods.
/// Overrides are typically used to represent deployment environments: test,
/// development and production.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Override {
    /// The name of this environment.
    name: String,

    // Right now, this is deliberately a very thin wrapper around the
    // `name` field, suitable for use as key in a BTreeMap.  If you add
    // more fields, you'll probably need to remove `PartialEq`, `Eq`,
    // `PartialOrd`, `Ord` from the `derive` list, and either implement
    // them manually or redesign the code that uses overlays as hash table
    // keys.
}

impl Override {
    /// Create a new override with the specified name.
    pub fn new<S>(name: S) -> Override where S: Into<String> {
        Override {
            name: name.into(),
        }
    }

    /// Get the name of this override.
    pub fn name(&self) -> &str {
        &self.name
    }
}
