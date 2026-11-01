use super::*;
use crate::lifecycle::skill_delivery::{SkillDeliveryPlan, SkillDeliveryState};
use crate::lifecycle::state::{CombinedState, validate_state};
use crate::native_delivery::state::NativeConfigState;
use crate::pipeline::ItemKind;

pub(super) fn graph(
    prior: &NativeDeliveryState,
    plan: &SkillDeliveryPlan,
) -> Result<CombinedState> {
    for old in &prior.skills.graph.installations {
        if old.owner.kind != ItemKind::Skill {
            ensure!(
                plan.graph.installations.iter().any(|new| new == old),
                "native skill coordination cannot change unrelated installations"
            );
        }
    }
    for new in &plan.graph.installations {
        if new.owner.kind != ItemKind::Skill {
            ensure!(
                prior.skills.graph.installations.contains(new),
                "native skill coordination cannot add unrelated installations"
            );
        }
    }
    let mut graph = prior.skills.graph.clone();
    graph.installations = plan.graph.installations.clone();
    graph.bundles = plan.graph.bundles.clone();
    graph.references = plan.graph.references.clone();
    validate_state(&graph)?;
    Ok(graph)
}
pub(super) fn completed(
    store: &NativeOperationStore<'_, '_>,
    intent: &NativeOperationIntent,
    observations: &[NativeSkillObservation],
) -> Result<NativeDeliveryState> {
    ensure!(
        observations.len() == intent.capture.native_observations.len(),
        "completed native observation coverage differs"
    );
    for (actual, expected) in observations.iter().zip(&intent.capture.native_observations) {
        validate_binding_attribution(actual, expected)?;
        ensure!(
            selection(actual)? == intended(intent, expected)?,
            "completed native binding differs from intent"
        );
        require_retention(store, intent, actual)?;
    }
    let prior = store.prior(&intent.prior_state)?;
    let mut receipts = prior.skills.receipts.clone();
    for change in &intent.plan.native_changes {
        receipts.retain(|receipt| receipt.key != change.key);
        receipts.push(NativeBindingReceipt {
            key: change.key.clone(),
            scope: change.scope.clone(),
            client_version: change.client_version.clone(),
            platform: change.platform.clone(),
            operation_id: intent.operation_id.clone(),
            before: change.before.clone(),
            after: change.after.clone(),
            captured_prestate: change.captured_prestate.clone(),
            outcome: NativeBindingOutcome::Confirmed,
        });
    }
    let mut configurations = prior.configurations.clone();
    for (config, patch) in intent
        .capture
        .configurations
        .iter()
        .zip(&intent.configuration_patches)
    {
        configurations.retain(|old| {
            old.capture.target.file.location != config.before.target.file.location
                || old.capture.surface != config.before.surface
        });
        if patch.ownership_after.is_some() {
            let mut capture = config.before.clone();
            capture.target.file.value = patch.file.after.clone();
            capture.ownership = patch.ownership_after.clone();
            configurations.push(NativeConfigState {
                capture,
                owners: config.owners_after.clone(),
            });
        }
    }
    Ok(NativeDeliveryState {
        context: intent.context.clone(),
        skills: SkillDeliveryState {
            scope: intent.context.scope.clone(),
            graph: graph(&prior, &intent.plan)?,
            revisions: intent.plan.revisions.clone(),
            staged: intent.plan.staged.clone(),
            receipts,
        },
        intents: intent.plan.intents.clone(),
        files: intent.plan.file_ownership.clone(),
        configurations,
    })
}

pub(super) fn restored(
    store: &NativeOperationStore<'_, '_>,
    intent: &NativeOperationIntent,
    observations: &[NativeSkillObservation],
    files: &[crate::files::plan::ObservedFile],
) -> Result<NativeDeliveryState> {
    use crate::lifecycle::reconcile::GraphChanges;
    use crate::lifecycle::skill_delivery::{SkillDeliveryInput, StagedSkillRevision};
    ensure!(
        observations.len() == intent.capture.native_observations.len(),
        "restore observation coverage differs"
    );
    for (actual, expected) in observations.iter().zip(&intent.capture.native_observations) {
        validate_binding_attribution(actual, expected)?;
        ensure!(
            selection(actual)? == selection(expected)?,
            "restored binding differs from captured prior state"
        );
    }
    let mut candidate = store.prior(&intent.prior_state)?;
    let key = |revision: &crate::lifecycle::skill_delivery::PackageRevision| {
        (
            revision.installation.owner.installation_id.clone(),
            revision.selected.surface.name(),
            revision.package.payload_sha256.clone(),
        )
    };
    for revision in intent
        .plan
        .revisions
        .iter()
        .chain(intent.plan.staged.iter().map(|staged| &staged.revision))
    {
        if candidate
            .skills
            .revisions
            .iter()
            .any(|old| key(old) == key(revision))
        {
            continue;
        }
        let mut paths = Vec::new();
        for file in &revision.package.revision_files {
            if let Some(actual) = files.iter().find(|row| {
                row.location.root == intent.context.scope && row.location.path == file.path
            }) {
                if let Some(value) = &actual.value {
                    ensure!(
                        value.bytes == file.bytes,
                        "retained staged bytes differ from captured revision"
                    );
                    paths.push(file.path.clone());
                }
            }
        }
        if paths.is_empty() {
            continue;
        }
        paths.sort();
        paths.dedup();
        if let Some(old) = candidate
            .skills
            .staged
            .iter_mut()
            .find(|old| key(&old.revision) == key(revision))
        {
            if old.paths != paths {
                old.paths = paths;
                old.operation_id = intent.operation_id.clone();
            }
        } else {
            candidate.skills.staged.push(StagedSkillRevision {
                revision: revision.clone(),
                operation_id: intent.operation_id.clone(),
                paths,
            });
        }
    }
    for config in &intent.capture.configurations {
        if let Some(old) = candidate.configurations.iter_mut().find(|old| {
            old.capture.target.file.location == config.before.target.file.location
                && old.capture.surface == config.before.surface
        }) {
            old.capture = config.before.clone();
        }
    }
    let owned_observations = files
        .iter()
        .filter(|file| {
            candidate
                .files
                .iter()
                .chain(&candidate.skills.graph.artifacts)
                .any(|owned| owned.root == file.location.root && owned.path == file.location.path)
                || candidate.skills.staged.iter().any(|staged| {
                    staged.revision.installation.root == file.location.root
                        && staged.paths.contains(&file.location.path)
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    let candidates = candidate
        .skills
        .revisions
        .iter()
        .filter(|revision| {
            candidate.intents.iter().any(|intent| {
                matches!(
                    intent.selection,
                    crate::lifecycle::skill_delivery::SkillIntentSelection::Selected { .. }
                ) && intent.key.installation_id == revision.installation.owner.installation_id
                    && intent.key.surface == revision.selected.surface
                    && candidate.skills.receipts.iter().any(|receipt| {
                        receipt.key == intent.key
                            && receipt.after
                                == Some(NativeSkillSelection::from_package(&revision.package))
                    })
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let plan = crate::native_delivery::plan_native_skill_files(&SkillDeliveryInput {
        current: &candidate.skills,
        scope: &candidate.context.scope,
        graph_changes: &GraphChanges::default(),
        intents: &candidate.intents,
        candidates: &candidates,
        native_observations: observations,
        file_observations: &owned_observations,
    })?;
    ensure!(
        plan.native_changes.is_empty()
            && plan
                .stage_files
                .iter()
                .chain(&plan.selector_files)
                .all(|file| file.before == file.after),
        "restore state requires unperformed file or native effects"
    );
    candidate.files = plan.file_ownership;
    Ok(candidate)
}
