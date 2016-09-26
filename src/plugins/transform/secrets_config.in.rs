// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// The secrets for a single service.
type ServiceSecrets = BTreeMap<String, String>;

/// The secrets for a pod.
type PodSecrets = BTreeMap<String, ServiceSecrets>;

/// The secrets for an override.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverrideSecrets {
    /// Shared between all services in this override.
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
    /// Secrets for each of our overrides.
    #[serde(default)]
    overrides: BTreeMap<String, OverrideSecrets>,
}

#[test]
fn can_deserialize_config() {
    let f = fs::File::open("examples/rails_hello/config/secrets.yml").unwrap();
    let config: Config = serde_yaml::from_reader(f).unwrap();
    assert_eq!(config.common.get("GLOBAL_PASSWORD").unwrap(), "magic");
}
