//! Lockfile (`.upskill-lock.json`, `schema: 1`).
//!
//! Per [ADR-0003](../../docs/adr/0003-generation-pipeline.md). Records what
//! the pipeline installed into a consumer project so a future `update` /
//! `remove` / `doctor` can reason about state.
//!
//! Scope:
//! - Types + load/save/upsert for `schema: 1` items and (placeholder) bundles.
//! - Wiring into [`crate::pipeline::install_with_lockfile`] so the lockfile
//!   is written after a successful install.
//! - Bundle entries are part of the schema but not yet populated (Phase 3
//!   bundle slice).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::pipeline::{InstallReport, ItemKind};

/// On-disk filename.
pub const LOCKFILE_NAME: &str = ".upskill-lock.json";

/// The schema version this module produces.
pub const CURRENT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lockfile {
    pub schema: u32,
    #[serde(default)]
    pub items: Vec<LockedItem>,
    #[serde(default)]
    pub bundles: Vec<LockedBundle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<LockedPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedItem {
    /// `rule` | `skill` | `agent`.
    pub kind: ItemKind,
    pub name: String,
    /// Canonical source label (see [`crate::source::InstallSource`]'s
    /// `Display` impl).
    pub source: String,
    /// Resolved git ref when the source pinned one (branch, tag, SHA).
    /// Absent for local-path sources or unpinned remotes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// SHA-256 of the SSOT item directory at install time. Used by future
    /// `update` / `doctor` for drift detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Original item name in the source registry. Only present when the
    /// consumer installed with `--as <alias>` so `name` differs from the
    /// SSOT name. Used by `update` to locate the correct source file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
}

/// Bundle entry recorded when an install resolves a `.bundle.yaml` file
/// (the entry bundle and every transitive `requires`). The lockfile's
/// per-item `items` array still carries each rule/skill/agent
/// independently — this entry is metadata that pairs each bundle name
/// to the items it resolved to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedBundle {
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// Item names declared by THIS bundle (not the transitive closure).
    /// Resolution flattens the closure into `items`; the per-bundle
    /// breakdown lives here so a future `remove <bundle>` can subtract
    /// only that bundle's contribution.
    #[serde(default)]
    pub items: Vec<String>,
}

/// Plugin entry recorded when a bundle's `plugins:` map is installed via
/// client CLI shellout (ADR-0008). One entry per (plugin-name, client)
/// pair so `remove`/`update`/`doctor` can invoke the inverse CLI command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedPlugin {
    /// Upskill-level plugin name (key in the bundle's `plugins:` map).
    pub name: String,
    /// Client identifier: `"claude"`, `"vscode"`, or `"opencode"`.
    pub client: String,
    /// Client-specific identifier used for uninstall:
    /// - claude: `"<plugin>@<source>"`
    /// - vscode: `"<extension-id>"`
    /// - opencode: `"<module>"`
    pub identifier: String,
    /// Install scope (only meaningful for claude: `"project"` or `"user"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Bundle that declared this plugin.
    pub bundle: String,
    /// Install outcome recorded at `upskill add` time.
    /// `installed` — CLI shellout succeeded.
    /// `skipped`   — CLI was not on PATH (warn-skip); doctor surfaces these.
    /// Defaults to `installed` for backward-compatible lockfiles without
    /// this field (pre-v0.6.x entries were always successful installs).
    #[serde(default)]
    pub status: PluginInstallStatus,
}

/// Install status of a plugin entry in the lockfile (ADR-0008 / issue #151).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginInstallStatus {
    /// Plugin was successfully installed via the client CLI.
    #[default]
    Installed,
    /// Plugin was not installed because the client CLI was not on PATH at
    /// install time (warn-skip outcome).  `upskill doctor` surfaces these
    /// so the user can install the CLI and re-run `upskill update`.
    Skipped,
    /// Plugin has manual installation instructions — user must follow a URL.
    Instructions,
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            schema: CURRENT_SCHEMA,
            items: Vec::new(),
            bundles: Vec::new(),
            plugins: Vec::new(),
        }
    }

    /// Load `<project_root>/.upskill-lock.json`. Returns an empty `schema: 1`
    /// lockfile when the file does not exist.
    ///
    /// Errors:
    /// - File exists but does not parse as a `schema: 1` lockfile.
    /// - File parses but its `schema` is greater than the version this
    ///   implementation supports.
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = lockfile_path(project_root);
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
        };

        let parsed: Self =
            serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;

        if parsed.schema > CURRENT_SCHEMA {
            anyhow::bail!(
                "{}: schema {} is newer than this implementation supports \
                 (max {}); upgrade upskill",
                path.display(),
                parsed.schema,
                CURRENT_SCHEMA
            );
        }
        if parsed.schema != CURRENT_SCHEMA {
            anyhow::bail!(
                "{}: unsupported schema {} (expected {})",
                path.display(),
                parsed.schema,
                CURRENT_SCHEMA
            );
        }
        Ok(parsed)
    }

    /// Add or replace by `(kind, name)`. Items are kept sorted for
    /// deterministic on-disk output.
    pub fn upsert(&mut self, item: LockedItem) {
        self.items
            .retain(|existing| !(existing.kind == item.kind && existing.name == item.name));
        self.items.push(item);
        self.items.sort();
    }

    /// Remove by `(kind, name)`.
    pub fn remove(&mut self, kind: ItemKind, name: &str) {
        self.items
            .retain(|existing| !(existing.kind == kind && existing.name == name));
    }

    /// Add or replace a bundle entry by `name`. Bundles are kept sorted
    /// for deterministic on-disk output.
    pub fn upsert_bundle(&mut self, bundle: LockedBundle) {
        self.bundles.retain(|existing| existing.name != bundle.name);
        self.bundles.push(bundle);
        self.bundles.sort();
    }

    /// Add or replace a plugin entry by `(name, client)`. Plugins are
    /// kept sorted for deterministic on-disk output.
    pub fn upsert_plugin(&mut self, plugin: LockedPlugin) {
        self.plugins
            .retain(|existing| !(existing.name == plugin.name && existing.client == plugin.client));
        self.plugins.push(plugin);
        self.plugins.sort();
    }

    /// Remove all plugin entries matching `name` (across all clients).
    pub fn remove_plugins_by_name(&mut self, name: &str) {
        self.plugins.retain(|existing| existing.name != name);
    }

    /// Persist to `<project_root>/.upskill-lock.json`. Pretty-printed JSON
    /// with a trailing newline so editors don't fight us.
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let path = lockfile_path(project_root);
        let json = serde_json::to_string_pretty(self).context("serialize lockfile")?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, format!("{json}\n"))
            .with_context(|| format!("write {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &path)
            .with_context(|| format!("rename {} to {}", tmp_path.display(), path.display()))
    }
}

fn lockfile_path(project_root: &Path) -> PathBuf {
    project_root.join(LOCKFILE_NAME)
}

/// Project an [`InstallReport`] into one [`LockedItem`] per unique `(kind,
/// name)` (the report has one entry per kind × name × client; the lockfile
/// records each item once).
///
/// `source_label` is taken verbatim from the caller — typically the
/// `Display` form of the [`InstallSource`] or the original CLI string.
/// `git_ref` and `hash` are looked up by the caller (the pipeline) per
/// item; threading them through here keeps the lockfile module free of
/// filesystem and source concerns.
pub fn items_from_report(
    report: &InstallReport,
    source_label: &str,
    git_ref: Option<&str>,
    mut hash_for: impl FnMut(ItemKind, &str) -> Option<String>,
) -> Vec<LockedItem> {
    use std::collections::BTreeSet;
    let mut seen: BTreeSet<(ItemKind, String)> = BTreeSet::new();
    let mut out = Vec::new();
    for entry in &report.items {
        if !seen.insert((entry.kind, entry.name.clone())) {
            continue;
        }
        out.push(LockedItem {
            kind: entry.kind,
            name: entry.name.clone(),
            source: source_label.to_string(),
            git_ref: git_ref.map(str::to_string),
            hash: hash_for(entry.kind, &entry.name),
            source_name: None,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Client;
    use crate::pipeline::InstalledItem;
    use std::path::PathBuf;

    fn item(kind: ItemKind, name: &str) -> LockedItem {
        LockedItem {
            kind,
            name: name.to_string(),
            source: "github:driftsys/skills".to_string(),
            git_ref: Some("v1.0.0".to_string()),
            hash: Some("sha256:deadbeef".to_string()),
            source_name: None,
        }
    }

    #[test]
    fn new_lockfile_has_current_schema() {
        let lock = Lockfile::new();
        assert_eq!(lock.schema, CURRENT_SCHEMA);
        assert!(lock.items.is_empty());
        assert!(lock.bundles.is_empty());
    }

    #[test]
    fn upsert_replaces_existing_item_with_same_kind_and_name() {
        let mut lock = Lockfile::new();
        lock.upsert(item(ItemKind::Skill, "code-review"));
        let mut updated = item(ItemKind::Skill, "code-review");
        updated.git_ref = Some("v2.0.0".to_string());
        lock.upsert(updated);
        assert_eq!(lock.items.len(), 1);
        assert_eq!(lock.items[0].git_ref, Some("v2.0.0".to_string()));
    }

    #[test]
    fn upsert_keeps_distinct_kinds_separate() {
        // A rule and a skill with the same name MUST coexist.
        let mut lock = Lockfile::new();
        lock.upsert(item(ItemKind::Rule, "shared-name"));
        lock.upsert(item(ItemKind::Skill, "shared-name"));
        assert_eq!(lock.items.len(), 2);
    }

    #[test]
    fn items_are_sorted_for_deterministic_output() {
        let mut lock = Lockfile::new();
        lock.upsert(item(ItemKind::Skill, "z"));
        lock.upsert(item(ItemKind::Skill, "a"));
        let names: Vec<_> = lock.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["a", "z"]);
    }

    #[test]
    fn remove_filters_by_kind_and_name() {
        let mut lock = Lockfile::new();
        lock.upsert(item(ItemKind::Rule, "shared-name"));
        lock.upsert(item(ItemKind::Skill, "shared-name"));
        lock.remove(ItemKind::Rule, "shared-name");
        assert_eq!(lock.items.len(), 1);
        assert_eq!(lock.items[0].kind, ItemKind::Skill);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.upsert(item(ItemKind::Skill, "code-review"));
        lock.save(tmp.path()).expect("save");

        let loaded = Lockfile::load(tmp.path()).expect("load");
        assert_eq!(loaded, lock);
    }

    #[test]
    fn save_is_atomic_no_tmp_file_remains() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.upsert(item(ItemKind::Skill, "test-atomic"));
        lock.save(tmp.path()).expect("save");

        // The .tmp file must not persist after a successful save.
        let tmp_path = tmp.path().join(".upskill-lock.json.tmp");
        assert!(
            !tmp_path.exists(),
            ".tmp file should not persist after save"
        );
        // The actual lockfile must exist.
        assert!(tmp.path().join(LOCKFILE_NAME).exists());
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let lock = Lockfile::load(tmp.path()).expect("load");
        assert_eq!(lock, Lockfile::new());
    }

    #[test]
    fn load_rejects_newer_schema() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(LOCKFILE_NAME),
            r#"{"schema": 99, "items": [], "bundles": []}"#,
        )
        .unwrap();
        let err = Lockfile::load(tmp.path()).expect_err("must reject");
        assert!(err.to_string().contains("schema 99"));
    }

    #[test]
    fn load_rejects_unrecognised_shape() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(LOCKFILE_NAME), r#"{"random": "shape"}"#).unwrap();
        Lockfile::load(tmp.path()).expect_err("must reject");
    }

    #[test]
    fn items_from_report_dedupes_per_kind_name() {
        // Build a report with one skill installed for all three clients —
        // the lockfile should record it as one item, not three.
        let report = InstallReport {
            bundles: Vec::new(),
            plugin_results: Vec::new(),
            items: vec![
                InstalledItem {
                    kind: ItemKind::Skill,
                    name: "code-review".into(),
                    client: Client::Claude,
                    output_path: PathBuf::from(".claude/skills/code-review/SKILL.md"),
                    source_hash: Some("sha256:abc".into()),
                },
                InstalledItem {
                    kind: ItemKind::Skill,
                    name: "code-review".into(),
                    client: Client::Copilot,
                    output_path: PathBuf::from(".github/skills/code-review/SKILL.md"),
                    source_hash: Some("sha256:abc".into()),
                },
                InstalledItem {
                    kind: ItemKind::Skill,
                    name: "code-review".into(),
                    client: Client::OpenCode,
                    output_path: PathBuf::from(".agents/skills/code-review/SKILL.md"),
                    source_hash: Some("sha256:abc".into()),
                },
            ],
        };

        let items = items_from_report(&report, "local:./src", None, |_, _| {
            Some("sha256:abc".into())
        });
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Skill);
        assert_eq!(items[0].name, "code-review");
        assert_eq!(items[0].source, "local:./src");
        assert_eq!(items[0].hash, Some("sha256:abc".into()));
    }

    #[test]
    fn upsert_plugin_adds_new_entry() {
        let mut lock = Lockfile::new();
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "claude".into(),
            identifier: "superpowers@anthropics/claude-plugins".into(),
            scope: Some("project".into()),
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        assert_eq!(lock.plugins.len(), 1);
        assert_eq!(lock.plugins[0].name, "superpowers");
    }

    #[test]
    fn upsert_plugin_replaces_by_name_and_client() {
        let mut lock = Lockfile::new();
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "claude".into(),
            identifier: "superpowers@old-source".into(),
            scope: Some("project".into()),
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "claude".into(),
            identifier: "superpowers@new-source".into(),
            scope: Some("user".into()),
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        assert_eq!(lock.plugins.len(), 1);
        assert_eq!(lock.plugins[0].identifier, "superpowers@new-source");
    }

    #[test]
    fn upsert_plugin_keeps_different_clients_separate() {
        let mut lock = Lockfile::new();
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "claude".into(),
            identifier: "superpowers@src".into(),
            scope: Some("project".into()),
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "vscode".into(),
            identifier: "anthropic.superpowers".into(),
            scope: None,
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        assert_eq!(lock.plugins.len(), 2);
    }

    #[test]
    fn remove_plugins_by_name_removes_all_clients() {
        let mut lock = Lockfile::new();
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "claude".into(),
            identifier: "superpowers@src".into(),
            scope: Some("project".into()),
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "vscode".into(),
            identifier: "anthropic.superpowers".into(),
            scope: None,
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        lock.remove_plugins_by_name("superpowers");
        assert!(lock.plugins.is_empty());
    }

    #[test]
    fn plugins_roundtrip_through_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "claude".into(),
            identifier: "superpowers@anthropics/claude-plugins".into(),
            scope: Some("project".into()),
            bundle: "baseline".into(),
            status: PluginInstallStatus::Installed,
        });
        lock.save(tmp.path()).expect("save");
        let loaded = Lockfile::load(tmp.path()).expect("load");
        assert_eq!(loaded.plugins, lock.plugins);
    }

    #[test]
    fn skipped_plugin_roundtrips_through_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = Lockfile::new();
        lock.upsert_plugin(LockedPlugin {
            name: "superpowers".into(),
            client: "vscode".into(),
            identifier: "anthropic.superpowers".into(),
            scope: None,
            bundle: "baseline".into(),
            status: PluginInstallStatus::Skipped,
        });
        lock.save(tmp.path()).expect("save");
        let loaded = Lockfile::load(tmp.path()).expect("load");
        assert_eq!(loaded.plugins[0].status, PluginInstallStatus::Skipped);
    }

    #[test]
    fn plugin_without_status_field_defaults_to_installed_on_load() {
        let tmp = tempfile::tempdir().unwrap();
        // Simulate a pre-v0.6.x lockfile that has plugins but no status field.
        std::fs::write(
            tmp.path().join(LOCKFILE_NAME),
            r#"{"schema": 1, "items": [], "bundles": [], "plugins": [{"name": "sp", "client": "claude", "identifier": "sp@src", "bundle": "b"}]}"#,
        )
        .unwrap();
        let lock = Lockfile::load(tmp.path()).expect("must parse");
        assert_eq!(lock.plugins[0].status, PluginInstallStatus::Installed);
    }

    #[test]
    fn locked_item_serializes_source_name_when_present() {
        let item = LockedItem {
            kind: ItemKind::Skill,
            name: "brainstorming-v2".to_string(),
            source: "github:other-org/repo".to_string(),
            git_ref: None,
            hash: None,
            source_name: Some("brainstorming".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"source_name\":\"brainstorming\""), "{json}");
    }

    #[test]
    fn locked_item_omits_source_name_when_none() {
        let item = LockedItem {
            kind: ItemKind::Skill,
            name: "brainstorming".to_string(),
            source: "github:driftsys/superpowers".to_string(),
            git_ref: None,
            hash: None,
            source_name: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("source_name"), "{json}");
    }

    #[test]
    fn existing_lockfile_without_plugins_field_loads_fine() {
        let tmp = tempfile::tempdir().unwrap();
        // Simulates a lockfile from before the plugins field existed.
        std::fs::write(
            tmp.path().join(LOCKFILE_NAME),
            r#"{"schema": 1, "items": [], "bundles": []}"#,
        )
        .unwrap();
        let lock = Lockfile::load(tmp.path()).expect("must parse");
        assert!(lock.plugins.is_empty());
    }
}
