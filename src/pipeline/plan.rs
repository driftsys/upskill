//! Install plan: every output file of every item, computed in memory before
//! anything is written. [`super::apply::apply_plan`] writes a plan.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::ItemKind;
use crate::generate::Client;

/// One file to write, relative to the install target root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedFile {
    pub client: Client,
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    /// Unix permission bits of a copied resource; `None` for generated files.
    pub mode: Option<u32>,
}

/// Every file of one item, for every targeted client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedItem {
    pub kind: ItemKind,
    /// Installed name: the alias when the user passed `--as`.
    pub name: String,
    /// Name in the source when `name` is an alias; `None` otherwise.
    pub source_name: Option<String>,
    pub files: Vec<PlannedFile>,
    /// Entrypoint paths of the clients that this item no longer targets
    /// (author `audience:` or consumer selection). `apply_plan` deletes
    /// each of them, and the item directory that holds it when empty.
    pub deselected_entries: Vec<PathBuf>,
}

/// Consumer naming choices applied at plan time.
#[derive(Debug, Clone, Default)]
pub(crate) struct PlanNames {
    /// Source name to installed name (`--as`). The key `""` applies to
    /// every item.
    pub aliases: BTreeMap<String, String>,
    /// Source names to skip (`--exclude`).
    pub excludes: Vec<String>,
}

impl PlanNames {
    /// The alias of the item named `name` in the source, if any.
    pub fn alias_for(&self, name: &str) -> Option<&String> {
        self.aliases.get(name).or_else(|| self.aliases.get(""))
    }
}

/// Keeps one copy of planned items that come from the same source item
/// under the same installed name, for example an item that is named on
/// the command line and is also a member of a named bundle. Two items are
/// such copies when they are equal: same kind, installed name, source
/// name, planned files and deselected entries. The first copy keeps its
/// position.
pub(crate) fn merge_duplicates(planned: &mut Vec<PlannedItem>) {
    let mut kept: Vec<PlannedItem> = Vec::with_capacity(planned.len());
    for item in planned.drain(..) {
        if !kept.contains(&item) {
            kept.push(item);
        }
    }
    *planned = kept;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, source_name: Option<&str>, body: &[u8]) -> PlannedItem {
        PlannedItem {
            kind: ItemKind::Skill,
            name: name.to_string(),
            source_name: source_name.map(str::to_string),
            files: vec![PlannedFile {
                client: Client::Claude,
                rel: PathBuf::from(format!(".claude/skills/{name}/SKILL.md")),
                bytes: body.to_vec(),
                mode: None,
            }],
            deselected_entries: vec![],
        }
    }

    #[test]
    fn merges_copies_of_one_source_item_only() {
        let mut planned = vec![
            item("a", None, b"a"),
            item("z", Some("x"), b"x"),
            item("a", None, b"a"),
            item("z", Some("y"), b"y"),
        ];
        merge_duplicates(&mut planned);
        assert_eq!(planned.len(), 3);
        assert_eq!(planned[0].name, "a");
        assert_eq!(planned[1].source_name.as_deref(), Some("x"));
        assert_eq!(planned[2].source_name.as_deref(), Some("y"));
    }

    #[test]
    fn keeps_two_items_that_differ_only_in_their_files() {
        let mut planned = vec![item("a", None, b"one"), item("a", None, b"two")];
        merge_duplicates(&mut planned);
        assert_eq!(planned.len(), 2);
        assert!(super::super::apply::check_unique_names(&planned).is_err());
    }
}
