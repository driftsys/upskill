//! Pure complete mixed owned-artifact plans.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

use anyhow::{Context, Result, bail};

use crate::files::plan::{
    DesiredFile, FileInstallationChange, FileLocation, FileOperationPlan, FileValue, ObservedFile,
    PlannedFile, plan_files,
};
use crate::instructions::plan::{
    DesiredInstructionContribution, InstructionInstallationChange, InstructionLocation,
    ObservedInstructionFile, plan_instructions,
};
use crate::ownership::{
    ArtifactContent, ArtifactOwner, InstallationRoot, OwnedArtifact, OwnershipScope,
    validate_artifacts, validate_relative_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesiredArtifact {
    File(DesiredFile),
    Instruction(DesiredInstructionContribution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallationChange {
    Set {
        owner: ArtifactOwner,
        artifacts: Vec<DesiredArtifact>,
    },
    Remove {
        installation_id: String,
    },
}

type LocationKey = (OwnershipScope, OsString, String);
type ArtifactKey = (LocationKey, Option<String>);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Form {
    File,
    Instruction,
}

struct Locations {
    forms: BTreeMap<LocationKey, BTreeSet<Form>>,
    affected: BTreeSet<LocationKey>,
}

fn location_key(root: &InstallationRoot, path: &str) -> LocationKey {
    (root.scope, root.path.as_os_str().to_owned(), path.into())
}

fn artifact_key(row: &OwnedArtifact) -> ArtifactKey {
    let selector = match &row.content {
        ArtifactContent::File { .. } => None,
        ArtifactContent::MarkdownContribution { contribution } => Some(contribution.id.clone()),
    };
    (location_key(&row.root, &row.path), selector)
}

fn form(row: &OwnedArtifact) -> Form {
    match row.content {
        ArtifactContent::File { .. } => Form::File,
        ArtifactContent::MarkdownContribution { .. } => Form::Instruction,
    }
}

fn validate_location(root: &InstallationRoot, path: &str) -> Result<()> {
    if root.path.as_os_str().is_empty() || !root.path.is_absolute() {
        bail!("installation root must be a nonempty absolute path");
    }
    validate_relative_path(path)
}

fn validate_mode(mode: Option<u32>) -> Result<()> {
    if mode.is_some_and(|mode| mode > 0o7777) {
        bail!("file mode must contain permission bits only (0..=0o7777)");
    }
    Ok(())
}

fn validate_binding(prior: &ArtifactOwner, desired: &ArtifactOwner) -> Result<()> {
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

fn index_locations(prior: &[OwnedArtifact], changes: &[InstallationChange]) -> Result<Locations> {
    let bindings: BTreeMap<_, _> = prior
        .iter()
        .flat_map(|row| &row.owners)
        .map(|owner| (owner.installation_id.as_str(), owner))
        .collect();
    let mut changed = BTreeSet::new();
    let mut locations = Locations {
        forms: BTreeMap::new(),
        affected: BTreeSet::new(),
    };
    for row in prior {
        locations
            .forms
            .entry(location_key(&row.root, &row.path))
            .or_default()
            .insert(form(row));
    }
    for change in changes {
        let id = match change {
            InstallationChange::Set { owner, .. } => owner.installation_id.as_str(),
            InstallationChange::Remove { installation_id } => installation_id.as_str(),
        };
        if !changed.insert(id) {
            bail!("duplicate owned-artifact change for installation {id}");
        }
        match change {
            InstallationChange::Set { owner, artifacts } => {
                if artifacts.is_empty() {
                    bail!("installation {id} has an empty complete set; use Remove");
                }
                // Form-local planners cannot see bindings that cross form boundaries.
                if let Some(prior) = bindings.get(id) {
                    validate_binding(prior, owner)?;
                }
                for artifact in artifacts {
                    let (root, path, form) = match artifact {
                        DesiredArtifact::File(file) => {
                            validate_mode(file.value.mode).with_context(|| {
                                format!("installation {id} file {}", file.location.path)
                            })?;
                            (&file.location.root, &file.location.path, Form::File)
                        }
                        DesiredArtifact::Instruction(value) => (
                            &value.location.root,
                            &value.location.path,
                            Form::Instruction,
                        ),
                    };
                    validate_location(root, path)
                        .with_context(|| format!("installation {id} desired location {path}"))?;
                    let key = location_key(root, path);
                    locations.forms.entry(key.clone()).or_default().insert(form);
                    locations.affected.insert(key);
                }
            }
            InstallationChange::Remove { .. } => {
                if !bindings.contains_key(id) {
                    bail!("unknown installation {id} for removal");
                }
            }
        }
    }
    for row in prior {
        if row
            .owners
            .iter()
            .any(|owner| changed.contains(owner.installation_id.as_str()))
        {
            locations
                .affected
                .insert(location_key(&row.root, &row.path));
        }
    }
    for row in prior {
        if locations
            .affected
            .contains(&location_key(&row.root, &row.path))
            && row.owners.len() != 1
        {
            bail!(
                "affected multi-owner artifact at {} is unsupported",
                row.path
            );
        }
    }
    Ok(locations)
}

fn index_observations<'a>(
    observations: &'a [ObservedFile],
    locations: &Locations,
) -> Result<BTreeMap<LocationKey, &'a ObservedFile>> {
    // Validate all supplied evidence, including extraneous observations.
    for observation in observations {
        let validate = || -> Result<()> {
            validate_location(&observation.location.root, &observation.location.path)?;
            validate_mode(observation.value.as_ref().and_then(|value| value.mode))
        };
        validate().with_context(|| {
            format!(
                "invalid owned-artifact observation ({})",
                observation.location.path
            )
        })?;
    }
    let mut indexed = BTreeMap::new();
    for observation in observations {
        let key = location_key(&observation.location.root, &observation.location.path);
        if indexed.insert(key.clone(), observation).is_some() {
            bail!("duplicate observation ({})", observation.location.path);
        }
        if !locations.affected.contains(&key) {
            bail!("extraneous observation ({})", observation.location.path);
        }
    }
    for key in &locations.affected {
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

fn validate_physical_locations(locations: &Locations) -> Result<()> {
    for (key, forms) in &locations.forms {
        if forms.len() != 1 {
            bail!("cross-form location conflict at {:?}:{}", key.1, key.2);
        }
    }
    let keys: Vec<_> = locations.forms.keys().collect();
    for (index, left) in keys.iter().enumerate() {
        for right in &keys[index + 1..] {
            if left.0 == right.0
                && left.1 == right.1
                && (locations.affected.contains(*left) || locations.affected.contains(*right))
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

fn partition_changes(
    prior: &[OwnedArtifact],
    changes: &[InstallationChange],
) -> (
    Vec<FileInstallationChange>,
    Vec<InstructionInstallationChange>,
) {
    let mut old_files = BTreeSet::new();
    let mut old_instructions = BTreeSet::new();
    for row in prior {
        let ids = match form(row) {
            Form::File => &mut old_files,
            Form::Instruction => &mut old_instructions,
        };
        ids.extend(
            row.owners
                .iter()
                .map(|owner| owner.installation_id.as_str()),
        );
    }
    let mut file_changes = Vec::new();
    let mut instruction_changes = Vec::new();
    for change in changes {
        let (id, owner, artifacts) = match change {
            InstallationChange::Set { owner, artifacts } => (
                owner.installation_id.as_str(),
                Some(owner),
                artifacts.as_slice(),
            ),
            InstallationChange::Remove { installation_id } => {
                (installation_id.as_str(), None, &[][..])
            }
        };
        let mut files = Vec::new();
        let mut contributions = Vec::new();
        for artifact in artifacts {
            match artifact {
                DesiredArtifact::File(file) => files.push(file.clone()),
                DesiredArtifact::Instruction(value) => contributions.push(value.clone()),
            }
        }
        if !files.is_empty() {
            file_changes.push(FileInstallationChange::Set {
                owner: owner.expect("only Set has desired files").clone(),
                files,
            });
        } else if old_files.contains(id) {
            file_changes.push(FileInstallationChange::Remove {
                installation_id: id.into(),
            });
        }
        if !contributions.is_empty() {
            instruction_changes.push(InstructionInstallationChange::Set {
                owner: owner.expect("only Set has desired contributions").clone(),
                contributions,
            });
        } else if old_instructions.contains(id) {
            instruction_changes.push(InstructionInstallationChange::Remove {
                installation_id: id.into(),
            });
        }
    }
    (file_changes, instruction_changes)
}

fn rebuild_artifacts(
    prior: &[OwnedArtifact],
    planned: Vec<OwnedArtifact>,
) -> Result<Vec<OwnedArtifact>> {
    let mut remaining = BTreeMap::new();
    for row in planned {
        if remaining.insert(artifact_key(&row), row).is_some() {
            bail!("duplicate combined artifact identity");
        }
    }
    let mut artifacts = Vec::new();
    for row in prior {
        if let Some(replacement) = remaining.remove(&artifact_key(row)) {
            artifacts.push(replacement);
        }
    }
    artifacts.extend(remaining.into_values());
    validate_artifacts(&artifacts).context("validate combined owned-artifact plan")?;
    Ok(artifacts)
}

/// Plan complete mixed installation sets from exact caller-supplied evidence.
///
/// A Set replaces both artifact forms; use Remove for an empty desired set.
/// Locations preserve raw root spellings. Every affected location requires one
/// observation, including no-ops. Aggregate actions preserve observed modes and
/// retain the file after its last contribution is removed. This pure boundary
/// grants no filesystem, permission, source or execution authority. Any error
/// returns no successful partial plan, without I/O or input mutation.
pub fn plan_owned_artifacts(
    prior: &[OwnedArtifact],
    changes: &[InstallationChange],
    observations: &[ObservedFile],
) -> Result<FileOperationPlan> {
    validate_artifacts(prior).context("validate prior owned-artifact records")?;
    let locations = index_locations(prior, changes)?;
    let observed = index_observations(observations, &locations)?;
    validate_physical_locations(&locations)?;
    let (file_prior, instruction_prior): (Vec<_>, Vec<_>) = prior
        .iter()
        .cloned()
        .partition(|row| form(row) == Form::File);
    let (file_changes, instruction_changes) = partition_changes(prior, changes);
    let mut file_observations = Vec::new();
    let mut instruction_observations = Vec::new();
    for (key, observation) in &observed {
        if locations.forms[key].contains(&Form::File) {
            file_observations.push((*observation).clone());
        }
        if locations.forms[key].contains(&Form::Instruction) {
            instruction_observations.push(ObservedInstructionFile {
                location: InstructionLocation {
                    root: observation.location.root.clone(),
                    path: observation.location.path.clone(),
                },
                content: observation.value.as_ref().map(|value| value.bytes.clone()),
            });
        }
    }
    let file_plan = plan_files(&file_prior, &file_changes, &file_observations)
        .context("plan dedicated owned files")?;
    let instruction_plan = plan_instructions(
        &instruction_prior,
        &instruction_changes,
        &instruction_observations,
    )
    .context("plan owned instruction contributions")?;
    let artifacts = rebuild_artifacts(
        prior,
        file_plan
            .artifacts
            .into_iter()
            .chain(instruction_plan.artifacts)
            .collect(),
    )?;
    let mut files = file_plan.files;
    for action in instruction_plan.files {
        let observed = observed[&location_key(&action.location.root, &action.location.path)];
        let after_mode = observed.value.as_ref().and_then(|value| value.mode);
        files.push(PlannedFile {
            location: FileLocation {
                root: action.location.root,
                path: action.location.path,
            },
            before: observed.value.clone(),
            after: Some(FileValue {
                bytes: action.after,
                mode: after_mode,
            }),
        });
    }
    files.sort_by_key(|action| location_key(&action.location.root, &action.location.path));
    for pair in files.windows(2) {
        if location_key(&pair[0].location.root, &pair[0].location.path)
            == location_key(&pair[1].location.root, &pair[1].location.path)
        {
            bail!(
                "duplicate combined physical action location ({})",
                pair[0].location.path
            );
        }
    }
    Ok(FileOperationPlan { artifacts, files })
}
