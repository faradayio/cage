//! Plugins which transform `dc::File` objects.

pub mod abs_path;
pub mod default_tags;
pub mod labels;
pub mod secrets;
pub mod sources;
#[cfg(feature="hashicorp_vault")]
pub mod vault;
