//! Pure complete installation upserts with separate restrictive and shared entrypoints.
use super::desired::DesiredInstallationSet;
use super::reconcile::{GraphChanges, reconcile_references};
use super::selection::{
    SelectedNodes, Selection, SelectionDiagnostic, SelectionOutcome, select_owned_nodes,
};
use super::state::{BundleNode, CombinedState, OwnerConsumers, Reference, validate_state};
use crate::files::plan::{ObservedFile, PlannedFile};
use crate::operations::plan::{DesiredArtifact, InstallationChange, plan_owned_artifacts};
use crate::ownership::{InstallationRoot, OwnershipScope};
use crate::pipeline::ItemKind;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

#[derive(Debug, Clone, Copy)]
pub struct UpsertRequest<'a> {
    pub selection: Option<&'a Selection>,
    /// Complete explicit evidence subset for changed owners; omitted rows become unknown.
    pub contracts: &'a [super::contracts::OwnerContract],
    pub sets: &'a [DesiredInstallationSet],
    pub bundles: &'a [BundleNode],
    pub add_references: &'a [Reference],
    pub remove_references: &'a [Reference],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertPlan {
    pub state: CombinedState,
    pub files: Vec<PlannedFile>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertOutcome {
    Ready(Box<UpsertPlan>),
    Blocked(Vec<SelectionDiagnostic>),
}

type AssociationKey = (OwnershipScope, OsString, String, Option<String>, String);
fn key(row: &OwnerConsumers) -> AssociationKey {
    (
        row.root.scope,
        row.root.path.as_os_str().to_owned(),
        row.path.clone(),
        row.contribution_id.clone(),
        row.installation_id.clone(),
    )
}
fn same_root(a: &InstallationRoot, b: &InstallationRoot) -> bool {
    a.scope == b.scope && a.path.as_os_str() == b.path.as_os_str()
}

pub(crate) fn validate_set(set: &DesiredInstallationSet) -> Result<Vec<OwnerConsumers>> {
    let item = &set.installation;
    ensure!(
        item.owner.kind == ItemKind::Rule,
        "only rule upserts are supported"
    );
    let mut surfaces = BTreeSet::new();
    let mut rows = Vec::new();
    for artifact in &set.artifacts {
        let (root, path, selector, consumers) = match artifact {
            DesiredArtifact::File(file) => (
                &file.location.root,
                &file.location.path,
                None,
                &file.consumers,
            ),
            DesiredArtifact::Instruction(value) => (
                &value.location.root,
                &value.location.path,
                Some(value.id.clone()),
                &value.consumers,
            ),
        };
        ensure!(
            same_root(root, &item.root),
            "desired root differs from installation root"
        );
        let unique: BTreeSet<_> = consumers.iter().map(|s| s.name()).collect();
        ensure!(
            !unique.is_empty() && unique.len() == consumers.len(),
            "desired consumers must be nonempty and unique"
        );
        surfaces.extend(unique);
        rows.push(OwnerConsumers {
            root: root.clone(),
            path: path.clone(),
            contribution_id: selector,
            installation_id: item.owner.installation_id.clone(),
            consumers: consumers.clone(),
        });
    }
    let excluded: BTreeSet<_> = set.audience_excluded.iter().map(|s| s.name()).collect();
    ensure!(
        excluded.len() == set.audience_excluded.len(),
        "duplicate audience exclusion"
    );
    ensure!(
        surfaces.is_disjoint(&excluded),
        "excluded consumer has desired output"
    );
    surfaces.extend(excluded);
    ensure!(
        surfaces == item.requested.iter().map(|s| s.name()).collect(),
        "desired outputs and exclusions must exactly account for requested surfaces"
    );
    Ok(rows)
}

fn authorize_reference(
    prior: &CombinedState,
    selected: &SelectedNodes,
    edge: &Reference,
) -> Result<()> {
    let authorized = match edge {
        Reference::DirectInstallation { installation_id } => {
            !prior
                .installations
                .iter()
                .any(|i| &i.owner.installation_id == installation_id)
                || selected.installation_ids.contains(installation_id)
        }
        Reference::DirectBundle { bundle_id }
        | Reference::BundleInstallation { bundle_id, .. }
        | Reference::BundleDependency { bundle_id, .. } => {
            !prior.bundles.iter().any(|b| &b.id == bundle_id)
                || selected.bundle_ids.contains(bundle_id)
        }
    };
    ensure!(
        authorized,
        "reference edit affects an unselected existing root"
    );
    Ok(())
}

fn validate_legacy_destinations(
    prior: &CombinedState,
    sets: &[DesiredInstallationSet],
    changed: &BTreeSet<String>,
) -> Result<()> {
    let Some(lock) = &prior.legacy else {
        return Ok(());
    };
    let projection = crate::legacy::project_legacy(lock);
    let mut paths: Vec<&str> = prior
        .artifacts
        .iter()
        .filter(|a| {
            a.owners
                .iter()
                .any(|o| changed.contains(&o.installation_id))
        })
        .map(|a| a.path.as_str())
        .collect();
    for set in sets {
        paths.extend(set.artifacts.iter().map(|a| match a {
            DesiredArtifact::File(f) => f.location.path.as_str(),
            DesiredArtifact::Instruction(v) => v.location.path.as_str(),
        }));
    }
    let overlaps = |a: &str, b: &str| {
        a == b
            || a.strip_prefix(b).is_some_and(|v| v.starts_with('/'))
            || b.strip_prefix(a).is_some_and(|v| v.starts_with('/'))
    };
    // Historical rows contain no qualified scope/root. Do not assume a different
    // supplied root makes a known relative destination conflict disappear.
    for row in &projection.items {
        for candidate in &row.candidates {
            for path in &paths {
                ensure!(
                    !overlaps(path, &candidate.entrypoint)
                        && !overlaps(path, &candidate.resource_search_base),
                    "known legacy destination conflict: item {} at {path}",
                    row.index
                );
            }
        }
    }
    Ok(())
}

/// Plan complete replacement sets and explicit reference edits without I/O.
///
/// None selects no prior nodes and permits only new roots. Every selected surviving
/// installation requires a complete set, including empty audience-filtered sets.
/// Selection conflicts return all selector diagnostics; graph/artifact errors
/// return no partial plan. Affected shared artifacts remain unsupported, and this
/// supplied-value result establishes no native or filesystem qualification.
pub fn plan_upsert(
    prior: &CombinedState,
    request: UpsertRequest<'_>,
    observations: &[ObservedFile],
) -> Result<UpsertOutcome> {
    plan_upsert_mode(prior, request, observations, false)
}

/// Plan complete shared replacements using explicit matching owner contracts.
///
/// This opt-in pure boundary grants no native or filesystem qualification.
/// Missing retained evidence blocks desired multi-owner outputs; pure owner release
/// preserves untouched survivors without requalification.
pub fn plan_shared_upsert(
    prior: &CombinedState,
    request: UpsertRequest<'_>,
    observations: &[ObservedFile],
) -> Result<UpsertOutcome> {
    plan_upsert_mode(prior, request, observations, true)
}

fn plan_upsert_mode(
    prior: &CombinedState,
    request: UpsertRequest<'_>,
    observations: &[ObservedFile],
    shared: bool,
) -> Result<UpsertOutcome> {
    validate_state(prior).context("validate prior upsert state")?;
    let mut blockers = Vec::new();
    let selected = match request.selection {
        None => SelectedNodes {
            installation_ids: Vec::new(),
            bundle_ids: Vec::new(),
        },
        Some(selection) => match select_owned_nodes(prior, selection)? {
            SelectionOutcome::Ready(nodes) => nodes,
            SelectionOutcome::Blocked {
                candidates,
                diagnostics,
            } => {
                blockers.extend(diagnostics);
                candidates
            }
        },
    };
    // New roots can collide with unresolved history even when nothing prior was selected.
    let names: BTreeSet<_> = request
        .sets
        .iter()
        .map(|s| s.installation.owner.installed_name.clone())
        .chain(request.bundles.iter().map(|b| b.name.clone()))
        .collect();
    if !names.is_empty() {
        let selection = Selection::Names(names.into_iter().collect());
        if let SelectionOutcome::Blocked { diagnostics, .. } =
            select_owned_nodes(prior, &selection)?
        {
            let legacy: Vec<_> = diagnostics
                .into_iter()
                .filter(|d| matches!(d, SelectionDiagnostic::LegacyRow(_)))
                .collect();
            for diagnostic in legacy {
                if !blockers.contains(&diagnostic) {
                    blockers.push(diagnostic);
                }
            }
        }
    }
    if !blockers.is_empty() {
        return Ok(UpsertOutcome::Blocked(blockers));
    }
    let mut changed = BTreeSet::new();
    let mut associations = BTreeMap::new();
    for set in request.sets {
        let id = &set.installation.owner.installation_id;
        ensure!(
            changed.insert(id.clone()),
            "duplicate complete desired set: {id}"
        );
        ensure!(
            !prior
                .installations
                .iter()
                .any(|i| &i.owner.installation_id == id)
                || selected.installation_ids.contains(id),
            "upsert affects unselected installation: {id}"
        );
        for row in validate_set(set).with_context(|| format!("desired installation {id}"))? {
            ensure!(
                associations.insert(key(&row), row).is_none(),
                "duplicate desired owner association"
            );
        }
    }
    for bundle in request.bundles {
        ensure!(
            !prior.bundles.iter().any(|b| b.id == bundle.id)
                || selected.bundle_ids.contains(&bundle.id),
            "upsert affects unselected bundle: {}",
            bundle.id
        );
    }
    for edge in request
        .add_references
        .iter()
        .chain(request.remove_references)
    {
        authorize_reference(prior, &selected, edge)?;
    }
    let graph = reconcile_references(
        prior,
        &GraphChanges {
            installations: request
                .sets
                .iter()
                .map(|s| s.installation.clone())
                .collect(),
            bundles: request.bundles.to_vec(),
            add_references: request.add_references.to_vec(),
            remove_references: request.remove_references.to_vec(),
        },
    )?;
    for id in &graph.removed_installation_ids {
        ensure!(
            selected.installation_ids.contains(id),
            "graph retires unselected installation: {id}"
        );
    }
    for id in &graph.removed_bundle_ids {
        ensure!(
            selected.bundle_ids.contains(id),
            "graph retires unselected bundle: {id}"
        );
    }
    for id in &selected.installation_ids {
        ensure!(
            changed.contains(id) || graph.removed_installation_ids.contains(id),
            "selected installation lacks complete desired set: {id}"
        );
    }
    ensure!(
        prior
            .installations
            .iter()
            .filter(|i| graph
                .removed_installation_ids
                .contains(&i.owner.installation_id))
            .all(|i| i.owner.kind == ItemKind::Rule),
        "unsupported kind selected for retirement"
    );
    let mut changes = Vec::new();
    for set in request.sets {
        let owner = &set.installation.owner;
        if !set.artifacts.is_empty() {
            changes.push(InstallationChange::Set {
                owner: owner.clone(),
                artifacts: set.artifacts.clone(),
            });
        } else if prior.artifacts.iter().any(|a| {
            a.owners
                .iter()
                .any(|o| o.installation_id == owner.installation_id)
        }) {
            // Remove here retires outputs only. Graph authority remains above.
            changes.push(InstallationChange::Remove {
                installation_id: owner.installation_id.clone(),
            });
        }
    }
    for id in &graph.removed_installation_ids {
        changed.insert(id.clone());
        if prior
            .artifacts
            .iter()
            .any(|a| a.owners.iter().any(|o| &o.installation_id == id))
        {
            changes.push(InstallationChange::Remove {
                installation_id: id.clone(),
            });
        }
    }
    validate_legacy_destinations(prior, request.sets, &changed)?;

    let mut owner_consumers = Vec::new();
    for old in &prior.owner_consumers {
        if !changed.contains(&old.installation_id) {
            owner_consumers.push(old.clone());
        } else if let Some(row) = associations.remove(&key(old)) {
            owner_consumers.push(row);
        }
    }
    owner_consumers.extend(associations.into_values());
    let mut supplied_contracts = BTreeMap::new();
    for contract in request.contracts {
        ensure!(
            changed.contains(&contract.installation_id),
            "contract evidence affects an unchanged installation"
        );
        ensure!(
            supplied_contracts
                .insert(super::contracts::key(contract), contract.clone())
                .is_none(),
            "duplicate supplied owner contract"
        );
    }
    let mut owner_contracts = Vec::new();
    for old in &prior.owner_contracts {
        if !changed.contains(&old.installation_id) {
            owner_contracts.push(old.clone());
        } else if let Some(contract) = supplied_contracts.remove(&super::contracts::key(old)) {
            owner_contracts.push(contract);
        }
    }
    owner_contracts.extend(supplied_contracts.into_values());
    let mut state = CombinedState {
        origin: prior.origin.clone(),
        owner_contracts,
        schema: prior.schema,
        installations: graph.installations,
        bundles: graph.bundles,
        references: graph.references,
        artifacts: prior.artifacts.clone(),
        owner_consumers,
        legacy: prior.legacy.clone(),
    };
    let files = if shared {
        super::sharing::plan(prior, &mut state, &changes, observations)?
    } else {
        let planned = plan_owned_artifacts(&prior.artifacts, &changes, observations)?;
        state.artifacts = planned.artifacts;
        planned.files
    };
    validate_state(&state).context("validate complete upsert state")?;
    Ok(UpsertOutcome::Ready(Box::new(UpsertPlan { state, files })))
}
