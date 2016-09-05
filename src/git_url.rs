//! Utilities for working with git-format "URLs".
//!
//! TODO MED: We may want to promote this upstream to the `docker_compose`
//! crate at some point.

use regex::Regex;
use std::ffi::{OsStr, OsString};
use std::fmt;
use url::Url;

use util::Error;

/// URL of a Git repository.  Git repositories may be specified as either
/// ordinary `http` or `https` URLs, or as `scp`-style remote directory
/// specifiers.
///
/// One of the goals behind this class is to be able to use it as an
/// "enhanced string", much like `PathBuf`, that can be passed to various
/// APIs using conversion via `AsRef` and `From`.  So we implement plenty
/// of conversions, plus `Ord` so we can be used as a key in a `BTreeMap`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitUrl {
    url: String,
}

impl GitUrl {
    /// Create a `GitUrl` from the specified string.
    pub fn new<S: Into<String>>(url: S) -> Result<GitUrl, Error> {
        let url = url.into();
        lazy_static! {
            static ref URL_VALIDATE: Regex =
                Regex::new(r#"^(?:https?://|git://|github\.com/|git@)"#)
                    .unwrap();
        }
        if URL_VALIDATE.is_match(&url) {
            Ok(GitUrl { url: url })
        } else {
            Err(err!("Not a docker-compatible GitHub URL: {}", &url))
        }
    }

    /// Convert a `GitUrl` to a regular `url::Url` object.
    pub fn to_url(&self) -> Result<Url, Error> {
        match Url::parse(&self.url) {
            Ok(url) => Ok(url),
            Err(_) => {
                lazy_static! {
                    static ref URL_PARSE: Regex =
                        Regex::new(r#"^(?:git@([^:]+):(.*))|(github\.com/.*)"#)
                            .unwrap();
                }
                let caps = try!(URL_PARSE.captures(&self.url).ok_or_else(|| {
                    err!("expected a git URL: {}", &self.url)
                }));
                let new =
                    if caps.at(1).is_some() {
                        format!("git://git@{}/{}", caps.at(1).unwrap(),
                                caps.at(2).unwrap())
                    } else {
                        format!("https://{}", caps.at(3).unwrap())
                    };
                Ok(try!(Url::parse(&new)))
            }
        }
    }
}
  
impl fmt::Display for GitUrl {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.url.fmt(f)
    }
}

impl AsRef<str> for GitUrl {
    fn as_ref(&self) -> &str {
        &self.url
    }
}

/// Convert to an `&OsStr`, which makes it easier to use APIs like
/// `std::process::Command` that take `AsRef<OsStr>` for their arguments.
impl AsRef<OsStr> for GitUrl {
    fn as_ref(&self) -> &OsStr {
        self.url.as_ref()
    }
}

impl From<GitUrl> for String {
    fn from(url: GitUrl) -> String {
        From::from(url.url)
    }
}

impl From<GitUrl> for OsString {
    fn from(url: GitUrl) -> OsString {
        From::from(url.url)
    }
}

#[test]
fn to_url_converts_git_urls_to_real_ones() {
    // Example URLs from http://stackoverflow.com/a/34120821/12089,
    // originally from `docker-compose` source code.
    let regular_urls =
        &["git://github.com/docker/docker",
          "https://github.com/docker/docker.git",
          "http://github.com/docker/docker.git"];
    for &url in regular_urls {
        assert_eq!(GitUrl::new(url).unwrap().to_url().unwrap().to_string(),
                   url);
    }

    // According to http://stackoverflow.com/a/34120821/12089, we also need
    // to special-case `git@` and `github.com/` prefixes.
    let fake_urls =
        &[("git@github.com:docker/docker.git",
           "git://git@github.com/docker/docker.git"),
          ("git@bitbucket.org:atlassianlabs/atlassian-docker.git",
           "git://git@bitbucket.org/atlassianlabs/atlassian-docker.git"),
          ("github.com/docker/docker.git",
           "https://github.com/docker/docker.git")];
    for &(fake_url, real_url) in fake_urls {
        assert_eq!(GitUrl::new(fake_url).unwrap().to_url().unwrap().to_string(),
                   real_url);
    }

    let invalid_urls = &["local/path.git"];
    for &url in invalid_urls {
        assert!(GitUrl::new(url).is_err());
    }
}
