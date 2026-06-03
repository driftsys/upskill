//! Skill items (§3.3 + §3.1 common fields).

use crate::model::common::{Audience, License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub schema: SchemaVersion,
    /// Effective name resolution is layout-dependent (§2.1): absent means
    /// the directory name is used. Resolved by the pipeline/lint layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub description: String,

    /// §3.1: top-level audience targeting. When present, generation only
    /// emits for listed clients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Audience>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// §3.7 directed dependencies. SSOT-only — stripped from output.
    #[serde(default, skip_serializing_if = "crate::model::ItemRequires::is_empty")]
    pub requires: crate::model::ItemRequires,
    /// §2.4 subtractive copy-scope patterns. SSOT-only — stripped from output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,

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
