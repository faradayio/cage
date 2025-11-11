//! Support for project-related template files and generation.  Used to
//! implementing things like the `new` command.

use handlebars as hb;
use include_dir::{include_dir, Dir, DirEntry};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf, MAIN_SEPARATOR};

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
    pub fn new(name: &str) -> Result<Template> {
        // include_dir! always uses forward slashes regardless of platform.
        let prefix = format!("templates/{}/", name);
        let sep_underscore = "/_";

        // Iterate over all matching files built into this library at compile time.
        let mut files = BTreeMap::new();

        // Use entries() and recursively check directories
        fn visit_dir(
            dir: &Dir,
            prefix: &str,
            sep_underscore: &str,
            files: &mut BTreeMap<PathBuf, String>,
        ) -> Result<()> {
            for entry in dir.entries() {
                match entry {
                    DirEntry::Dir(subdir) => {
                        visit_dir(subdir, prefix, sep_underscore, files)?;
                    }
                    DirEntry::File(file) => {
                        let path_str = file.path().to_string_lossy();
                        trace!(
                            "checking template path {} in prefix {}",
                            path_str,
                            prefix
                        );
                        if let Some(rel) = path_str.strip_prefix(prefix) {
                            // Make sure it doesn't belong to a child template.
                            if !rel.starts_with('_') && !rel.contains(sep_underscore) {
                                // Load this file and add it to our list.
                                let raw_data = file.contents().to_owned();
                                let data = String::from_utf8(raw_data)?;
                                // Convert the path to use the platform's separator when storing.
                                let platform_path =
                                    if MAIN_SEPARATOR != '/' {
                                        PathBuf::from(rel.replace(
                                            '/',
                                            std::path::MAIN_SEPARATOR_STR,
                                        ))
                                    } else {
                                        Path::new(rel).to_owned()
                                    };
                                files.insert(platform_path, data);
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        visit_dir(&DATA, &prefix, sep_underscore, &mut files)?;

        Ok(Template {
            name: name.to_string(),
            files,
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

            // Make sure our parent directory exists.
            path.with_guaranteed_parent()?;

            // Create our output file.
            let out = fs::File::create(&path).map_err(|e| {
                anyhow::Error::new(e).context(Error::CouldNotWriteFile(path.clone()))
            })?;
            let mut writer = io::BufWriter::new(out);

            // Render our template to the file.
            // Create our Handlebars template engine.
            let mut hb = hb::Handlebars::new();
            hb.register_escape_fn(escape_double_quotes);
            hb.render_template_to_write(tmpl, &data, &mut writer)
                .map_err(|e| {
                    anyhow::Error::new(e)
                        .context(Error::CouldNotWriteFile(path.clone()))
                })?;
        }
        Ok(())
    }
}

#[test]
fn loads_correct_files_for_template() {
    let tmpl = Template::new("test_tmpl").unwrap();
    let keys: Vec<_> = tmpl.files.keys().cloned().collect();
    assert!(keys.contains(&Path::new("test.txt").to_owned()));
    assert!(keys.contains(&Path::new("nested").join("nested.txt")));
    assert!(!keys.contains(&Path::new("_child_tmpl").join("child.txt")));
}
