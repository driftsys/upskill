use super::{CombinedState, validate_state};
use crate::lockfile::Lockfile;
use crate::ownership::ArtifactContent;
use anyhow::{Context, Result, ensure};

/// Decode and validate schema 2. This neither reads files nor adopts legacy output.
pub fn parse_state(json: &str) -> Result<CombinedState> {
    let state = serde_json::from_str(json).context("parse combined lifecycle state")?;
    validate_state(&state).context("validate parsed combined lifecycle state")?;
    Ok(state)
}

/// Serialize a validated clone deterministically, preserving retained legacy order.
pub fn serialize_state(state: &CombinedState) -> Result<String> {
    validate_state(state).context("validate combined lifecycle state for serialization")?;
    let mut ordered = state.clone();
    ordered
        .installations
        .sort_by(|a, b| a.owner.installation_id.cmp(&b.owner.installation_id));
    for installation in &mut ordered.installations {
        installation.requested.sort_by_key(|s| s.name());
    }
    ordered.bundles.sort_by(|a, b| a.id.cmp(&b.id));
    ordered.references.sort();
    for artifact in &mut ordered.artifacts {
        artifact
            .owners
            .sort_by(|a, b| a.installation_id.cmp(&b.installation_id));
        artifact.consumers.sort_by_key(|s| s.name());
    }
    ordered.artifacts.sort_by(|a, b| {
        (
            a.root.scope,
            a.root.path.as_os_str(),
            &a.path,
            selector(&a.content),
        )
            .cmp(&(
                b.root.scope,
                b.root.path.as_os_str(),
                &b.path,
                selector(&b.content),
            ))
    });
    for association in &mut ordered.owner_consumers {
        association.consumers.sort_by_key(|s| s.name());
    }
    ordered.owner_consumers.sort_by(|a, b| {
        (
            a.root.scope,
            a.root.path.as_os_str(),
            &a.path,
            &a.contribution_id,
            &a.installation_id,
        )
            .cmp(&(
                b.root.scope,
                b.root.path.as_os_str(),
                &b.path,
                &b.contribution_id,
                &b.installation_id,
            ))
    });
    for contract in &mut ordered.owner_contracts {
        contract.profiles.sort_by_key(|p| p.surface.name());
    }
    ordered
        .owner_contracts
        .sort_by_key(super::super::contracts::key);
    let mut json =
        serde_json::to_string_pretty(&ordered).context("serialize combined lifecycle state")?;
    json.push('\n');
    Ok(json)
}

/// Retain the typed schema-1 snapshot without creating owners or inferring selection.
///
/// Unknown JSON fields and original serialization bytes are not part of this typed
/// projection. The applying lifecycle must preserve those bytes in its exact backup.
pub fn project_legacy(lockfile: &Lockfile) -> Result<CombinedState> {
    ensure!(lockfile.schema == 1, "legacy projection requires schema 1");
    Ok(CombinedState {
        origin: None,
        owner_contracts: Vec::new(),
        schema: 2,
        installations: Vec::new(),
        bundles: Vec::new(),
        references: Vec::new(),
        artifacts: Vec::new(),
        owner_consumers: Vec::new(),
        legacy: Some(lockfile.clone()),
    })
}

fn selector(content: &ArtifactContent) -> Option<&str> {
    match content {
        ArtifactContent::File { .. } => None,
        ArtifactContent::MarkdownContribution { contribution } => Some(&contribution.id),
    }
}
