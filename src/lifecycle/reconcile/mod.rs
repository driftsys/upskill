//! Explicit reference reconciliation; graph removal is not artifact deletion authority.
mod reachability;

use super::state::{BundleNode, CombinedState, Installation, Reference, validate_state};
use crate::files::plan::validate_binding;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphChanges {
    pub installations: Vec<Installation>,
    pub bundles: Vec<BundleNode>,
    pub add_references: Vec<Reference>,
    pub remove_references: Vec<Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciledGraph {
    pub installations: Vec<Installation>,
    pub bundles: Vec<BundleNode>,
    pub references: Vec<Reference>,
    pub removed_installation_ids: Vec<String>,
    pub removed_bundle_ids: Vec<String>,
}

/// Reconcile exact graph edits without I/O or mutation of supplied state.
///
/// Surviving prior records retain order; new nodes and edges append in ID/enum
/// order. Referenced empty installations survive. Results contain neither owned
/// artifacts nor tagged legacy state and cannot authorize effects: a later
/// coordinator must reconcile ownership and preserve the prior legacy snapshot.
pub fn reconcile_references(
    prior: &CombinedState,
    changes: &GraphChanges,
) -> Result<ReconciledGraph> {
    validate_state(prior).context("validate prior combined state")?;
    let mut installations: BTreeMap<_, _> = prior
        .installations
        .iter()
        .map(|item| (item.owner.installation_id.clone(), item.clone()))
        .collect();
    let mut bundles: BTreeMap<_, _> = prior
        .bundles
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect();
    let mut changed_items = BTreeSet::new();
    for item in &changes.installations {
        let id = &item.owner.installation_id;
        ensure!(
            changed_items.insert(id.clone()),
            "duplicate installation upsert: {id}"
        );
        if let Some(old) = installations.get(id) {
            validate_binding(&old.owner, &item.owner).context("installation binding")?;
            ensure!(
                old.root.scope == item.root.scope
                    && old.root.path.as_os_str() == item.root.path.as_os_str(),
                "installation binding cannot change scope or raw root"
            );
        }
        installations.insert(id.clone(), item.clone());
    }
    let mut changed_bundles = BTreeSet::new();
    for node in &changes.bundles {
        ensure!(
            changed_bundles.insert(node.id.clone()),
            "duplicate bundle upsert: {}",
            node.id
        );
        if let Some(old) = bundles.get(&node.id) {
            ensure!(
                old.source == node.source && old.name == node.name,
                "bundle binding cannot change source or name"
            );
        }
        bundles.insert(node.id.clone(), node.clone());
    }
    let existing: BTreeSet<_> = prior.references.iter().cloned().collect();
    let added: BTreeSet<_> = changes.add_references.iter().cloned().collect();
    let removed: BTreeSet<_> = changes.remove_references.iter().cloned().collect();
    ensure!(
        added.len() == changes.add_references.len(),
        "duplicate added reference"
    );
    ensure!(
        removed.len() == changes.remove_references.len(),
        "duplicate removed reference"
    );
    ensure!(
        added.is_disjoint(&removed),
        "reference cannot be added and removed together"
    );
    ensure!(
        added.is_disjoint(&existing),
        "added reference already exists"
    );
    ensure!(
        removed.is_subset(&existing),
        "removed reference does not exist"
    );
    let mut references: Vec<_> = prior
        .references
        .iter()
        .filter(|edge| !removed.contains(*edge))
        .cloned()
        .collect();
    references.extend(added);
    let mut candidate = graph_state(
        installations.values().cloned().collect(),
        bundles.values().cloned().collect(),
        references.clone(),
    );
    // Validation roots expose every supplied node/edge to the existing validator.
    // They never participate in actual retention or escape this private value.
    let mut validation_roots: BTreeSet<_> = candidate.references.iter().cloned().collect();
    for id in installations.keys() {
        let edge = Reference::DirectInstallation {
            installation_id: id.clone(),
        };
        if validation_roots.insert(edge.clone()) {
            candidate.references.push(edge);
        }
    }
    for id in bundles.keys() {
        let edge = Reference::DirectBundle {
            bundle_id: id.clone(),
        };
        if validation_roots.insert(edge.clone()) {
            candidate.references.push(edge);
        }
    }
    validate_state(&candidate).context("validate complete candidate graph")?;
    let (retained_items, retained_bundles) = reachability::reachable(&references);
    ensure!(
        changed_items.is_subset(&retained_items),
        "upserted installation is unreachable"
    );
    ensure!(
        changed_bundles.is_subset(&retained_bundles),
        "upserted bundle is unreachable"
    );
    let removed_installation_ids = prior
        .installations
        .iter()
        .filter(|item| !retained_items.contains(&item.owner.installation_id))
        .map(|item| item.owner.installation_id.clone())
        .collect();
    let removed_bundle_ids = prior
        .bundles
        .iter()
        .filter(|node| !retained_bundles.contains(&node.id))
        .map(|node| node.id.clone())
        .collect();
    installations.retain(|id, _| retained_items.contains(id));
    bundles.retain(|id, _| retained_bundles.contains(id));
    references.retain(|edge| match edge {
        Reference::DirectInstallation { installation_id } => {
            retained_items.contains(installation_id)
        }
        Reference::DirectBundle { bundle_id } => retained_bundles.contains(bundle_id),
        Reference::BundleInstallation {
            bundle_id,
            installation_id,
        } => retained_bundles.contains(bundle_id) && retained_items.contains(installation_id),
        Reference::BundleDependency {
            bundle_id,
            dependency_id,
        } => retained_bundles.contains(bundle_id) && retained_bundles.contains(dependency_id),
    });
    ensure!(
        changes
            .add_references
            .iter()
            .all(|edge| references.contains(edge)),
        "added reference is unreachable"
    );
    let mut ordered_items = Vec::new();
    for old in &prior.installations {
        if let Some(item) = installations.remove(&old.owner.installation_id) {
            ordered_items.push(item);
        }
    }
    ordered_items.extend(installations.into_values());
    let mut ordered_bundles = Vec::new();
    for old in &prior.bundles {
        if let Some(node) = bundles.remove(&old.id) {
            ordered_bundles.push(node);
        }
    }
    ordered_bundles.extend(bundles.into_values());
    validate_state(&graph_state(
        ordered_items.clone(),
        ordered_bundles.clone(),
        references.clone(),
    ))
    .context("validate reconciled graph")?;
    Ok(ReconciledGraph {
        installations: ordered_items,
        bundles: ordered_bundles,
        references,
        removed_installation_ids,
        removed_bundle_ids,
    })
}

fn graph_state(
    installations: Vec<Installation>,
    bundles: Vec<BundleNode>,
    references: Vec<Reference>,
) -> CombinedState {
    CombinedState {
        schema: 2,
        installations,
        bundles,
        references,
        artifacts: Vec::new(),
        owner_consumers: Vec::new(),
        legacy: None,
    }
}
