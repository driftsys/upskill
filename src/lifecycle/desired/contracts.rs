//! Captured desired sets with explicitly supplied target contract revisions.
use super::{DesiredInstallationSet, RuleDesiredSetRequest, compile_rule_desired_set};
use crate::lifecycle::contracts::{ConsumerContract, ContractContent, OwnerContract};
use crate::operations::plan::DesiredArtifact;
use crate::target::TargetProfile;
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractedDesiredSet {
    pub set: DesiredInstallationSet,
    pub contracts: Vec<OwnerContract>,
}

/// Compile captured rule bytes and bind explicit supplied revisions to each output.
///
/// Profiles describe recorded contracts, not native qualification. Missing revisions
/// are never filled from defaults. Audience exclusions retain history without proof.
pub fn compile_rule_desired_set_with_contracts(
    request: RuleDesiredSetRequest<'_>,
    profiles: &[TargetProfile],
) -> Result<ContractedDesiredSet> {
    let mut indexed = BTreeMap::new();
    for profile in profiles {
        ensure!(
            profile.family == profile.surface.family(),
            "profile family differs from surface"
        );
        ensure!(
            profile.contract_revision > 0,
            "explicit contract revision must be nonzero"
        );
        ensure!(
            indexed.insert(profile.surface.name(), profile).is_none(),
            "duplicate explicit profile"
        );
    }
    let requested: BTreeSet<_> = request
        .installation
        .requested
        .iter()
        .map(|s| s.name())
        .collect();
    ensure!(
        indexed.keys().copied().collect::<BTreeSet<_>>() == requested,
        "profiles must exactly account for requested surfaces"
    );
    let set = compile_rule_desired_set(request)?;
    let mut contracts = Vec::new();
    for artifact in &set.artifacts {
        let DesiredArtifact::File(file) = artifact else {
            anyhow::bail!("rule compiler returned an unsupported contribution");
        };
        let mut consumers = Vec::new();
        for surface in &file.consumers {
            let profile = indexed
                .get(surface.name())
                .context("output consumer has no explicit profile")?;
            consumers.push(ConsumerContract {
                surface: *surface,
                contract_revision: profile.contract_revision,
            });
        }
        consumers.sort_by_key(|p| p.surface.name());
        contracts.push(OwnerContract {
            root: file.location.root.clone(),
            path: file.location.path.clone(),
            contribution_id: None,
            installation_id: set.installation.owner.installation_id.clone(),
            source_manifest_sha256: request.source.content_sha256.clone(),
            content: ContractContent::File {
                content_sha256: Sha256::digest(&file.value.bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                mode: file.value.mode,
            },
            profiles: consumers,
        });
    }
    Ok(ContractedDesiredSet { set, contracts })
}
