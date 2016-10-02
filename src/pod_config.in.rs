// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// Indicates whether a pod is a regular service or a one-shot task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodType {
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
    /// Only use this pod in the specified overrides.  If this field is
    /// omitted, we apply the plguin in all overrides.
    enable_in_overrides: Option<Vec<String>>,

    /// What kind of pod is this?
    pod_type: Option<PodType>,
}
