//! Private real-owner reconciliation followed by physical planning projection.
mod artifacts;
mod physical;
use super::contracts::{ContractAgreement, check_contract_agreement};
use super::state::{CombinedState, validate_state};
use crate::files::plan::{ObservedFile, PlannedFile};
use crate::operations::plan::{DesiredArtifact, InstallationChange};
use crate::ownership::{ArtifactContent, InstallationRoot, OwnedArtifact, OwnershipScope};
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::ffi::OsString;

type Key = (OwnershipScope, OsString, String, Option<String>);
fn key(root: &InstallationRoot, path: &str, selector: Option<&str>) -> Key {
    (
        root.scope,
        root.path.as_os_str().to_owned(),
        path.into(),
        selector.map(str::to_owned),
    )
}
fn owned_key(a: &OwnedArtifact) -> Key {
    key(
        &a.root,
        &a.path,
        match &a.content {
            ArtifactContent::File { .. } => None,
            ArtifactContent::MarkdownContribution { contribution } => Some(&contribution.id),
        },
    )
}
fn desired_key(a: &DesiredArtifact) -> Key {
    match a {
        DesiredArtifact::File(f) => key(&f.location.root, &f.location.path, None),
        DesiredArtifact::Instruction(v) => key(&v.location.root, &v.location.path, Some(&v.id)),
    }
}
fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn same_content(a: &ArtifactContent, b: &ArtifactContent) -> bool {
    match (a, b) {
        (
            ArtifactContent::File {
                content_sha256: a,
                mode: am,
            },
            ArtifactContent::File {
                content_sha256: b,
                mode: bm,
            },
        ) => a == b && am == bm,
        (
            ArtifactContent::MarkdownContribution { contribution: a },
            ArtifactContent::MarkdownContribution { contribution: b },
        ) => a.id == b.id && a.content_sha256 == b.content_sha256,
        _ => false,
    }
}
pub(crate) fn plan(
    prior: &CombinedState,
    candidate: &mut CombinedState,
    changes: &[InstallationChange],
    observations: &[ObservedFile],
) -> Result<Vec<PlannedFile>> {
    let rebuilt = artifacts::rebuild(prior, candidate, changes)?;
    candidate.artifacts = rebuilt.artifacts;
    validate_state(candidate)?;
    for a in &candidate.artifacts {
        let identity = owned_key(a);
        if rebuilt.affected.contains(&identity) && a.owners.len() > 1 {
            ensure!(
                check_contract_agreement(candidate, &a.root, &a.path, identity.3.as_deref())?
                    == ContractAgreement::Matching,
                "affected shared artifact requires matching complete contracts"
            );
        }
    }
    let files = physical::plan(
        prior,
        candidate,
        &rebuilt.desired,
        &rebuilt.affected,
        observations,
    )?;
    validate_state(candidate)?;
    Ok(files)
}
