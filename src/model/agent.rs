//! Agent items (§3.4 + §3.1 common fields).

use crate::model::common::{Audience, License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Agent run mode (§3.4). Spec default is `subagent`; the default is applied
/// at generation time, not at parse time, so this field stays `Option`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Primary,
    Subagent,
    All,
}

/// Capability-level tool name (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolCap {
    Read,
    Write,
    Edit,
    Bash,
    Grep,
    Glob,
    WebFetch,
    WebSearch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub schema: SchemaVersion,
    /// Effective name resolution is layout-dependent (§2.1): absent means
    /// the directory name is used. Resolved by the pipeline/lint layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub description: String,

    /// §3.1: top-level audience targeting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Audience>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolCap>,
    #[serde(
        rename = "preload-skills",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub preload_skills: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// §3.7 directed dependencies. SSOT-only — stripped from output.
    #[serde(default, skip_serializing_if = "crate::model::ItemRequires::is_empty")]
    pub requires: crate::model::ItemRequires,
    /// §2.4 subtractive copy-scope patterns. SSOT-only — stripped from output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<serde_yaml_ng::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copilot: Option<serde_yaml_ng::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode: Option<serde_yaml_ng::Value>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}
