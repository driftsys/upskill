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
use std::collections::{BTreeMap, BTreeSet};

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

    /// LSP servers written into client plugins by `pack`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lsps: BTreeMap<String, LspEntry>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Pass-through for unknown top-level fields, mirrored from the
    /// item models (§3.3).
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml_ng::Value>,
}

/// A kind that `upskill pack` renders and `add` and `update` skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackOnlyKind {
    Command,
    LspServer,
}

impl PackOnlyKind {
    pub const ALL: [PackOnlyKind; 2] = [PackOnlyKind::Command, PackOnlyKind::LspServer];

    pub fn label(self) -> &'static str {
        match self {
            Self::Command => "command item(s)",
            Self::LspServer => "LSP server(s)",
        }
    }
}

impl Bundle {
    /// Names that `add` and `update` skip for this kind.
    pub fn pack_only(&self, kind: PackOnlyKind) -> Vec<&str> {
        match kind {
            PackOnlyKind::Command => self.items.commands.iter().map(String::as_str).collect(),
            PackOnlyKind::LspServer => self.lsps.keys().map(String::as_str).collect(),
        }
    }
}

/// Each `(kind, name)` of `bundles` once, sorted.
pub fn pack_only_items(bundles: &[Bundle]) -> Vec<(PackOnlyKind, String)> {
    bundles
        .iter()
        .flat_map(|bundle| {
            PackOnlyKind::ALL.into_iter().flat_map(|kind| {
                bundle
                    .pack_only(kind)
                    .into_iter()
                    .map(move |name| (kind, name.to_string()))
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<String>,
}

impl BundleItems {
    /// True when no rule, skill, agent, command, or hook is named.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
            && self.skills.is_empty()
            && self.agents.is_empty()
            && self.commands.is_empty()
            && self.hooks.is_empty()
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
    /// Cross-source locator (the `upskill add` source DSL — `owner/repo@ref`,
    /// https URLs, local paths). Absent means the required bundle is resolved
    /// in the same source registry as the requiring bundle. Mirrors the
    /// item-level `RequireRef::Detailed.source` (ADR-0009).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
    /// Install via `opencode plugin add` (opencode 2 or later).
    Install {
        /// Module name (passed to `opencode plugin add`).
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

    /// Validate the `lsps:` map and return the first invalid entry.
    pub fn validate_lsps(&self) -> Result<(), String> {
        for (name, entry) in &self.lsps {
            if entry.command.trim().is_empty() {
                return Err(format!("lsp `{name}`: command must not be empty"));
            }
            if entry.extensions.is_empty() {
                return Err(format!("lsp `{name}`: extensions must not be empty"));
            }
            for (extension, language) in &entry.extensions {
                if !extension.starts_with('.') {
                    return Err(format!(
                        "lsp `{name}`: extension `{extension}` must start with a dot"
                    ));
                }
                if extension == "." {
                    return Err(format!(
                        "lsp `{name}`: extension `.` must have at least one character after the dot"
                    ));
                }
                if language.trim().is_empty() {
                    return Err(format!(
                        "lsp `{name}`: the language of extension `{extension}` must not be empty"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// One entry in the bundle `lsps:` map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LspEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    pub extensions: BTreeMap<String, String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_only_items_lists_each_command_once_in_sorted_order() {
        let bundle = |name: &str, commands: &str| {
            serde_yaml_ng::from_str::<Bundle>(&format!(
                "schema: 1\nname: {name}\ndescription: Test\nitems:\n  commands: {commands}\n"
            ))
            .unwrap()
        };
        assert_eq!(
            pack_only_items(&[bundle("first", "[b, a]"), bundle("second", "[a]")]),
            vec![
                (PackOnlyKind::Command, "a".into()),
                (PackOnlyKind::Command, "b".into())
            ]
        );
    }

    #[test]
    fn pack_only_items_sorts_and_deduplicates_lsp_servers_by_kind_and_name() {
        let mut first = lsp_bundle("command: rust-analyzer\n    extensions: { .rs: rust }");
        let entry = first.lsps["rust-analyzer"].clone();
        first.lsps = BTreeMap::from([("z-server".to_string(), entry.clone())]);
        first.items.commands = vec!["rust-analyzer".into()];
        let mut second = first.clone();
        second.lsps = BTreeMap::from([("a-server".to_string(), entry)]);
        assert_eq!(
            pack_only_items(&[first, second]),
            vec![
                (PackOnlyKind::Command, "rust-analyzer".into()),
                (PackOnlyKind::LspServer, "a-server".into()),
                (PackOnlyKind::LspServer, "z-server".into())
            ]
        );
        assert_eq!(
            PackOnlyKind::ALL,
            [PackOnlyKind::Command, PackOnlyKind::LspServer]
        );
        assert_eq!(PackOnlyKind::Command.label(), "command item(s)");
        assert_eq!(PackOnlyKind::LspServer.label(), "LSP server(s)");
    }

    fn lsp_bundle(entry: &str) -> Bundle {
        serde_yaml_ng::from_str(&format!(
            "schema: 1\nname: lsp-demo\ndescription: d\nitems: {{}}\nlsps:\n  rust-analyzer:\n    {entry}\n"
        )).unwrap()
    }

    #[test]
    fn bundle_parses_lsps() {
        let bundle =
            lsp_bundle("command: rust-analyzer\n    args: []\n    extensions: { .rs: rust }");
        assert_eq!(bundle.lsps.len(), 1);
        let entry = &bundle.lsps["rust-analyzer"];
        assert_eq!(entry.command, "rust-analyzer");
        assert!(entry.args.is_empty());
        assert!(entry.env.is_empty());
        assert_eq!(
            entry.extensions,
            BTreeMap::from([(".rs".into(), "rust".into())])
        );
        assert_eq!(bundle.validate_lsps(), Ok(()));
        let serialized = serde_yaml_ng::to_value(&bundle).unwrap();
        assert!(serialized["lsps"]["rust-analyzer"].get("args").is_none());
        assert!(serialized["lsps"]["rust-analyzer"].get("env").is_none());
    }

    #[test]
    fn mcp_entries_accept_unknown_fields() {
        let bundle: Bundle = serde_yaml_ng::from_str(concat!(
            "schema: 1\nname: mcp-demo\ndescription: d\nitems: {}\nmcps:\n",
            "  loc:\n    local:\n      command: npx\n      bogus: 1\n",
            "  rem:\n    remote:\n      type: http\n      url: https://example.com/mcp\n      bogus: 1\n",
        ))
        .unwrap();
        assert_eq!(bundle.mcps.len(), 2);
        assert_eq!(bundle.validate_mcps(), Ok(()));
    }

    #[test]
    fn bundle_without_lsps_serializes_without_the_key() {
        let bundle: Bundle =
            serde_yaml_ng::from_str("schema: 1\nname: old\ndescription: d\nitems: {}\n").unwrap();
        assert!(bundle.lsps.is_empty());
        let serialized = serde_yaml_ng::to_value(&bundle).unwrap();
        assert!(serialized.get("lsps").is_none());
    }

    #[test]
    fn validate_lsps_reports_each_invalid_entry() {
        for (entry, expected) in [
            (
                "command: ''\n    extensions: { .rs: rust }",
                "lsp `rust-analyzer`: command must not be empty",
            ),
            (
                "command: '  '\n    extensions: { .rs: rust }",
                "lsp `rust-analyzer`: command must not be empty",
            ),
            (
                "command: rust-analyzer\n    extensions: {}",
                "lsp `rust-analyzer`: extensions must not be empty",
            ),
            (
                "command: rust-analyzer\n    extensions: { rs: rust }",
                "lsp `rust-analyzer`: extension `rs` must start with a dot",
            ),
            (
                "command: rust-analyzer\n    extensions: { .rs: '' }",
                "lsp `rust-analyzer`: the language of extension `.rs` must not be empty",
            ),
            (
                "command: rust-analyzer\n    extensions: { .rs: '  ' }",
                "lsp `rust-analyzer`: the language of extension `.rs` must not be empty",
            ),
        ] {
            assert_eq!(lsp_bundle(entry).validate_lsps(), Err(expected.into()));
        }
    }

    #[test]
    fn validate_lsps_rejects_a_dot_without_a_name() {
        assert_eq!(
            lsp_bundle("command: rust-analyzer\n    extensions: { .: rust }").validate_lsps(),
            Err(
                "lsp `rust-analyzer`: extension `.` must have at least one character after the dot"
                    .into()
            )
        );
    }

    #[test]
    fn validate_lsps_checks_every_server_and_extension() {
        let two_servers: Bundle = serde_yaml_ng::from_str(
            "schema: 1\nname: d\ndescription: d\nitems: {}\nlsps:\n  a:\n    command: x\n    extensions: { .a: l }\n  b:\n    command: y\n    extensions: { b: l }\n",
        )
        .unwrap();
        assert_eq!(
            two_servers.validate_lsps(),
            Err("lsp `b`: extension `b` must start with a dot".into())
        );
        assert_eq!(
            lsp_bundle("command: x\n    extensions: { .a: rust, b: rust }").validate_lsps(),
            Err("lsp `rust-analyzer`: extension `b` must start with a dot".into())
        );
        assert_eq!(
            lsp_bundle("command: x\n    extensions: { .a: rust, .b: '' }").validate_lsps(),
            Err("lsp `rust-analyzer`: the language of extension `.b` must not be empty".into())
        );
    }

    #[test]
    fn lsp_entry_requires_command_and_extensions() {
        let no_command =
            serde_yaml_ng::from_str::<LspEntry>("extensions: { .rs: rust }\n").unwrap_err();
        assert!(no_command.to_string().contains("missing field `command`"));
        let no_extensions = serde_yaml_ng::from_str::<LspEntry>("command: x\n").unwrap_err();
        assert!(
            no_extensions
                .to_string()
                .contains("missing field `extensions`")
        );
    }

    #[test]
    fn lsp_entry_refuses_an_unknown_key() {
        let error = serde_yaml_ng::from_str::<LspEntry>(
            "command: rust-analyzer\nextensions: { .rs: rust }\nextension: { .rs: rust }\n",
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `extension`"));
    }
}
