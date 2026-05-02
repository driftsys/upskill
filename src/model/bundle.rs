//! Bundle items (§3.7).
//!
//! Bundles are source-registry artifacts that reference items by `name`.
//! Consumer projects do not contain bundle files — installs record the
//! bundle (and the items it resolves to) in the consumer-side state file
//! `.upskill-lock.json`.
//!
//! Per format-spec §3.7 (post-PR #76 fixes): `requires` is map-only — no
//! polymorphic string-or-map alternative. Each `requires` entry references
//! another bundle by `name`, optionally pinned with a semver `version`
//! constraint string.

use crate::model::common::{License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub schema: SchemaVersion,
    pub name: String,
    pub description: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,

    pub items: BundleItems,

    /// Bundle dependencies — references to other bundles by `name`. Each
    /// entry MAY pin a semver `version` constraint. Single-form (map-only)
    /// per §3.7; the parser rejects bare strings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Requires>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Pass-through for unknown top-level fields, mirrored from the
    /// item models (§3.3).
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

/// What this bundle provides. Every list is optional and defaults to empty
/// — a bundle that only carries `requires` (a meta-bundle composing other
/// bundles) is valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BundleItems {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
}

impl BundleItems {
    /// True when no rule, skill, or agent is named.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.skills.is_empty() && self.agents.is_empty()
    }
}

/// One dependency edge in `Bundle.requires`. Map-only by design: bare
/// strings (`requires: ["other-bundle"]`) are rejected by serde because
/// this struct cannot be deserialized from a scalar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Requires {
    pub name: String,
    /// Semver-style version constraint (e.g., `"^1.0.0"`, `"1.2.3"`).
    /// Constraint syntax is not parsed at this layer — kept as a string for
    /// the resolver to interpret in C2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
