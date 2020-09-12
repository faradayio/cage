//! What sources should a command act on?

use std::{collections::BTreeSet, iter::FromIterator};

use crate::sources::{Source, Sources};

/// What sources do we want to operate on?
#[derive(Debug)]
pub enum ActOnSources {
    /// All our sources.
    All,
    /// Only the specified sources. Names that do not correspond to any source
    /// will be ignored, so check that before getting here.
    Named(Vec<String>),
}

impl ActOnSources {
    /// Iterate over the pods or services specified by this `ActOn` object.
    pub fn sources_mut<'a>(
        &'a self,
        sources: &'a mut Sources,
    ) -> impl Iterator<Item = &'a mut Source> + 'a {
        match self {
            ActOnSources::All => Box::new(sources.iter_mut())
                as Box<dyn Iterator<Item = &'a mut Source> + 'a>,
            ActOnSources::Named(aliases) => {
                let aliases = BTreeSet::from_iter(aliases.iter().cloned());
                Box::new(
                    sources
                        .iter_mut()
                        .filter(move |source| aliases.contains(source.alias())),
                )
            }
        }
    }
}
