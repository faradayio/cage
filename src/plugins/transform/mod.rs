//! Plugins which transform `dc::File` objects.

use docker_compose::v2 as dc;
use std::marker::Sized;

use plugins;
use project::Project;
use util::Error;

pub mod secrets;
// pub mod vault;

/// What kind of transform operation are we performing?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// We're outputting a file for local development with conductor.
    Output,
    /// We're exporting a file for use by another tool.
    Export,
    /// PRIVATE: Allow future extensibility without breaking compatibility.
    #[doc(hidden)]
    _NonExclusive,
}

/// A plugin which transforms a `dc::File` object.
pub trait Plugin: Sized {
    /// Should we enable this plugin for this project?
    fn should_enable_for(project: &Project) -> Result<bool, Error>;

    /// Create a new plugin.
    fn new(project: &Project) -> Result<Self, Error>;

    /// Transform the specified file.
    fn transform(&self,
                 op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<(), Error>;
}
