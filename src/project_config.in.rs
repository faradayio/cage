// This is not a standalone Rust module.  It gets processed by serde to
// generate serialization code and included directly into another module.

lazy_static! {
    /// The path to our project configuration file, relative to our project
    /// root.
    pub static ref PROJECT_CONFIG_PATH: PathBuf =
        Path::new("config/project.yml").to_owned();
}

/// Configuration information about a project, read in from
/// `PROJECT_CONFIG_PATH`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// A semantic version requirement specifying compatible versions of
    /// this tool.
    #[serde(default, deserialize_with = "deserialize_parsable_opt")]
    pub cage_version: Option<semver::VersionReq>,

    /// Ensure that this struct has at least one private field so we
    /// can extend it in the future.
    #[serde(default, skip_deserializing)]
    _phantom: PhantomData<()>,
}

impl ProjectConfig {
    /// Load a config file from the specified path.
    pub fn new(path: &Path) -> Result<Self> {
        if path.exists() {
            let mkerr = || ErrorKind::CouldNotReadFile(path.to_owned());
            let mut f = try!(fs::File::open(path).chain_err(&mkerr));
            let mut yaml = String::new();
            try!(f.read_to_string(&mut yaml).chain_err(&mkerr));
            try!(Self::check_config_version(&path, &yaml));
            serde_yaml::from_str(&yaml).chain_err(&mkerr)
        } else {
            warn!("No {} file, using default values", path.display());
            Ok(Default::default())
        }
    }

    /// Check a config file to see if it's a version we support.  We only
    /// use the `path` argument to report errors.
    fn check_config_version(path: &Path, config_yml: &str) -> Result<()> {
        /// A stripped-down version of `ProjectConfig` without
        /// `#[serde(deny_unknown_fields)]`, so that we should be able to parse
        /// any possible version of the config file.
        #[derive(Debug, Deserialize)]
        struct VersionOnly {
            /// Our version requirement.
            #[serde(default, deserialize_with = "deserialize_parsable_opt")]
            cage_version: Option<semver::VersionReq>,
        }

        let config: VersionOnly = try!(serde_yaml::from_str(config_yml)
            .chain_err(|| ErrorKind::CouldNotReadFile(path.to_owned())));
        if let Some(ref req) = config.cage_version {
            if !req.matches(&version()) {
                return Err(ErrorKind::MismatchedVersion(req.to_owned()).into());
            }
        } else {
            warn!("No cage_version specified in {}, trying anyway",
                  path.display());
        }
        Ok(())
    }
}

#[test]
fn semver_behaves_as_expected() {
    // We expect this to be interpreted as "^0.2.3", with the special
    // semantics for versions less than 1.0, where the minor version is
    // used to indicate a breaking change.
    let req = semver::VersionReq::parse("0.2.3").unwrap();
    let examples = &[
        ("0.2.2", false),
        ("0.2.3", true),
        ("0.2.4", true),
        ("0.3.0", false),
    ];

    for &(version, expected_to_match) in examples {
        assert_eq!(req.matches(&semver::Version::parse(version).unwrap()),
                   expected_to_match);
    }
}

#[test]
fn check_config_version() {
    let p = Path::new("dummy.yml");

    // Check to make sure we're compatible with ourself.
    let yaml = format!("cage_version: \"{}\"", version());
    assert!(ProjectConfig::check_config_version(&p, &yaml).is_ok());

    // Check to make sure we can load a file with no version.
    let yaml = "---\n{}";
    ProjectConfig::check_config_version(&p, yaml).unwrap();
    assert!(ProjectConfig::check_config_version(&p, yaml).is_ok());

    // Check to make sure we fail with the correct error if we can't read
    // this version of the file format.
    let yaml = "---\ncage_version: \"0.0.1\"\nunknown_field: true";
    let res = ProjectConfig::check_config_version(&p, yaml);
    assert!(res.is_err());
    match *res.unwrap_err().kind() {
        ErrorKind::MismatchedVersion(_) => {},
        ref e => panic!("Unexpected error type {}", e),
    }
}
