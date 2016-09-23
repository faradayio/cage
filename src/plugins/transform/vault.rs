//! Plugin which issues vault tokens to containers.

use docker_compose::v2 as dc;
use serde_yaml;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "serde_macros")]
include!(concat!("vault_config.in.rs"));
#[cfg(feature = "serde_codegen")]
include!(concat!(env!("OUT_DIR"), "/plugins/transform/vault_config.rs"));
