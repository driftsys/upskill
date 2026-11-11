//! Per-client output path computation, and the best-effort deletion of an
//! item's outputs that `remove` and `update` use.
//!
//! The install pipeline plans (and `remove` deletes) per-client output
//! files at the paths defined in format-spec §7 / ADR-0003. The frozen
//! legacy layout keeps install, remove and legacy candidates in lockstep;
//! pipeline callers go through [`output_path`].

use std::fs;
use std::path::{Path, PathBuf};

use super::{ALL_CLIENTS, ItemKind};
use crate::generate::Client;

/// Where the install pipeline writes — and `remove` deletes — the per-
/// client output for a given `(kind, name)`. Path is relative to the
/// install target root and matches format-spec §7. Used by both
/// `install_*` and `remove` so the two stay in lockstep.
pub(super) fn output_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    crate::legacy::layout::output_path(kind, client.into(), name)
}

/// Whether resources live beside the entrypoint in the item's own directory.
pub(super) fn is_dir_backed(kind: ItemKind, client: Client) -> bool {
    crate::legacy::layout::is_dir_backed(kind, client.into())
}

/// Historical resource namespace, relative to the installation root.
pub(super) fn resource_base_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    crate::legacy::layout::resource_base_path(kind, client.into(), name)
}

/// Delete all per-client output files for an item (best-effort).
pub(super) fn remove_item_outputs(target: &Path, kind: ItemKind, name: &str) {
    for client in ALL_CLIENTS {
        // Remove the entrypoint file. For directory-backed kinds this also
        // gets swept by the resource-base removal below; doing it first is
        // harmless (the second remove is a no-op).
        let _ = fs::remove_file(target.join(output_path(kind, client, name)));
        // Remove the item's resource base directory. For directory-backed
        // kinds this is the item's own `<name>/` dir (entrypoint + resources);
        // for flat kinds it is the sibling `<name>/` namespace dir. Both are
        // item-specific — `resource_base_path` never returns the shared kind
        // directory — so removing an item named like its kind dir (e.g. a
        // rule named `rules`) cannot wipe sibling items.
        let base = target.join(resource_base_path(kind, client, name));
        if base.is_dir() {
            let _ = fs::remove_dir_all(&base);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_paths_and_resource_bases_match_historical_spec() {
        let cases = [
            (
                ItemKind::Skill,
                Client::Claude,
                ".claude/skills/x/SKILL.md",
                ".claude/skills/x",
            ),
            (
                ItemKind::Skill,
                Client::Copilot,
                ".github/skills/x/SKILL.md",
                ".github/skills/x",
            ),
            (
                ItemKind::Skill,
                Client::OpenCode,
                ".agents/skills/x/SKILL.md",
                ".agents/skills/x",
            ),
            (
                ItemKind::Rule,
                Client::Claude,
                ".claude/rules/x.md",
                ".claude/rules/x",
            ),
            (
                ItemKind::Rule,
                Client::Copilot,
                ".github/instructions/x.instructions.md",
                ".github/instructions/x",
            ),
            (
                ItemKind::Rule,
                Client::OpenCode,
                ".agents/rules/x/RULE.md",
                ".agents/rules/x",
            ),
            (
                ItemKind::Agent,
                Client::Claude,
                ".claude/agents/x.md",
                ".claude/agents/x",
            ),
            (
                ItemKind::Agent,
                Client::Copilot,
                ".github/agents/x.agent.md",
                ".github/agents/x",
            ),
            (
                ItemKind::Agent,
                Client::OpenCode,
                ".opencode/agents/x.md",
                ".opencode/agents/x",
            ),
        ];
        for (kind, client, entrypoint, resources) in cases {
            assert_eq!(output_path(kind, client, "x"), PathBuf::from(entrypoint));
            assert_eq!(
                resource_base_path(kind, client, "x"),
                PathBuf::from(resources)
            );
        }
    }

    #[test]
    fn raw_separator_names_retain_native_pipeline_layout() {
        assert_eq!(
            output_path(ItemKind::Rule, Client::Claude, "a/b"),
            PathBuf::from(".claude/rules/a/b.md")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "a/b"),
            PathBuf::from(".claude/rules/a/a/b")
        );
        #[cfg(unix)]
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "a\\b"),
            PathBuf::from(".claude/rules/a\\b")
        );
        #[cfg(windows)]
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "a\\b"),
            PathBuf::from(".claude/rules/a/a/b")
        );
    }

    #[test]
    fn is_dir_backed_matches_layout() {
        // Skills are directory-backed on every client.
        for c in ALL_CLIENTS {
            assert!(is_dir_backed(ItemKind::Skill, c));
        }
        // Rules: only opencode is directory-backed.
        assert!(is_dir_backed(ItemKind::Rule, Client::OpenCode));
        assert!(!is_dir_backed(ItemKind::Rule, Client::Claude));
        assert!(!is_dir_backed(ItemKind::Rule, Client::Copilot));
        // Agents are flat on every client.
        for c in ALL_CLIENTS {
            assert!(!is_dir_backed(ItemKind::Agent, c));
        }
    }

    #[test]
    fn resource_base_dir_backed_is_entrypoint_dir() {
        assert_eq!(
            resource_base_path(ItemKind::Skill, Client::Claude, "x"),
            PathBuf::from(".claude/skills/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::OpenCode, "x"),
            PathBuf::from(".agents/rules/x")
        );
    }

    #[test]
    fn resource_base_flat_is_sibling_namespace_dir() {
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "x"),
            PathBuf::from(".claude/rules/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Copilot, "x"),
            PathBuf::from(".github/instructions/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Agent, Client::Claude, "x"),
            PathBuf::from(".claude/agents/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Agent, Client::OpenCode, "x"),
            PathBuf::from(".opencode/agents/x")
        );
    }

    #[test]
    fn remove_item_outputs_deletes_flat_kind_resource_dir() {
        let target = tempfile::tempdir().unwrap();
        // Flat-kind (Claude rule) entrypoint + sibling resource namespace dir.
        std::fs::create_dir_all(target.path().join(".claude/rules/demo/scripts")).unwrap();
        std::fs::write(target.path().join(".claude/rules/demo.md"), "x").unwrap();
        std::fs::write(
            target.path().join(".claude/rules/demo/scripts/gate.sh"),
            "x",
        )
        .unwrap();

        remove_item_outputs(target.path(), ItemKind::Rule, "demo");

        assert!(!target.path().join(".claude/rules/demo.md").exists());
        assert!(
            !target.path().join(".claude/rules/demo").exists(),
            "the sibling resource namespace dir must be removed too"
        );
    }

    #[test]
    fn remove_item_outputs_never_wipes_sibling_items_in_shared_dir() {
        let target = tempfile::tempdir().unwrap();
        // Two flat-kind (Claude) rules share `.claude/rules/`. One is named
        // `rules`, matching the shared kind-directory name — the case that
        // previously caused `remove_dir_all(.claude/rules)` to wipe everything.
        fs::create_dir_all(target.path().join(".claude/rules/rules")).unwrap();
        fs::write(target.path().join(".claude/rules/other.md"), "keep").unwrap();
        fs::write(target.path().join(".claude/rules/rules.md"), "x").unwrap();
        fs::write(target.path().join(".claude/rules/rules/g.sh"), "x").unwrap();

        remove_item_outputs(target.path(), ItemKind::Rule, "rules");

        assert!(!target.path().join(".claude/rules/rules.md").exists());
        assert!(!target.path().join(".claude/rules/rules").exists());
        assert!(
            target.path().join(".claude/rules/other.md").exists(),
            "removing the 'rules'-named item must not wipe sibling rules"
        );
    }
}
