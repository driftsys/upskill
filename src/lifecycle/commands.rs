//! Opt-in guarded adapters over caller-authorized captured lifecycle requests.
//!
//! Desired values describe authorized content; retained source snapshots establish
//! freshness, not that arbitrary supplied output was produced by a compiler.
use anyhow::{Context, Result, ensure};

use super::application::{
    BoundSchema2Write, SCHEMA1_BACKUP_NAME, apply_bound_schema2_write, recover,
};
use super::contracts::{ContractContent, OwnerContract};
use super::desired::DesiredInstallationSet;
use super::selection::{Selection, SelectionDiagnostic, SelectionOutcome, select_owned_nodes};
use super::state::{
    BundleNode, CombinedState, Reference, SourceEvidence, StateOrigin, parse_state, project_legacy,
};
use super::upsert::{UpsertOutcome, UpsertRequest, plan_shared_upsert};
use crate::files::permissions::{
    DestinationMode, PermissionSource, ResourceExecutionPolicy, select_file_mode,
};
use crate::files::plan::{FileLocation, FileOperationPlan, FileValue};
use crate::filesystem::{
    DestinationSnapshot, ExecutionReport, QualifiedScope, ScopeWriterGuard, SourceSnapshot,
};
use crate::operations::plan::DesiredArtifact;
use crate::ownership::InstallationRoot;

/// Recovery always stops the new request. Callers retry after inspecting the report.
pub enum SessionOutcome<'scope, 'guard> {
    Ready(Box<LifecycleSession<'scope, 'guard>>),
    Recovery(ExecutionReport),
}

/// A single captured state under a caller-retained matching writer guard.
pub struct LifecycleSession<'scope, 'guard> {
    scope: &'scope QualifiedScope,
    guard: &'guard ScopeWriterGuard<'scope>,
    root: InstallationRoot,
    metadata: DestinationSnapshot<'scope>,
    state: CombinedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandDiagnostic {
    Preflight(super::preflight::PreflightReport),
    Selection(SelectionDiagnostic),
    /// Select the declaring bundle instead of deleting its member implicitly.
    NoDirectReference(String),
}

pub enum PrepareOutcome<'session, 'scope, 'guard, 'source> {
    Ready(Box<PreparedOperation<'session, 'scope, 'guard, 'source>>),
    Blocked(Vec<CommandDiagnostic>),
}

/// Complete supplied sets and graph changes. Construction grants no execution authority.
#[derive(Debug, Clone, Default)]
pub struct CapturedUpsert {
    pub sets: Vec<DesiredInstallationSet>,
    pub contracts: Vec<OwnerContract>,
    pub bundles: Vec<BundleNode>,
    pub add_references: Vec<Reference>,
    pub remove_references: Vec<Reference>,
}

/// Immutable prepared operation bound to its session and exact source evidence.
pub struct PreparedOperation<'session, 'scope, 'guard, 'source> {
    session: &'session LifecycleSession<'scope, 'guard>,
    destinations: DestinationSnapshot<'scope>,
    sources: Vec<&'source SourceSnapshot<'source>>,
    state: CombinedState,
    plan: FileOperationPlan,
}

impl<'scope, 'guard> LifecycleSession<'scope, 'guard> {
    /// Recover first, then capture canonical state and backup without refreshing later.
    pub fn open(
        scope: &'scope QualifiedScope,
        guard: &'guard ScopeWriterGuard<'scope>,
        root: InstallationRoot,
    ) -> Result<SessionOutcome<'scope, 'guard>> {
        if let Some(report) = recover(scope, guard)? {
            return Ok(SessionOutcome::Recovery(report));
        }
        let locations = [
            location(&root, crate::lockfile::LOCKFILE_NAME),
            location(&root, SCHEMA1_BACKUP_NAME),
        ];
        let metadata = scope.observe_destinations(&locations)?;
        let before = &metadata.observations()[0].value;
        let state = match before {
            None => CombinedState {
                schema: 2,
                origin: Some(StateOrigin::Fresh {}),
                installations: vec![],
                bundles: vec![],
                references: vec![],
                artifacts: vec![],
                owner_consumers: vec![],
                owner_contracts: vec![],
                legacy: None,
            },
            Some(value) => {
                let json: serde_json::Value = serde_json::from_slice(&value.bytes)?;
                match json.get("schema").and_then(serde_json::Value::as_u64) {
                    Some(1) => {
                        let mut state = project_legacy(&serde_json::from_slice(&value.bytes)?)?;
                        state.origin = Some(StateOrigin::from_schema1_bytes(&value.bytes)?);
                        state
                    }
                    Some(2) => parse_state(std::str::from_utf8(&value.bytes)?)?,
                    _ => anyhow::bail!("unsupported lifecycle state schema"),
                }
            }
        };
        ensure!(
            state.origin.is_some(),
            "state origin is unknown; reconcile before applying"
        );
        // Verify backup binding before caller selection or source work.
        let backup = &metadata.observations()[1].value;
        match state.origin.as_ref().expect("checked origin") {
            StateOrigin::Fresh {} => ensure!(
                backup.is_none(),
                "fresh state has unexpected historical backup"
            ),
            origin @ StateOrigin::Migrated { .. } => {
                if before.as_ref().is_some_and(|v| {
                    serde_json::from_slice::<serde_json::Value>(&v.bytes)
                        .is_ok_and(|v| v["schema"] == 1)
                }) {
                    if let Some(backup) = backup {
                        ensure!(
                            backup.mode == Some(0o600) && origin.matches_backup(&backup.bytes),
                            "schema-1 backup differs from captured state"
                        );
                    }
                } else {
                    let backup = backup
                        .as_ref()
                        .context("original schema-1 backup is missing")?;
                    ensure!(
                        backup.mode == Some(0o600) && origin.matches_backup(&backup.bytes),
                        "original schema-1 backup binding changed"
                    );
                    ensure!(
                        state.legacy.as_ref() == Some(&serde_json::from_slice(&backup.bytes)?),
                        "legacy history differs from original backup"
                    );
                }
            }
        }
        Ok(SessionOutcome::Ready(Box::new(Self {
            scope,
            guard,
            root,
            metadata,
            state,
        })))
    }

    pub fn state(&self) -> &CombinedState {
        &self.state
    }

    /// Complete preflight precedes the callback, which may compile supplied snapshots.
    /// None selects no existing roots (add); Some selects complete update roots.
    pub fn prepare_upsert<'session, 'source>(
        &'session self,
        selection: Option<&Selection>,
        sources: &[&'source SourceSnapshot<'source>],
        execution: ResourceExecutionPolicy,
        build: impl FnOnce(&CombinedState) -> Result<CapturedUpsert>,
    ) -> Result<PrepareOutcome<'session, 'scope, 'guard, 'source>> {
        if let Some(selection) = selection {
            if let SelectionOutcome::Blocked { diagnostics, .. } =
                select_owned_nodes(&self.state, selection)?
            {
                return Ok(PrepareOutcome::Blocked(
                    diagnostics
                        .into_iter()
                        .map(CommandDiagnostic::Selection)
                        .collect(),
                ));
            }
        }
        self.metadata.verify_unchanged()?;
        for source in sources {
            source.verify_unchanged()?;
        }
        let mut request = build(&self.state)?;
        for set in &request.sets {
            let digest = match &set.installation.source_evidence {
                SourceEvidence::Local { manifest_sha256 }
                | SourceEvidence::Git {
                    manifest_sha256, ..
                } => manifest_sha256,
            };
            ensure!(
                sources
                    .iter()
                    .any(|s| &s.manifest().content_sha256 == digest),
                "desired installation lacks matching captured source snapshot"
            );
        }
        let mut locations = self.old_locations()?;
        for set in &request.sets {
            for artifact in &set.artifacts {
                add_location(
                    &mut locations,
                    &match artifact {
                        DesiredArtifact::File(f) => f.location.clone(),
                        DesiredArtifact::Instruction(i) => {
                            location(&i.location.root, &i.location.path)
                        }
                    },
                )?;
            }
        }
        let destinations = self.scope.observe_destinations(&locations)?;
        for set in &mut request.sets {
            for artifact in &mut set.artifacts {
                if let DesiredArtifact::File(file) = artifact {
                    let before = destinations
                        .observations()
                        .iter()
                        .find(|o| o.location == file.location)
                        .context("missing output observation")?;
                    let source = match file.value.mode {
                        None => PermissionSource::Generated,
                        mode => PermissionSource::Resource {
                            captured_mode: mode,
                        },
                    };
                    let mode =
                        select_file_mode(destination_mode(&before.value), source, execution)?;
                    file.value.mode = Some(mode);
                    for contract in &mut request.contracts {
                        if contract.installation_id == set.installation.owner.installation_id
                            && contract.root == file.location.root
                            && contract.path == file.location.path
                            && contract.contribution_id.is_none()
                        {
                            if let ContractContent::File {
                                mode: contract_mode,
                                ..
                            } = &mut contract.content
                            {
                                *contract_mode = Some(mode);
                            }
                        }
                    }
                }
            }
        }
        let graph = super::reconcile::reconcile_references(
            &self.state,
            &super::reconcile::GraphChanges {
                installations: request
                    .sets
                    .iter()
                    .map(|s| s.installation.clone())
                    .collect(),
                bundles: request.bundles.clone(),
                add_references: request.add_references.clone(),
                remove_references: request.remove_references.clone(),
            },
        )?;
        let changed: Vec<_> = request
            .sets
            .iter()
            .map(|s| s.installation.owner.installation_id.as_str())
            .chain(graph.removed_installation_ids.iter().map(String::as_str))
            .collect();
        let observations: Vec<_> = destinations
            .observations()
            .iter()
            .filter(|o| {
                self.state.artifacts.iter().any(|a| {
                    a.root == o.location.root
                        && a.path == o.location.path
                        && a.owners
                            .iter()
                            .any(|owner| changed.contains(&owner.installation_id.as_str()))
                }) || request
                    .sets
                    .iter()
                    .flat_map(|s| &s.artifacts)
                    .any(|a| match a {
                        DesiredArtifact::File(f) => f.location == o.location,
                        DesiredArtifact::Instruction(i) => {
                            i.location.root == o.location.root && i.location.path == o.location.path
                        }
                    })
            })
            .cloned()
            .collect();
        let outcome = plan_shared_upsert(
            &self.state,
            UpsertRequest {
                selection,
                contracts: &request.contracts,
                sets: &request.sets,
                bundles: &request.bundles,
                add_references: &request.add_references,
                remove_references: &request.remove_references,
            },
            &observations,
        )?;
        match outcome {
            UpsertOutcome::Blocked(diagnostics) => Ok(PrepareOutcome::Blocked(
                diagnostics
                    .into_iter()
                    .map(CommandDiagnostic::Selection)
                    .collect(),
            )),
            UpsertOutcome::Ready(plan) => {
                let mut files = plan.files;
                select_instruction_modes(&mut files, &self.state, &plan.state)?;
                Ok(PrepareOutcome::Ready(Box::new(PreparedOperation {
                    session: self,
                    destinations,
                    sources: sources.to_vec(),
                    plan: FileOperationPlan {
                        artifacts: plan.state.artifacts.clone(),
                        files,
                    },
                    state: plan.state,
                })))
            }
        }
    }

    /// Remove only explicitly selected root references, preserving independent roots.
    pub fn prepare_remove<'session>(
        &'session self,
        selection: &Selection,
    ) -> Result<PrepareOutcome<'session, 'scope, 'guard, 'session>> {
        if let SelectionOutcome::Blocked { diagnostics, .. } =
            select_owned_nodes(&self.state, selection)?
        {
            return Ok(PrepareOutcome::Blocked(
                diagnostics
                    .into_iter()
                    .map(CommandDiagnostic::Selection)
                    .collect(),
            ));
        }
        let mut remove = Vec::new();
        let mut diagnostics = Vec::new();
        for edge in &self.state.references {
            let name = match edge {
                Reference::DirectInstallation { installation_id } => {
                    &self
                        .state
                        .installations
                        .iter()
                        .find(|i| &i.owner.installation_id == installation_id)
                        .context("missing direct installation")?
                        .owner
                        .installed_name
                }
                Reference::DirectBundle { bundle_id } => {
                    &self
                        .state
                        .bundles
                        .iter()
                        .find(|b| &b.id == bundle_id)
                        .context("missing direct bundle")?
                        .name
                }
                _ => continue,
            };
            if matches!(selection, Selection::All)
                || matches!(selection, Selection::Names(names) if names.contains(name))
            {
                remove.push(edge.clone());
            }
        }
        if let Selection::Names(names) = selection {
            for name in names {
                let has_direct = remove.iter().any(|edge| match edge {
                    Reference::DirectInstallation { installation_id } => {
                        self.state.installations.iter().any(|i| {
                            &i.owner.installation_id == installation_id
                                && &i.owner.installed_name == name
                        })
                    }
                    Reference::DirectBundle { bundle_id } => self
                        .state
                        .bundles
                        .iter()
                        .any(|b| &b.id == bundle_id && &b.name == name),
                    _ => false,
                });
                if !has_direct {
                    diagnostics.push(CommandDiagnostic::NoDirectReference(name.clone()));
                }
            }
        }
        if !diagnostics.is_empty() {
            return Ok(PrepareOutcome::Blocked(diagnostics));
        }
        let destinations = self.scope.observe_destinations(&self.old_locations()?)?;
        let graph = super::reconcile::reconcile_references(
            &self.state,
            &super::reconcile::GraphChanges {
                remove_references: remove.clone(),
                ..Default::default()
            },
        )?;
        let observations: Vec<_> = destinations
            .observations()
            .iter()
            .filter(|o| {
                self.state.artifacts.iter().any(|a| {
                    a.root == o.location.root
                        && a.path == o.location.path
                        && a.owners.iter().any(|owner| {
                            graph
                                .removed_installation_ids
                                .contains(&owner.installation_id)
                        })
                })
            })
            .cloned()
            .collect();
        let mut plan = super::retire::plan_reference_removal(&self.state, &remove, &observations)?;
        select_instruction_modes(&mut plan.files, &self.state, &plan.state)?;
        Ok(PrepareOutcome::Ready(Box::new(PreparedOperation {
            session: self,
            destinations,
            sources: vec![],
            plan: FileOperationPlan {
                artifacts: plan.state.artifacts.clone(),
                files: plan.files,
            },
            state: plan.state,
        })))
    }

    fn old_locations(&self) -> Result<Vec<FileLocation>> {
        let mut locations = Vec::new();
        for artifact in &self.state.artifacts {
            let value = location(&artifact.root, &artifact.path);
            add_location(&mut locations, &value)?;
        }
        Ok(locations)
    }
}

impl PreparedOperation<'_, '_, '_, '_> {
    pub fn state(&self) -> &CombinedState {
        &self.state
    }
    pub fn plan(&self) -> &FileOperationPlan {
        &self.plan
    }

    /// Consume the private plan; callers cannot substitute state, evidence or guard.
    pub fn apply(self) -> Result<ExecutionReport> {
        self.session.metadata.verify_unchanged()?;
        self.destinations.verify_unchanged()?;
        for source in &self.sources {
            source.verify_unchanged()?;
        }
        let observations = self.session.metadata.observations();
        apply_bound_schema2_write(
            self.session.scope,
            self.session.guard,
            BoundSchema2Write {
                plan: self.plan,
                state_before: observations[0].value.clone(),
                backup_before: observations[1].value.clone(),
                state_location: location(&self.session.root, crate::lockfile::LOCKFILE_NAME),
                backup_location: location(&self.session.root, SCHEMA1_BACKUP_NAME),
                state_after: &self.state,
                sources: &self.sources,
            },
        )
    }
}

fn select_instruction_modes(
    files: &mut [crate::files::plan::PlannedFile],
    prior: &CombinedState,
    next: &CombinedState,
) -> Result<()> {
    for file in files {
        if prior.artifacts.iter().chain(&next.artifacts).any(|a| {
            a.root == file.location.root
                && a.path == file.location.path
                && matches!(
                    a.content,
                    crate::ownership::ArtifactContent::MarkdownContribution { .. }
                )
        }) {
            if let Some(after) = &mut file.after {
                after.mode = Some(select_file_mode(
                    destination_mode(&file.before),
                    PermissionSource::Generated,
                    ResourceExecutionPolicy::Disabled,
                )?);
            }
        }
    }
    Ok(())
}

fn location(root: &InstallationRoot, path: &str) -> FileLocation {
    FileLocation {
        root: root.clone(),
        path: path.into(),
    }
}
fn destination_mode(value: &Option<FileValue>) -> DestinationMode {
    match value {
        None => DestinationMode::Absent,
        Some(v) => DestinationMode::Present(v.mode),
    }
}
fn add_location(locations: &mut Vec<FileLocation>, value: &FileLocation) -> Result<()> {
    for reserved in [
        crate::lockfile::LOCKFILE_NAME,
        SCHEMA1_BACKUP_NAME,
        ".upskill/state",
    ] {
        // Match portable case aliases before observing output and metadata in
        // separate snapshots. Prefix components also reject parent destinations.
        let overlaps = value
            .path
            .split('/')
            .zip(reserved.split('/'))
            .all(|(a, b)| a.eq_ignore_ascii_case(b));
        ensure!(
            !overlaps,
            "output overlaps reserved lifecycle path: {}",
            value.path
        );
    }
    if !locations.contains(value) {
        locations.push(value.clone());
    }
    Ok(())
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;
    use crate::filesystem::execution::{ExecutionOutcome, TEST_HOOK};
    use crate::ownership::OwnershipScope;

    #[test]
    fn recovery_complete_and_reconcile_both_stop_new_session_before_state_capture() {
        for stale in [false, true] {
            let tmp = tempfile::tempdir().unwrap();
            let root = InstallationRoot {
                scope: OwnershipScope::Project,
                path: tmp.path().into(),
            };
            let scope = QualifiedScope::open(&root).unwrap();
            let guard = scope.try_lock_writer().unwrap();
            let SessionOutcome::Ready(session) =
                LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
            else {
                panic!()
            };
            let PrepareOutcome::Ready(operation) = session
                .prepare_upsert(None, &[], ResourceExecutionPolicy::Disabled, |_| {
                    Ok(CapturedUpsert::default())
                })
                .unwrap()
            else {
                panic!()
            };
            TEST_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(|point, _| {
                    if point == "after_initial_journal" {
                        anyhow::bail!("interrupt before effects");
                    }
                    Ok(())
                }))
            });
            let result = operation.apply();
            TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
            assert!(result.is_err());
            if stale {
                std::fs::write(
                    tmp.path().join(crate::lockfile::LOCKFILE_NAME),
                    b"late unrelated state",
                )
                .unwrap();
            }
            let SessionOutcome::Recovery(report) =
                LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
            else {
                panic!("recovery must stop new request")
            };
            if stale {
                assert!(matches!(report.outcome, ExecutionOutcome::Reconcile { .. }));
                assert_eq!(
                    std::fs::read(tmp.path().join(crate::lockfile::LOCKFILE_NAME)).unwrap(),
                    b"late unrelated state"
                );
            } else {
                assert_eq!(report.outcome, ExecutionOutcome::Complete);
                assert!(matches!(
                    LifecycleSession::open(&scope, &guard, root).unwrap(),
                    SessionOutcome::Ready(_)
                ));
            }
        }
    }
}
