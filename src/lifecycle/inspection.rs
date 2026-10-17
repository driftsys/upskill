//! Read-only lifecycle descriptions over captured bytes and qualified observations.
//! These APIs do not establish ownership, source freshness or native discovery.

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};

use crate::files::plan::{FileLocation, ObservedFile};
use crate::filesystem::QualifiedScope;
use crate::legacy::LegacyProjection;
use crate::lifecycle::state::{CombinedState, parse_state, project_legacy, validate_state};
use crate::ownership::{ArtifactContent, InstallationRoot};

/// Validated list state plus descriptive legacy rows and exact original bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInspection {
    pub original_bytes: Vec<u8>,
    pub state: CombinedState,
    pub legacy: Option<LegacyProjection>,
}

/// One artifact's observed drift, independent of any repair authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactDiagnostic {
    pub artifact_index: usize,
    pub location: FileLocation,
    pub issue: ArtifactIssue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactIssue {
    Unobserved,
    Missing,
    ContentChanged,
    ModeChanged,
    ModeUnknown,
    ContributionChanged { message: String },
}

/// Qualified sampled inspection; pending recovery is reported, never performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeInspection {
    pub state: Option<StateInspection>,
    pub diagnostics: Vec<ArtifactDiagnostic>,
    pub pending_recovery: bool,
}

/// Describe schema 1 or strict schema 2 without fetching or rewriting anything.
/// Unknown schema-1 JSON fields remain in `original_bytes`, not the typed projection.
pub fn inspect_state(bytes: &[u8]) -> Result<StateInspection> {
    let text = std::str::from_utf8(bytes).context("lifecycle state is not UTF-8")?;
    let json: serde_json::Value = serde_json::from_str(text).context("parse lifecycle state")?;
    let state = match json.get("schema").and_then(serde_json::Value::as_u64) {
        Some(1) => project_legacy(&serde_json::from_str(text).context("parse legacy state")?)?,
        Some(2) => parse_state(text)?,
        other => bail!("unsupported lifecycle state schema: {other:?}"),
    };
    let legacy = state.legacy.as_ref().map(crate::legacy::project_legacy);
    Ok(StateInspection {
        original_bytes: bytes.to_vec(),
        state,
        legacy,
    })
}

/// Inspect supplied output evidence, distinguishing absent evidence from missing files.
/// Extra observations are permitted; duplicate locations are rejected. Shared
/// contributions are checked individually using the same verifier as reconciliation.
pub fn doctor(
    inspection: &StateInspection,
    observations: &[ObservedFile],
) -> Result<Vec<ArtifactDiagnostic>> {
    validate_state(&inspection.state)?;
    for (index, observation) in observations.iter().enumerate() {
        ensure!(
            !observations[..index]
                .iter()
                .any(|prior| prior.location == observation.location),
            "duplicate lifecycle output observation"
        );
    }
    let mut diagnostics = Vec::new();
    for (artifact_index, artifact) in inspection.state.artifacts.iter().enumerate() {
        let location = FileLocation {
            root: artifact.root.clone(),
            path: artifact.path.clone(),
        };
        let mut issues = Vec::new();
        match observations.iter().find(|entry| entry.location == location) {
            None => issues.push(ArtifactIssue::Unobserved),
            Some(ObservedFile { value: None, .. }) => issues.push(ArtifactIssue::Missing),
            Some(ObservedFile {
                value: Some(value), ..
            }) => match &artifact.content {
                ArtifactContent::File {
                    content_sha256,
                    mode,
                } => {
                    let hash: String = Sha256::digest(&value.bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    if &hash != content_sha256 {
                        issues.push(ArtifactIssue::ContentChanged);
                    }
                    if let Some(expected) = mode {
                        match value.mode {
                            None => issues.push(ArtifactIssue::ModeUnknown),
                            Some(actual) if actual != *expected => {
                                issues.push(ArtifactIssue::ModeChanged)
                            }
                            Some(_) => {}
                        }
                    }
                }
                ArtifactContent::MarkdownContribution { .. } => {
                    let records: Vec<_> = inspection
                        .state
                        .artifacts
                        .iter()
                        .filter(|other| other.root == artifact.root && other.path == artifact.path)
                        .filter_map(|other| match &other.content {
                            ArtifactContent::MarkdownContribution { contribution } => {
                                Some(contribution.clone())
                            }
                            ArtifactContent::File { .. } => None,
                        })
                        .collect();
                    let verified = std::str::from_utf8(&value.bytes)
                        .context("instruction output is not UTF-8")
                        .and_then(|text| {
                            crate::instructions::aggregate::inspect_owned(text, &records)
                        });
                    if let Err(error) = verified {
                        issues.push(ArtifactIssue::ContributionChanged {
                            message: format!("{error:#}"),
                        });
                    }
                }
            },
        }
        diagnostics.extend(issues.into_iter().map(|issue| ArtifactDiagnostic {
            artifact_index,
            location: location.clone(),
            issue,
        }));
    }
    Ok(diagnostics)
}

/// Observe one scope's state and owned outputs without writer locks or application.
/// State, destinations and root bindings are rechecked before returning. This is
/// observational sampling, not a transaction or a guarantee against later edits.
/// Unsafe evidence, foreign artifact roots and malformed journals return errors.
pub fn inspect_scope(scope: &QualifiedScope, root: &InstallationRoot) -> Result<ScopeInspection> {
    let state_location = FileLocation {
        root: root.clone(),
        path: crate::lockfile::LOCKFILE_NAME.into(),
    };
    let state_snapshot = scope.observe_destinations(&[state_location])?;
    let state = state_snapshot.observations()[0]
        .value
        .as_ref()
        .map(|value| inspect_state(&value.bytes))
        .transpose()?;
    let mut locations = Vec::new();
    if let Some(state) = &state {
        for artifact in &state.state.artifacts {
            ensure!(
                &artifact.root == root,
                "lifecycle artifact belongs to another scope"
            );
            let location = FileLocation {
                root: artifact.root.clone(),
                path: artifact.path.clone(),
            };
            if !locations.contains(&location) {
                locations.push(location);
            }
        }
    }
    let outputs = scope.observe_destinations(&locations)?;
    let diagnostics = state
        .as_ref()
        .map(|state| doctor(state, outputs.observations()))
        .transpose()?
        .unwrap_or_default();
    let pending_recovery = scope.has_pending_operation()?;
    outputs.verify_unchanged()?;
    state_snapshot.verify_unchanged()?;
    Ok(ScopeInspection {
        state,
        diagnostics,
        pending_recovery,
    })
}
