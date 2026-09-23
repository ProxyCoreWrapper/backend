use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigMetadata {
    pub core_tag: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MetadataDB {
    pub configs_meta: BTreeMap<String, ConfigMetadata>,
}
