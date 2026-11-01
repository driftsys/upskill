//! Pure native skill intent and file planning; no completed state or codec.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod files;
mod validation;
use super::native_receipt::{
    NativeBindingReceipt, NativeResourceRetention, NativeSkillChange, NativeSkillObservation,
    NativeSkillSelection, SkillTarget,
};
use super::reconcile::{GraphChanges, ReconciledGraph};
use super::state::{CombinedState, Installation};
use crate::compile::skill_package::SkillPackage;
use crate::compile::skill_selection::{SelectedSkill, SkillBodyOrigin};
use crate::files::plan::{ObservedFile, PlannedFile};
use crate::ownership::{InstallationRoot, OwnedArtifact};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRevision {
    pub installation: Installation,
    pub selected: SelectedSkill,
    pub package: SkillPackage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SkillIntentSelection {
    Selected { body_origin: SkillBodyOrigin },
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDeliveryIntent {
    pub key: SkillTarget,
    pub selection: SkillIntentSelection,
}

/// A single in-memory view, not a persisted installation lock.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillDeliveryState {
    pub scope: InstallationRoot,
    pub graph: CombinedState,
    pub revisions: Vec<PackageRevision>,
    pub receipts: Vec<NativeBindingReceipt>,
}

#[derive(Debug)]
pub struct SkillDeliveryInput<'a> {
    pub current: &'a SkillDeliveryState,
    pub scope: &'a InstallationRoot,
    pub graph_changes: &'a GraphChanges,
    pub intents: &'a [SkillDeliveryIntent],
    pub candidates: &'a [PackageRevision],
    pub native_observations: &'a [NativeSkillObservation],
    pub file_observations: &'a [ObservedFile],
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDeliveryPlan {
    pub graph: ReconciledGraph,
    pub intents: Vec<SkillDeliveryIntent>,
    pub revisions: Vec<PackageRevision>,
    pub file_ownership: Vec<OwnedArtifact>,
    pub stage_files: Vec<PlannedFile>,
    pub selector_files: Vec<PlannedFile>,
    pub native_changes: Vec<NativeSkillChange>,
}

type TargetKey = (String, &'static str);
type RevisionKey = (String, &'static str, String);

fn revision_key(revision: &PackageRevision) -> RevisionKey {
    (
        revision.installation.owner.installation_id.clone(),
        revision.selected.surface.name(),
        revision.package.payload_sha256.clone(),
    )
}
fn target(revision: &PackageRevision) -> SkillTarget {
    SkillTarget {
        installation_id: revision.installation.owner.installation_id.clone(),
        surface: revision.selected.surface,
    }
}

/// Return a complete pure preview, never a completed native state or write authority.
pub fn plan_skill_delivery(input: &SkillDeliveryInput<'_>) -> Result<SkillDeliveryPlan> {
    plan_with_profile(input, files::PermissionProfile::Captured)
}

pub(crate) fn plan_native_files(input: &SkillDeliveryInput<'_>) -> Result<SkillDeliveryPlan> {
    plan_with_profile(input, files::PermissionProfile::NativePrivateV1)
}

fn plan_with_profile(
    input: &SkillDeliveryInput<'_>,
    profile: files::PermissionProfile,
) -> Result<SkillDeliveryPlan> {
    let checked = validation::validate(input)?;
    let mut revisions: BTreeMap<_, _> = input
        .current
        .revisions
        .iter()
        .map(|revision| (revision_key(revision), revision.clone()))
        .collect();
    for candidate in input.candidates {
        if let Some(old) = revisions.get(&revision_key(candidate)) {
            ensure!(
                old.package == candidate.package && old.selected == candidate.selected,
                "same immutable revision has conflicting captured evidence"
            );
        } else {
            revisions.insert(revision_key(candidate), candidate.clone());
        }
    }
    let revisions: Vec<_> = revisions.into_values().collect();
    let mut native_changes = Vec::new();
    for (key, target) in &checked.targets {
        let before = checked
            .receipts
            .get(key)
            .and_then(|receipt| receipt.after.clone());
        let after = checked
            .candidates
            .get(key)
            .map(|candidate| NativeSkillSelection::from_package(&candidate.package));
        if before == after {
            continue;
        }
        let observation = checked.observations[key];
        let has_retained_resources = input
            .current
            .revisions
            .iter()
            .any(|revision| self::target(revision).index_key() == *key);
        if before.is_some() || has_retained_resources {
            ensure!(
                !matches!(observation.retention, NativeResourceRetention::Unknown),
                "native cache preservation or session clearance is required before refresh or detach"
            );
        }
        native_changes.push(NativeSkillChange {
            key: target.clone(),
            scope: input.scope.clone(),
            client_version: observation.client_version.clone(),
            platform: observation.platform.clone(),
            observation_id: observation.observation_id.clone(),
            before,
            after,
            captured_prestate: observation.captured_prestate.clone(),
            retention: observation.retention.clone(),
        });
    }
    let file_plan = files::plan(input, &checked, &revisions, profile)?;
    Ok(SkillDeliveryPlan {
        graph: checked.graph,
        intents: input.intents.to_vec(),
        revisions,
        file_ownership: file_plan.ownership,
        stage_files: file_plan.staging,
        selector_files: file_plan.selectors,
        native_changes,
    })
}
