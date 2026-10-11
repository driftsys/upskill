//! Validated records for explicitly owned files and Markdown contributions.
//!
//! The JSON array codec is an internal boundary, independent of lockfile schemas.
//! Validation is lexical: it performs no filesystem access or containment checks.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::instructions::aggregate::OwnedContribution;
use crate::pipeline::ItemKind;
use crate::target::TargetSurface;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnershipScope {
    Project,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallationRoot {
    pub scope: OwnershipScope,
    pub path: PathBuf,
}

/// The source/item/alias binding associated with one installation identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactOwner {
    pub installation_id: String,
    pub source: String,
    pub source_revision: Option<String>,
    pub kind: ItemKind,
    pub source_name: String,
    pub installed_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ArtifactContent {
    File {
        content_sha256: String,
        mode: Option<u32>,
    },
    MarkdownContribution {
        contribution: OwnedContribution,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedArtifact {
    pub root: InstallationRoot,
    pub path: String,
    pub content: ArtifactContent,
    pub owners: Vec<ArtifactOwner>,
    pub consumers: Vec<TargetSurface>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_hash(hash: &str) -> Result<()> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("SHA-256 must contain exactly 64 lowercase hexadecimal characters");
    }
    Ok(())
}

fn validate_artifact(artifact: &OwnedArtifact) -> Result<()> {
    if artifact.root.path.as_os_str().is_empty() || !artifact.root.path.is_absolute() {
        bail!("installation root must be a nonempty absolute path");
    }
    if artifact.path.contains(['\\', ':'])
        || artifact.path.chars().any(char::is_control)
        || artifact
            .path
            .split('/')
            .any(|component| matches!(component, "" | "." | ".."))
    {
        bail!("artifact path must contain portable forward-slash relative components");
    }
    if artifact.owners.is_empty() || artifact.consumers.is_empty() {
        bail!("at least one owner and one consumer are required");
    }
    let mut owner_ids = BTreeSet::new();
    for owner in &artifact.owners {
        if !valid_id(&owner.installation_id) {
            bail!("invalid installation id: {}", owner.installation_id);
        }
        if !owner_ids.insert(&owner.installation_id) {
            bail!("duplicate installation id: {}", owner.installation_id);
        }
        if owner.source.is_empty()
            || owner.source_name.is_empty()
            || owner.installed_name.is_empty()
            || owner.source_revision.as_ref().is_some_and(String::is_empty)
        {
            bail!("owner source, names, and present revision must be nonempty");
        }
    }
    let mut consumers = BTreeSet::new();
    for consumer in &artifact.consumers {
        if !consumers.insert(consumer.name()) {
            bail!("duplicate consumer: {}", consumer.name());
        }
    }
    match &artifact.content {
        ArtifactContent::File {
            content_sha256,
            mode,
        } => {
            validate_hash(content_sha256)?;
            if mode.is_some_and(|mode| mode > 0o7777) {
                bail!("file mode must contain permission bits only (0..=0o7777)");
            }
        }
        ArtifactContent::MarkdownContribution { contribution } => {
            if !valid_id(&contribution.id) {
                bail!("invalid instruction contribution id: {}", contribution.id);
            }
            validate_hash(&contribution.content_sha256)?;
            if contribution.leading_newlines > 2 {
                bail!("contribution leading_newlines must be 0..=2");
            }
        }
    }
    Ok(())
}

fn selector(content: &ArtifactContent) -> Option<&str> {
    match content {
        ArtifactContent::File { .. } => None,
        ArtifactContent::MarkdownContribution { contribution } => Some(&contribution.id),
    }
}

/// Validate records and their shared locations and installation bindings.
///
/// Roots use host absolute-path rules. Location comparisons preserve exact root
/// spellings; they do not normalize components, resolve symlinks, or fold case.
pub fn validate_artifacts(artifacts: &[OwnedArtifact]) -> Result<()> {
    let mut locations = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let mut validate = || -> Result<()> {
            validate_artifact(artifact)?;
            for owner in &artifact.owners {
                if bindings
                    .insert(&owner.installation_id, owner)
                    .is_some_and(|prior| prior != owner)
                {
                    bail!(
                        "conflicting owner binding for installation {}",
                        owner.installation_id
                    );
                }
            }
            // Path equality normalizes components; raw OsStr equality keeps
            // this pure boundary's exact lexical location contract.
            let location = (
                artifact.root.scope,
                artifact.root.path.as_os_str(),
                artifact.path.as_str(),
            );
            let selectors: &mut BTreeSet<Option<&str>> = locations.entry(location).or_default();
            let current = selector(&artifact.content);
            if (current.is_none() && selectors.iter().any(Option::is_some))
                || (current.is_some() && selectors.contains(&None))
            {
                bail!("whole-file and contribution ownership cannot share a location");
            }
            if !selectors.insert(current) {
                bail!("duplicate artifact selector at the same location");
            }
            Ok(())
        };
        validate()
            .with_context(|| format!("invalid owned artifact {index} ({})", artifact.path))?;
    }
    Ok(())
}

/// Parse the internal JSON array and reject invalid or ambiguous ownership.
pub fn parse_artifacts(json: &str) -> Result<Vec<OwnedArtifact>> {
    let artifacts: Vec<OwnedArtifact> =
        serde_json::from_str(json).context("parse owned artifacts")?;
    validate_artifacts(&artifacts).context("validate parsed owned artifacts")?;
    Ok(artifacts)
}

/// Emit deterministic pretty JSON with one trailing newline, without changing input.
pub fn serialize_artifacts(artifacts: &[OwnedArtifact]) -> Result<String> {
    validate_artifacts(artifacts).context("validate owned artifacts for serialization")?;
    let mut ordered = artifacts.to_vec();
    for artifact in &mut ordered {
        artifact
            .owners
            .sort_by(|left, right| left.installation_id.cmp(&right.installation_id));
        artifact.consumers.sort_by_key(|consumer| consumer.name());
    }
    ordered.sort_by(|left, right| {
        (
            left.root.scope,
            left.root.path.as_os_str(),
            &left.path,
            selector(&left.content),
        )
            .cmp(&(
                right.root.scope,
                right.root.path.as_os_str(),
                &right.path,
                selector(&right.content),
            ))
    });
    let mut json = serde_json::to_string_pretty(&ordered).context("serialize owned artifacts")?;
    json.push('\n');
    Ok(json)
}
