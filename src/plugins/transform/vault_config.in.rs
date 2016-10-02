// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

/// How should our applications authenticate themselves with vault?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum AuthType {
    /// Issue time-limited VAULT_TOKEN values to each service, setting
    /// appropriate policies on each token.
    #[serde(rename = "token")]
    Token,
}

/// The policies associated with a specific pod.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceConfig {
    /// Policies to apply to this service.
    #[serde(default)]
    policies: Vec<dc::RawOr<String>>,
}

/// Policies to apply to each service in a pod.
type PodConfig = BTreeMap<String, ServiceConfig>;

/// The configuration for our Vault plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Only apply this plugin in the specified overrides.  If this
    /// field is omitted, we apply the plguin in all overrides.
    enable_in_overrides: Option<Vec<String>>,

    /// The kind of authentication to use.
    auth_type: AuthType,

    /// Extra environment variables to inject into each service.
    #[serde(default)]
    extra_environment: BTreeMap<String, dc::RawOr<String>>,

    /// How long should tokens be valid for?
    default_ttl: u64,

    /// Default policies to apply to every service.
    #[serde(default)]
    default_policies: Vec<dc::RawOr<String>>,

    /// More specific policies to apply to individual
    #[serde(default)]
    pods: BTreeMap<String, PodConfig>,
}

#[test]
fn can_deserialize_config() {
    let f = fs::File::open("examples/vault_integration/config/vault.yml").unwrap();
    let config: Config = serde_yaml::from_reader(f).unwrap();
    assert_eq!(config.auth_type, AuthType::Token);
}
