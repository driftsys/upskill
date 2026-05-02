//! Rule items (§3.2 + §3.1 common fields).

use crate::model::common::{Audience, License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `scope` block (§3.2). Empty `paths` means always-on.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub schema: SchemaVersion,
    pub name: String,
    pub description: String,

    /// §3.1: top-level audience targeting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Audience>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<serde_yaml_ng::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copilot: Option<serde_yaml_ng::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode: Option<serde_yaml_ng::Value>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}
