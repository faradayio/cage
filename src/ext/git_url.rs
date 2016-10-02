//! Extension methods for `compose_yml::v2::GitUrl`.

use compose_yml::v2 as dc;
use std::ffi::OsString;
use std::path::Path;
use url;

use util::Error;

/// These methods will appear as regular methods on `Context` in any module
/// which includes `ContextExt`.
pub trait GitUrlExt {
    /// Construct a short, easy-to-type alias for this URL, suitable for
    /// use as a command-line argument or a directory name.
    fn human_alias(&self) -> Result<String, Error>;

    /// Turn this URL into arguments to `git clone`.
    fn clone_args(&self) -> Result<Vec<OsString>, Error>;
}

impl GitUrlExt for dc::GitUrl {
    fn human_alias(&self) -> Result<String, Error> {
        // Convert a regular URL so we can parse it.
        let url: url::Url = try!(self.to_url());

        // Get the last component of the path.  The `unwrap` should be safe
        // because we construct the path from the URL and we know it's
        // UTF-8.
        //
        // TODO LOW: We may need to unescape the path.
        let url_path = Path::new(url.path()).to_owned();
        let base_alias = try!(url_path.file_stem()
                .ok_or_else(|| err!("Can't get repo name from {}", self)))
            .to_str()
            .unwrap()
            .to_owned();

        // Get the branch.  If available, this will be stored in the query.
        match url.fragment() {
            None => Ok(base_alias),
            Some(branch) => Ok(format!("{}_{}", base_alias, branch)),
        }
    }

    fn clone_args(&self) -> Result<Vec<OsString>, Error> {
        let url_str: &str = self.as_ref();
        if let Some(pos) = url_str.find('#') {
            let (base, branch) = url_str.split_at(pos);
            Ok(vec!["-b".into(), branch[1..].into(), base.into()])
        } else {
            Ok(vec![url_str.into()])
        }
    }
}

#[test]
fn human_alias_uses_dir_name_and_branch() {
    let master = dc::GitUrl::new("https://github.com/faradayio/rails_hello.git")
        .unwrap();
    assert_eq!(master.human_alias().unwrap(), "rails_hello");

    let branch = dc::GitUrl::new("https://github.com/faradayio/rails_hello.git#dev")
        .unwrap();
    assert_eq!(branch.human_alias().unwrap(), "rails_hello_dev");
}

#[test]
fn clone_args_handles_branch() {
    let master = dc::GitUrl::new("https://github.com/faradayio/rails_hello.git")
        .unwrap();
    let expected_url: OsString = master.to_string().into();
    assert_eq!(master.clone_args().unwrap(), vec![expected_url.clone()]);

    let branch = dc::GitUrl::new("https://github.com/faradayio/rails_hello.git#dev")
        .unwrap();
    assert_eq!(branch.clone_args().unwrap(),
               vec!["-b".into(), "dev".into(), expected_url.clone()]);
}
