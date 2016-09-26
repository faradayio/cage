//! Plugin support for conductor.

use docker_compose::v2 as dc;
use std::fmt;
use std::io;
use std::marker::PhantomData;

use ovr::Override;
use pod::Pod;
use project::Project;
use template::Template;
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

/// What kind of transform operation are we performing?  (Adding new kinds
/// of operations will be a breaking API change for plugins.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// We're outputting a file for local development with conductor.
    Output,
    /// We're exporting a file for use by another tool.
    Export,
}

/// The "super-trait" of all our specific plugin traits.  This needs to be
/// usable as a [trait object][], so it may not contain any static "class"
/// methods or methods with type parameters.  Those can be found in
/// `PluginNew` instead.  This trait needs to be usable as a trait object
/// so that we can create a `Vec` containing multiple different plugin
/// implementations.  Trait objects are as close as Rust comes to object
/// orientation.
///
/// [trait object]: https://doc.rust-lang.org/book/trait-objects.html
pub trait Plugin {
    /// The name of this plugin (available after we create an instance).
    fn name(&self) -> &'static str;
}

/// Initialization for `Plugin`.  These methods can't be part of `Plugin`
/// itself, because they would prevent us from using `Plugin` as a [trait
/// object][].
///
/// [trait object]: https://doc.rust-lang.org/book/trait-objects.html
pub trait PluginNew: Plugin + Sized + fmt::Debug {
    /// The name of this plugin (available before we create an instance).
    fn plugin_name() -> &'static str;

    /// Has this plugin been configured for this project?  This will be
    /// called before instantiating any plugin type except
    /// `PluginGenerate`.
    fn is_configured_for(_project: &Project) -> Result<bool, Error> {
        Ok(true)
    }

    /// Create a new plugin.
    fn new(project: &Project) -> Result<Self, Error>;
}

/// A plugin which transforms a `dc::File` object.
pub trait PluginTransform: Plugin {
    /// Transform the specified file.
    fn transform(&self,
                 op: Operation,
                 ctx: &Context,
                 file: &mut dc::File)
                 -> Result<(), Error>;
}

/// A plugin which can generate source code.
pub trait PluginGenerate: Plugin {
    /// A short, human-readable description of what this generator does in
    /// fewer than 60 characters (for display on monospaced terminals).
    fn generator_description(&self) -> &'static str;

    /// Generate source code.  The default implementation generates the
    /// template of the same name as the plugin, using the project as
    /// input.  This is a good default.
    fn generate(&self, project: &Project, out: &mut io::Write) -> Result<(), Error> {
        let mut proj_tmpl = try!(Template::new(self.name()));
        try!(proj_tmpl.generate(&project.root_dir(), project, out));
        Ok(())
    }
}

/// A collection of plugins, normally associated with a project.
pub struct Manager {
    /// Our `dc::File` transforming plugins.
    transforms: Vec<Box<PluginTransform>>,

    /// Our code generator plugins.
    generators: Vec<Box<PluginGenerate>>,
}

impl Manager {
    /// Create a new manager for the specified project.
    pub fn new(proj: &Project) -> Result<Manager, Error> {
        let mut manager = Manager {
            transforms: vec![],
            generators: vec![],
        };
        // We instantiate some of these plugins twice, could we be more
        // clever about it?
        try!(manager.register_generator::<transform::secrets::Plugin>(proj));
        try!(manager.register_generator::<transform::vault::Plugin>(proj));

        try!(manager.register_transform::<transform::repos::Plugin>(proj));
        try!(manager.register_transform::<transform::secrets::Plugin>(proj));
        try!(manager.register_transform::<transform::vault::Plugin>(proj));
        try!(manager.register_transform::<transform::default_tags::Plugin>(proj));

        // TODO LOW: Final plugin to remove `io.fdy.conductor.` labels?

        Ok(manager)
    }

    /// Get the generators registered with this plugin manager.
    pub fn generators(&self) -> &[Box<PluginGenerate>] {
        &self.generators
    }

    /// Create a new plugin, returning a reasonably helpful error if we
    /// fail.
    fn new_plugin<T>(&self, proj: &Project) -> Result<T, Error>
        where T: PluginNew + 'static
    {
        T::new(proj)
            .map_err(|e| err!("Error initializing plugin {}: {}", T::plugin_name(), e))
    }

    /// Register a generator with this manager.
    fn register_generator<T>(&mut self, proj: &Project) -> Result<(), Error>
        where T: PluginNew + PluginGenerate + 'static
    {
        let plugin: T = try!(self.new_plugin(&proj));
        self.generators.push(Box::new(plugin));
        Ok(())
    }

    /// Register a transform with this manager.
    fn register_transform<T>(&mut self, proj: &Project) -> Result<(), Error>
        where T: PluginNew + PluginTransform + 'static
    {
        if try!(T::is_configured_for(&proj)) {
            let plugin: T = try!(self.new_plugin(&proj));
            self.transforms.push(Box::new(plugin));
        }
        Ok(())
    }

    /// Run the specified generator in the current project.
    pub fn generate(&self,
                    project: &Project,
                    name: &str,
                    out: &mut io::Write)
                    -> Result<(), Error> {
        let generator = try!(self.generators
            .iter()
            .find(|g| g.name() == name)
            .ok_or_else(|| err!("Cannot find a generator named {}", name)));
        debug!("Generating {}", generator.name());
        generator.generate(project, out)
    }

    /// Apply all our transform plugins.
    pub fn transform(&self,
                     op: Operation,
                     ctx: &Context,
                     file: &mut dc::File)
                     -> Result<(), Error> {
        for plugin in &self.transforms {
            try!(plugin.transform(op, ctx, file)
                .map_err(|e| err!("Error applying plugin {}: {}", plugin.name(), e)));
        }
        Ok(())
    }
}

impl fmt::Debug for Manager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut names: Vec<_> = vec![];
        names.extend_from_slice(&self.transforms
            .iter()
            .map(|p| p.name())
            .collect::<Vec<_>>());
        write!(f, "plugins::Manager {{ {:?} }}", &names)
    }
}
