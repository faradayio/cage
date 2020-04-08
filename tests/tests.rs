extern crate cli_test_dir;
extern crate copy_dir;

use cli_test_dir::*;
use copy_dir::copy_dir;
use std::env;

#[test]
fn project_from_current_dir() {
    let testdir = TestDir::new("cage", "project_from_current_dir");
    let saved = env::current_dir().expect("Could not get current_dir");

    copy_dir(testdir.src_path("examples/hello"), testdir.path("hello"))
        .expect("could not copy hello example");

    testdir.expect_path("hello/pods");

    testdir
        .cmd()
        // We want to make sure this test runs in a subdirectory in the project
        // and is able to resolve the project root.
        .current_dir(testdir.path("hello/pods"))
        .args(&["export", "exported"])
        .expect_success();

    testdir.expect_path("hello/pods/exported/frontend.yml");
}
