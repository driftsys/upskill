//! Install plan: every output file of every item, computed in memory before
//! anything is written. [`super::apply::apply_plan`] writes a plan.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::ItemKind;
use crate::generate::Client;

/// One file to write, relative to the install target root.
#[derive(Debug, Clone)]
pub(crate) struct PlannedFile {
    pub client: Client,
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    /// Unix permission bits of a copied resource; `None` for generated files.
    pub mode: Option<u32>,
}

/// Every file of one item, for every targeted client.
#[derive(Debug, Clone)]
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
