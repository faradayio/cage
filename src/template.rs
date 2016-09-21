//! Support for project-related template files and generation.  Used to
//! implementing things like the `new` command.

use handlebars as hb;
use rustc_serialize::json::ToJson;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use data;
use util::{ConductorPathExt, Error};

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
    name: String,
    files: BTreeMap<PathBuf, String>,
    handlebars: hb::Handlebars,
}

impl Template {
    /// Create a new template, loading it from a subdirectory of `data/`
    /// specified by `template_name`.
    pub fn new<S: Into<String>>(name: S) -> Result<Template, Error> {
        let name = name.into();
        let prefix = format!("data/templates/{}/", &name);

        // Iterate over all files built into this library at compile time,
        // loading the ones which belong to us.
        //
        // We cheat and use a private API, but see
        // https://github.com/tilpner/includedir/pull/1 for a proposed API.
        let mut files = BTreeMap::new();
        for key in data::DATA.files.keys() {
            // Does this file belong to our template?
            if key.starts_with(&prefix) {
                let rel: &str = &key[prefix.len()..];
                // Make sure it doesn't belong to a child template.
                if !rel.starts_with('_') && !rel.contains("/_") {
                    // Load this file and add it to our list.
                    let raw_data = try!(data::DATA.get(key)).into_owned();
                    let data = try!(String::from_utf8(raw_data));
                    files.insert(Path::new(rel).to_owned(), data);
                }
            }
        }

        // Create our Handlebars template engine.
        let mut hb = hb::Handlebars::new();
        hb.register_escape_fn(escape_double_quotes);

        Ok(Template {
            name: name,
            files: files,
            handlebars: hb,
        })
    }

    /// Generate this template into `target_dir`, passing `data` to the
    /// Handlebars templates.
    pub fn generate<T>(&mut self, target_dir: &Path, data: &T) ->
        Result<(), Error>
        where T: ToJson + fmt::Debug
    {
        debug!("Generating {} with {:?}", &self.name, data);
        for (rel_path, tmpl) in &self.files {
            let path = target_dir.join(rel_path);
            debug!("Output {}", path.display());
            println!("Generating: {}", rel_path.display());

            // Make sure our parent directory exists.
            try!(path.with_guaranteed_parent());

            // Create our output file.
            let mut out = try!(fs::File::create(&path).map_err(|e| {
                err!("Unable to create file {}: {}", path.display(), &e)
            }));

            // Render our template to the file.
            let ctx = hb::Context::wraps(data);
            try!(self.handlebars.template_renderw(tmpl, &ctx, &mut out).map_err(|e| {
                err!("Unable to generate {}: {}", path.display(), &e)
            }));
        }
        Ok(())
    }
}

#[test]
fn loads_correct_files_for_template() {
    let tmpl = Template::new("test_tmpl").unwrap();
    let keys: Vec<_> = tmpl.files.keys().cloned().collect();
    assert!(keys.contains(&Path::new("test.txt" ).to_owned()));
    assert!(!keys.contains(&Path::new("_child_tmpl/child.txt" ).to_owned()));
}
