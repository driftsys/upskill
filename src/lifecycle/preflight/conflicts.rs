//! Reporting-only location projections retain every owner at the inspected location.
use super::{PreflightCategory, PreflightDiagnostic, PreflightReport, normalize_report};
use crate::files::plan::{FileLocation, ObservedFile};
use crate::lifecycle::commands::CapturedUpsert;
use crate::lifecycle::state::{CombinedState, Reference, validate_state};
use crate::operations::plan::{DesiredArtifact, InstallationChange};
use crate::ownership::{InstallationRoot, OwnershipScope, validate_relative_path};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

type Key = (OwnershipScope, OsString, String);
fn key(root: &InstallationRoot, path: &str) -> Key {
    (root.scope, root.path.as_os_str().to_owned(), path.into())
}
fn location(k: &Key) -> FileLocation {
    FileLocation {
        root: InstallationRoot {
            scope: k.0,
            path: k.1.clone().into(),
        },
        path: k.2.clone(),
    }
}
pub(crate) fn artifact_location(artifact: &DesiredArtifact) -> FileLocation {
    match artifact {
        DesiredArtifact::File(f) => f.location.clone(),
        DesiredArtifact::Instruction(i) => FileLocation {
            root: i.location.root.clone(),
            path: i.location.path.clone(),
        },
    }
}
fn diagnostic(
    ids: &BTreeSet<String>,
    k: &Key,
    category: PreflightCategory,
    message: String,
) -> PreflightDiagnostic {
    PreflightDiagnostic {
        installation_ids: ids.iter().cloned().collect(),
        category,
        location: Some(location(k)),
        message,
    }
}

/// Structural checks precede qualified observation. Unsafe groups are excluded,
/// allowing unrelated safe locations to be inspected without guessing their bytes.
pub(crate) fn locations(
    prior: &CombinedState,
    request: &CapturedUpsert,
) -> (Vec<FileLocation>, Vec<PreflightDiagnostic>) {
    let changed: BTreeSet<_> = request
        .sets
        .iter()
        .map(|s| s.installation.owner.installation_id.as_str())
        .collect();
    let mut owners: BTreeMap<Key, BTreeSet<String>> = BTreeMap::new();
    let mut affected = BTreeSet::new();
    for a in &prior.artifacts {
        let k = key(&a.root, &a.path);
        owners
            .entry(k.clone())
            .or_default()
            .extend(a.owners.iter().map(|o| o.installation_id.clone()));
        if a.owners
            .iter()
            .any(|o| changed.contains(o.installation_id.as_str()))
        {
            affected.insert(k);
        }
    }
    for set in &request.sets {
        for artifact in &set.artifacts {
            let output = artifact_location(artifact);
            let k = key(&output.root, &output.path);
            owners
                .entry(k.clone())
                .or_default()
                .insert(set.installation.owner.installation_id.clone());
            affected.insert(k);
        }
    }
    let mut blocked = BTreeSet::new();
    let mut diagnostics = Vec::new();
    for (k, ids) in &owners {
        if !affected.contains(k) {
            continue;
        }
        let invalid = validate_relative_path(&k.2).err().map(|e| format!("{e:#}"));
        let reserved = [
            crate::lockfile::LOCKFILE_NAME,
            crate::lifecycle::application::SCHEMA1_BACKUP_NAME,
            ".upskill/state",
        ]
        .iter()
        .any(|reserved| crate::filesystem::paths_may_overlap(&k.2, reserved));
        if let Some(message) =
            invalid.or_else(|| reserved.then(|| "output overlaps reserved lifecycle path".into()))
        {
            blocked.insert(k.clone());
            diagnostics.push(diagnostic(ids, k, PreflightCategory::Destination, message));
        }
    }
    let keys: Vec<_> = owners.keys().collect();
    for (index, left) in keys.iter().enumerate() {
        for right in &keys[index + 1..] {
            if left.0 != right.0
                || left.1 != right.1
                || (!affected.contains(*left) && !affected.contains(*right))
            {
                continue;
            }
            if crate::filesystem::paths_may_overlap(&left.2, &right.2) {
                let ids = owners[*left].union(&owners[*right]).cloned().collect();
                blocked.insert((*left).clone());
                blocked.insert((*right).clone());
                diagnostics.push(diagnostic(
                    &ids,
                    left,
                    PreflightCategory::Destination,
                    format!(
                        "potentially overlapping destinations: {} and {}",
                        left.2, right.2
                    ),
                ));
            }
        }
    }
    if let Some(legacy) = &prior.legacy {
        let projection = crate::legacy::project_legacy(legacy);
        for k in &affected {
            for row in &projection.items {
                if row.candidates.iter().any(|c| {
                    crate::filesystem::paths_may_overlap(&k.2, &c.entrypoint)
                        || crate::filesystem::paths_may_overlap(&k.2, &c.resource_search_base)
                }) {
                    blocked.insert(k.clone());
                    diagnostics.push(diagnostic(
                        &owners[k],
                        k,
                        PreflightCategory::Ownership,
                        format!("known legacy destination conflict: item {}", row.index),
                    ));
                }
            }
        }
    }
    // Keep unrelated retained locations in the qualified observation inventory:
    // existing preparation captures them as well, and all topology stays explicit.
    (
        owners
            .keys()
            .filter(|k| !blocked.contains(*k))
            .map(location)
            .collect(),
        diagnostics,
    )
}

fn project(prior: &CombinedState, output: &FileLocation) -> CombinedState {
    let mut state = prior.clone();
    state
        .artifacts
        .retain(|a| key(&a.root, &a.path) == key(&output.root, &output.path));
    state
        .owner_consumers
        .retain(|a| key(&a.root, &a.path) == key(&output.root, &output.path));
    state
        .owner_contracts
        .retain(|a| key(&a.root, &a.path) == key(&output.root, &output.path));
    state
}

/// Check every independently observable location without creating a plan for
/// application. Failed source installations remain retained prior owners here.
pub fn collect_conflicts(
    prior: &CombinedState,
    successful: &CapturedUpsert,
    observations: &[ObservedFile],
) -> Result<Vec<PreflightDiagnostic>> {
    validate_state(prior)?;
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for set in &successful.sets {
        let id = &set.installation.owner.installation_id;
        if !seen.insert(id) {
            duplicates.insert(id.clone());
        }
        crate::lifecycle::upsert::validate_set(set)?;
    }
    for id in &duplicates {
        diagnostics.push(PreflightDiagnostic {
            installation_ids: vec![id.clone()],
            category: PreflightCategory::Binding,
            location: None,
            message: "duplicate complete desired set".into(),
        });
    }
    // Duplicate bindings have no trustworthy desired destination. Retain their
    // prior ownership and inspect only the independently compiled other items.
    let mut independent = successful.clone();
    independent
        .sets
        .retain(|s| !duplicates.contains(&s.installation.owner.installation_id));
    independent
        .contracts
        .retain(|c| !duplicates.contains(&c.installation_id));
    let successful = &independent;
    let (locations, structural) = locations(prior, successful);
    diagnostics.extend(structural);
    for output in locations {
        let changed_here = successful
            .sets
            .iter()
            .filter(|s| {
                s.artifacts.iter().any(|a| artifact_location(a) == output)
                    || prior.artifacts.iter().any(|a| {
                        key(&a.root, &a.path) == key(&output.root, &output.path)
                            && a.owners
                                .iter()
                                .any(|o| o.installation_id == s.installation.owner.installation_id)
                    })
            })
            .collect::<Vec<_>>();
        if changed_here.is_empty() {
            continue;
        }
        let before = project(prior, &output);
        let mut candidate = before.clone();
        let mut changes = Vec::new();
        let mut participants = before
            .artifacts
            .iter()
            .flat_map(|a| a.owners.iter().map(|o| o.installation_id.clone()))
            .collect::<BTreeSet<_>>();
        for set in changed_here {
            let id = &set.installation.owner.installation_id;
            participants.insert(id.clone());
            candidate
                .installations
                .retain(|i| i.owner.installation_id != *id);
            candidate.installations.push(set.installation.clone());
            // A local reporting projection needs reachability, not new authority.
            // It is private and is never passed to an applying entry point.
            if !candidate.references.iter().any(|r| matches!(r, Reference::DirectInstallation { installation_id } if installation_id == id)) {
                candidate.references.push(Reference::DirectInstallation { installation_id: id.clone() });
            }
            candidate
                .owner_consumers
                .retain(|a| a.installation_id != *id);
            candidate
                .owner_contracts
                .retain(|a| a.installation_id != *id);
            candidate.owner_consumers.extend(
                crate::lifecycle::upsert::validate_set(set)?
                    .into_iter()
                    .filter(|a| key(&a.root, &a.path) == key(&output.root, &output.path)),
            );
            candidate.owner_contracts.extend(
                successful
                    .contracts
                    .iter()
                    .filter(|a| {
                        a.installation_id == *id
                            && key(&a.root, &a.path) == key(&output.root, &output.path)
                    })
                    .cloned(),
            );
            let artifacts = set
                .artifacts
                .iter()
                .filter(|a| artifact_location(a) == output)
                .cloned()
                .collect::<Vec<_>>();
            if artifacts.is_empty() {
                changes.push(InstallationChange::Remove {
                    installation_id: id.clone(),
                });
            } else {
                changes.push(InstallationChange::Set {
                    owner: set.installation.owner.clone(),
                    artifacts,
                });
            }
        }
        changes.sort_by(|a, b| {
            let id = |c: &InstallationChange| match c {
                InstallationChange::Set { owner, .. } => owner.installation_id.clone(),
                InstallationChange::Remove { installation_id } => installation_id.clone(),
            };
            id(a).cmp(&id(b))
        });
        let observed = observations
            .iter()
            .filter(|o| key(&o.location.root, &o.location.path) == key(&output.root, &output.path))
            .cloned()
            .collect::<Vec<_>>();
        if let Err(error) =
            crate::lifecycle::sharing::plan_reported(&before, &mut candidate, &changes, &observed)
        {
            let (category, error) = match error {
                crate::lifecycle::sharing::SharingFailure::Ownership(error) => {
                    (PreflightCategory::Ownership, error)
                }
                crate::lifecycle::sharing::SharingFailure::Destination(error) => {
                    (PreflightCategory::Destination, error)
                }
            };
            diagnostics.push(diagnostic(
                &participants,
                &key(&output.root, &output.path),
                category,
                format!("{error:#}"),
            ));
        }
    }
    let mut report = PreflightReport {
        diagnostics,
        ..Default::default()
    };
    normalize_report(&mut report);
    Ok(report.diagnostics)
}
