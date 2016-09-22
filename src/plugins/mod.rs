//! Plugin support for conductor.

use std::marker::PhantomData;

use ovr::Override;
use pod::Pod;
use project::Project;

pub mod transform;

/// The context in which a plugin is being applied.
#[derive(Debug)]
pub struct Context<'a> {
    /// The project to which we're applying this plugin.
    pub project: &'a Project,
    /// The overlay which we're currently using.
    pub ovr: &'a Override,
    /// The pod to which we're applying this plugin.
    pub pod: &'a Pod,
    /// PRIVATE. Allow future extensibility without breaking the API.
    _nonexclusive: PhantomData<()>,
}

impl<'a> Context<'a> {
    /// Create a new plugin context.
    pub fn new(project: &'a Project, ovr: &'a Override, pod: &'a Pod) -> Context<'a> {
        Context {
            project: project,
            ovr: ovr,
            pod: pod,
            _nonexclusive: PhantomData,
        }
    }
}
