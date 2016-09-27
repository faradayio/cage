//! Overrides modify a pod for use in a specific environment.
//!
//! This module has a plural name to avoid clashing with a keyword.

use project::Project;

/// An `Override` is a collection of extensions to a project's basic pods.
/// Overrides are typically used to represent deployment environments: test,
/// development and production.
///
/// (Right now, this is deliberately a very thin wrapper around the `name`
/// field, suitable for use as key in a `BTreeMap`.  If you add more
/// fields, you'll probably need to remove `PartialEq`, `Eq`, `PartialOrd`,
/// `Ord` from the `derive` list, and either implement them manually or
/// redesign the code that uses overrides as hash table keys.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Override {
    /// The name of this environment.
    name: String,
}

impl Override {
    /// Create a new override with the specified name.
    pub fn new<S>(name: S) -> Override
        where S: Into<String>
    {
        Override { name: name.into() }
    }

    /// Get the name of this override.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check to see if this override should be included in some operation,
    /// given an optional `only_in_overrides` overrides list.  If no list
    /// is supplied, we're always included.
    ///
    /// We have a weird calling convention because our typical callers are
    /// invoking us using a member field of a `Config` struct that they
    /// own.
    ///
    /// ```
    /// let ovr = conductor::Override::new("development");
    /// assert!(ovr.included_by(&None));
    /// assert!(ovr.included_by(&Some(vec!["development".to_owned()])));
    /// assert!(!ovr.included_by(&Some(vec!["production".to_owned()])));
    /// ```
    pub fn included_by(&self, only_in_overrides: &Option<Vec<String>>) -> bool {
        if let Some(ref only_in) = *only_in_overrides {
            // If a list is supplied, we need to appear in it.
            only_in.contains(&self.name().to_owned())
        } else {
            // If no list is supplied, we're always included.
            true
        }
    }

    /// Get a value for `docker-compose`'s `-p` argument for a given project.
    pub fn compose_project_name(&self, project: &Project) -> String {
        if self.name == "test" {
            format!("{}test", project.name())
        } else {
            project.name().to_owned()
        }
    }
}
