//! `conductor` as a reusable API, so that you can call it from other tools.

#![warn(missing_docs)]

extern crate docker_compose;
extern crate glob;
#[macro_use] extern crate log;
extern crate rand;
extern crate regex;

pub use util::Error;
pub use ovr::Override;
pub use project::Project;
pub use pod::Pod;

#[macro_use] mod util;
#[macro_use] pub mod command_runner;
pub mod cmd;
pub mod dir;
mod ext;
mod ovr;
mod pod;
mod project;

// TODO: Save this code; we're just about to write unit tests for it.
//
//        // Figure out where we'll keep the local checkout, if any.
//        let build_dir = try!(service.local_build_dir());
//
//        // If we have a local build directory, update the service to use it.
//        if let Some(ref dir) = build_dir {
//            if dir.exists() {
//                // Make build dir path relative to `.output/pods`.
//                let rel = Path::new("../../").join(dir);
//
//                // Mount the local build directory as `/app` inside the
//                // container.
//                let mount = dc::VolumeMount::host(&rel, "/app");
//                service.volumes.push(dc::value(mount));
//
//                // Update the `build` field if present.
//                if let Some(ref mut build) = service.build {
//                    build.context = dc::value(dc::Context::Dir(rel.clone()));
//                }
//            }
//        }
