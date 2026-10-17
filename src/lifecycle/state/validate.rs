use super::{CombinedState, Reference, SourceEvidence};
use anyhow::{Result, bail, ensure};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

/// Validate graph authority and recorded evidence without I/O or input mutation.
///
/// A valid state does not establish containment, source authenticity or native support.
pub fn validate_state(state: &CombinedState) -> Result<()> {
    ensure!(state.schema == 2, "combined state requires schema 2");
    if let Some(origin) = &state.origin {
        origin.validate(state.legacy.as_ref())?;
    }
    let mut installations = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for item in &state.installations {
        let owner = &item.owner;
        ensure!(valid_id(&owner.installation_id), "invalid installation id");
        ensure!(
            installations
                .insert(owner.installation_id.as_str(), item)
                .is_none(),
            "duplicate installation id"
        );
        ensure!(
            !owner.source.is_empty()
                && !owner.source_name.is_empty()
                && !owner.installed_name.is_empty(),
            "installation source and names must be nonempty"
        );
        ensure!(
            item.root.path.is_absolute(),
            "installation root must be absolute"
        );
        let identity = (
            item.root.scope,
            item.root.path.as_os_str(),
            &owner.source,
            owner.kind,
            &owner.source_name,
            &owner.installed_name,
        );
        ensure!(
            identities.insert(identity),
            "duplicate installation identity"
        );
        let requested: BTreeSet<_> = item.requested.iter().map(|s| s.name()).collect();
        ensure!(
            !requested.is_empty() && requested.len() == item.requested.len(),
            "requested surfaces must be nonempty and unique"
        );
        let hash = match &item.source_evidence {
            SourceEvidence::Local { manifest_sha256 } => {
                ensure!(
                    owner.source_revision.is_none(),
                    "local installation must not claim a revision"
                );
                manifest_sha256
            }
            SourceEvidence::Git {
                resolved_revision,
                manifest_sha256,
            } => {
                ensure!(
                    !resolved_revision.is_empty()
                        && owner.source_revision.as_ref() == Some(resolved_revision),
                    "Git revision must match resolved evidence"
                );
                manifest_sha256
            }
        };
        ensure!(valid_hash(hash), "invalid source manifest SHA-256");
    }
    let mut bundles = BTreeSet::new();
    for bundle in &state.bundles {
        ensure!(valid_id(&bundle.id), "invalid bundle id");
        ensure!(bundles.insert(bundle.id.as_str()), "duplicate bundle id");
        ensure!(
            !bundle.source.is_empty() && !bundle.name.is_empty(),
            "bundle source and name must be nonempty"
        );
        ensure!(
            !bundle
                .source_revision
                .as_ref()
                .is_some_and(String::is_empty),
            "bundle revision must be nonempty when present"
        );
    }
    let mut seen = BTreeSet::new();
    let mut reachable_items = BTreeSet::new();
    let mut reachable_bundles = BTreeSet::new();
    let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut members: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut indegrees: BTreeMap<&str, usize> = bundles.iter().map(|id| (*id, 0)).collect();
    for edge in &state.references {
        ensure!(seen.insert(edge), "duplicate reference");
        match edge {
            Reference::DirectInstallation { installation_id } => {
                ensure!(
                    installations.contains_key(installation_id.as_str()),
                    "dangling installation reference"
                );
                reachable_items.insert(installation_id.as_str());
            }
            Reference::DirectBundle { bundle_id } => {
                ensure!(
                    bundles.contains(bundle_id.as_str()),
                    "dangling bundle reference"
                );
                reachable_bundles.insert(bundle_id.as_str());
            }
            Reference::BundleInstallation {
                bundle_id,
                installation_id,
            } => {
                ensure!(
                    bundles.contains(bundle_id.as_str())
                        && installations.contains_key(installation_id.as_str()),
                    "dangling bundle installation reference"
                );
                members.entry(bundle_id).or_default().push(installation_id);
            }
            Reference::BundleDependency {
                bundle_id,
                dependency_id,
            } => {
                ensure!(
                    bundles.contains(bundle_id.as_str())
                        && bundles.contains(dependency_id.as_str()),
                    "dangling bundle dependency"
                );
                outgoing.entry(bundle_id).or_default().push(dependency_id);
                *indegrees
                    .get_mut(dependency_id.as_str())
                    .expect("validated endpoint") += 1;
            }
        }
    }
    // Kahn traversal avoids recursion limits on caller-supplied dependency graphs.
    let mut queue: VecDeque<_> = indegrees
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect();
    let mut visited = 0;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        for dependency in outgoing.get(id).into_iter().flatten() {
            let count = indegrees.get_mut(dependency).expect("validated endpoint");
            *count -= 1;
            if *count == 0 {
                queue.push_back(*dependency);
            }
        }
    }
    ensure!(visited == bundles.len(), "cyclic bundle references");
    let mut queue: VecDeque<_> = reachable_bundles.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        reachable_items.extend(members.get(id).into_iter().flatten().copied());
        for dependency in outgoing.get(id).into_iter().flatten() {
            if reachable_bundles.insert(*dependency) {
                queue.push_back(*dependency);
            }
        }
    }
    ensure!(
        reachable_items.len() == installations.len(),
        "unreachable installation"
    );
    ensure!(
        reachable_bundles.len() == bundles.len(),
        "unreachable bundle"
    );
    if state
        .legacy
        .as_ref()
        .is_some_and(|legacy| legacy.schema != 1)
    {
        bail!("retained legacy state requires schema 1");
    }
    crate::ownership::validate_artifacts(&state.artifacts)?;
    super::associations::validate_associations(state, &installations)?;
    super::super::contracts::validate_contracts(state)?;
    Ok(())
}
