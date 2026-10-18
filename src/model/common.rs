//! Common types shared across all item kinds.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

use crate::target::TargetFamily;

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

/// Source audience family for an item or content block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    Claude,
    Copilot,
    Codex,
    OpenCode,
}

impl std::str::FromStr for Audience {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "claude" => Ok(Self::Claude),
            "copilot" => Ok(Self::Copilot),
            "codex" => Ok(Self::Codex),
            "opencode" => Ok(Self::OpenCode),
            other => anyhow::bail!("unknown client identifier: {other}"),
        }
    }
}

impl From<Audience> for TargetFamily {
    fn from(audience: Audience) -> Self {
        match audience {
            Audience::Claude => Self::Claude,
            Audience::Copilot => Self::Copilot,
            Audience::Codex => Self::Codex,
            Audience::OpenCode => Self::OpenCode,
        }
    }
}

impl TryFrom<TargetFamily> for Audience {
    type Error = anyhow::Error;

    fn try_from(family: TargetFamily) -> anyhow::Result<Self> {
        match family {
            TargetFamily::Claude => Ok(Self::Claude),
            TargetFamily::Copilot => Ok(Self::Copilot),
            TargetFamily::Codex => Ok(Self::Codex),
            TargetFamily::OpenCode => Ok(Self::OpenCode),
            TargetFamily::Cursor | TargetFamily::Gemini => {
                anyhow::bail!("unsupported audience target family: {family:?}")
            }
        }
    }
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
    pub updated: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::TargetFamily;

    #[test]
    fn codex_audience_parses_and_serializes() {
        let audience: Audience = serde_yaml_ng::from_str("codex").unwrap();
        assert_eq!(audience, Audience::Codex);
        assert_eq!(serde_yaml_ng::to_string(&audience).unwrap(), "codex\n");
        assert_eq!("codex".parse::<Audience>().unwrap(), Audience::Codex);
    }

    #[test]
    fn audience_converts_supported_families() {
        for (audience, family) in [
            (Audience::Claude, TargetFamily::Claude),
            (Audience::Copilot, TargetFamily::Copilot),
            (Audience::Codex, TargetFamily::Codex),
            (Audience::OpenCode, TargetFamily::OpenCode),
        ] {
            assert_eq!(TargetFamily::from(audience), family);
            assert_eq!(Audience::try_from(family).unwrap(), audience);
        }
    }

    #[test]
    fn audience_rejects_unknown_and_unsupported_families() {
        for name in ["cursor", "gemini", "foo", "Codex", "codex.cli", ""] {
            assert!(serde_yaml_ng::from_str::<Audience>(name).is_err(), "{name}");
            assert!(name.parse::<Audience>().is_err(), "{name}");
        }
        for family in [TargetFamily::Cursor, TargetFamily::Gemini] {
            assert!(Audience::try_from(family).is_err());
        }
    }
}
