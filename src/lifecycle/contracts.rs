//! Recorded per-owner evidence and conservative agreement, without native authority.
use super::state::{CombinedState, SourceEvidence, validate_state};
use crate::ownership::{ArtifactContent, InstallationRoot, OwnershipScope};
use crate::pipeline::ItemKind;
use crate::target::TargetSurface;
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerContract {
    pub surface: TargetSurface,
    pub contract_revision: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ContractContent {
    File {
        content_sha256: String,
        mode: Option<u32>,
    },
    Instruction {
        content_sha256: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerContract {
    pub root: InstallationRoot,
    pub path: String,
    pub contribution_id: Option<String>,
    pub installation_id: String,
    pub source_manifest_sha256: String,
    pub content: ContractContent,
    pub profiles: Vec<ConsumerContract>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractIssue {
    MissingEvidence,
    SourceManifestMismatch { reference_installation_id: String },
    TargetProfilesMismatch { reference_installation_id: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractConflict {
    pub installation_id: String,
    pub issue: ContractIssue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractAgreement {
    Matching,
    Blocked(Vec<ContractConflict>),
}

pub(crate) type ContractKey = (OwnershipScope, OsString, String, Option<String>, String);
fn binding_key(
    root: &InstallationRoot,
    path: &str,
    selector: Option<&str>,
    owner: &str,
) -> ContractKey {
    (
        root.scope,
        root.path.as_os_str().to_owned(),
        path.into(),
        selector.map(str::to_owned),
        owner.into(),
    )
}
pub(crate) fn key(c: &OwnerContract) -> ContractKey {
    binding_key(
        &c.root,
        &c.path,
        c.contribution_id.as_deref(),
        &c.installation_id,
    )
}
fn selector(content: &ArtifactContent) -> Option<&str> {
    match content {
        ArtifactContent::File { .. } => None,
        ArtifactContent::MarkdownContribution { contribution } => Some(&contribution.id),
    }
}
fn same_location(root: &InstallationRoot, path: &str, id: Option<&str>, c: &OwnerContract) -> bool {
    root.scope == c.root.scope
        && root.path.as_os_str() == c.root.path.as_os_str()
        && path == c.path
        && id == c.contribution_id.as_deref()
}
fn content(value: &ArtifactContent) -> ContractContent {
    match value {
        ArtifactContent::File {
            content_sha256,
            mode,
        } => ContractContent::File {
            content_sha256: content_sha256.clone(),
            mode: *mode,
        },
        ArtifactContent::MarkdownContribution { contribution } => ContractContent::Instruction {
            content_sha256: contribution.content_sha256.clone(),
        },
    }
}
fn profiles(c: &OwnerContract) -> BTreeMap<&'static str, u32> {
    c.profiles
        .iter()
        .map(|p| (p.surface.name(), p.contract_revision))
        .collect()
}

pub(crate) fn validate_contracts(state: &CombinedState) -> Result<()> {
    let associations: BTreeMap<_, _> = state
        .owner_consumers
        .iter()
        .map(|a| {
            (
                binding_key(
                    &a.root,
                    &a.path,
                    a.contribution_id.as_deref(),
                    &a.installation_id,
                ),
                a,
            )
        })
        .collect();
    let mut seen = BTreeSet::new();
    for c in &state.owner_contracts {
        let k = key(c);
        ensure!(seen.insert(k.clone()), "duplicate owner contract");
        let association = associations
            .get(&k)
            .ok_or_else(|| anyhow!("owner contract has no consumer association"))?;
        let installation = state
            .installations
            .iter()
            .find(|i| i.owner.installation_id == c.installation_id)
            .ok_or_else(|| anyhow!("owner contract names unknown installation"))?;
        ensure!(
            installation.owner.kind == ItemKind::Rule,
            "owner contracts currently support only rules"
        );
        let source = match &installation.source_evidence {
            SourceEvidence::Local { manifest_sha256 }
            | SourceEvidence::Git {
                manifest_sha256, ..
            } => manifest_sha256,
        };
        ensure!(
            source == &c.source_manifest_sha256,
            "owner contract differs from source evidence"
        );
        let artifact = state
            .artifacts
            .iter()
            .find(|a| same_location(&a.root, &a.path, selector(&a.content), c))
            .ok_or_else(|| anyhow!("owner contract has no artifact"))?;
        ensure!(
            content(&artifact.content) == c.content,
            "owner contract differs from artifact content"
        );
        let map = profiles(c);
        ensure!(
            map.len() == c.profiles.len()
                && map.len() == association.consumers.len()
                && association
                    .consumers
                    .iter()
                    .all(|s| map.contains_key(s.name())),
            "contract profiles must exactly account for owner consumers"
        );
        ensure!(
            c.profiles.iter().all(|p| p.contract_revision > 0),
            "contract revision must be nonzero"
        );
    }
    Ok(())
}

/// Compare every recorded owner's supplied evidence at one exact artifact identity.
///
/// Missing evidence stays unknown. Matching requires exact captured source and
/// complete surface/revision-map equality; it establishes no native compatibility,
/// source authenticity, physical equivalence, new ownership or execution authority.
pub fn check_contract_agreement(
    state: &CombinedState,
    root: &InstallationRoot,
    path: &str,
    contribution_id: Option<&str>,
) -> Result<ContractAgreement> {
    validate_state(state)?;
    let artifact = state
        .artifacts
        .iter()
        .find(|a| {
            a.root.scope == root.scope
                && a.root.path.as_os_str() == root.path.as_os_str()
                && a.path == path
                && selector(&a.content) == contribution_id
        })
        .ok_or_else(|| anyhow!("unknown artifact contract identity"))?;
    let contracts: BTreeMap<_, _> = state.owner_contracts.iter().map(|c| (key(c), c)).collect();
    let mut baseline: Option<&OwnerContract> = None;
    let mut conflicts = Vec::new();
    for owner in &artifact.owners {
        let id = &owner.installation_id;
        let Some(c) = contracts
            .get(&binding_key(root, path, contribution_id, id))
            .copied()
        else {
            conflicts.push(ContractConflict {
                installation_id: id.clone(),
                issue: ContractIssue::MissingEvidence,
            });
            continue;
        };
        if let Some(reference) = baseline {
            if reference.source_manifest_sha256 != c.source_manifest_sha256 {
                conflicts.push(ContractConflict {
                    installation_id: id.clone(),
                    issue: ContractIssue::SourceManifestMismatch {
                        reference_installation_id: reference.installation_id.clone(),
                    },
                });
            }
            if profiles(reference) != profiles(c) {
                conflicts.push(ContractConflict {
                    installation_id: id.clone(),
                    issue: ContractIssue::TargetProfilesMismatch {
                        reference_installation_id: reference.installation_id.clone(),
                    },
                });
            }
        } else {
            baseline = Some(c);
        }
    }
    if conflicts.is_empty() {
        Ok(ContractAgreement::Matching)
    } else {
        Ok(ContractAgreement::Blocked(conflicts))
    }
}
