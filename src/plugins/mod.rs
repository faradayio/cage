//! Plugin support for conductor.

use docker_compose::v2 as dc;
use std::fmt;
use std::marker::PhantomData;

use ovr::Override;
use pod::Pod;
use plugins::transform::PluginNew;
use project::Project;
use util::Error;

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

/// A collection of plugins, normally associated with a project.
pub struct Manager {
    /// Our `dc::File` transforming plugins.
    transforms: Vec<Box<transform::Plugin>>,
}

impl Manager {
    /// Create a new manager for the specified project.
    pub fn new(proj: &Project) -> Result<Manager, Error> {
        let mut manager = Manager { transforms: vec![] };
        if try!(transform::secrets::Plugin::should_enable_for(&proj)) {
            let plugin = try!(transform::secrets::Plugin::new(&proj));
            manager.transforms.push(Box::new(plugin));
        }
        Ok(manager)
    }

    /// Apply all our transform plugins.
    pub fn transform(&self,
                     op: transform::Operation,
                     ctx: &Context,
                     file: &mut dc::File)
                     -> Result<(), Error>
    {
        for plugin in &self.transforms {
            try!(plugin.transform(op, ctx, file));
        }
        Ok(())
    }
}

impl fmt::Debug for Manager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut names: Vec<_> = vec![];
        names.extend_from_slice(&self.transforms.iter().map(|p| p.name()).collect::<Vec<_>>());
        write!(f, "plugins::Manager {{ {:?} }}", &names)
    }
}
