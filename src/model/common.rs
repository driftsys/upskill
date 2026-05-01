//! Common types shared across all item kinds.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// Highest schema version this implementation supports.
pub const CURRENT_SCHEMA: u32 = 1;

/// `schema` field with §8.1 rejection logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersion(u32);

impl SchemaVersion {
    pub fn new(v: u32) -> anyhow::Result<Self> {
        if v == 0 {
            anyhow::bail!("schema version must be >= 1, got 0");
        }
        if v > CURRENT_SCHEMA {
            anyhow::bail!(
                "schema version {} is not supported (this build of upskill supports up to schema {}). Upgrade upskill.",
                v,
                CURRENT_SCHEMA
            );
        }
        Ok(Self(v))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(ser)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let v = u32::deserialize(de)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

/// Target client for an item or content block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    Claude,
    Copilot,
    OpenCode,
}

/// `license` field — SPDX identifier or custom string (§3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct License(pub String);

/// `metadata` block (§3.6) — preserves unknown keys via the `extra` catchall.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Audience>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}
