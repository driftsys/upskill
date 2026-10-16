//! Pure reference-driven retirement, without establishing new sharing contracts.
mod files;

use super::reconcile::{GraphChanges, reconcile_references};
use super::state::{CombinedState, Reference};
use crate::files::plan::{ObservedFile, PlannedFile};
use crate::ownership::{ArtifactContent, OwnedArtifact, OwnershipScope};
use anyhow::{Result, ensure};
use std::collections::BTreeSet;
use std::ffi::OsString;

type LocationKey = (OwnershipScope, OsString, String);
fn key(row: &OwnedArtifact) -> LocationKey {
    (
        row.root.scope,
        row.root.path.as_os_str().to_owned(),
        row.path.clone(),
    )
}
fn selector(row: &OwnedArtifact) -> Option<&str> {
    match &row.content {
        ArtifactContent::File { .. } => None,
        ArtifactContent::MarkdownContribution { contribution } => Some(&contribution.id),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRemovalPlan {
    pub state: CombinedState,
    pub files: Vec<PlannedFile>,
}

/// Plan exact reference removals from validated state and supplied observations.
///
/// Every affected physical location requires one observation, including shared
/// ownership changes whose bytes stay identical. This pure result establishes
/// neither qualified filesystem provenance nor native client compatibility.
pub fn plan_reference_removal(
    prior: &CombinedState,
    remove: &[Reference],
    observations: &[ObservedFile],
) -> Result<ReferenceRemovalPlan> {
    let graph = reconcile_references(
        prior,
        &GraphChanges {
            remove_references: remove.to_vec(),
            ..Default::default()
        },
    )?;
    let retired: BTreeSet<_> = graph
        .removed_installation_ids
        .iter()
        .map(String::as_str)
        .collect();
    ensure!(
        prior
            .installations
            .iter()
            .filter(|item| retired.contains(item.owner.installation_id.as_str()))
            .all(|item| item.owner.kind == crate::pipeline::ItemKind::Rule),
        "unsupported owned kind selected for retirement"
    );
    let affected: BTreeSet<_> = prior
        .artifacts
        .iter()
        .filter(|row| {
            row.owners
                .iter()
                .any(|owner| retired.contains(owner.installation_id.as_str()))
        })
        .map(key)
        .collect();
    let mut state = prior.clone();
    state.installations = graph.installations;
    state.bundles = graph.bundles;
    state.references = graph.references;
    state
        .owner_consumers
        .retain(|association| !retired.contains(association.installation_id.as_str()));
    for row in &mut state.artifacts {
        row.owners
            .retain(|owner| !retired.contains(owner.installation_id.as_str()));
    }
    state.artifacts.retain(|row| !row.owners.is_empty());
    for row in &mut state.artifacts {
        let selected = selector(row).map(str::to_owned);
        row.consumers.retain(|surface| {
            state.owner_consumers.iter().any(|association| {
                association.root.scope == row.root.scope
                    && association.root.path.as_os_str() == row.root.path.as_os_str()
                    && association.path == row.path
                    && association.contribution_id.as_deref() == selected.as_deref()
                    && association.consumers.contains(surface)
            })
        });
    }
    state
        .owner_contracts
        .retain(|c| !retired.contains(c.installation_id.as_str()));
    let files = files::plan(prior, &mut state, &affected, observations)?;
    super::state::validate_state(&state)?;
    Ok(ReferenceRemovalPlan { state, files })
}
