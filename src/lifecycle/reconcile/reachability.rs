use crate::lifecycle::state::Reference;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn reachable(references: &[Reference]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut installations = BTreeSet::new();
    let mut bundles = BTreeSet::new();
    let mut dependencies: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut members: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for reference in references {
        match reference {
            Reference::DirectInstallation { installation_id } => {
                installations.insert(installation_id.clone());
            }
            Reference::DirectBundle { bundle_id } => {
                bundles.insert(bundle_id.clone());
            }
            Reference::BundleInstallation {
                bundle_id,
                installation_id,
            } => {
                members.entry(bundle_id).or_default().push(installation_id);
            }
            Reference::BundleDependency {
                bundle_id,
                dependency_id,
            } => {
                dependencies
                    .entry(bundle_id)
                    .or_default()
                    .push(dependency_id);
            }
        }
    }
    let mut queue: VecDeque<_> = bundles.iter().cloned().collect();
    while let Some(id) = queue.pop_front() {
        installations.extend(
            members
                .get(id.as_str())
                .into_iter()
                .flatten()
                .map(|id| (*id).to_owned()),
        );
        for dependency in dependencies.get(id.as_str()).into_iter().flatten() {
            if bundles.insert((*dependency).to_owned()) {
                queue.push_back((*dependency).to_owned());
            }
        }
    }
    (installations, bundles)
}
