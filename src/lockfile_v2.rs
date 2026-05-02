//! v0.2 lockfile (`.upskill-lock.json`, `schema: 2`).
//!
//! Per [ADR-0003](../../docs/adr/0003-generation-pipeline.md). Records what
//! the pipeline installed into a consumer project so a future `update` /
//! `remove` / `doctor` can reason about state. The filename is unchanged
//! from v0.1; the `schema:` field discriminates.
//!
//! Scope:
//! - Types + load/save/upsert for `schema: 2` items and (placeholder) bundles.
//! - Wiring into [`crate::pipeline::install_with_lockfile`] so the lockfile
//!   is written after a successful install.
//! - In-place v0.1 → v0.2 migration on first load: a v0.1
//!   `.upskill-lock.json` (no `schema` field, top-level `skills` array) is
//!   detected, translated to the v0.2 shape with `kind: "skill"` per entry,
//!   and rewritten in place. Subsequent loads see schema 2 directly.
//! - Bundle entries are part of the schema but not yet populated (Phase 3
//!   bundle slice).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::pipeline::{InstallReport, ItemKind};

/// On-disk filename. Same as v0.1 — `schema` discriminates the shape.
pub const LOCKFILE_NAME: &str = ".upskill-lock.json";

/// The schema version this module produces.
pub const CURRENT_SCHEMA: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockfileV2 {
    pub schema: u32,
    #[serde(default)]
    pub items: Vec<LockedItem>,
    #[serde(default)]
    pub bundles: Vec<LockedBundle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedItem {
    /// `rule` | `skill` | `agent`.
    pub kind: String,
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
}

/// Placeholder for the bundle-install slice. Schema is finalised so the
/// shape is forward-compatible; no bundle entries are produced today.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedBundle {
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// Item names this bundle resolved to.
    #[serde(default)]
    pub items: Vec<String>,
}

impl Default for LockfileV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl LockfileV2 {
    pub fn new() -> Self {
        Self {
            schema: CURRENT_SCHEMA,
            items: Vec::new(),
            bundles: Vec::new(),
        }
    }

    /// Load `<project_root>/.upskill-lock.json`. Returns an empty `schema: 2`
    /// lockfile when the file does not exist.
    ///
    /// Migration: a v0.1-shape file (no `schema` field, top-level `skills`
    /// array) is detected, translated in memory to schema 2, and rewritten
    /// in place. Subsequent loads see the v0.2 shape directly. Existing v0.1
    /// users are not silently broken.
    ///
    /// Errors:
    /// - File exists but is neither v0.2 nor recognisable v0.1 JSON.
    /// - File parses as v0.2 but its `schema` is greater than the version
    ///   this implementation supports.
    pub fn load(project_root: &Path) -> Result<Self> {
        let path = lockfile_path(project_root);
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
        };

        if let Ok(parsed) = serde_json::from_str::<Self>(&raw) {
            if parsed.schema == CURRENT_SCHEMA {
                return Ok(parsed);
            }
            if parsed.schema > CURRENT_SCHEMA {
                anyhow::bail!(
                    "{}: schema {} is newer than this implementation supports \
                     (max {}); upgrade upskill",
                    path.display(),
                    parsed.schema,
                    CURRENT_SCHEMA
                );
            }
            // schema < CURRENT_SCHEMA but with a `schema` field is unexpected
            // (v0.1 had no schema field) — fall through to the v0.1 attempt.
        }

        if let Ok(v1) = serde_json::from_str::<V1Lockfile>(&raw) {
            let migrated = migrate_v1(&v1);
            migrated
                .save(project_root)
                .with_context(|| format!("persist v0.1 → v0.2 migration of {}", path.display()))?;
            return Ok(migrated);
        }

        anyhow::bail!(
            "{}: unrecognised lockfile shape (neither v0.2 schema-2 nor v0.1)",
            path.display()
        );
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
    pub fn remove(&mut self, kind: &str, name: &str) {
        self.items
            .retain(|existing| !(existing.kind == kind && existing.name == name));
    }

    /// Persist to `<project_root>/.upskill-lock.json`. Pretty-printed JSON
    /// with a trailing newline so editors don't fight us.
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let path = lockfile_path(project_root);
        let json = serde_json::to_string_pretty(self).context("serialize lockfile")?;
        std::fs::write(&path, format!("{json}\n"))
            .with_context(|| format!("write {}", path.display()))
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
            kind: kind_label(entry.kind).to_string(),
            name: entry.name.clone(),
            source: source_label.to_string(),
            git_ref: git_ref.map(str::to_string),
            hash: hash_for(entry.kind, &entry.name),
        });
    }
    out
}

fn kind_label(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Rule => "rule",
        ItemKind::Skill => "skill",
        ItemKind::Agent => "agent",
    }
}

/// In-memory representation of the v0.1 `.upskill-lock.json` shape, used
/// only by the migration path in [`LockfileV2::load`]. Kept private — the
/// v0.1 lockfile module is gone, and this struct exists solely so we can
/// continue to read v0.1 files written by the previous binary.
#[derive(Debug, Deserialize)]
struct V1Lockfile {
    skills: Vec<V1LockedSkill>,
}

#[derive(Debug, Deserialize)]
struct V1LockedSkill {
    name: String,
    source: String,
    #[serde(default, rename = "ref")]
    git_ref: Option<String>,
    #[serde(default)]
    hash: Option<String>,
}

/// Translate a v0.1 lockfile (skills-only) into a v0.2 lockfile. Every
/// v0.1 entry becomes a `kind: "skill"` item; v0.2-only fields (bundles)
/// stay empty.
fn migrate_v1(v1: &V1Lockfile) -> LockfileV2 {
    let mut out = LockfileV2::new();
    for s in &v1.skills {
        out.upsert(LockedItem {
            kind: "skill".to_string(),
            name: s.name.clone(),
            source: s.source.clone(),
            git_ref: s.git_ref.clone(),
            hash: s.hash.clone(),
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

    fn item(kind: &str, name: &str) -> LockedItem {
        LockedItem {
            kind: kind.to_string(),
            name: name.to_string(),
            source: "github:driftsys/skills".to_string(),
            git_ref: Some("v1.0.0".to_string()),
            hash: Some("sha256:deadbeef".to_string()),
        }
    }

    #[test]
    fn new_lockfile_has_current_schema() {
        let lock = LockfileV2::new();
        assert_eq!(lock.schema, CURRENT_SCHEMA);
        assert!(lock.items.is_empty());
        assert!(lock.bundles.is_empty());
    }

    #[test]
    fn upsert_replaces_existing_item_with_same_kind_and_name() {
        let mut lock = LockfileV2::new();
        lock.upsert(item("skill", "code-review"));
        let mut updated = item("skill", "code-review");
        updated.git_ref = Some("v2.0.0".to_string());
        lock.upsert(updated);
        assert_eq!(lock.items.len(), 1);
        assert_eq!(lock.items[0].git_ref, Some("v2.0.0".to_string()));
    }

    #[test]
    fn upsert_keeps_distinct_kinds_separate() {
        // A rule and a skill with the same name MUST coexist.
        let mut lock = LockfileV2::new();
        lock.upsert(item("rule", "shared-name"));
        lock.upsert(item("skill", "shared-name"));
        assert_eq!(lock.items.len(), 2);
    }

    #[test]
    fn items_are_sorted_for_deterministic_output() {
        let mut lock = LockfileV2::new();
        lock.upsert(item("skill", "z"));
        lock.upsert(item("skill", "a"));
        let names: Vec<_> = lock.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["a", "z"]);
    }

    #[test]
    fn remove_filters_by_kind_and_name() {
        let mut lock = LockfileV2::new();
        lock.upsert(item("rule", "shared-name"));
        lock.upsert(item("skill", "shared-name"));
        lock.remove("rule", "shared-name");
        assert_eq!(lock.items.len(), 1);
        assert_eq!(lock.items[0].kind, "skill");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut lock = LockfileV2::new();
        lock.upsert(item("skill", "code-review"));
        lock.save(tmp.path()).expect("save");

        let loaded = LockfileV2::load(tmp.path()).expect("load");
        assert_eq!(loaded, lock);
    }

    #[test]
    fn load_missing_file_returns_empty_v2() {
        let tmp = tempfile::tempdir().unwrap();
        let lock = LockfileV2::load(tmp.path()).expect("load");
        assert_eq!(lock, LockfileV2::new());
    }

    #[test]
    fn load_rejects_newer_schema() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(LOCKFILE_NAME),
            r#"{"schema": 99, "items": [], "bundles": []}"#,
        )
        .unwrap();
        let err = LockfileV2::load(tmp.path()).expect_err("must reject");
        assert!(err.to_string().contains("schema 99"));
    }

    #[test]
    fn load_migrates_v1_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(LOCKFILE_NAME);
        // v0.1 shape: no `schema` field, top-level `skills` array.
        std::fs::write(
            &path,
            r#"{
                "skills": [
                    {"name": "code-review", "source": "github:driftsys/skills@v1.0",
                     "ref": "v1.0", "hash": "abc"}
                ]
            }"#,
        )
        .unwrap();

        let loaded = LockfileV2::load(tmp.path()).expect("load with migration");
        assert_eq!(loaded.schema, CURRENT_SCHEMA);
        assert_eq!(loaded.items.len(), 1);
        let it = &loaded.items[0];
        assert_eq!(it.kind, "skill");
        assert_eq!(it.name, "code-review");
        assert_eq!(it.source, "github:driftsys/skills@v1.0");
        assert_eq!(it.git_ref, Some("v1.0".to_string()));
        assert_eq!(it.hash, Some("abc".to_string()));

        // Migration was persisted: the file now contains the schema field
        // and a second load no longer triggers migration.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(on_disk.contains("\"schema\""));
        let reloaded = LockfileV2::load(tmp.path()).expect("reload");
        assert_eq!(reloaded, loaded);
    }

    #[test]
    fn load_rejects_unrecognised_shape() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(LOCKFILE_NAME), r#"{"random": "shape"}"#).unwrap();
        let err = LockfileV2::load(tmp.path()).expect_err("must reject");
        assert!(err.to_string().contains("unrecognised"));
    }

    #[test]
    fn items_from_report_dedupes_per_kind_name() {
        // Build a report with one skill installed for all three clients —
        // the lockfile should record it as one item, not three.
        let report = InstallReport {
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
        assert_eq!(items[0].kind, "skill");
        assert_eq!(items[0].name, "code-review");
        assert_eq!(items[0].source, "local:./src");
        assert_eq!(items[0].hash, Some("sha256:abc".into()));
    }
}
