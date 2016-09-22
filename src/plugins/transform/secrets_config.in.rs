// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// The secrets for a single container.
type ContainerSecrets = BTreeMap<String, String>;

/// The secrets for a pod.
type PodSecrets = BTreeMap<String, ContainerSecrets>;

/// The secrets for an override.
#[derive(Debug, Serialize, Deserialize)]
struct OverrideSecrets {
    /// Shared between all containers in this override.
    common: ContainerSecrets,
    /// Secrets for each of our pods.
    pods: BTreeMap<String, PodSecrets>,
}

/// The deserialized form of `secrets.yml`.  This is basically
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// Shared between all containers in this pod.
    common: ContainerSecrets,
    /// Secrets for each of our pods.
    pods: BTreeMap<String, PodSecrets>,
    /// Secrets for each of our overrides.
    overrides: BTreeMap<String, OverrideSecrets>,
}

#[test]
fn can_deserialize_config() {
    let f = fs::File::open("examples/rails_hello/config/secrets.yml").unwrap();
    let config: Config = serde_yaml::from_reader(f).unwrap();
    assert_eq!(config.common.get("GLOBAL_PASSWORD").unwrap(), "magic");
}
