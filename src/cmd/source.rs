//! The `source` subcommand.

use command_runner::CommandRunner;
use errors::*;
use project::Project;

/// We implement `source` with a trait so we put it in its own module.
pub trait CommandSource {
    /// List all the source trees associated with a project.
    fn source_list<CR>(&self, runner: &CR) -> Result<()> where CR: CommandRunner;

    /// Clone the specified source tree.
    fn source_clone<CR>(&self, runner: &CR, alias: &str) -> Result<()>
        where CR: CommandRunner;
}


impl CommandSource for Project {
    fn source_list<CR>(&self, _runner: &CR) -> Result<()>
        where CR: CommandRunner
    {
        for source in self.sources().iter() {
            println!("{:25} {}", source.alias(), source.context());
            if source.is_available_locally(self) {
                let path = try!(source.path(self)
                        .strip_prefix(self.root_dir()))
                    .to_owned();
                println!("  Available at {}", path.display());
            }
        }
        Ok(())
    }

    fn source_clone<CR>(&self, runner: &CR, alias: &str) -> Result<()>
        where CR: CommandRunner
    {
        let source = try!(self.sources()
                .find_by_alias(alias)
                .ok_or_else(|| {
                    err!("Could not find a source with short alias \"{}\"", alias)
                }));
        if !source.is_available_locally(self) {
            try!(source.clone_source(runner, self));
        } else {
            println!("'{}' is already available locally", source.alias());
        }
        Ok(())
    }
}

// No tests because this is a very thin wrapper over `Sources` and `Source`.
