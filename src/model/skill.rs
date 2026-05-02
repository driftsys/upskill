//! Skill items (§3.3 + §3.1 common fields).

use crate::model::common::{Audience, License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub schema: SchemaVersion,
    pub name: String,
    pub description: String,

    /// §3.1: top-level audience targeting (promoted from `metadata.audience`
    /// per format-spec PR #76). When present, generation only emits for
    /// listed clients. `metadata.audience` is still accepted as a fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Audience>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// §3.5 client-specific passthrough block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<serde_yaml_ng::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copilot: Option<serde_yaml_ng::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode: Option<serde_yaml_ng::Value>,

    /// §3.3: pass-through unknown top-level fields preserved during processing.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}
