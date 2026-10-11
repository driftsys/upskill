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
use crate::target::TargetSurface;

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

    pub const fn surface(self) -> TargetSurface {
        match self {
            Self::Claude => TargetSurface::ClaudeCli,
            Self::Copilot => TargetSurface::CopilotCli,
            Self::VsCode => TargetSurface::CopilotVscodeLocal,
            Self::OpenCode => TargetSurface::OpenCodeCli,
        }
    }

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

/// A consumer's selected clients and whether the built-in default supplied
/// them. Keep the origin because `update` treats an explicit selection of
/// every current client differently from an unspecified selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSelection {
    clients: BTreeSet<SelectedClient>,
    implicit_default: bool,
}

impl Default for ClientSelection {
    fn default() -> Self {
        Self::all()
    }
}

impl ClientSelection {
    const DEFAULT_CLIENTS: [SelectedClient; 4] = [
        SelectedClient::Claude,
        SelectedClient::Copilot,
        SelectedClient::VsCode,
        SelectedClient::OpenCode,
    ];

    pub const fn default_clients() -> &'static [SelectedClient] {
        &Self::DEFAULT_CLIENTS
    }

    /// The original default selection, held constant as new clients are added.
    pub fn all() -> Self {
        Self {
            clients: Self::DEFAULT_CLIENTS.into_iter().collect(),
            implicit_default: true,
        }
    }

    /// Restrict to `clients`. An empty slice yields the original default,
    /// so "no flags set" and "empty config list" have the same selection.
    pub fn restrict(clients: &[SelectedClient]) -> Self {
        if clients.is_empty() {
            Self::all()
        } else {
            Self {
                clients: clients.iter().copied().collect(),
                implicit_default: false,
            }
        }
    }

    /// True when selection was implicit rather than explicitly requested.
    pub fn is_all(&self) -> bool {
        self.implicit_default
    }

    /// True when generation client `c` is targeted. copilot and vscode both
    /// target [`Client::Copilot`], so the shared `.github/**` tree renders
    /// once regardless of which (or both) are selected.
    pub fn targets_generation(&self, c: Client) -> bool {
        self.clients.iter().any(|s| s.generation() == c)
    }

    /// True when MCP target `t` is configured by this selection.
    pub fn targets_mcp(&self, t: McpTarget) -> bool {
        self.clients.iter().any(|s| s.mcp() == t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::TargetSurface;

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
    fn default_and_empty_restrict_use_original_targets() {
        for sel in [ClientSelection::all(), ClientSelection::restrict(&[])] {
            assert!(sel.is_all());
            for c in [Client::Claude, Client::Copilot, Client::OpenCode] {
                assert!(sel.targets_generation(c));
            }
            for t in [
                McpTarget::Claude,
                McpTarget::Copilot,
                McpTarget::VsCode,
                McpTarget::OpenCode,
            ] {
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

    #[test]
    fn default_selection_is_the_existing_four_surfaces() {
        let selected = ClientSelection::all();
        assert_eq!(
            selected.clients,
            ClientSelection::default_clients().iter().copied().collect()
        );
        assert_eq!(
            ClientSelection::default_clients()
                .iter()
                .map(|client| client.surface())
                .collect::<Vec<_>>(),
            vec![
                TargetSurface::ClaudeCli,
                TargetSurface::CopilotCli,
                TargetSurface::CopilotVscodeLocal,
                TargetSurface::OpenCodeCli,
            ]
        );
        assert!(
            !ClientSelection::default_clients()
                .iter()
                .any(|client| client.surface() == TargetSurface::CodexCli)
        );
        assert_eq!(ClientSelection::all(), ClientSelection::restrict(&[]));
        let explicit_all = ClientSelection::restrict(ClientSelection::default_clients());
        assert_ne!(selected, explicit_all);
        assert!(!explicit_all.is_all());
    }

    #[test]
    fn surface_ids_are_stable_and_new_surface_is_not_selectable() {
        assert_eq!(TargetSurface::CodexCli.name(), "codex.cli");
        assert_eq!(TargetSurface::CodexIde.name(), "codex.ide");
        assert_eq!(
            TargetSurface::CopilotVscodeLocal.name(),
            "copilot.vscode-local"
        );
        assert!("codex".parse::<SelectedClient>().is_err());
    }
}
