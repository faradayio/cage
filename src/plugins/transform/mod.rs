//! Plugins which transform `dc::File` objects.

pub mod abs_path;
pub mod default_tags;
pub mod host_dns;
pub mod labels;
pub mod remove_build;
pub mod secrets;
pub mod sources;
pub mod vault;
