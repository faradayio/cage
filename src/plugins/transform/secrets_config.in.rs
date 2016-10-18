// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// The secrets for a single service.
type ServiceSecrets = BTreeMap<String, String>;

/// The secrets for a pod.
type PodSecrets = BTreeMap<String, ServiceSecrets>;

/// The secrets for an target.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetSecrets {
    /// Shared between all services in this target.
    #[serde(default)]
    common: ServiceSecrets,
    /// Secrets for each of our pods.
    #[serde(default)]
    pods: BTreeMap<String, PodSecrets>,
}

/// The deserialized form of `secrets.yml`.  This is basically
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Shared between all services in this pod.
    #[serde(default)]
    common: ServiceSecrets,
    /// Secrets for each of our pods.
    #[serde(default)]
    pods: BTreeMap<String, PodSecrets>,
    /// Secrets for each of our targets.
    #[serde(default)]
    targets: BTreeMap<String, TargetSecrets>,
}

#[test]
fn can_deserialize_config() {
    let path = Path::new("examples/rails_hello/config/secrets.yml");
    let config: Config = load_yaml(path).unwrap();
    assert_eq!(config.common.get("GLOBAL_PASSWORD").unwrap(), "magic");
}
