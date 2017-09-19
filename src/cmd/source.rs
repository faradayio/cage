//! The `source` subcommand.

use command_runner::CommandRunner;
use errors::*;
use project::Project;

extern crate prettytable;
use self::prettytable::Table;
use self::prettytable::format::FORMAT_BLANK;

/// We implement `source` with a trait so we put it in its own module.
pub trait CommandSource {
    /// List all the source trees associated with a project.
    fn source_list<CR>(&self, runner: &CR) -> Result<()> where CR: CommandRunner;

    /// Clone the specified source tree.
    fn source_clone<CR>(&self, runner: &CR, alias: &str) -> Result<()>
        where CR: CommandRunner;

    /// Set the `mounted` flag on the specified source tree.
    fn source_set_mounted<CR>(&mut self,
                              runner: &CR,
                              alias: &str,
                              mounted: bool)
                              -> Result<()>
        where CR: CommandRunner;
}


impl CommandSource for Project {
    fn source_list<CR>(&self, _runner: &CR) -> Result<()>
        where CR: CommandRunner
    {
        let mut table = Table::new();
        table.set_format(FORMAT_BLANK);

        table.add_row(row!["SERVICE", "IMAGE", "STATUS", "LOCAL", "REMOTE"]);

        for source in self.sources().iter() {

            let available_locally = source.is_available_locally(self);

            let status = if available_locally && source.mounted() {
                "mounted"
            } else {
                "unmounted"
            };

            let local = if available_locally {
                let canonical = source.path(self).canonicalize()?;
                // Try to strip the prefix, but this may fail on Windows
                // or if the source is in a weird location.
                let path = match canonical.strip_prefix(self.root_dir()) {
                    Ok(stripped) => stripped.to_owned(),
                    Err(_) => canonical.to_owned(),
                };
                format!("{}", path.display()).to_owned()
            } else {
                String::from("")
            };

            table.add_row(row![
                source.alias(),
                source.image(),
                status,
                local,
                source.context()
            ]);

        }

        table.printstd();

        Ok(())
    }

    fn source_clone<CR>(&self, runner: &CR, alias: &str) -> Result<()>
        where CR: CommandRunner
    {
        let source = self.sources()
            .find_by_alias(alias)
            .ok_or_else(|| ErrorKind::UnknownSource(alias.to_owned()))?;
        if !source.is_available_locally(self) {
            source.clone_source(runner, self)?;
        } else {
            println!("'{}' is already available locally", source.alias());
        }
        Ok(())
    }

    fn source_set_mounted<CR>(&mut self,
                              runner: &CR,
                              alias: &str,
                              mounted: bool)
                              -> Result<()>
        where CR: CommandRunner
    {
        {
            // Look up the source mutably.  We do this in a block so we can
            // drop the mutable borrow before continuing and keep Rust
            // happy.
            let source = self.sources_mut()
                .find_by_alias_mut(alias)
                .ok_or_else(|| ErrorKind::UnknownSource(alias.to_owned()))?;

            // Set the mounted flag on our source.
            source.set_mounted(mounted);
        }

        // Write our persistent project settings back to disk.
        self.save_settings()?;

        // Clone the source if we're mounting it but don't have a local
        // copy yet.
        let source = self.sources()
            .find_by_alias(alias)
            .ok_or_else(|| ErrorKind::UnknownSource(alias.to_owned()))?;
        if source.mounted() && !source.is_available_locally(self) {
            self.source_clone(runner, alias)?;
        }

        // Notify the user that they need to run `up`.
        println!("Now run `cage up` for these changes to take effect.");

        Ok(())
    }
}

// No tests because this is a very thin wrapper over `Sources` and `Source`.
