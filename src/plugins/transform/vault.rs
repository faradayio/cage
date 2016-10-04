//! Plugin which issues vault tokens to services.

use compose_yml::v2 as dc;
use std::result;
use serde_yaml;
#[cfg(test)]
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::env;
use std::fmt::Debug;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
#[cfg(test)]
use std::rc::Rc;
use vault;
use vault::client::VaultDuration;

use errors::*;
use plugins;
use plugins::{Operation, PluginGenerate, PluginNew, PluginTransform};
use project::Project;
use util::err;

#[cfg(feature = "serde_derive")]
include!(concat!("vault_config.in.rs"));
#[cfg(feature = "serde_codegen")]
include!(concat!(env!("OUT_DIR"), "/plugins/transform/vault_config.rs"));

/// Load a vault token from `~/.vault-token`, where the command line client
/// puts it.
fn load_vault_token_from_file() -> Result<String> {
    let path = try!(env::home_dir()
            .ok_or_else(|| err("You do not appear to have a home directory")))
        .join(".vault-token");
    let mut f = try!(fs::File::open(&path)
        .map_err(|e| err!("Error opening {}: {}", path.display(), e)));
    let mut result = String::new();
    try!(f.read_to_string(&mut result)
        .map_err(|e| err!("Error reading {}: {}", path.display(), e)));
    Ok(result.trim().to_owned())
}

/// Find the vault token we'll use to generate new tokens.
fn find_vault_token() -> Result<String> {
    env::var("VAULT_MASTER_TOKEN")
        .or_else(|_| env::var("VAULT_TOKEN"))
        .or_else(|_| load_vault_token_from_file())
        .map_err(|e| {
            err!("{}.  You probably want to log in using the vault client or set \
                  VAULT_MASTER_TOKEN",
                 e)
        })
}

/// The "environment" in which to interpret a configuration file.  We don't
/// want to use the OS environment variables, but rather a fake environment
/// with a few carefully selected values.
#[derive(Debug)]
struct ConfigEnvironment<'a> {
    /// The context for our transformation, including the project, pod,
    /// etc.
    ctx: &'a plugins::Context<'a>,
    /// The name of the current service.
    service: &'a str,
}

impl<'a> dc::Environment for ConfigEnvironment<'a> {
    fn var(&self, key: &str) -> result::Result<String, env::VarError> {
        let result = match key {
            "PROJECT" => Ok(self.ctx.project.name()),
            "OVERRIDE" => Ok(self.ctx.ovr.name()),
            "POD" => Ok(self.ctx.pod.name()),
            "SERVICE" => Ok(self.service),
            _ => Err(env::VarError::NotPresent),
        };
        result.map(|s| s.to_owned())
    }
}

/// An abstract interface to Vault's token-generation capabilities.  We use
/// this to mock vault during tests.
trait GenerateToken: Debug {
    /// Get a `VAULT_ADDR` value to use along with this token.
    fn addr(&self) -> &str;
    /// Generate a token with the specified parameters.
    fn generate_token(&self,
                      display_name: &str,
                      policies: Vec<String>,
                      ttl: VaultDuration)
                      -> Result<String>;
}

/// A list of calls made to a `MockVault` instance.
#[cfg(test)]
type MockVaultCalls = Rc<RefCell<Vec<(String, Vec<String>, VaultDuration)>>>;

/// A fake interface to vault for testing purposes.
#[derive(Debug)]
#[cfg(test)]
struct MockVault {
    /// The tokens we were asked to generate.  We store these in a RefCell
    /// so that we can have "interior" mutability, because we don't want
    /// `generate_token` to be `&mut self` in the general case.
    calls: MockVaultCalls,
}

#[cfg(test)]
impl MockVault {
    /// Create a new MockVault.
    fn new() -> MockVault {
        MockVault { calls: Rc::new(RefCell::new(vec![])) }
    }

    /// Return a reference to record of calls made to our vault.
    fn calls(&self) -> MockVaultCalls {
        self.calls.clone()
    }
}

#[cfg(test)]
impl GenerateToken for MockVault {
    fn addr(&self) -> &str {
        "http://example.com:8200/"
    }

    fn generate_token(&self,
                      display_name: &str,
                      policies: Vec<String>,
                      ttl: VaultDuration)
                      -> Result<String> {
        self.calls.borrow_mut().push((display_name.to_owned(), policies, ttl));
        Ok("fake_token".to_owned())
    }
}

/// An interface to an actual vault server.
#[derive(Debug)]
struct Vault {
    /// The address of our vault server.
    addr: String,
    /// The master token that we'll use to issue new tokens.
    token: String,
}

impl Vault {
    /// Create a new vault client.
    fn new() -> Result<Vault> {
        let addr = try!(env::var("VAULT_ADDR").map_err(|_| {
            err("Please set the environment variable VAULT_ADDR to the URL of \
                 your vault server")
        }));
        let token = try!(find_vault_token());
        Ok(Vault {
            addr: addr,
            token: token,
        })
    }
}

impl GenerateToken for Vault {
    fn addr(&self) -> &str {
        &self.addr
    }

    fn generate_token(&self,
                      display_name: &str,
                      policies: Vec<String>,
                      ttl: VaultDuration)
                      -> Result<String> {
        // We can't store `client` in `self`, because it has some obnoxious
        // lifetime parameters.  So we'll just recreate it.  This is
        // probably not the worst idea, because it uses `hyper` for HTTP,
        // and `hyper` HTTP connections used to have expiration issues that
        // were tricky for clients to deal with correctly.
        let client = try!(vault::Client::new(&self.addr, &self.token));
        let opts = vault::client::TokenOptions::default()
            .display_name(display_name)
            .renewable(false)
            .ttl(ttl)
            .policies(policies);
        let auth = try!(client.create_token(&opts));
        Ok(auth.client_token)
    }
}

/// Issues `VAULT_TOKEN` values to services.
#[derive(Debug)]
pub struct Plugin {
    /// Our `config/vault.yml` YAML file, parsed and read into memory.
    /// Optional because if we're being run as a `PluginGenerate`, we won't
    /// have it (but it's guaranteed otherwise).
    config: Option<Config>,
    /// Our source of tokens.
    generator: Option<Box<GenerateToken>>,
}

impl Plugin {
    /// Get the path to this plugin's config file.
    fn config_path(project: &Project) -> PathBuf {
        project.root_dir().join("config/vault.yml")
    }

    /// Create a new plugin, specifying an alternate source for tokens.
    fn new_with_generator<G>(project: &Project, generator: Option<G>) -> Result<Plugin>
        where G: GenerateToken + 'static
    {
        let path = Self::config_path(project);
        let config = if path.exists() {
            let f = try!(fs::File::open(&path));
            Some(try!(serde_yaml::from_reader(f)
                .map_err(|e| err!("Error reading {}: {}", path.display(), e))))
        } else {
            None
        };
        Ok(Plugin {
            config: config,
            generator: generator.map(|gen: G| -> Box<GenerateToken> { Box::new(gen) }),
        })
    }
}

impl plugins::Plugin for Plugin {
    fn name(&self) -> &'static str {
        Self::plugin_name()
    }
}

impl PluginNew for Plugin {
    fn plugin_name() -> &'static str {
        "vault"
    }

    fn is_configured_for(project: &Project) -> Result<bool> {
        let path = Self::config_path(project);
        Ok(path.exists())
    }

    fn new(project: &Project) -> Result<Self> {
        // An annoying special case.  We may be called as a code generator,
        // in which case we don't want to try to create a `GenerateToken`
        // instance.
        let token_gen = if try!(Self::is_configured_for(project)) {
            Some(try!(Vault::new()))
        } else {
            None
        };
        Self::new_with_generator(project, token_gen)
    }
}

impl PluginGenerate for Plugin {
    fn generator_description(&self) -> &'static str {
        "Get passwords & other secrets from a Vault server"
    }
}

impl PluginTransform for Plugin {
    fn transform(&self,
                 _op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<()> {

        // Get our plugin config.
        let config = self.config
            .as_ref()
            .expect("config should always be present for transform");
        let generator = self.generator
            .as_ref()
            .expect("generator should always be present for transform");

        // Should this plugin be excluded in this override?
        if !ctx.ovr.is_enabled_by(&config.enable_in_overrides) {
            return Ok(());
        }

        // Apply to each service.
        for (name, mut service) in &mut file.services {
            // Set up a ConfigEnvironment that we can use to perform
            // interpolations of values like `$SERVICE` in our config file.
            let env = ConfigEnvironment {
                ctx: ctx,
                service: name,
            };

            // Define a local helper function to interpolate
            // `RawOr<String>` values using `env`.
            let interpolated = |raw_val: &dc::RawOr<String>| -> Result<String> {
                let mut val = raw_val.to_owned();
                Ok(try!(val.interpolate_env(&env)).to_owned())
            };

            // Insert our VAULT_ADDR value into the generated files.
            service.environment
                .insert("VAULT_ADDR".to_owned(), generator.addr().to_owned());

            // Get a list of policy "patterns" that apply to this service.
            let mut raw_policies = config.default_policies.clone();
            raw_policies.extend(config.pods
                .get(ctx.pod.name())
                .and_then(|pod| pod.get(name))
                .map_or_else(|| vec![], |s| s.policies.clone()));

            // Interpolate the variables found in our policy patterns.
            let mut policies = vec![];
            for result in raw_policies.iter().map(|p| interpolated(p)) {
                // We'd like to use std::result::fold here but it's unstable.
                policies.push(try!(result));
            }

            // Generate a VAULT_TOKEN.
            let display_name = format!("{}_{}_{}_{}",
                                       ctx.project.name(),
                                       ctx.ovr.name(),
                                       ctx.pod.name(),
                                       name);
            let ttl = VaultDuration::seconds(config.default_ttl);
            let token = try!(generator.generate_token(&display_name, policies, ttl));
            service.environment.insert("VAULT_TOKEN".to_owned(), token);

            // Add in any extra environment variables.
            for (var, val) in &config.extra_environment {
                service.environment.insert(var.to_owned(), try!(interpolated(val)));
            }
        }
        Ok(())
    }
}

#[test]
fn interpolates_policies() {
    use env_logger;
    let _ = env_logger::init();

    env::set_var("VAULT_ADDR", "http://example.com:8200/");
    env::set_var("VAULT_MASTER_TOKEN", "fake master token");

    let proj = Project::from_example("vault_integration").unwrap();
    let ovr = proj.ovr("production").unwrap();

    let vault = MockVault::new();
    let calls = vault.calls();
    let plugin = Plugin::new_with_generator(&proj, Some(vault)).unwrap();

    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, ovr, frontend);
    let mut file = frontend.merged_file(ovr).unwrap();
    plugin.transform(Operation::Output, &ctx, &mut file).unwrap();
    let web = file.services.get("web").unwrap();
    assert_eq!(web.environment.get("VAULT_ADDR").expect("has VAULT_ADDR"),
               "http://example.com:8200/");
    assert_eq!(web.environment.get("VAULT_TOKEN").expect("has VAULT_TOKEN"),
               "fake_token");
    assert_eq!(web.environment.get("VAULT_ENV").expect("has VAULT_ENV"),
               "production");

    let calls = calls.borrow();
    assert_eq!(calls.len(), 1);
    let (ref display_name, ref policies, ref ttl) = calls[0];
    assert_eq!(display_name, "vault_integration_production_frontend_web");
    assert_eq!(policies,
               &["vault_integration-production".to_owned(),
                 "vault_integration-production-frontend-web".to_owned(),
                 "vault_integration-production-ssl".to_owned()]);
    assert_eq!(ttl, &VaultDuration::seconds(2592000));
}

#[test]
fn only_applied_in_specified_overrides() {
    use env_logger;
    let _ = env_logger::init();

    let proj = Project::from_example("vault_integration").unwrap();
    let ovr = proj.ovr("test").unwrap();

    let vault = MockVault::new();
    let plugin = Plugin::new_with_generator(&proj, Some(vault)).unwrap();

    let frontend = proj.pod("frontend").unwrap();
    let ctx = plugins::Context::new(&proj, ovr, frontend);
    let mut file = frontend.merged_file(ovr).unwrap();
    plugin.transform(Operation::Output, &ctx, &mut file).unwrap();
    let web = file.services.get("web").unwrap();
    assert_eq!(web.environment.get("VAULT_ADDR"), None);
}
