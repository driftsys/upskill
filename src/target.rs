//! Stable identities for coding-client configuration surfaces.
//!
//! A family can have more than one surface with different discovery and
//! configuration behavior. A surface in this module is not necessarily
//! selectable or implemented by an adapter.

use std::str::FromStr;

use anyhow::{Result, bail};

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

    pub const fn profile(self) -> TargetProfile {
        TargetProfile {
            family: self.family(),
            surface: self,
            contract_revision: 1,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetProfile {
    pub family: TargetFamily,
    pub surface: TargetSurface,
    pub contract_revision: u32,
}

impl TargetProfile {
    pub fn new(
        family: TargetFamily,
        surface: TargetSurface,
        contract_revision: u32,
    ) -> Result<Self> {
        if family != surface.family() {
            bail!(
                "surface {} does not belong to family {family:?}",
                surface.name()
            );
        }
        if contract_revision == 0 {
            bail!("target contract revision must be at least 1");
        }
        Ok(Self {
            family,
            surface,
            contract_revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_identifiers_round_trip() {
        for surface in TargetSurface::ALL {
            assert_eq!(surface.name().parse::<TargetSurface>().unwrap(), surface);
            let profile = surface.profile();
            assert_eq!(profile.family, surface.family());
            assert_eq!(profile.contract_revision, 1);
        }
        assert!("cursor.editor".parse::<TargetSurface>().is_err());
    }

    #[test]
    fn profile_rejects_cross_family_and_zero_revision() {
        assert!(TargetProfile::new(TargetFamily::Claude, TargetSurface::CodexCli, 1).is_err());
        assert!(TargetProfile::new(TargetFamily::Codex, TargetSurface::CodexCli, 0).is_err());
    }
}
