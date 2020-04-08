//! Targets modify a pod for use in a specific environment, such as
//! `development`, `test` or `production`.

use regex::Regex;

use crate::project::Project;

/// An `Target` provides collection of extensions to a project's basic
/// pods.  Targets are typically used to represent deployment environments:
/// test, development and production.
///
/// (Right now, this is deliberately a very thin wrapper around the `name`
/// field, suitable for use as key in a `BTreeMap`.  If you add more
/// fields, you'll probably need to remove `PartialEq`, `Eq`, `PartialOrd`,
/// `Ord` from the `derive` list, and either implement them manually or
/// redesign the code that uses targets as hash table keys.)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    /// The name of this environment.
    name: String,
}

impl Target {
    /// Create a new target with the specified name.
    pub fn new<S>(name: S) -> Target
    where
        S: Into<String>,
    {
        Target { name: name.into() }
    }

    /// Get the name of this target.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check to see if this target should be included in some operation,
    /// given an optional `enable_in_targets` targets list.  If no list
    /// is supplied, we'll act as those we were passed a default list
    /// including all targets except `test`.
    ///
    /// We have a weird calling convention because our typical callers are
    /// invoking us using a member field of a `Config` struct that they
    /// own.
    ///
    /// ```
    /// let target = cage::Target::new("development");
    /// assert!(target.is_enabled_by(&None));
    /// assert!(target.is_enabled_by(&Some(vec!["development".to_owned()])));
    /// assert!(!target.is_enabled_by(&Some(vec!["production".to_owned()])));
    ///
    /// let test_target = cage::Target::new("test");
    /// assert!(!test_target.is_enabled_by(&None));
    /// ```
    pub fn is_enabled_by(&self, enable_in_targets: &Option<Vec<String>>) -> bool {
        if let Some(ref enable_in) = *enable_in_targets {
            // If a list is supplied, we need to appear in it.
            enable_in.contains(&self.name().to_owned())
        } else {
            // All other targets except `test` are included by default.
            self.name() != "test"
        }
    }

    /// Get a value for `docker-compose`'s `-p` argument for a given project.
    pub fn compose_project_name(&self, project: &Project) -> String {
        let base_name: String = if self.name == "test" {
            format!("{}test", project.name())
        } else {
            project.name().to_owned()
        };

        // We strip out non-alphabetic characters and convert everything to
        // lowercase, which is what the `docker-compose` source code does.
        lazy_static! {
            static ref NON_ALNUM: Regex = Regex::new(r#"[^a-z0-9]"#).unwrap();
        }
        NON_ALNUM
            .replace_all(&base_name.to_lowercase(), "")
            .into_owned()
    }
}
