use super::{LocationKey, key, selector};
use crate::files::plan::{FileValue, ObservedFile, PlannedFile};
use crate::instructions::aggregate;
use crate::lifecycle::state::CombinedState;
use crate::ownership::{ArtifactContent, validate_relative_path};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn plan(
    prior: &CombinedState,
    state: &mut CombinedState,
    affected: &BTreeSet<LocationKey>,
    observations: &[ObservedFile],
) -> Result<Vec<PlannedFile>> {
    let mut indexed = BTreeMap::new();
    for observation in observations {
        let location = &observation.location;
        ensure!(
            location.root.path.is_absolute(),
            "observation root must be absolute"
        );
        validate_relative_path(&location.path)?;
        ensure!(
            !observation
                .value
                .as_ref()
                .and_then(|v| v.mode)
                .is_some_and(|m| m > 0o7777),
            "invalid observation mode"
        );
        let key = (
            location.root.scope,
            location.root.path.as_os_str().to_owned(),
            location.path.clone(),
        );
        ensure!(affected.contains(&key), "extraneous retirement observation");
        ensure!(
            indexed.insert(key, observation).is_none(),
            "duplicate retirement observation"
        );
    }
    ensure!(
        indexed.len() == affected.len(),
        "missing retirement observation"
    );
    let locations: BTreeSet<_> = prior.artifacts.iter().map(key).collect();
    for left in affected {
        for right in &locations {
            if left.0 == right.0 && left.1 == right.1 && left.2 != right.2 {
                ensure!(
                    !left
                        .2
                        .strip_prefix(&right.2)
                        .is_some_and(|s| s.starts_with('/'))
                        && !right
                            .2
                            .strip_prefix(&left.2)
                            .is_some_and(|s| s.starts_with('/')),
                    "retirement file topology conflict"
                );
            }
        }
    }
    let mut files = Vec::new();
    for location in affected {
        let observed = indexed[location];
        let value = observed
            .value
            .as_ref()
            .context("owned retirement file is absent")?;
        let rows: Vec<_> = prior
            .artifacts
            .iter()
            .filter(|row| &key(row) == location)
            .collect();
        let after = match &rows[0].content {
            ArtifactContent::File {
                content_sha256,
                mode,
            } => {
                let hash: String = Sha256::digest(&value.bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                ensure!(
                    &hash == content_sha256,
                    "owned retirement file bytes changed"
                );
                ensure!(&value.mode == mode, "owned retirement file mode changed");
                state
                    .artifacts
                    .iter()
                    .any(|row| &key(row) == location)
                    .then(|| value.clone())
            }
            ArtifactContent::MarkdownContribution { .. } => {
                let records: Vec<_> = rows
                    .iter()
                    .filter_map(|row| match &row.content {
                        ArtifactContent::MarkdownContribution { contribution } => {
                            Some(contribution.clone())
                        }
                        ArtifactContent::File { .. } => None,
                    })
                    .collect();
                let existing = std::str::from_utf8(&value.bytes)
                    .context("invalid retirement instruction UTF-8")?;
                let mut desired = aggregate::inspect_owned(existing, &records)?;
                desired.retain(|body| {
                    state
                        .artifacts
                        .iter()
                        .any(|row| &key(row) == location && selector(row) == Some(body.id.as_str()))
                });
                let edit = aggregate::reconcile(existing, &records, &desired)?;
                for row in &mut state.artifacts {
                    if &key(row) == location {
                        let id = selector(row).context("instruction selector is missing")?;
                        let contribution = edit
                            .owned
                            .iter()
                            .find(|record| record.id == id)
                            .context("retained instruction metadata is missing")?
                            .clone();
                        row.content = ArtifactContent::MarkdownContribution { contribution };
                    }
                }
                Some(FileValue {
                    bytes: edit.content.into_bytes(),
                    mode: value.mode,
                })
            }
        };
        files.push(PlannedFile {
            location: observed.location.clone(),
            before: observed.value.clone(),
            after,
        });
    }
    Ok(files)
}
