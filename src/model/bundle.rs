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

    /// Client-native plugins installed via shellout (ADR-0008, §3.7).
    /// Map key is the upskill-level plugin name; value carries per-client
    /// install descriptors.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub plugins: BTreeMap<String, PluginEntry>,

    /// MCP servers configured into each targeted client (ADR-0010, §3.7).
    /// Map key is the upskill-level MCP name; value carries the transport
    /// descriptor and declared required env vars.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mcps: BTreeMap<String, McpEntry>,

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

/// Per-plugin entry in the `plugins:` map. Contains optional descriptors
/// for each supported client. A plugin MAY target a single client, a
/// subset, or all of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Claude Code plugin descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<ClaudePluginDescriptor>,

    /// GitHub Copilot CLI plugin descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copilot: Option<CopilotPluginDescriptor>,

    /// VS Code extension descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vscode: Option<VscodePluginDescriptor>,

    /// opencode module descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode: Option<OpencodePluginDescriptor>,
}

/// Descriptor for Claude Code plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudePluginDescriptor {
    /// Install via `claude plugin marketplace add` + `claude plugin install`.
    Install {
        /// Marketplace source (passed to `claude plugin marketplace add`):
        /// `owner/repo`, a URL, or a path.
        source: String,
        /// Marketplace name used in the install ref `<plugin>@<marketplace>`.
        /// `claude` derives this name from the marketplace manifest, so it is
        /// distinct from `source` (e.g. source `anthropics/claude-plugins`,
        /// marketplace `claude-plugins`).
        marketplace: String,
        /// Plugin identifier (passed to `claude plugin install`).
        plugin: String,
        /// URL shown in warn-skip message when CLI not found.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Manual instructions — user must follow a URL to install.
    Instructions {
        /// URL with installation instructions.
        instructions_url: String,
        /// Optional human-readable summary shown to the user.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}

/// Descriptor for GitHub Copilot CLI plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CopilotPluginDescriptor {
    /// Install via `copilot plugin marketplace add` + `copilot plugin install`.
    Install {
        /// Marketplace source (passed to `copilot plugin marketplace add`):
        /// `owner/repo`, a URL, or a path.
        source: String,
        /// Marketplace name used in the install ref `<plugin>@<marketplace>`,
        /// distinct from `source` (the CLI derives the name from the
        /// marketplace manifest).
        marketplace: String,
        /// Plugin identifier (passed to `copilot plugin install`).
        plugin: String,
        /// URL shown in warn-skip message when CLI not found.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Manual instructions — user must follow a URL to install.
    Instructions {
        /// URL with installation instructions.
        instructions_url: String,
        /// Optional human-readable summary shown to the user.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}

/// Descriptor for VS Code extensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VscodePluginDescriptor {
    /// Install via `code --install-extension`.
    Install {
        /// Extension ID (passed to `code --install-extension`).
        extension: String,
        /// URL shown in warn-skip message when CLI not found.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Manual instructions — user must follow a URL to install.
    Instructions {
        /// URL with installation instructions.
        instructions_url: String,
        /// Optional human-readable summary shown to the user.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}

/// Descriptor for opencode modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpencodePluginDescriptor {
    /// Install via `opencode plugin`.
    Install {
        /// Module name (passed to `opencode plugin`).
        module: String,
        /// URL shown in warn-skip message when CLI not found.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Manual instructions — user must follow a URL to install.
    Instructions {
        /// URL with installation instructions.
        instructions_url: String,
        /// Optional human-readable summary shown to the user.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
    /// Config-write mode — write a plugin URI into opencode config.
    ConfigWrite {
        /// Plugin URI to write into configuration.
        plugin_uri: String,
        /// URL shown in warn-skip message when CLI not found.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
}

/// One entry in the bundle `mcps:` map. Exactly one transport (`remote`
/// or `local`) is present; `validate_mcps` enforces this at parse time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpEntry {
    /// Transport descriptor — flattened so YAML carries either a
    /// `remote:` or a `local:` key directly under the server name.
    #[serde(flatten)]
    pub transport: McpTransport,

    /// Environment variables the server requires. Declared (not valued) so
    /// `upskill doctor` can warn when one is unset. upskill never reads the
    /// values — secret custody stays with the user's environment.
    #[serde(
        default,
        rename = "requires-env",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub requires_env: Vec<String>,
}

/// MCP transport: a hosted server reached by URL, or a local process the
/// client spawns and speaks to over stdio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum McpTransport {
    #[serde(rename = "remote")]
    Remote(McpRemote),
    #[serde(rename = "local")]
    Local(McpLocal),
}

/// Remote (hosted) MCP server descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpRemote {
    /// `http` or `sse`.
    #[serde(rename = "type")]
    pub transport_type: String,
    /// Endpoint URL the client connects to.
    pub url: String,
    /// Optional headers. Values pass through verbatim (use `${VAR}`
    /// references for secrets — upskill never expands them).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

impl Bundle {
    /// Validate the `mcps:` map beyond what serde enforces. Returns the
    /// first offending `(server-name, reason)` as an error.
    pub fn validate_mcps(&self) -> Result<(), String> {
        for (name, entry) in &self.mcps {
            match &entry.transport {
                McpTransport::Remote(r) => {
                    if r.transport_type != "http" && r.transport_type != "sse" {
                        return Err(format!(
                            "mcp `{name}`: transport type `{}` is not one of http, sse",
                            r.transport_type
                        ));
                    }
                    if r.url.trim().is_empty() {
                        return Err(format!("mcp `{name}`: remote url must not be empty"));
                    }
                }
                McpTransport::Local(l) => {
                    if l.command.trim().is_empty() {
                        return Err(format!("mcp `{name}`: local command must not be empty"));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Local (stdio) MCP server descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpLocal {
    /// Launcher command (e.g. `npx`, `uvx`, `docker`, or a bare binary).
    pub command: String,
    /// Arguments passed to the command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables. Values pass through verbatim (use `${VAR}`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}
