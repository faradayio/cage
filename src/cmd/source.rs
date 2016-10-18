//! The `source` subcommand.

use colored::*;

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

    /// Set the `mounted` flag on the specified source tree.
    fn source_set_mounted(&mut self, alias: &str, mounted: bool) -> Result<()>;
}


impl CommandSource for Project {
    fn source_list<CR>(&self, _runner: &CR) -> Result<()>
        where CR: CommandRunner
    {
        for source in self.sources().iter() {
            println!("{:25} {}", source.alias().green(), source.context());
            if source.is_available_locally(self) {
                let path = try!(try!(source.path(self).canonicalize())
                        .strip_prefix(self.root_dir()))
                    .to_owned();
                let mounted = if source.mounted() {
                    "(mounted)".normal()
                } else {
                    "(NOT MOUNTED)".red().bold()
                };
                println!("  Available at {} {}", path.display(), mounted);
            }
        }
        Ok(())
    }

    fn source_clone<CR>(&self, runner: &CR, alias: &str) -> Result<()>
        where CR: CommandRunner
    {
        let source = try!(self.sources()
            .find_by_alias(alias)
            .ok_or_else(|| ErrorKind::UnknownSource(alias.to_owned())));
        if !source.is_available_locally(self) {
            try!(source.clone_source(runner, self));
        } else {
            println!("'{}' is already available locally", source.alias());
        }
        Ok(())
    }

    fn source_set_mounted(&mut self, alias: &str, mounted: bool) -> Result<()> {
        {
            let source = try!(self.sources_mut()
                .find_by_alias_mut(alias)
                .ok_or_else(|| ErrorKind::UnknownSource(alias.to_owned())));
            source.set_mounted(mounted);
        }
        try!(self.save_settings());
        Ok(())
    }
}

// No tests because this is a very thin wrapper over `Sources` and `Source`.
