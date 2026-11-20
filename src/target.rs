//! Stable identities for coding-client configuration surfaces.
//!
//! A family can have more than one surface with different discovery and
//! configuration behavior. A surface in this module is not necessarily
//! selectable or implemented by an adapter.

use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetFamily {
    Claude,
    Copilot,
    Codex,
    OpenCode,
    Cursor,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetSurface {
    ClaudeCli,
    CopilotCli,
    CopilotVscodeLocal,
    CodexCli,
    CodexIde,
    OpenCodeCli,
}

impl TargetSurface {
    pub const ALL: [Self; 6] = [
        Self::ClaudeCli,
        Self::CopilotCli,
        Self::CopilotVscodeLocal,
        Self::CodexCli,
        Self::CodexIde,
        Self::OpenCodeCli,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::ClaudeCli => "claude.cli",
            Self::CopilotCli => "copilot.cli",
            Self::CopilotVscodeLocal => "copilot.vscode-local",
            Self::CodexCli => "codex.cli",
            Self::CodexIde => "codex.ide",
            Self::OpenCodeCli => "opencode.cli",
        }
    }

    pub const fn family(self) -> TargetFamily {
        match self {
            Self::ClaudeCli => TargetFamily::Claude,
            Self::CopilotCli | Self::CopilotVscodeLocal => TargetFamily::Copilot,
            Self::CodexCli | Self::CodexIde => TargetFamily::Codex,
            Self::OpenCodeCli => TargetFamily::OpenCode,
        }
    }
}

impl FromStr for TargetSurface {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|surface| surface.name() == s)
            .ok_or_else(|| anyhow::anyhow!("unknown target surface: {s}"))
    }
}

impl Serialize for TargetSurface {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for TargetSurface {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        name.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_identifiers_match_contract() {
        let cases = [
            (TargetSurface::ClaudeCli, "claude.cli", TargetFamily::Claude),
            (
                TargetSurface::CopilotCli,
                "copilot.cli",
                TargetFamily::Copilot,
            ),
            (
                TargetSurface::CopilotVscodeLocal,
                "copilot.vscode-local",
                TargetFamily::Copilot,
            ),
            (TargetSurface::CodexCli, "codex.cli", TargetFamily::Codex),
            (TargetSurface::CodexIde, "codex.ide", TargetFamily::Codex),
            (
                TargetSurface::OpenCodeCli,
                "opencode.cli",
                TargetFamily::OpenCode,
            ),
        ];
        assert_eq!(TargetSurface::ALL.len(), cases.len());
        for (surface, name, family) in cases {
            assert_eq!(surface.name(), name);
            assert_eq!(name.parse::<TargetSurface>().unwrap(), surface);
            assert_eq!(surface.family(), family);
        }
        assert!("cursor.editor".parse::<TargetSurface>().is_err());
    }
}
