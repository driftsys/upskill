use super::*;
use crate::files::plan::{
    DesiredFile, FileInstallationChange, FileLocation, FileValue, plan_files,
};
use crate::ownership::{ArtifactContent, ArtifactOwner};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct PlannedSkillFiles {
    pub ownership: Vec<OwnedArtifact>,
    pub staging: Vec<PlannedFile>,
    pub selectors: Vec<PlannedFile>,
}
#[derive(Clone, Copy)]
pub(super) enum PermissionProfile {
    Captured,
    NativePrivateV1,
}

fn desired(
    scope: &InstallationRoot,
    file: &crate::compile::item::CompiledItemFile,
    revision: &PackageRevision,
    profile: PermissionProfile,
) -> Result<DesiredFile> {
    let mode = match profile {
        PermissionProfile::Captured => file.mode,
        PermissionProfile::NativePrivateV1 => {
            use crate::files::permissions::{
                DestinationMode, PermissionSource, ResourceExecutionPolicy, select_file_mode,
            };
            let prefix = format!(
                "{}/skills/{}/",
                revision.package.revision_root, revision.selected.installed_name
            );
            let resource = file.path.strip_prefix(&prefix).and_then(|relative| {
                revision
                    .selected
                    .resources
                    .binary_search_by(|resource| resource.path.as_str().cmp(relative))
                    .ok()
                    .map(|index| &revision.selected.resources[index])
            });
            let source = resource.map_or(PermissionSource::Generated, |resource| {
                PermissionSource::Resource {
                    captured_mode: resource.mode,
                }
            });
            // Immutable exports have one fixed creation policy. Prior exports
            // must match that same policy; observations are never rewritten.
            Some(select_file_mode(
                DestinationMode::Absent,
                source,
                ResourceExecutionPolicy::OwnerExecutable,
            )?)
        }
    };
    Ok(DesiredFile {
        location: FileLocation {
            root: scope.clone(),
            path: file.path.clone(),
        },
        value: FileValue {
            bytes: file.bytes.clone(),
            mode,
        },
        consumers: vec![revision.selected.surface],
    })
}
fn logical_owner(graph: &[Installation], revision: &PackageRevision) -> ArtifactOwner {
    graph
        .iter()
        .find(|row| row.owner.installation_id == revision.installation.owner.installation_id)
        .map_or_else(
            || {
                let mut owner = revision.installation.owner.clone();
                // Detached resources have no current logical revision. The exact
                // historical revision remains in the immutable capture record.
                owner.source_revision = None;
                owner
            },
            |row| row.owner.clone(),
        )
}

fn owned(file: &DesiredFile, owner: &ArtifactOwner) -> OwnedArtifact {
    let hash: String = Sha256::digest(&file.value.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    OwnedArtifact {
        root: file.location.root.clone(),
        path: file.location.path.clone(),
        content: ArtifactContent::File {
            content_sha256: hash,
            mode: file.value.mode,
        },
        owners: vec![owner.clone()],
        consumers: file.consumers.clone(),
    }
}

pub(super) fn plan(
    input: &SkillDeliveryInput<'_>,
    checked: &validation::Validated<'_>,
    revisions: &[PackageRevision],
    profile: PermissionProfile,
) -> Result<PlannedSkillFiles> {
    let mut prior = input.current.graph.artifacts.clone();
    let mut selectors = BTreeSet::new();
    for revision in &input.current.revisions {
        let owner = logical_owner(&input.current.graph.installations, revision);
        for file in &revision.package.revision_files {
            prior.push(owned(
                &desired(input.scope, file, revision, profile)?,
                &owner,
            ));
        }
    }
    for receipt in checked.receipts.values() {
        if let Some(selection) = &receipt.after {
            let revision = input
                .current
                .revisions
                .iter()
                .find(|revision| {
                    target(revision) == receipt.key
                        && revision.package.payload_sha256 == selection.payload_sha256
                })
                .expect("validated native selection");
            if let Some(file) = &revision.package.catalog_file {
                selectors.insert(file.path.clone());
                prior.push(owned(
                    &desired(input.scope, file, revision, profile)?,
                    &logical_owner(&input.current.graph.installations, revision),
                ));
            }
        }
    }
    let mut sets: BTreeMap<String, (ArtifactOwner, Vec<DesiredFile>)> = BTreeMap::new();
    for revision in revisions {
        let owner = logical_owner(&checked.graph.installations, revision);
        let row = sets
            .entry(owner.installation_id.clone())
            .or_insert_with(|| (owner, Vec::new()));
        for file in &revision.package.revision_files {
            row.1.push(desired(input.scope, file, revision, profile)?);
        }
    }
    for candidate in checked.candidates.values() {
        let owner = logical_owner(&checked.graph.installations, candidate);
        let row = sets
            .entry(owner.installation_id.clone())
            .or_insert_with(|| (owner, Vec::new()));
        if let Some(file) = &candidate.package.catalog_file {
            selectors.insert(file.path.clone());
            row.1.push(desired(input.scope, file, candidate, profile)?);
        }
    }
    let changes: Vec<_> = sets
        .into_values()
        .map(|(owner, files)| FileInstallationChange::Set { owner, files })
        .collect();
    let result = plan_files(&prior, &changes, input.file_observations)?;
    let (selector_files, stage_files) = result
        .files
        .into_iter()
        .partition(|file| selectors.contains(&file.location.path));
    Ok(PlannedSkillFiles {
        ownership: result.artifacts,
        staging: stage_files,
        selectors: selector_files,
    })
}
