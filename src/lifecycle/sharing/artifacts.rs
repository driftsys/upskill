use super::{Key, desired_key, hash, owned_key, same_content};
use crate::files::plan::validate_binding;
use crate::instructions::aggregate::OwnedContribution;
use crate::lifecycle::state::CombinedState;
use crate::operations::plan::{DesiredArtifact, InstallationChange};
use crate::ownership::{ArtifactContent, ArtifactOwner, OwnedArtifact};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Rebuilt {
    pub artifacts: Vec<OwnedArtifact>,
    pub desired: BTreeMap<Key, DesiredArtifact>,
    pub affected: BTreeSet<Key>,
}
fn content(a: &DesiredArtifact) -> ArtifactContent {
    match a {
        DesiredArtifact::File(f) => ArtifactContent::File {
            content_sha256: hash(&f.value.bytes),
            mode: f.value.mode,
        },
        DesiredArtifact::Instruction(v) => ArtifactContent::MarkdownContribution {
            contribution: OwnedContribution {
                id: v.id.clone(),
                content_sha256: hash(v.markdown.as_bytes()),
                leading_newlines: 0,
                preceding_newlines: 0,
            },
        },
    }
}
fn equal_desired(a: &DesiredArtifact, b: &DesiredArtifact) -> bool {
    match (a, b) {
        (DesiredArtifact::File(a), DesiredArtifact::File(b)) => a.value == b.value,
        (DesiredArtifact::Instruction(a), DesiredArtifact::Instruction(b)) => {
            a.markdown == b.markdown
        }
        _ => false,
    }
}
pub(super) fn rebuild(
    prior: &CombinedState,
    candidate: &CombinedState,
    changes: &[InstallationChange],
) -> Result<Rebuilt> {
    let bindings: BTreeMap<_, _> = prior
        .artifacts
        .iter()
        .flat_map(|a| &a.owners)
        .map(|o| (&o.installation_id, o))
        .collect();
    let mut changed = BTreeSet::new();
    let mut desired = BTreeMap::new();
    let mut owners: BTreeMap<Key, BTreeMap<String, ArtifactOwner>> = BTreeMap::new();
    let mut affected = BTreeSet::new();
    for change in changes {
        let id = match change {
            InstallationChange::Set { owner, .. } => &owner.installation_id,
            InstallationChange::Remove { installation_id } => installation_id,
        };
        ensure!(
            changed.insert(id.clone()),
            "duplicate shared installation change"
        );
        if let InstallationChange::Set { owner, artifacts } = change {
            if let Some(old) = bindings.get(id) {
                validate_binding(old, owner)?;
            }
            for a in artifacts {
                let k = desired_key(a);
                ensure!(
                    owners
                        .entry(k.clone())
                        .or_default()
                        .insert(id.clone(), owner.clone())
                        .is_none(),
                    "duplicate shared desired identity"
                );
                if let Some(previous) = desired.get(&k) {
                    ensure!(
                        equal_desired(previous, a),
                        "desired shared bytes and modes differ"
                    );
                } else {
                    desired.insert(k.clone(), a.clone());
                }
                affected.insert(k);
            }
        }
    }
    let mut result = Vec::new();
    for old in &prior.artifacts {
        let k = owned_key(old);
        let touched = old
            .owners
            .iter()
            .any(|o| changed.contains(&o.installation_id));
        if touched {
            affected.insert(k.clone());
        }
        let mut additions = owners.remove(&k).unwrap_or_default();
        let retained = old
            .owners
            .iter()
            .any(|o| !changed.contains(&o.installation_id));
        if retained {
            if let Some(value) = desired.get(&k) {
                ensure!(
                    same_content(&old.content, &content(value)),
                    "desired content changes an untouched owner"
                );
            }
        }
        let mut row = old.clone();
        row.owners.clear();
        for owner in &old.owners {
            if !changed.contains(&owner.installation_id) {
                row.owners.push(owner.clone());
            } else if let Some(replacement) = additions.remove(&owner.installation_id) {
                row.owners.push(replacement);
            }
        }
        row.owners.extend(additions.into_values());
        if row.owners.is_empty() {
            continue;
        }
        if let Some(value) = desired.get(&k) {
            row.content = content(value);
        }
        if touched || desired.contains_key(&k) {
            set_consumers(&mut row, candidate);
        }
        result.push(row);
    }
    for (k, owners) in owners {
        let value = &desired[&k];
        let (root, path) = match value {
            DesiredArtifact::File(f) => (&f.location.root, &f.location.path),
            DesiredArtifact::Instruction(v) => (&v.location.root, &v.location.path),
        };
        let mut row = OwnedArtifact {
            root: root.clone(),
            path: path.clone(),
            content: content(value),
            owners: owners.into_values().collect(),
            consumers: vec![],
        };
        set_consumers(&mut row, candidate);
        result.push(row);
    }
    Ok(Rebuilt {
        artifacts: result,
        desired,
        affected,
    })
}
fn set_consumers(row: &mut OwnedArtifact, candidate: &CombinedState) {
    let k = owned_key(row);
    let mut consumers = BTreeMap::new();
    for association in &candidate.owner_consumers {
        if super::key(
            &association.root,
            &association.path,
            association.contribution_id.as_deref(),
        ) == k
            && row
                .owners
                .iter()
                .any(|o| o.installation_id == association.installation_id)
        {
            for s in &association.consumers {
                consumers.insert(s.name(), *s);
            }
        }
    }
    row.consumers = consumers.into_values().collect();
}
