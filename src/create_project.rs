//! Put some module docs here to avoid the warning.

use std::env;
use std::fs;
use std::path::PathBuf;
use util::{Error};

pub fn create_project(name: &str) -> Result<PathBuf, Error> {
    // src/create_project.rs:14:5: 14:8 error: cannot borrow immutable local variable `cwd` as mutable
    // src/create_project.rs:14     cwd.push(name);
    //                              ^~~
    //
    // Just add `mut` and you're good.
    let mut cwd = try!(env::current_dir());
    cwd.push(name);

    // src/create_project.rs:18:8: 18:11 error: use of moved value: `cwd` [E0382]
    // src/create_project.rs:18     Ok(cwd)
    //                                 ^~~
    // src/create_project.rs:16:20: 16:23 note: value moved here
    // src/create_project.rs:16     fs::create_dir(cwd);
    //                                        ^~~
    //
    // Well, this one is easy: Just pass a reference to `cwd` using `&`.
    //
    // src/create_project.rs:24:5: 24:26 warning: unused result which must be used, #[warn(unused_must_use)] on by default
    // src/create_project.rs:24     fs::create_dir(&cwd);
    //                              ^~~~~~~~~~~~~~~~~~~~~
    //
    // ...and wrap it in `try`:
    try!(fs::create_dir(&cwd));

    // src/create_project.rs:13:5: 13:7 error: unable to infer enough type information about `_`; type
    // annotations or generic parameter binding required [E0282]
    //
    // Your first error was caused by the trailing comma after this line
    // (`Ok(cwd);`), which turned it into a statement.  In this case, the compiler still needs to figure out the type of the expression, and it can see that it's clearly `Result<PathBuf>
    Ok(cwd)
}

#[test]
fn create_project_default() {
    create_project("test").unwrap();
}
