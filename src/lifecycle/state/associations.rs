use super::{CombinedState, Installation};
use crate::ownership::{ArtifactContent, InstallationRoot, OwnershipScope};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;

type AssociationKey<'a> = (OwnershipScope, &'a OsStr, &'a str, Option<&'a str>, &'a str);
fn key<'a>(
    root: &'a InstallationRoot,
    path: &'a str,
    selector: Option<&'a str>,
    owner: &'a str,
) -> AssociationKey<'a> {
    (root.scope, root.path.as_os_str(), path, selector, owner)
}

pub(super) fn validate_associations(
    state: &CombinedState,
    installations: &BTreeMap<&str, &Installation>,
) -> Result<()> {
    let mut expected = BTreeSet::new();
    let mut associations = BTreeMap::new();
    for row in &state.owner_consumers {
        let Some(installation) = installations.get(row.installation_id.as_str()) else {
            anyhow::bail!("consumer association names an unknown installation");
        };
        let surfaces: BTreeSet<_> = row.consumers.iter().map(|s| s.name()).collect();
        ensure!(
            !surfaces.is_empty() && surfaces.len() == row.consumers.len(),
            "owner consumers must be nonempty and unique"
        );
        ensure!(
            row.consumers
                .iter()
                .all(|s| installation.requested.contains(s)),
            "owner consumer is outside requested surfaces"
        );
        ensure!(
            associations
                .insert(
                    key(
                        &row.root,
                        &row.path,
                        row.contribution_id.as_deref(),
                        &row.installation_id
                    ),
                    surfaces
                )
                .is_none(),
            "duplicate owner consumer association"
        );
    }
    for artifact in &state.artifacts {
        let selector = match &artifact.content {
            ArtifactContent::File { .. } => None,
            ArtifactContent::MarkdownContribution { contribution } => {
                Some(contribution.id.as_str())
            }
        };
        let mut union = BTreeSet::new();
        for owner in &artifact.owners {
            let Some(installation) = installations.get(owner.installation_id.as_str()) else {
                anyhow::bail!("artifact names an unknown installation");
            };
            ensure!(
                &installation.owner == owner,
                "artifact owner differs from installation binding"
            );
            ensure!(
                installation.root.scope == artifact.root.scope
                    && installation.root.path.as_os_str() == artifact.root.path.as_os_str(),
                "artifact root differs from installation root"
            );
            let binding = key(
                &artifact.root,
                &artifact.path,
                selector,
                &owner.installation_id,
            );
            expected.insert(binding);
            let Some(surfaces) = associations.get(&binding) else {
                anyhow::bail!("artifact owner lacks its consumer association");
            };
            union.extend(surfaces.iter().copied());
        }
        ensure!(
            union == artifact.consumers.iter().map(|s| s.name()).collect(),
            "artifact consumers differ from owner consumer union"
        );
    }
    ensure!(
        expected.len() == associations.len(),
        "consumer association has no artifact owner"
    );
    Ok(())
}
