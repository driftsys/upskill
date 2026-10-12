//! Pure plans for complete owned whole-file sets and captured before-state values.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
    validate_artifacts, validate_relative_path,
};
use crate::target::TargetSurface;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLocation {
    pub root: InstallationRoot,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileValue {
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedFile {
    pub location: FileLocation,
    pub value: Option<FileValue>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredFile {
    pub location: FileLocation,
    pub value: FileValue,
    pub consumers: Vec<TargetSurface>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileInstallationChange {
    Set {
        owner: ArtifactOwner,
        files: Vec<DesiredFile>,
    },
    Remove {
        installation_id: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub location: FileLocation,
    pub before: Option<FileValue>,
    pub after: Option<FileValue>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOperationPlan {
    pub artifacts: Vec<OwnedArtifact>,
    pub files: Vec<PlannedFile>,
}
type LocationKey = (OwnershipScope, OsString, String);
type PriorLocations<'a> = BTreeMap<LocationKey, Vec<&'a OwnedArtifact>>;

struct DesiredEntry<'a> {
    file: &'a DesiredFile,
    record: OwnedArtifact,
}

struct IndexedChanges<'a> {
    ids: BTreeSet<&'a str>,
    desired: BTreeMap<LocationKey, DesiredEntry<'a>>,
}

fn location_key(location: &FileLocation) -> LocationKey {
    (
        location.root.scope,
        location.root.path.as_os_str().to_owned(),
        location.path.clone(),
    )
}

fn artifact_key(artifact: &OwnedArtifact) -> LocationKey {
    (
        artifact.root.scope,
        artifact.root.path.as_os_str().to_owned(),
        artifact.path.clone(),
    )
}

fn content_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_observations(observations: &[ObservedFile]) -> Result<()> {
    for observation in observations {
        let validate = || -> Result<()> {
            if observation.location.root.path.as_os_str().is_empty()
                || !observation.location.root.path.is_absolute()
            {
                bail!("installation root must be a nonempty absolute path");
            }
            validate_relative_path(&observation.location.path)?;
            if observation
                .value
                .as_ref()
                .and_then(|value| value.mode)
                .is_some_and(|mode| mode > 0o7777)
            {
                bail!("file mode must contain permission bits only (0..=0o7777)");
            }
            Ok(())
        };
        validate()
            .with_context(|| format!("invalid file observation ({})", observation.location.path))?;
    }
    Ok(())
}

fn desired_record(owner: &ArtifactOwner, file: &DesiredFile) -> Result<OwnedArtifact> {
    let record = OwnedArtifact {
        root: file.location.root.clone(),
        path: file.location.path.clone(),
        content: ArtifactContent::File {
            content_sha256: content_hash(&file.value.bytes),
            mode: file.value.mode,
        },
        owners: vec![owner.clone()],
        consumers: file.consumers.clone(),
    };
    validate_artifacts(std::slice::from_ref(&record)).with_context(|| {
        format!(
            "invalid desired file for installation {}",
            owner.installation_id
        )
    })?;
    Ok(record)
}

pub(crate) fn validate_binding(prior: &ArtifactOwner, desired: &ArtifactOwner) -> Result<()> {
    let changed = if prior.source != desired.source {
        Some("source")
    } else if prior.kind != desired.kind {
        Some("kind")
    } else if prior.source_name != desired.source_name {
        Some("source_name")
    } else if prior.installed_name != desired.installed_name {
        Some("installed_name")
    } else {
        None
    };
    if let Some(field) = changed {
        bail!(
            "installation {} cannot change {field}",
            desired.installation_id
        );
    }
    Ok(())
}

fn index_changes<'a>(
    prior: &[OwnedArtifact],
    changes: &'a [FileInstallationChange],
) -> Result<IndexedChanges<'a>> {
    let bindings: BTreeMap<_, _> = prior
        .iter()
        .flat_map(|row| &row.owners)
        .map(|owner| (owner.installation_id.as_str(), owner))
        .collect();
    let mut indexed = IndexedChanges {
        ids: BTreeSet::new(),
        desired: BTreeMap::new(),
    };
    for change in changes {
        let id = match change {
            FileInstallationChange::Set { owner, .. } => owner.installation_id.as_str(),
            FileInstallationChange::Remove { installation_id } => installation_id.as_str(),
        };
        if !indexed.ids.insert(id) {
            bail!("duplicate file change for installation {id}");
        }
        match change {
            FileInstallationChange::Set { owner, files } => {
                if files.is_empty() {
                    bail!("installation {id} has an empty whole-file set; use Remove");
                }
                if let Some(prior) = bindings.get(id) {
                    validate_binding(prior, owner)?;
                }
                for file in files {
                    let record = desired_record(owner, file)?;
                    let key = location_key(&file.location);
                    if indexed
                        .desired
                        .insert(key, DesiredEntry { file, record })
                        .is_some()
                    {
                        bail!(
                            "duplicate desired location ({}) for installation {id}",
                            file.location.path
                        );
                    }
                }
            }
            FileInstallationChange::Remove { .. } => {
                if !bindings.contains_key(id) {
                    bail!("unknown installation {id} for removal");
                }
            }
        }
    }
    Ok(indexed)
}

fn affected_locations(
    prior: &PriorLocations<'_>,
    changes: &IndexedChanges<'_>,
) -> Result<BTreeSet<LocationKey>> {
    let mut affected: BTreeSet<_> = changes.desired.keys().cloned().collect();
    for (key, rows) in prior {
        if rows.iter().any(|row| {
            row.owners
                .iter()
                .any(|owner| changes.ids.contains(owner.installation_id.as_str()))
        }) {
            affected.insert(key.clone());
        }
    }
    for key in &affected {
        if let Some(rows) = prior.get(key) {
            for row in rows {
                if matches!(row.content, ArtifactContent::MarkdownContribution { .. }) {
                    bail!(
                        "affected Markdown contribution ({}) cannot be planned as a whole file",
                        row.path
                    );
                }
                if row.owners.len() != 1 {
                    bail!(
                        "affected multi-owner whole file ({}) is unsupported",
                        row.path
                    );
                }
                let owner = &row.owners[0];
                if !changes.ids.contains(owner.installation_id.as_str()) {
                    bail!(
                        "desired file ({}) cannot take over untouched owner {}",
                        row.path,
                        owner.installation_id
                    );
                }
            }
        }
    }
    Ok(affected)
}

fn index_observations<'a>(
    observations: &'a [ObservedFile],
    affected: &BTreeSet<LocationKey>,
) -> Result<BTreeMap<LocationKey, &'a ObservedFile>> {
    let mut indexed = BTreeMap::new();
    for observation in observations {
        let key = location_key(&observation.location);
        if indexed.insert(key.clone(), observation).is_some() {
            bail!("duplicate observation ({})", observation.location.path);
        }
        if !affected.contains(&key) {
            bail!("extraneous observation ({})", observation.location.path);
        }
    }
    for key in affected {
        if !indexed.contains_key(key) {
            bail!("missing observation for {:?}:{}", key.1, key.2);
        }
    }
    Ok(indexed)
}

fn is_descendant(parent: &str, child: &str) -> bool {
    child
        .strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_topology(
    prior: &PriorLocations<'_>,
    changes: &IndexedChanges<'_>,
    affected: &BTreeSet<LocationKey>,
) -> Result<()> {
    let union: BTreeSet<_> = prior.keys().chain(changes.desired.keys()).collect();
    let locations: Vec<_> = union.into_iter().collect();
    for (index, left) in locations.iter().enumerate() {
        for right in &locations[index + 1..] {
            if left.0 == right.0
                && left.1 == right.1
                && (affected.contains(*left) || affected.contains(*right))
                && (is_descendant(&left.2, &right.2) || is_descendant(&right.2, &left.2))
            {
                bail!(
                    "file topology conflict at {:?}: {} and {}",
                    left.1,
                    left.2,
                    right.2
                );
            }
        }
    }
    Ok(())
}

fn validate_before(prior: Option<&OwnedArtifact>, observed: &ObservedFile) -> Result<()> {
    if let Some(prior) = prior {
        let owner = &prior.owners[0].installation_id;
        let Some(value) = &observed.value else {
            bail!(
                "missing owned file ({}) for installation {owner}",
                prior.path
            );
        };
        // Affected locations have already rejected contributions and shared files.
        if let ArtifactContent::File {
            content_sha256,
            mode,
        } = &prior.content
        {
            if content_hash(&value.bytes) != *content_sha256 {
                bail!(
                    "owned file bytes changed ({}) for installation {owner}",
                    prior.path
                );
            }
            if value.mode != *mode {
                bail!(
                    "owned file mode evidence changed ({}) for installation {owner}",
                    prior.path
                );
            }
        }
    } else if observed.value.is_some() {
        bail!("unowned file already exists ({})", observed.location.path);
    }
    Ok(())
}

/// Plan complete whole-file replacement sets without I/O or input mutation.
///
/// Supplied observations are evidence, not qualified filesystem provenance.
/// Every affected location requires one exact before-state observation, including
/// no-ops. Deletions identify individual files and grant no directory authority.
/// Failure returns no successful partial plan.
pub fn plan_files(
    prior: &[OwnedArtifact],
    changes: &[FileInstallationChange],
    observations: &[ObservedFile],
) -> Result<FileOperationPlan> {
    validate_artifacts(prior).context("validate prior file ownership")?;
    validate_observations(observations)?;
    let mut changes = index_changes(prior, changes)?;
    let mut prior_locations: PriorLocations<'_> = BTreeMap::new();
    for row in prior {
        prior_locations
            .entry(artifact_key(row))
            .or_default()
            .push(row);
    }
    let affected = affected_locations(&prior_locations, &changes)?;
    let observations = index_observations(observations, &affected)?;
    validate_topology(&prior_locations, &changes, &affected)?;
    let mut files = Vec::with_capacity(affected.len());
    for key in &affected {
        let observed = observations[key];
        let old = prior_locations.get(key).map(|rows| rows[0]);
        validate_before(old, observed)?;
        files.push(PlannedFile {
            location: observed.location.clone(),
            before: observed.value.clone(),
            after: changes
                .desired
                .get(key)
                .map(|entry| entry.file.value.clone()),
        });
    }
    let mut artifacts = Vec::new();
    for row in prior {
        let key = artifact_key(row);
        if !affected.contains(&key) {
            artifacts.push(row.clone());
        } else if let Some(entry) = changes.desired.remove(&key) {
            artifacts.push(entry.record);
        }
    }
    artifacts.extend(changes.desired.into_values().map(|entry| entry.record));
    validate_artifacts(&artifacts).context("validate planned file ownership")?;
    Ok(FileOperationPlan { artifacts, files })
}
