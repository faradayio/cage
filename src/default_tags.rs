//! Tools for working with lists of image tags (versions) provided by an
//! external source.

use compose_yml::v2 as dc;
use std::collections::btree_map;
use std::collections::BTreeMap;
use std::io::{self, BufRead};
use std::str::FromStr;

use crate::errors::*;

/// This is typically used to incorporate image tags for specific builds
/// generated by a continuous integration system (such as [Go][GoCD]).
///
/// The on-disk format is a text file with one tagged image name per line:
///
/// ```txt
/// example.com/app1:30
/// example.com/app2:57
/// alpine:4.3
/// ```
///
/// The tags from this file will be used as default tags for these images.
/// So for example, `example.com/app1` would default to
/// `example.com/app1:30`, and `alpine` would default to `alpine:4.3`.
///
/// [GoCD]: https://www.go.cd/
#[derive(Debug)]
pub struct DefaultTags {
    /// Our default tags. All the `Image` keys should have a tag of `None`,
    /// and the values should have a tag of `Some(...)`.
    tags: BTreeMap<dc::Image, dc::Image>,
}

impl DefaultTags {
    /// Read in tag defaults from a stream.
    pub fn read<R>(r: R) -> Result<Self>
    where
        R: io::Read,
    {
        let mut tags = BTreeMap::new();
        let reader = io::BufReader::new(r);
        for line_result in reader.lines() {
            let line = line_result?;
            let image = dc::Image::from_str(&line)?;
            if let (key, Some(_)) = (image.without_version(), image.version.as_ref()) {
                match tags.entry(key.to_owned()) {
                    btree_map::Entry::Vacant(vacant) => {
                        vacant.insert(image.to_owned());
                    }
                    btree_map::Entry::Occupied(occupied) => {
                        if occupied.get() != &image {
                            return Err(err!("Conflicting versions for {}", &key));
                        }
                    }
                }
            } else {
                return Err(err!("Default image must have tag: {}", &image));
            }
        }
        Ok(DefaultTags { tags })
    }

    /// Default the `tag` field of `image` if necessary, returning the old
    /// image if possible.
    pub fn default_for(&self, image: &dc::Image) -> dc::Image {
        if image.version.is_some() {
            // Already tagged, so assume the user knows what they're doing.
            image.to_owned()
        } else if let Some(default) = self.tags.get(image) {
            debug!("Defaulting {} to {}", image, &default);
            default.to_owned()
        } else {
            // If we have a list of default tags, but it doesn't
            // include all the images we use, then we consider that
            // mildy alarming.  Note that we do show warnings by
            // default.
            warn!("Could not find default tag for {}", image);
            image.to_owned()
        }
    }
}

#[test]
fn defaults_tags_using_data_from_file() {
    let file = "example.com/app1:30
alpine:4.3
busybox@sha256:cbbf2f9a99b47fc460d422812b6a5adff7dfee951d8fa2e4a98caa0382cfbdbf
";
    let cursor = io::Cursor::new(file);
    let default_tags = DefaultTags::read(cursor).unwrap();
    assert_eq!(
        default_tags.default_for(&dc::Image::new("alpine").unwrap()),
        dc::Image::new("alpine:4.3").unwrap()
    );
    assert_eq!(
        default_tags.default_for(&dc::Image::new("alpine:4.2").unwrap()),
        dc::Image::new("alpine:4.2").unwrap()
    );
    assert_eq!(
        default_tags.default_for(&dc::Image::new("busybox").unwrap()),
        dc::Image::new("busybox@sha256:cbbf2f9a99b47fc460d422812b6a5adff7dfee951d8fa2e4a98caa0382cfbdbf").unwrap()
    );
    // TODO LOW: I'm not sure how we should actually handle `latest`.
    // Should it default?
    assert_eq!(
        default_tags.default_for(&dc::Image::new("alpine:latest").unwrap()),
        dc::Image::new("alpine:latest").unwrap()
    );
}
