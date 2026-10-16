//! Pure all-record or literal-name selection preflight over captured state.
//!
//! Passing this boundary does not authorize reference edits, writes or native effects.
use super::state::{BundleNode, CombinedState, Installation, Reference, validate_state};
use crate::pipeline::ItemKind;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    All,
    Names(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedNodes {
    pub installation_ids: Vec<String>,
    pub bundle_ids: Vec<String>,
}

/// Index within the corresponding unchanged typed legacy vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyRow {
    Item(usize),
    Bundle(usize),
    Plugin(usize),
    Mcp(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionDiagnostic {
    UnknownName(String),
    AmbiguousName {
        name: String,
        matches: usize,
    },
    LegacyRow(LegacyRow),
    UnsupportedInstallation {
        installation_id: String,
        kind: ItemKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionOutcome {
    Ready(SelectedNodes),
    /// Candidates are descriptive only; no selected subset is ready to apply.
    Blocked {
        candidates: SelectedNodes,
        diagnostics: Vec<SelectionDiagnostic>,
    },
}

enum Node<'a> {
    Installation(&'a Installation),
    Bundle(&'a BundleNode),
    Legacy(LegacyRow),
}
struct Entry<'a> {
    name: &'a str,
    bundle: Option<&'a str>,
    node: Node<'a>,
}
impl Entry<'_> {
    fn matches(&self, name: &str) -> bool {
        self.name == name || self.bundle == Some(name)
    }
}

/// Resolve complete captured selection without I/O, adoption or graph changes.
///
/// Names match installed aliases, bundle names, or native records' names/declaring
/// bundle labels literally. Source selectors and scope selectors are not represented.
/// Every selected legacy row blocks, regardless of its source/content validity.
/// Malformed requests and invalid entire state return errors before selection.
pub fn select_owned_nodes(
    prior: &CombinedState,
    selection: &Selection,
) -> Result<SelectionOutcome> {
    validate_state(prior)?;
    let names = match selection {
        Selection::All => None,
        Selection::Names(values) => {
            ensure!(!values.is_empty(), "named selection must not be empty");
            let mut names = BTreeSet::new();
            for value in values {
                ensure!(!value.is_empty(), "selected name must not be empty");
                ensure!(
                    names.insert(value.as_str()),
                    "duplicate selected name: {value}"
                );
            }
            Some(names)
        }
    };
    let entries = entries(prior);
    let mut diagnostics = Vec::new();
    if let Some(names) = &names {
        for name in names {
            match entries.iter().filter(|entry| entry.matches(name)).count() {
                0 => diagnostics.push(SelectionDiagnostic::UnknownName((*name).into())),
                1 => {}
                matches => diagnostics.push(SelectionDiagnostic::AmbiguousName {
                    name: (*name).into(),
                    matches,
                }),
            }
        }
    }
    let mut installation_ids = BTreeSet::new();
    let mut bundle_ids = BTreeSet::new();
    for entry in &entries {
        if names
            .as_ref()
            .is_some_and(|names| !names.iter().any(|name| entry.matches(name)))
        {
            continue;
        }
        match &entry.node {
            Node::Installation(item) => {
                installation_ids.insert(item.owner.installation_id.as_str());
            }
            Node::Bundle(bundle) => {
                bundle_ids.insert(bundle.id.as_str());
            }
            Node::Legacy(_) => {}
        }
    }
    expand_bundles(prior, &mut installation_ids, &mut bundle_ids);
    let selected_bundle_names: BTreeSet<_> = prior
        .bundles
        .iter()
        .filter(|bundle| bundle_ids.contains(bundle.id.as_str()))
        .map(|bundle| bundle.name.as_str())
        .collect();
    let mut candidates = SelectedNodes {
        installation_ids: Vec::new(),
        bundle_ids: Vec::new(),
    };
    let mut unsupported = Vec::new();
    for entry in entries {
        let selected = match &entry.node {
            Node::Installation(item) => {
                installation_ids.contains(item.owner.installation_id.as_str())
            }
            Node::Bundle(bundle) => bundle_ids.contains(bundle.id.as_str()),
            Node::Legacy(_) => {
                names
                    .as_ref()
                    .is_none_or(|names| names.iter().any(|name| entry.matches(name)))
                    || entry
                        .bundle
                        .is_some_and(|name| selected_bundle_names.contains(name))
            }
        };
        if !selected {
            continue;
        }
        match entry.node {
            Node::Installation(item) => {
                candidates
                    .installation_ids
                    .push(item.owner.installation_id.clone());
                if item.owner.kind != ItemKind::Rule {
                    unsupported.push(SelectionDiagnostic::UnsupportedInstallation {
                        installation_id: item.owner.installation_id.clone(),
                        kind: item.owner.kind,
                    });
                }
            }
            Node::Bundle(bundle) => candidates.bundle_ids.push(bundle.id.clone()),
            Node::Legacy(row) => diagnostics.push(SelectionDiagnostic::LegacyRow(row)),
        }
    }
    diagnostics.extend(unsupported);
    Ok(if diagnostics.is_empty() {
        SelectionOutcome::Ready(candidates)
    } else {
        SelectionOutcome::Blocked {
            candidates,
            diagnostics,
        }
    })
}

fn entries(state: &CombinedState) -> Vec<Entry<'_>> {
    let mut entries = Vec::new();
    for item in &state.installations {
        entries.push(Entry {
            name: &item.owner.installed_name,
            bundle: None,
            node: Node::Installation(item),
        });
    }
    for bundle in &state.bundles {
        entries.push(Entry {
            name: &bundle.name,
            bundle: None,
            node: Node::Bundle(bundle),
        });
    }
    if let Some(legacy) = &state.legacy {
        for (index, item) in legacy.items.iter().enumerate() {
            entries.push(Entry {
                name: &item.name,
                bundle: None,
                node: Node::Legacy(LegacyRow::Item(index)),
            });
        }
        for (index, item) in legacy.bundles.iter().enumerate() {
            entries.push(Entry {
                name: &item.name,
                bundle: None,
                node: Node::Legacy(LegacyRow::Bundle(index)),
            });
        }
        for (index, item) in legacy.plugins.iter().enumerate() {
            entries.push(Entry {
                name: &item.name,
                bundle: Some(&item.bundle),
                node: Node::Legacy(LegacyRow::Plugin(index)),
            });
        }
        for (index, item) in legacy.mcps.iter().enumerate() {
            entries.push(Entry {
                name: &item.name,
                bundle: Some(&item.bundle),
                node: Node::Legacy(LegacyRow::Mcp(index)),
            });
        }
    }
    entries
}

fn expand_bundles<'a>(
    state: &'a CombinedState,
    installations: &mut BTreeSet<&'a str>,
    bundles: &mut BTreeSet<&'a str>,
) {
    let mut edges: BTreeMap<&str, Vec<&Reference>> = BTreeMap::new();
    for reference in &state.references {
        match reference {
            Reference::BundleInstallation { bundle_id, .. }
            | Reference::BundleDependency { bundle_id, .. } => {
                edges.entry(bundle_id).or_default().push(reference)
            }
            _ => {}
        }
    }
    let mut queue: VecDeque<_> = bundles.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        if let Some(edges) = edges.get(id) {
            for edge in edges {
                match edge {
                    Reference::BundleInstallation {
                        installation_id, ..
                    } => {
                        installations.insert(installation_id);
                    }
                    Reference::BundleDependency { dependency_id, .. } => {
                        if bundles.insert(dependency_id) {
                            queue.push_back(dependency_id);
                        }
                    }
                    _ => unreachable!("only bundle edges are indexed"),
                }
            }
        }
    }
}
