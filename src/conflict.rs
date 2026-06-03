//! Item conflict detection for `upskill add`.
//!
//! Detects when an incoming item would collide with an already-installed
//! item from a different source.

use crate::lockfile::Lockfile;
use crate::pipeline::ItemKind;

/// A conflict between an incoming item and an already-installed item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemConflict {
    pub kind: ItemKind,
    pub name: String,
    pub existing_source: String,
    pub incoming_source: String,
}

/// Detect conflicts between incoming items and the current lockfile.
///
/// An item conflicts if the same `(kind, name)` pair exists in the lockfile
/// with a different `source`.
pub fn detect_conflicts(
    incoming: &[(ItemKind, String)],
    lockfile: &Lockfile,
    incoming_source: &str,
) -> Vec<ItemConflict> {
    let mut conflicts = Vec::new();
    for (kind, name) in incoming {
        for locked in &lockfile.items {
            if locked.kind == *kind && locked.name == *name && locked.source != incoming_source {
                conflicts.push(ItemConflict {
                    kind: *kind,
                    name: name.clone(),
                    existing_source: locked.source.clone(),
                    incoming_source: incoming_source.to_owned(),
                });
                break;
            }
        }
    }
    conflicts
}

/// Format conflicts into a user-facing error message.
pub fn format_conflict_error(conflicts: &[ItemConflict]) -> String {
    match conflicts.len() {
        0 => String::new(),
        1 => {
            let c = &conflicts[0];
            format!(
                "{} `{}` is already installed from `{}`.\n\
                 Use --force to replace, or --as <alt-name> to keep both.",
                c.kind, c.name, c.existing_source,
            )
        }
        _ => {
            let mut msg =
                String::from("The following items conflict with existing installations:\n");
            for c in conflicts {
                msg.push_str(&format!(
                    "  - {} `{}` (installed from `{}`)\n",
                    c.kind, c.name, c.existing_source,
                ));
            }
            msg.push_str(
                "Use --force to replace all, or resolve individually with --exclude or --as.",
            );
            msg
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::Lockfile;

    fn make_lockfile(items: Vec<(ItemKind, &str, &str)>) -> Lockfile {
        use crate::lockfile::LockedItem;
        Lockfile {
            schema: 1,
            items: items
                .into_iter()
                .map(|(kind, name, source)| LockedItem {
                    kind,
                    name: name.to_owned(),
                    source: source.to_owned(),
                    git_ref: None,
                    hash: None,
                    source_name: None,
                    required_by: vec![],
                    group: None,
                })
                .collect(),
            bundles: vec![],
            plugins: vec![],
            mcps: vec![],
        }
    }

    #[test]
    fn no_conflict_same_source() {
        let lf = make_lockfile(vec![(
            ItemKind::Skill,
            "brainstorming",
            "driftsys/superpowers",
        )]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_owned())];
        let conflicts = detect_conflicts(&incoming, &lf, "driftsys/superpowers");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn conflict_different_source() {
        let lf = make_lockfile(vec![(
            ItemKind::Skill,
            "brainstorming",
            "driftsys/superpowers",
        )]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_owned())];
        let conflicts = detect_conflicts(&incoming, &lf, "other/repo");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].name, "brainstorming");
        assert_eq!(conflicts[0].existing_source, "driftsys/superpowers");
    }

    #[test]
    fn no_conflict_item_not_in_lockfile() {
        let lf = make_lockfile(vec![(ItemKind::Skill, "debugging", "driftsys/superpowers")]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_owned())];
        let conflicts = detect_conflicts(&incoming, &lf, "other/repo");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn same_name_different_kind_no_conflict() {
        let lf = make_lockfile(vec![(
            ItemKind::Rule,
            "brainstorming",
            "driftsys/superpowers",
        )]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_owned())];
        let conflicts = detect_conflicts(&incoming, &lf, "other/repo");
        assert!(conflicts.is_empty());
    }
}
