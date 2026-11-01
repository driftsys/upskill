use super::*;
use crate::files::plan::{FileLocation, FileValue, ObservedFile};
use crate::lifecycle::native_receipt::{
    NativeObservationState, NativeResourceRetention, NativeSkillObservation, NativeSkillSelection,
};
use crate::lifecycle::reconcile::GraphChanges;
use crate::lifecycle::skill_delivery::{SkillDeliveryInput, SkillIntentSelection};
use crate::ownership::{ArtifactContent, OwnershipScope, validate_artifacts};
use crate::pipeline::ItemKind;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

type Location = (OwnershipScope, std::path::PathBuf, String);
fn location(root: &InstallationRoot, path: &str) -> Location {
    (root.scope, root.path.clone(), path.into())
}
fn target(key: &SkillTarget) -> (String, &'static str) {
    (key.installation_id.clone(), key.surface.name())
}
pub(super) fn context(context: &NativeStateContext) -> Result<()> {
    for root in std::iter::once(&context.scope)
        .chain(std::iter::once(&context.private_root))
        .chain(&context.additional_roots)
    {
        crate::compile::skill_package::scope_identity(root)?;
    }
    ensure!(
        context.private_root.scope == OwnershipScope::User,
        "private evidence requires a user root"
    );
    ensure!(
        !context.namespace.is_empty()
            && context.namespace.len() <= 128
            && context
                .namespace
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')),
        "invalid private evidence namespace"
    );
    let mut roots = vec![&context.scope, &context.private_root];
    for root in &context.additional_roots {
        ensure!(!roots.contains(&root), "duplicate additional state root");
        roots.push(root);
    }
    Ok(())
}

pub(super) fn state(state: &NativeDeliveryState) -> Result<()> {
    context(&state.context)?;
    ensure!(
        state.context.scope == state.skills.scope,
        "integrated state scope differs from skill view"
    );
    validate_artifacts(&state.files)?;
    let mut file_rows = BTreeMap::new();
    for file in &state.files {
        crate::compile::skill_package::scope_identity(&file.root)?;
        ensure!(
            file.root == state.context.scope,
            "native export ownership belongs to another root"
        );
        ensure!(
            matches!(file.content,ArtifactContent::File{mode:Some(mode),..} if mode<=0o777),
            "native export requires ordinary applied mode evidence"
        );
        ensure!(
            file.owners.len() == 1 && file.owners[0].kind == ItemKind::Skill,
            "native export requires exactly one skill owner"
        );
        ensure!(
            file_rows
                .insert(location(&file.root, &file.path), file)
                .is_none(),
            "duplicate native export ownership"
        );
    }
    let mut bytes = BTreeMap::new();
    let mut keys = BTreeMap::new();
    for installation in state
        .skills
        .graph
        .installations
        .iter()
        .filter(|i| i.owner.kind == ItemKind::Skill)
    {
        for surface in &installation.requested {
            let key = SkillTarget {
                installation_id: installation.owner.installation_id.clone(),
                surface: *surface,
            };
            keys.insert(target(&key), key);
        }
    }
    for revision in &state.skills.revisions {
        let key = SkillTarget {
            installation_id: revision.installation.owner.installation_id.clone(),
            surface: revision.selected.surface,
        };
        keys.insert(target(&key), key);
        for file in &revision.package.revision_files {
            let key = location(&revision.installation.root, &file.path);
            if let Some(prior) = bytes.insert(key, file.bytes.clone()) {
                ensure!(prior == file.bytes, "conflicting retained revision bytes");
            }
        }
    }
    let configuration_keys = keys.clone();
    for staged in &state.skills.staged {
        let revision = &staged.revision;
        let key = SkillTarget {
            installation_id: revision.installation.owner.installation_id.clone(),
            surface: revision.selected.surface,
        };
        keys.insert(target(&key), key);
        for file in revision
            .package
            .revision_files
            .iter()
            .filter(|file| staged.paths.binary_search(&file.path).is_ok())
        {
            ensure!(
                bytes
                    .insert(
                        location(&revision.installation.root, &file.path),
                        file.bytes.clone()
                    )
                    .is_none(),
                "staged export overlaps another retained revision"
            );
        }
    }
    for receipt in &state.skills.receipts {
        if let Some(selection) = &receipt.after {
            let revision = state
                .skills
                .revisions
                .iter()
                .find(|r| {
                    r.installation.owner.installation_id == receipt.key.installation_id
                        && r.selected.surface == receipt.key.surface
                        && NativeSkillSelection::from_package(&r.package) == *selection
                })
                .context("confirmed selection has no matching revision capture")?;
            if let Some(file) = &revision.package.catalog_file {
                ensure!(
                    bytes
                        .insert(
                            location(&revision.installation.root, &file.path),
                            file.bytes.clone()
                        )
                        .is_none(),
                    "duplicate selected catalog"
                );
            }
        }
    }
    ensure!(
        bytes.len() == file_rows.len() && bytes.keys().eq(file_rows.keys()),
        "complete native export ownership is required"
    );
    let observations = bytes
        .into_iter()
        .map(|((scope, path, file), bytes)| {
            let row = file_rows[&(scope, path.clone(), file.clone())];
            let ArtifactContent::File { mode, .. } = row.content else {
                unreachable!()
            };
            ObservedFile {
                location: FileLocation {
                    root: InstallationRoot { scope, path },
                    path: file,
                },
                value: Some(FileValue { bytes, mode }),
            }
        })
        .collect::<Vec<_>>();
    let natives = keys
        .values()
        .map(|key| {
            let receipt = state.skills.receipts.iter().find(|r| r.key == *key);
            NativeSkillObservation {
                key: key.clone(),
                scope: state.context.scope.clone(),
                client_version: receipt
                    .map_or("state-validation", |r| r.client_version.as_str())
                    .into(),
                platform: receipt
                    .map_or("state-validation", |r| r.platform.as_str())
                    .into(),
                observation_id: "state-validation".into(),
                captured_prestate: vec![],
                state: receipt.and_then(|r| r.after.clone()).map_or(
                    NativeObservationState::Absent,
                    NativeObservationState::Present,
                ),
                retention: NativeResourceRetention::Unknown,
            }
        })
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for intent in &state.intents {
        if matches!(intent.selection, SkillIntentSelection::Selected { .. }) {
            let selection = state
                .skills
                .receipts
                .iter()
                .find(|r| r.key == intent.key)
                .and_then(|r| r.after.as_ref())
                .context("selected intent requires a confirmed current binding")?;
            candidates.push(
                state
                    .skills
                    .revisions
                    .iter()
                    .find(|r| {
                        r.installation.owner.installation_id == intent.key.installation_id
                            && r.selected.surface == intent.key.surface
                            && NativeSkillSelection::from_package(&r.package) == *selection
                    })
                    .context("selected intent has no captured package")?
                    .clone(),
            );
        }
    }
    let changes = GraphChanges::default();
    let plan = super::super::plan_native_skill_files(&SkillDeliveryInput {
        current: &state.skills,
        scope: &state.context.scope,
        graph_changes: &changes,
        intents: &state.intents,
        candidates: &candidates,
        native_observations: &natives,
        file_observations: &observations,
    })?;
    ensure!(
        plan.native_changes.is_empty()
            && plan
                .stage_files
                .iter()
                .chain(&plan.selector_files)
                .all(|f| f.before == f.after),
        "committed native state still requires delivery effects"
    );
    let expected = plan
        .file_ownership
        .iter()
        .filter(|a| a.owners.iter().any(|o| o.kind == ItemKind::Skill))
        .map(|a| (location(&a.root, &a.path), a))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        expected == file_rows,
        "applied native ownership differs from captured packages"
    );
    configurations(state, &configuration_keys)
}

fn configurations(
    state: &NativeDeliveryState,
    targets: &BTreeMap<(String, &str), SkillTarget>,
) -> Result<()> {
    use crate::native_config::{ConfigContribution, plan_config_patch};
    let allowed_roots = std::iter::once(&state.context.scope)
        .chain(std::iter::once(&state.context.private_root))
        .chain(&state.context.additional_roots)
        .collect::<Vec<_>>();
    let mut locations = Vec::new();
    for config in &state.configurations {
        let capture = &config.capture;
        let here = &capture.target.file.location;
        crate::compile::skill_package::scope_identity(&capture.scope)?;
        crate::compile::skill_package::scope_identity(&here.root)?;
        ensure!(
            allowed_roots.contains(&&capture.scope),
            "configuration snapshot root is outside the state context"
        );
        ensure!(
            !locations.contains(&here),
            "duplicate configuration document"
        );
        locations.push(here);
        for owned in state.files.iter().chain(&state.skills.graph.artifacts) {
            if owned.root == here.root {
                ensure!(
                    !crate::filesystem::paths_may_overlap(&owned.path, &here.path),
                    "configuration document overlaps another ownership representation"
                );
            }
        }
        let value = capture
            .target
            .file
            .value
            .as_ref()
            .context("owned configuration document is absent")?;
        ensure!(
            value.mode.is_some_and(|mode| mode <= 0o777),
            "owned configuration mode is unknown or special"
        );
        let ownership = capture
            .ownership
            .as_ref()
            .context("configuration snapshot has no ownership")?;
        crate::compile::skill_package::scope_identity(&ownership.location.root)?;
        ensure!(
            !ownership.entries.is_empty(),
            "configuration snapshot ownership is empty"
        );
        let mut bindings = BTreeSet::new();
        for owner in &config.owners {
            ensure!(
                bindings.insert(owner.entry_id.as_str()),
                "duplicate configuration owner binding"
            );
            ensure!(
                owner.target.surface == capture.surface
                    && targets.contains_key(&target(&owner.target)),
                "configuration owner target is unknown or has a different surface"
            );
        }
        ensure!(
            bindings == ownership.entries.iter().map(|e| e.id.as_str()).collect(),
            "configuration owner bindings must exactly cover entries"
        );
        for entry in &ownership.entries {
            let patch = plan_config_patch(
                capture,
                &ConfigContribution {
                    id: entry.id.clone(),
                    selector: entry.selector.clone(),
                    expected: Some(entry.clone()),
                    desired: Some(entry.value.clone()),
                },
            )?;
            ensure!(
                patch.file.before == patch.file.after,
                "configuration snapshot requires a file change"
            );
            let updated = patch
                .ownership_after
                .context("configuration no-op lost ownership")?;
            ensure!(
                updated == *ownership,
                "configuration snapshot has stale ownership or rollback evidence"
            );
        }
    }
    Ok(())
}
