//! Consumer-side client selection (ADR-0012, issue #238).
//!
//! A consumer restricts which clients an install targets. The selection
//! surface is the **union** of the generation clients
//! ([`Client`](crate::generate::Client): claude/copilot/opencode) and the
//! MCP-only target VS Code — the four `SelectedClient`s a consumer names with
//! `--claude` / `--copilot` / `--vscode` / `--opencode`. Each maps onto the
//! generation space (VS Code shares Copilot's `.github/**` tree) and the MCP
//! space (1:1) internally.

use std::collections::BTreeSet;
use std::str::FromStr;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::generate::Client;
use crate::mcp::McpTarget;

/// One consumer-selectable client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub enum SelectedClient {
    Claude,
    Copilot,
    VsCode,
    OpenCode,
}

impl SelectedClient {
    /// Every selectable client, in declaration order.
    pub const ALL: [SelectedClient; 4] =
        [Self::Claude, Self::Copilot, Self::VsCode, Self::OpenCode];

    /// Stable identifier used on the CLI and in config.
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Copilot => "copilot",
            Self::VsCode => "vscode",
            Self::OpenCode => "opencode",
        }
    }

    /// The generation client whose output tree this selection writes. VS Code
    /// shares Copilot's `.github/**` rules/skills/agents tree, so it maps to
    /// [`Client::Copilot`].
    pub fn generation(self) -> Client {
        match self {
            Self::Claude => Client::Claude,
            Self::Copilot | Self::VsCode => Client::Copilot,
            Self::OpenCode => Client::OpenCode,
        }
    }

    /// The MCP config-write target this selection configures (1:1).
    pub fn mcp(self) -> McpTarget {
        match self {
            Self::Claude => McpTarget::Claude,
            Self::Copilot => McpTarget::Copilot,
            Self::VsCode => McpTarget::VsCode,
            Self::OpenCode => McpTarget::OpenCode,
        }
    }
}

impl FromStr for SelectedClient {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "claude" => Ok(Self::Claude),
            "copilot" => Ok(Self::Copilot),
            "vscode" => Ok(Self::VsCode),
            "opencode" => Ok(Self::OpenCode),
            other => bail!(
                "unknown client '{other}' (expected one of: claude, copilot, vscode, opencode)"
            ),
        }
    }
}

impl TryFrom<String> for SelectedClient {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> {
        s.parse()
    }
}

/// A consumer's client selection. `None` = the built-in default (all
/// clients); `Some(set)` = restrict to exactly this set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientSelection(Option<BTreeSet<SelectedClient>>);

impl ClientSelection {
    /// The default selection: every client.
    pub fn all() -> Self {
        Self(None)
    }

    /// Restrict to `clients`. An empty slice yields the all-clients default,
    /// so "no flags set" and "empty config list" both mean all.
    pub fn restrict(clients: &[SelectedClient]) -> Self {
        if clients.is_empty() {
            Self(None)
        } else {
            Self(Some(clients.iter().copied().collect()))
        }
    }

    /// True when this selection is the built-in default (all clients).
    pub fn is_all(&self) -> bool {
        self.0.is_none()
    }

    /// True when generation client `c` is targeted. copilot and vscode both
    /// target [`Client::Copilot`], so the shared `.github/**` tree renders
    /// once regardless of which (or both) are selected.
    pub fn targets_generation(&self, c: Client) -> bool {
        match &self.0 {
            None => true,
            Some(set) => set.iter().any(|s| s.generation() == c),
        }
    }

    /// True when MCP target `t` is configured by this selection.
    pub fn targets_mcp(&self, t: McpTarget) -> bool {
        match &self.0 {
            None => true,
            Some(set) => set.iter().any(|s| s.mcp() == t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_round_trips_all_names() {
        for c in SelectedClient::ALL {
            assert_eq!(c.name().parse::<SelectedClient>().unwrap(), c);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!("cursor".parse::<SelectedClient>().is_err());
    }

    #[test]
    fn vscode_maps_to_copilot_generation() {
        assert_eq!(SelectedClient::VsCode.generation(), Client::Copilot);
        assert_eq!(SelectedClient::Copilot.generation(), Client::Copilot);
        assert_eq!(SelectedClient::Claude.generation(), Client::Claude);
        assert_eq!(SelectedClient::OpenCode.generation(), Client::OpenCode);
    }

    #[test]
    fn mcp_mapping_is_one_to_one() {
        assert_eq!(SelectedClient::VsCode.mcp(), McpTarget::VsCode);
        assert_eq!(SelectedClient::Copilot.mcp(), McpTarget::Copilot);
    }

    #[test]
    fn default_and_empty_restrict_target_everything() {
        for sel in [ClientSelection::all(), ClientSelection::restrict(&[])] {
            assert!(sel.is_all());
            for c in Client::ALL {
                assert!(sel.targets_generation(c));
            }
            for t in McpTarget::ALL {
                assert!(sel.targets_mcp(t));
            }
        }
    }

    #[test]
    fn claude_only_excludes_other_generation_clients() {
        let sel = ClientSelection::restrict(&[SelectedClient::Claude]);
        assert!(sel.targets_generation(Client::Claude));
        assert!(!sel.targets_generation(Client::Copilot));
        assert!(!sel.targets_generation(Client::OpenCode));
    }

    #[test]
    fn copilot_and_vscode_share_one_generation_client() {
        // Both select the Copilot generation tree; nothing else.
        let sel = ClientSelection::restrict(&[SelectedClient::Copilot, SelectedClient::VsCode]);
        assert!(sel.targets_generation(Client::Copilot));
        assert!(!sel.targets_generation(Client::Claude));
        assert!(!sel.targets_generation(Client::OpenCode));
        // But MCP forks into two distinct targets.
        assert!(sel.targets_mcp(McpTarget::Copilot));
        assert!(sel.targets_mcp(McpTarget::VsCode));
        assert!(!sel.targets_mcp(McpTarget::Claude));
    }

    #[test]
    fn vscode_only_targets_copilot_generation_but_only_vscode_mcp() {
        let sel = ClientSelection::restrict(&[SelectedClient::VsCode]);
        assert!(sel.targets_generation(Client::Copilot));
        assert!(sel.targets_mcp(McpTarget::VsCode));
        assert!(!sel.targets_mcp(McpTarget::Copilot));
    }
}
