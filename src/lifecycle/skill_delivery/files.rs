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
fn value(file: &crate::compile::item::CompiledItemFile) -> FileValue {
    FileValue {
        bytes: file.bytes.clone(),
        mode: file.mode,
    }
}
fn desired(
    scope: &InstallationRoot,
    file: &crate::compile::item::CompiledItemFile,
    surface: crate::target::TargetSurface,
) -> DesiredFile {
    DesiredFile {
        location: FileLocation {
            root: scope.clone(),
            path: file.path.clone(),
        },
        value: value(file),
        consumers: vec![surface],
    }
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
) -> Result<PlannedSkillFiles> {
    let mut prior = input.current.graph.artifacts.clone();
    let mut selectors = BTreeSet::new();
    for revision in &input.current.revisions {
        let owner = logical_owner(&input.current.graph.installations, revision);
        for file in &revision.package.revision_files {
            prior.push(owned(
                &desired(input.scope, file, revision.selected.surface),
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
                    &desired(input.scope, file, revision.selected.surface),
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
        row.1.extend(
            revision
                .package
                .revision_files
                .iter()
                .map(|file| desired(input.scope, file, revision.selected.surface)),
        );
    }
    for candidate in checked.candidates.values() {
        let owner = logical_owner(&checked.graph.installations, candidate);
        let row = sets
            .entry(owner.installation_id.clone())
            .or_insert_with(|| (owner, Vec::new()));
        if let Some(file) = &candidate.package.catalog_file {
            selectors.insert(file.path.clone());
            row.1
                .push(desired(input.scope, file, candidate.selected.surface));
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
