use super::{Key, owned_key, same_content};
use crate::files::plan::{DesiredFile, FileLocation, ObservedFile, PlannedFile};
use crate::instructions::aggregate;
use crate::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use crate::lifecycle::state::CombinedState;
use crate::operations::plan::{DesiredArtifact, InstallationChange, plan_owned_artifacts};
use crate::ownership::{ArtifactContent, ArtifactOwner, OwnedArtifact};
use crate::pipeline::ItemKind;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

fn token(index: usize) -> ArtifactOwner {
    ArtifactOwner {
        installation_id: format!("physical_{index}"),
        source: "physical".into(),
        source_revision: None,
        kind: ItemKind::Rule,
        source_name: "physical".into(),
        installed_name: "physical".into(),
    }
}
pub(super) fn plan(
    prior: &CombinedState,
    candidate: &mut CombinedState,
    desired: &BTreeMap<Key, DesiredArtifact>,
    affected: &BTreeSet<Key>,
    observations: &[ObservedFile],
) -> Result<Vec<PlannedFile>> {
    // One private owner per logical artifact permits reuse of the restrictive
    // physical planner after real ownership and agreement have been validated.
    let keys: BTreeSet<_> = prior
        .artifacts
        .iter()
        .chain(&candidate.artifacts)
        .map(owned_key)
        .collect();
    let tokens: BTreeMap<_, _> = keys
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, token(i)))
        .collect();
    let projected: Vec<_> = prior
        .artifacts
        .iter()
        .map(|a| {
            let mut a = a.clone();
            a.owners = vec![tokens[&owned_key(&a)].clone()];
            a
        })
        .collect();
    let final_rows: BTreeMap<_, _> = candidate
        .artifacts
        .iter()
        .map(|a| (owned_key(a), a))
        .collect();
    let mut changes = Vec::new();
    for k in affected {
        let owner = tokens[k].clone();
        if let Some(row) = final_rows.get(k) {
            let mut value = if let Some(value) = desired.get(k) {
                value.clone()
            } else {
                retained_value(prior, row, observations)?
            };
            match &mut value {
                DesiredArtifact::File(f) => f.consumers = row.consumers.clone(),
                DesiredArtifact::Instruction(v) => v.consumers = row.consumers.clone(),
            }
            changes.push(InstallationChange::Set {
                owner,
                artifacts: vec![value],
            });
        } else {
            changes.push(InstallationChange::Remove {
                installation_id: owner.installation_id,
            });
        }
    }
    let planned = plan_owned_artifacts(&projected, &changes, observations)
        .map_err(|error| real_diagnostic(error, &tokens, prior, candidate))?;
    let mut mapped = BTreeMap::new();
    for row in planned.artifacts {
        ensure!(
            mapped.insert(owned_key(&row), row.content).is_none(),
            "duplicate projected result"
        );
    }
    ensure!(
        mapped.len() == candidate.artifacts.len(),
        "projected result identity count differs"
    );
    for row in &mut candidate.artifacts {
        let content = mapped
            .remove(&owned_key(row))
            .context("projected result missing identity")?;
        ensure!(
            same_content(&row.content, &content),
            "projected content differs from real candidate"
        );
        row.content = content;
    }
    ensure!(mapped.is_empty(), "unexpected projected result identity");
    Ok(planned.files)
}
fn retained_value(
    prior: &CombinedState,
    row: &OwnedArtifact,
    observations: &[ObservedFile],
) -> Result<DesiredArtifact> {
    let observed = observations
        .iter()
        .find(|o| {
            o.location.root.scope == row.root.scope
                && o.location.root.path.as_os_str() == row.root.path.as_os_str()
                && o.location.path == row.path
        })
        .context("missing retained observation")?;
    let value = observed
        .value
        .as_ref()
        .context("retained output is absent")?;
    Ok(match &row.content {
        ArtifactContent::File { .. } => DesiredArtifact::File(DesiredFile {
            location: FileLocation {
                root: row.root.clone(),
                path: row.path.clone(),
            },
            value: value.clone(),
            consumers: row.consumers.clone(),
        }),
        ArtifactContent::MarkdownContribution { contribution } => {
            let records = prior
                .artifacts
                .iter()
                .filter(|a| {
                    a.root.scope == row.root.scope
                        && a.root.path.as_os_str() == row.root.path.as_os_str()
                        && a.path == row.path
                })
                .filter_map(|a| match &a.content {
                    ArtifactContent::MarkdownContribution { contribution } => {
                        Some(contribution.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let bodies = aggregate::inspect_owned(std::str::from_utf8(&value.bytes)?, &records)?;
            let body = bodies
                .into_iter()
                .find(|v| v.id == contribution.id)
                .context("missing retained contribution")?;
            DesiredArtifact::Instruction(DesiredInstructionContribution {
                location: InstructionLocation {
                    root: row.root.clone(),
                    path: row.path.clone(),
                },
                id: body.id,
                markdown: body.markdown,
                consumers: row.consumers.clone(),
            })
        }
    })
}

// The lower planner uses private installation IDs in its error context. Render
// those contexts with authoritative real owners before the chain crosses this API.
fn real_diagnostic(
    error: anyhow::Error,
    tokens: &BTreeMap<Key, ArtifactOwner>,
    prior: &CombinedState,
    candidate: &CombinedState,
) -> anyhow::Error {
    let mut message = format!("{error:#}");
    for (key, token) in tokens {
        let names: BTreeSet<_> = prior
            .artifacts
            .iter()
            .chain(&candidate.artifacts)
            .filter(|a| owned_key(a) == *key)
            .flat_map(|a| a.owners.iter().map(|o| o.installation_id.as_str()))
            .collect();
        let replacement = format!(
            "artifact owners [{}]",
            names.into_iter().collect::<Vec<_>>().join(", ")
        );
        let needle = format!("installation {}", token.installation_id);
        let mut offset = 0;
        while let Some(relative) = message[offset..].find(&needle) {
            let start = offset + relative;
            let end = start + needle.len();
            if message
                .as_bytes()
                .get(end)
                .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            {
                offset = end;
                continue;
            }
            message.replace_range(start..end, &replacement);
            offset = start + replacement.len();
        }
    }
    anyhow::anyhow!(message)
}
