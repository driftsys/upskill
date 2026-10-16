//! Pure owner-aware instruction operation plans from caller-supplied observations.

use super::aggregate::{self, Contribution};
use crate::ownership::{ArtifactContent, OwnershipScope, validate_artifacts};
use crate::ownership::{ArtifactOwner, InstallationRoot, OwnedArtifact};
use crate::target::TargetSurface;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionLocation {
    pub root: InstallationRoot,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedInstructionFile {
    pub location: InstructionLocation,
    pub content: Option<Vec<u8>>,
}
/// Internal location context retained for callers that translate diagnostics.
#[derive(Debug)]
pub(crate) struct InstructionFileContext {
    pub(crate) action: &'static str,
    pub(crate) location: InstructionLocation,
}
impl std::fmt::Display for InstructionFileContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instruction file {}", self.action, self.location.path)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredInstructionContribution {
    pub location: InstructionLocation,
    pub id: String,
    pub markdown: String,
    pub consumers: Vec<TargetSurface>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionInstallationChange {
    Set {
        owner: ArtifactOwner,
        contributions: Vec<DesiredInstructionContribution>,
    },
    Remove {
        installation_id: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedInstructionFile {
    pub location: InstructionLocation,
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionOperationPlan {
    pub artifacts: Vec<OwnedArtifact>,
    pub files: Vec<PlannedInstructionFile>,
}
/// Plan complete installation contribution replacements using supplied bytes.
///
/// Locations use exact lexical root spellings. Every affected file requires one
/// observation, including operations whose resulting bytes are unchanged. This
/// pure boundary establishes no source, reference-graph or filesystem authority.
pub fn plan_instructions(
    prior: &[OwnedArtifact],
    changes: &[InstructionInstallationChange],
    observations: &[ObservedInstructionFile],
) -> Result<InstructionOperationPlan> {
    validate_artifacts(prior).context("validate prior instruction ownership")?;
    let bindings: BTreeMap<_, _> = prior
        .iter()
        .flat_map(|row| &row.owners)
        .map(|owner| (owner.installation_id.as_str(), owner))
        .collect();
    let changed = validate_changes(changes, &bindings)?;
    let desired = index_desired(changes)?;
    let mut affected = BTreeSet::new();
    for row in prior {
        if row
            .owners
            .iter()
            .any(|owner| changed.contains(owner.installation_id.as_str()))
        {
            if row.owners.len() != 1 {
                bail!(
                    "installation {} affects a multi-owner instruction region at {}",
                    row.owners[0].installation_id,
                    row.path
                );
            }
            affected.insert(artifact_key(row));
        }
    }
    affected.extend(desired.keys().map(|(location, _)| location.clone()));
    let observed = index_observations(observations, &affected)?;
    let mut rebuilt = BTreeMap::new();
    let mut files = Vec::new();
    for key in affected {
        let observation = observed[&key];
        let rows: Vec<_> = prior
            .iter()
            .filter(|row| artifact_key(row) == key)
            .collect();
        let mut records = Vec::new();
        for row in &rows {
            match &row.content {
                ArtifactContent::File { .. } => {
                    bail!("affected whole-file ownership at {}", row.path)
                }
                ArtifactContent::MarkdownContribution { contribution } => {
                    records.push(contribution.clone())
                }
            }
        }
        let (existing, bodies) = (|| -> Result<_> {
            if observation.content.is_none() && !records.is_empty() {
                bail!(
                    "owned instruction file {} is absent",
                    observation.location.path
                );
            }
            let bytes = observation.content.as_deref().unwrap_or_default();
            let existing = std::str::from_utf8(bytes)
                .with_context(|| format!("invalid UTF-8 at {}", observation.location.path))?;
            let bodies = aggregate::inspect_owned(existing, &records)?;
            let bodies: BTreeMap<_, _> = bodies
                .into_iter()
                .map(|body| (body.id.clone(), body))
                .collect();
            Ok((existing, bodies))
        })()
        .with_context(|| InstructionFileContext {
            action: "inspect",
            location: observation.location.clone(),
        })?;
        let mut metadata = BTreeMap::new();
        let mut contributions = Vec::new();
        for row in rows {
            let ArtifactContent::MarkdownContribution { contribution } = &row.content else {
                unreachable!()
            };
            if !row
                .owners
                .iter()
                .any(|owner| changed.contains(owner.installation_id.as_str()))
            {
                contributions.push(bodies[&contribution.id].clone());
                metadata.insert(contribution.id.clone(), row.clone());
            }
        }
        for ((location, id), (owner, contribution)) in &desired {
            if location != &key {
                continue;
            }
            if metadata.contains_key(id) {
                bail!(
                    "installation {} cannot take over untouched instruction selector {} at {}",
                    owner.installation_id,
                    id,
                    contribution.location.path
                );
            }
            contributions.push(Contribution::new(id, &contribution.markdown));
            metadata.insert(id.clone(), desired_artifact(owner, contribution)?);
        }
        let edit = aggregate::reconcile(existing, &records, &contributions).with_context(|| {
            InstructionFileContext {
                action: "reconcile",
                location: observation.location.clone(),
            }
        })?;
        for contribution in edit.owned {
            let mut row = metadata
                .remove(&contribution.id)
                .expect("editor returns supplied selectors");
            let selector = contribution.id.clone();
            row.content = ArtifactContent::MarkdownContribution { contribution };
            rebuilt.insert((key.clone(), selector), row);
        }
        files.push(PlannedInstructionFile {
            location: observation.location.clone(),
            before: observation.content.clone(),
            after: edit.content.into_bytes(),
        });
    }
    let mut artifacts = Vec::new();
    for row in prior {
        let key = artifact_key(row);
        if !observed.contains_key(&key) {
            artifacts.push(row.clone());
        } else if let ArtifactContent::MarkdownContribution { contribution } = &row.content {
            if let Some(replacement) = rebuilt.remove(&(key, contribution.id.clone())) {
                artifacts.push(replacement);
            }
        }
    }
    artifacts.extend(rebuilt.into_values());
    validate_artifacts(&artifacts).context("validate planned instruction ownership")?;
    Ok(InstructionOperationPlan { artifacts, files })
}

type LocationKey = (OwnershipScope, OsString, String);
type SelectorKey = (LocationKey, String);
type DesiredIndex<'a> =
    BTreeMap<SelectorKey, (&'a ArtifactOwner, &'a DesiredInstructionContribution)>;

fn location_key(location: &InstructionLocation) -> LocationKey {
    (
        location.root.scope,
        location.root.path.as_os_str().to_owned(),
        location.path.clone(),
    )
}

fn artifact_key(row: &OwnedArtifact) -> LocationKey {
    (
        row.root.scope,
        row.root.path.as_os_str().to_owned(),
        row.path.clone(),
    )
}

fn validate_changes<'a>(
    changes: &'a [InstructionInstallationChange],
    bindings: &BTreeMap<&str, &ArtifactOwner>,
) -> Result<BTreeSet<&'a str>> {
    let mut changed = BTreeSet::new();
    for change in changes {
        let id = match change {
            InstructionInstallationChange::Set {
                owner,
                contributions,
            } => {
                let id = owner.installation_id.as_str();
                if contributions.is_empty() {
                    bail!("installation {id} has an empty contribution set");
                }
                if let Some(prior) = bindings.get(id) {
                    if prior.source != owner.source
                        || prior.kind != owner.kind
                        || prior.source_name != owner.source_name
                        || prior.installed_name != owner.installed_name
                    {
                        bail!("installation {id} cannot change its source/item/alias binding");
                    }
                }
                id
            }
            InstructionInstallationChange::Remove { installation_id } => {
                if !bindings.contains_key(installation_id.as_str()) {
                    bail!("unknown installation {installation_id} for removal");
                }
                installation_id
            }
        };
        if !changed.insert(id) {
            bail!("duplicate instruction change for installation {id}");
        }
    }
    Ok(changed)
}

fn desired_artifact(
    owner: &ArtifactOwner,
    contribution: &DesiredInstructionContribution,
) -> Result<OwnedArtifact> {
    // The editor validates bodies and IDs and supplies real hash/separator data;
    // the ownership validator validates locations, owner bindings and consumers.
    let edit = aggregate::reconcile(
        "",
        &[],
        &[Contribution::new(&contribution.id, &contribution.markdown)],
    )
    .with_context(|| {
        format!(
            "installation {} selector {} at {}",
            owner.installation_id, contribution.id, contribution.location.path
        )
    })?;
    let row = OwnedArtifact {
        root: contribution.location.root.clone(),
        path: contribution.location.path.clone(),
        content: ArtifactContent::MarkdownContribution {
            contribution: edit.owned[0].clone(),
        },
        owners: vec![owner.clone()],
        consumers: contribution.consumers.clone(),
    };
    validate_artifacts(std::slice::from_ref(&row)).with_context(|| {
        format!(
            "installation {} selector {} at {}",
            owner.installation_id, contribution.id, contribution.location.path
        )
    })?;
    Ok(row)
}

fn index_desired(changes: &[InstructionInstallationChange]) -> Result<DesiredIndex<'_>> {
    let mut desired = BTreeMap::new();
    for change in changes {
        if let InstructionInstallationChange::Set {
            owner,
            contributions,
        } = change
        {
            for contribution in contributions {
                desired_artifact(owner, contribution)?;
                let selector = (
                    location_key(&contribution.location),
                    contribution.id.clone(),
                );
                if desired.insert(selector, (owner, contribution)).is_some() {
                    bail!(
                        "duplicate desired instruction selector {} at {} for installation {}",
                        contribution.id,
                        contribution.location.path,
                        owner.installation_id
                    );
                }
            }
        }
    }
    Ok(desired)
}

fn index_observations<'a>(
    observations: &'a [ObservedInstructionFile],
    affected: &BTreeSet<LocationKey>,
) -> Result<BTreeMap<LocationKey, &'a ObservedInstructionFile>> {
    let mut observed = BTreeMap::new();
    for observation in observations {
        let key = location_key(&observation.location);
        if !affected.contains(&key) {
            bail!(
                "extraneous instruction observation at {}",
                observation.location.path
            );
        }
        if observed.insert(key, observation).is_some() {
            bail!(
                "duplicate instruction observation at {}",
                observation.location.path
            );
        }
    }
    for key in affected {
        if !observed.contains_key(key) {
            bail!("missing instruction observation at {}", key.2);
        }
    }
    Ok(observed)
}
