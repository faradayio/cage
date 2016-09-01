//! Extension methods for `docker_compose::v2::Service`.

use docker_compose::v2 as dc;
use regex::Regex;
use std::path::{Path, PathBuf};

/// These methods will appear as regular methods on `Context` in any module
/// which includes `ContextExt`.
pub trait ContextExt {
    /// Return a local directory that we can use for building this service.
    /// This may be either an existing directory, or a directory where we
    /// can put a Git checkout.
    fn local(&self) -> Result<PathBuf, dc::Error>;
}

impl ContextExt for dc::Context {
    fn local(&self) -> Result<PathBuf, dc::Error> {
        match self {
            // Simulate a local checkout of the remote Git repository
            // mentioned in `build`.
            &dc::Context::GitUrl(ref url) => {
                let re = Regex::new(r#"/([^./]+)(?:\.git)?$"#).unwrap();
                match re.captures(url) {
                    None =>
                        Err(err!("Can't get dir name from Git URL: {}", url)),
                    Some(caps) => {
                        let path = Path::new("src")
                            .join(caps.at(1).unwrap())
                            .to_owned();
                        Ok(path)
                    }
                }
            }
            // Interpret `dir` relative to `pods` directory where we keep
            // our main `docker-compose.yml` files.
            &dc::Context::Dir(ref dir) =>
                Ok(Path::new("pods").join(dir).to_owned()),
        }
    }
}

#[test]
fn git_to_local_fixes_local_directory_paths_as_needed() {
    let ctx = dc::Context::new("/src/foo");
    assert_eq!(ctx.local().unwrap(),
               Path::new("/src/foo").to_owned());

    let ctx = dc::Context::new("../src/foo");
    assert_eq!(ctx.local().unwrap(),
               Path::new("pods/../src/foo").to_owned());
}

#[test]
fn git_to_local_extracts_directory_part_of_git_urls() {
    let examples = &[
        // Example URLs from http://stackoverflow.com/a/34120821/12089,
        // originally from `docker-compose` source code.
        ("git://github.com/docker/docker", Some("docker")),
        ("git@github.com:docker/docker.git", Some("docker")),
        ("git@bitbucket.org:atlassianlabs/atlassian-docker.git",
         Some("atlassian-docker")),
        ("https://github.com/docker/docker.git", Some("docker")),
        ("http://github.com/docker/docker.git", Some("docker")),
        ("github.com/docker/docker.git", Some("docker")),
        // A URL from which we can't extract a local directory.
        ("http://www.example.com/", None),
    ];

    for &(url, dir) in examples {
        let in_ctx = dc::Context::new(url);
        if let Some(dir) = dir {
            let out_dir = Path::new("src").join(dir).to_owned();
            assert_eq!(in_ctx.local().unwrap(), out_dir);
        } else {
            assert!(in_ctx.local().is_err());
        }
    }
}

