//! Support for project-related template files and generation.  Used to
//! implementing things like the `new` command.

use handlebars as hb;
use include_dir::{include_dir, Dir, DirEntry};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::errors::*;
use crate::util::ConductorPathExt;

/// A data directory, built into our app at compile-time.
static DATA: Dir = include_dir!("data");

/// Escape double quotes and backslashes in a string that we're rendering,
/// which should work well more-or-less well enough for all the formats
/// we're generating.
///
/// If we need to add new formats, we can add more escape functions and
/// switch between them based on file extension.
fn escape_double_quotes(data: &str) -> String {
    data.replace(r#"\"#, r#"\\"#).replace(r#"""#, r#"\""#)
}

/// A set of files which can be generated.
pub struct Template {
    /// The name used to create this template.
    name: String,
    /// File data associated with this template.
    files: BTreeMap<PathBuf, String>,
}

impl Template {
    /// Create a new template, loading it from a subdirectory of `data/`
    /// specified by `template_name`.
    pub fn new<S: Into<String>>(name: S) -> Result<Template> {
        let name = name.into();
        let prefix = format!("templates/{}/", &name);
        let glob = format!("{}**/*", prefix);

        // Iterate over all matching files built into this library at compile time.
        let mut files = BTreeMap::new();
        for entry in DATA.find(&glob)? {
            if let DirEntry::File(file) = entry {
                assert!(file.path.starts_with(&prefix));
                let rel: &str = &file.path[prefix.len()..];
                // Make sure it doesn't belong to a child template.
                if !rel.starts_with('_') && !rel.contains("/_") {
                    // Load this file and add it to our list.
                    let raw_data = file.contents().to_owned();
                    let data = String::from_utf8(raw_data)?;
                    files.insert(Path::new(rel).to_owned(), data);
                }
            }
        }

        Ok(Template {
            name: name,
            files: files,
        })
    }

    /// Generate this template into `target_dir`, passing `data` to the
    /// Handlebars templates, and writing progress messages to `out`.
    pub fn generate<T>(
        &mut self,
        target_dir: &Path,
        data: &T,
        out: &mut dyn io::Write,
    ) -> Result<()>
    where
        T: Serialize + fmt::Debug,
    {
        debug!("Generating {} with {:?}", &self.name, data);
        for (rel_path, tmpl) in &self.files {
            let path = target_dir.join(rel_path);
            debug!("Output {}", path.display());
            writeln!(out, "Generating: {}", rel_path.display())?;
            let mkerr = || ErrorKind::CouldNotWriteFile(path.clone());

            // Make sure our parent directory exists.
            path.with_guaranteed_parent()?;

            // Create our output file.
            let out = fs::File::create(&path).chain_err(&mkerr)?;
            let mut writer = io::BufWriter::new(out);

            // Render our template to the file.
            // Create our Handlebars template engine.
            let mut hb = hb::Handlebars::new();
            hb.register_escape_fn(escape_double_quotes);
            hb.render_template_to_write(tmpl, &data, &mut writer)
                .chain_err(&mkerr)?;
        }
        Ok(())
    }
}

#[test]
fn loads_correct_files_for_template() {
    let tmpl = Template::new("test_tmpl").unwrap();
    let keys: Vec<_> = tmpl.files.keys().cloned().collect();
    assert!(keys.contains(&Path::new("test.txt").to_owned()));
    assert!(!keys.contains(&Path::new("_child_tmpl/child.txt").to_owned()));
}
