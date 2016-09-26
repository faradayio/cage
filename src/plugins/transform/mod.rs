//! Plugins which transform `dc::File` objects.

use docker_compose::v2 as dc;
use std::fmt::Debug;
use std::marker::Sized;

use plugins;
use project::Project;
use util::Error;

pub mod secrets;
pub mod vault;

/// What kind of transform operation are we performing?  (Adding new kinds
/// of operations will be a breaking API change for plugins.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// We're outputting a file for local development with conductor.
    Output,
    /// We're exporting a file for use by another tool.
    Export,
}

/// A plugin which transforms a `dc::File` object.
pub trait Plugin {
    /// The name of this plugin.
    fn name(&self) -> &'static str;

    /// Transform the specified file.
    fn transform(&self,
                 op: Operation,
                 ctx: &plugins::Context,
                 file: &mut dc::File)
                 -> Result<(), Error>;
}

/// Initialization for `Plugin`.  These methods can't be part of `Plugin`
/// itself, because that would prevent us from working with runtime
/// polymorphic trait objects such as `&Plugin` or `Box<Plugin>` (i.e.,
/// runtime object orientation).
pub trait PluginNew: Plugin + Sized + Debug {
    /// Should we enable this plugin for this project?
    fn should_enable_for(project: &Project) -> Result<bool, Error>;

    /// Create a new plugin.
    fn new(project: &Project) -> Result<Self, Error>;
}
