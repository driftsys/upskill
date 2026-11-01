use super::*;
use crate::compile::skill_package::{SkillPackageRequest, compile_skill_package, scope_identity};
use crate::files::plan::validate_binding;
use crate::lifecycle::native_receipt::{
    NativeBindingOutcome, NativeObservationState, NativeReceiptError,
};
use crate::lifecycle::reconcile::reconcile_references;
use crate::lifecycle::state::{Reference, SourceEvidence, validate_state};
use crate::pipeline::ItemKind;
use anyhow::{Context, bail};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Validated<'a> {
    pub graph: ReconciledGraph,
    pub candidates: BTreeMap<TargetKey, &'a PackageRevision>,
    pub receipts: BTreeMap<TargetKey, &'a NativeBindingReceipt>,
    pub observations: BTreeMap<TargetKey, &'a NativeSkillObservation>,
    pub targets: BTreeMap<TargetKey, SkillTarget>,
}

fn require_scope(actual: &InstallationRoot, expected: &InstallationRoot) -> Result<()> {
    scope_identity(actual)?;
    ensure!(
        actual.scope == expected.scope && actual.path.as_os_str() == expected.path.as_os_str(),
        "skill delivery scope differs from captured absolute-root state"
    );
    Ok(())
}

fn validate_key(key: &SkillTarget) -> Result<()> {
    ensure!(
        !key.installation_id.is_empty()
            && key
                .installation_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
        "invalid native installation identity"
    );
    ensure!(
        matches!(
            key.surface,
            crate::target::TargetSurface::ClaudeCli
                | crate::target::TargetSurface::CopilotCli
                | crate::target::TargetSurface::CodexCli
                | crate::target::TargetSurface::OpenCodeCli
        ),
        "unsupported native skill target"
    );
    Ok(())
}

fn text(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && !value.chars().any(char::is_control),
        "native attribution must be nonempty without control characters"
    );
    Ok(())
}

fn validate_revision(revision: &PackageRevision, scope: &InstallationRoot) -> Result<()> {
    require_scope(&revision.installation.root, scope)?;
    ensure!(
        revision.installation.owner.kind == ItemKind::Skill,
        "package revision requires a skill installation"
    );
    let mut graph = CombinedState {
        schema: 2,
        origin: None,
        installations: vec![revision.installation.clone()],
        bundles: vec![],
        references: vec![],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    };
    graph.references.push(Reference::DirectInstallation {
        installation_id: revision.installation.owner.installation_id.clone(),
    });
    validate_state(&graph).context("validate revision capture installation")?;
    ensure!(
        revision
            .installation
            .requested
            .contains(&revision.selected.surface),
        "revision surface was not requested by its captured installation"
    );
    let digest = match &revision.installation.source_evidence {
        SourceEvidence::Local { manifest_sha256 }
        | SourceEvidence::Git {
            manifest_sha256, ..
        } => manifest_sha256,
    };
    ensure!(
        digest == &revision.selected.source_manifest_sha256,
        "revision capture source evidence differs from selected source"
    );
    let package = compile_skill_package(&SkillPackageRequest {
        selected: &revision.selected,
        surface: revision.selected.surface,
        scope,
        owner: &revision.installation.owner,
    })?;
    ensure!(
        package == revision.package,
        "captured package differs from recompiled selected evidence"
    );
    Ok(())
}

fn recovery(key: &SkillTarget) -> anyhow::Error {
    NativeReceiptError::RecoveryRequired {
        installation_id: key.installation_id.clone(),
        surface: key.surface,
    }
    .into()
}

pub(super) fn validate<'a>(input: &SkillDeliveryInput<'a>) -> Result<Validated<'a>> {
    scope_identity(input.scope)?;
    require_scope(&input.current.scope, input.scope)?;
    validate_state(&input.current.graph)?;
    ensure!(
        input
            .current
            .graph
            .artifacts
            .iter()
            .all(|row| row.owners.iter().all(|owner| owner.kind != ItemKind::Skill)),
        "skill package artifacts cannot have a second ownership representation in the combined graph"
    );
    let graph = reconcile_references(&input.current.graph, input.graph_changes)?;
    let prior_non_skills: Vec<_> = input
        .current
        .graph
        .installations
        .iter()
        .filter(|row| row.owner.kind != ItemKind::Skill)
        .collect();
    let next_non_skills: Vec<_> = graph
        .installations
        .iter()
        .filter(|row| row.owner.kind != ItemKind::Skill)
        .collect();
    ensure!(
        prior_non_skills == next_non_skills,
        "skill planner cannot change non-skill installations"
    );

    let mut targets = BTreeMap::new();
    let mut desired_keys = BTreeSet::new();
    for installation in input
        .current
        .graph
        .installations
        .iter()
        .chain(&graph.installations)
        .filter(|row| row.owner.kind == ItemKind::Skill)
    {
        require_scope(&installation.root, input.scope)?;
        for surface in &installation.requested {
            let key = SkillTarget {
                installation_id: installation.owner.installation_id.clone(),
                surface: *surface,
            };
            validate_key(&key)?;
            targets.insert(key.index_key(), key);
        }
    }
    for installation in graph
        .installations
        .iter()
        .filter(|row| row.owner.kind == ItemKind::Skill)
    {
        for surface in &installation.requested {
            desired_keys.insert((installation.owner.installation_id.clone(), surface.name()));
        }
    }
    let mut current_revision_keys = BTreeSet::new();
    let mut historical_bindings = BTreeMap::new();
    for revision in input
        .current
        .revisions
        .iter()
        .chain(input.current.staged.iter().map(|staged| &staged.revision))
    {
        validate_revision(revision, input.scope)?;
        ensure!(
            current_revision_keys.insert(revision_key(revision)),
            "duplicate captured revision"
        );
        if let Some(prior) = historical_bindings.insert(
            revision.installation.owner.installation_id.clone(),
            &revision.installation.owner,
        ) {
            validate_binding(prior, &revision.installation.owner)
                .context("conflicting historical installation captures")?;
        }
        let key = target(revision);
        targets.insert(key.index_key(), key);
        for installation in input
            .current
            .graph
            .installations
            .iter()
            .chain(&graph.installations)
            .filter(|row| row.owner.installation_id == revision.installation.owner.installation_id)
        {
            validate_binding(&revision.installation.owner, &installation.owner)
                .context("historical revision installation identity")?;
        }
    }

    for staged in &input.current.staged {
        ensure!(
            staged.operation_id.len() == 32
                && staged
                    .operation_id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "invalid staging operation identity"
        );
        ensure!(
            !staged.paths.is_empty() && staged.paths.windows(2).all(|pair| pair[0] < pair[1]),
            "staged paths must be a nonempty unique sorted subset"
        );
        ensure!(
            staged.paths.iter().all(|path| staged
                .revision
                .package
                .revision_files
                .iter()
                .any(|file| file.path == *path)),
            "staging ownership cannot select a catalog or foreign file"
        );
    }

    let mut intents = BTreeMap::new();
    for intent in input.intents {
        validate_key(&intent.key)?;
        ensure!(
            intents.insert(intent.key.index_key(), intent).is_none(),
            "duplicate desired skill intent"
        );
    }
    ensure!(
        intents.keys().cloned().collect::<BTreeSet<_>>() == desired_keys,
        "complete explicit intent is required for every surviving skill target"
    );
    let mut candidates = BTreeMap::new();
    for candidate in input.candidates {
        validate_revision(candidate, input.scope)?;
        let key = target(candidate).index_key();
        ensure!(
            candidates.insert(key.clone(), candidate).is_none(),
            "conflicting package candidates for one installation/client"
        );
        let installation = graph
            .installations
            .iter()
            .find(|row| row.owner.installation_id == candidate.installation.owner.installation_id)
            .context("package candidate has no surviving installation")?;
        ensure!(
            installation == &candidate.installation,
            "package candidate does not match desired installation evidence"
        );
        let intent = intents
            .get(&key)
            .context("candidate has no explicit intent")?;
        ensure!(
            matches!(&intent.selection, SkillIntentSelection::Selected { body_origin } if body_origin == &candidate.selected.body_origin),
            "candidate differs from explicit representation intent"
        );
    }
    for (key, intent) in &intents {
        ensure!(
            matches!(intent.selection, SkillIntentSelection::Unavailable)
                != candidates.contains_key(key),
            "selected intent needs exactly one candidate; unavailable intent must not stage a fallback"
        );
    }

    let mut receipts = BTreeMap::new();
    for receipt in &input.current.receipts {
        validate_key(&receipt.key)?;
        require_scope(&receipt.scope, input.scope)?;
        text(&receipt.client_version)?;
        text(&receipt.platform)?;
        text(&receipt.operation_id)?;
        ensure!(
            targets.contains_key(&receipt.key.index_key()),
            "native receipt has no installation or retained capture"
        );
        ensure!(
            receipts.insert(receipt.key.index_key(), receipt).is_none(),
            "duplicate native receipt"
        );
        if receipt.outcome != NativeBindingOutcome::Confirmed {
            return Err(recovery(&receipt.key));
        }
        for selection in receipt.before.iter().chain(&receipt.after) {
            ensure!(
                input
                    .current
                    .revisions
                    .iter()
                    .any(|revision| target(revision) == receipt.key
                        && NativeSkillSelection::from_package(&revision.package) == *selection),
                "native receipt selection has no matching captured revision"
            );
        }
    }
    for revision in &input.current.revisions {
        if !input
            .current
            .graph
            .installations
            .iter()
            .any(|row| row.owner.installation_id == revision.installation.owner.installation_id)
        {
            let receipt = receipts
                .get(&target(revision).index_key())
                .context("detached revision needs a confirmed detachment receipt")?;
            ensure!(
                receipt.after.is_none() && receipt.before.is_some(),
                "detached capture must not invent an active graph installation"
            );
        }
    }
    let mut observations = BTreeMap::new();
    for observation in input.native_observations {
        validate_key(&observation.key)?;
        require_scope(&observation.scope, input.scope)?;
        text(&observation.client_version)?;
        text(&observation.platform)?;
        text(&observation.observation_id)?;
        match &observation.retention {
            NativeResourceRetention::SessionsCleared { evidence_id }
            | NativeResourceRetention::CachePreserved { evidence_id } => text(evidence_id)?,
            NativeResourceRetention::Unknown => {}
        }
        ensure!(
            targets.contains_key(&observation.key.index_key()),
            "native observation is outside selected and retained targets"
        );
        ensure!(
            observations
                .insert(observation.key.index_key(), observation)
                .is_none(),
            "duplicate native observation"
        );
        if matches!(observation.state, NativeObservationState::Unknown) {
            return Err(recovery(&observation.key));
        }
        let receipt = receipts.get(&observation.key.index_key());
        if let Some(receipt) = receipt {
            ensure!(
                receipt.client_version == observation.client_version
                    && receipt.platform == observation.platform,
                "native observation attribution differs from confirmed receipt"
            );
        }
        let expected = receipt.and_then(|receipt| receipt.after.as_ref());
        match (&observation.state, expected) {
            (NativeObservationState::Absent, None) => {}
            (NativeObservationState::Present(actual), Some(expected)) if actual == expected => {}
            (NativeObservationState::Present(_), None) => {
                bail!("unowned native binding cannot be adopted")
            }
            _ => bail!("native observation contradicts confirmed receipt"),
        }
    }
    ensure!(
        observations.len() == targets.len(),
        "complete native observations are required before planning"
    );
    Ok(Validated {
        graph,
        candidates,
        receipts,
        observations,
        targets,
    })
}
