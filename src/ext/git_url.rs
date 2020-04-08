//! Extension methods for `compose_yml::v2::GitUrl`.

use compose_yml::v2 as dc;
use std::ffi::OsString;

use crate::errors::*;

/// These methods will appear as regular methods on `Context` in any module
/// which includes `ContextExt`.
pub trait GitUrlExt {
    /// Turn this URL into arguments to `git clone`.
    fn clone_args(&self) -> Result<Vec<OsString>>;
}

impl GitUrlExt for dc::GitUrl {
    fn clone_args(&self) -> Result<Vec<OsString>> {
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
fn clone_args_handles_branch() {
    let master =
        dc::GitUrl::new("https://github.com/faradayio/rails_hello.git").unwrap();
    let expected_url: OsString = master.to_string().into();
    assert_eq!(master.clone_args().unwrap(), vec![expected_url.clone()]);

    let branch =
        dc::GitUrl::new("https://github.com/faradayio/rails_hello.git#dev").unwrap();
    assert_eq!(
        branch.clone_args().unwrap(),
        vec!["-b".into(), "dev".into(), expected_url.clone()]
    );
}
