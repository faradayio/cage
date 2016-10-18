// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// Indicates whether a pod is a regular service or a one-shot task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PodType {
    /// A placeholder represents an externally-managed service, and it is
    /// generally only present in development mode.  This is mostly treated
    /// as though it were a service, but with different defaults in several
    /// places.
    #[serde(rename = "placeholder")]
    Placeholder,

    /// A service is normally started up and left running.
    #[serde(rename = "service")]
    Service,

    /// A task is run once and expected to exit.
    #[serde(rename = "task")]
    Task,
}

/// Configuration information about a pod.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Only use this pod in the specified targets.  If this field is
    /// omitted, we apply the plguin in all targets.
    enable_in_targets: Option<Vec<String>>,

    /// What kind of pod is this?
    pod_type: Option<PodType>,
}
