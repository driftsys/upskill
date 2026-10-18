//! Opt-in applying orchestration for explicitly selected source-backed rule roots.
//!
//! Sessions recover before source work; opaque scoped preparation retains compiler
//! bindings and full source identity through application. Production is unchanged.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

use super::commands::{
    CapturedUpsert, CommandDiagnostic, LifecycleSession, PrepareOutcome, SessionOutcome,
};
use super::contracts::{ConsumerContract, ContractContent};
use super::desired::CompilerBinding;
use super::desired::codex::{InstructionReaderReport, contribution_id};
use super::preflight::{PreflightCategory, PreflightDiagnostic, PreflightReport, normalize_report};
use super::selection::{Selection, SelectionDiagnostic, SelectionOutcome, select_owned_nodes};
use super::sources::{
    CodexSourceBinding, ResolvedRuleSource, RuleSourceBinding, normalized_source_label,
};
use super::state::{CombinedState, Installation, Reference};
use crate::files::permissions::ResourceExecutionPolicy;
use crate::filesystem::{ExecutionReport, QualifiedScope, ScopeWriterGuard};
use crate::instructions::plan::InstructionLocation;
use crate::ownership::{ArtifactContent, InstallationRoot, OwnershipScope};
use crate::pipeline::ItemKind;
use crate::source::{InstallSource, parse_install_source_label};
use crate::target::{
    CapabilityCategory, CapabilityError, CapabilityKey, TargetProfile, TargetSurface,
};

/// Nonempty selections override lower-precedence selections. Empty means absent.
#[derive(Debug, Clone, Copy, Default)]
pub struct TargetSelection<'a> {
    pub invocation: &'a [TargetSurface],
    pub project: &'a [TargetSurface],
    pub global: &'a [TargetSurface],
}

/// Resolve requested targets without mutating preferences or inventing defaults.
pub fn effective_targets(
    selection: TargetSelection<'_>,
    fallback: &[TargetSurface],
) -> Result<Vec<TargetSurface>> {
    let targets = [
        selection.invocation,
        selection.project,
        selection.global,
        fallback,
    ]
    .into_iter()
    .find(|values| !values.is_empty())
    .context("explicit or configured targets are required")?;
    let mut names = std::collections::BTreeSet::new();
    for target in targets {
        ensure!(
            names.insert(target.name()),
            "duplicate requested target: {}",
            target.name()
        );
    }
    Ok(targets.to_vec())
}

/// Caller-supplied compiler contracts; native Codex compilation is unsupported.
pub struct CompilerContext {
    pub bindings: Vec<CompilerBinding>,
    pub profiles: Vec<TargetProfile>,
    pub execution: ResourceExecutionPolicy,
}

/// One selected rule item root. No repository-wide discovery is performed.
pub struct AddRule {
    pub source: InstallSource,
    pub source_name: String,
    pub installed_name: String,
}

pub enum SourceRequest {
    Add(Vec<AddRule>),
    Update(Selection),
    Remove(Selection),
}

/// Recovery stops the new request. Ready contains the scoped callback's result;
/// it does not imply application when the callback chose to discard preparation.
pub enum OrchestrationOutcome<T = ExecutionReport> {
    Ready(T),
    Recovery(ExecutionReport),
    Blocked(Vec<CommandDiagnostic>),
}

/// Complete newly compiled instruction exposure for one installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInstructionReaders {
    pub installation_id: String,
    pub report: InstructionReaderReport,
}

/// An opaque source-bound operation. No replacement state or output is accepted.
pub struct SourcePrepared<'a> {
    instruction_readers: Vec<SourceInstructionReaders>,
    apply: Box<dyn FnOnce() -> Result<ExecutionReport> + 'a>,
}
impl SourcePrepared<'_> {
    /// Complete newly compiled reader reports, separate from recorded ownership.
    pub fn instruction_readers(&self) -> &[SourceInstructionReaders] {
        &self.instruction_readers
    }

    /// Revalidate retained captures, including Git identity, immediately before apply.
    pub fn apply(self) -> Result<ExecutionReport> {
        (self.apply)()
    }
}

struct Binding {
    id: String,
    source_name: String,
    installed_name: String,
    root: InstallationRoot,
    targets: Vec<TargetSurface>,
}

/// Target policy and compiler authority for this invocation.
pub struct SourceOptions<'a> {
    pub targets: TargetSelection<'a>,
    pub add_defaults: &'a [TargetSurface],
    pub compiler: &'a CompilerContext,
}

/// Open/recover first, preflight the complete selection, resolve explicit sources,
/// then invoke a scoped callback while all resolvers and captures remain alive.
///
/// The caller retains the matching scope writer guard. The callback cannot return
/// the prepared operation outside this scope. Remove performs no source work and
/// uses recorded references/ownership regardless of current target preferences.
pub fn with_prepared<T>(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    root: InstallationRoot,
    request: SourceRequest,
    options: SourceOptions<'_>,
    use_prepared: impl FnOnce(SourcePrepared<'_>) -> Result<T>,
) -> Result<OrchestrationOutcome<T>> {
    with_prepared_route(
        scope,
        guard,
        root,
        request,
        RouteOptions {
            targets: options.targets,
            add_defaults: options.add_defaults,
            route: SourceRoute::Historical(options.compiler),
        },
        use_prepared,
    )
}

/// Explicit authority for captured Codex CLI revision-1 source compilation.
pub struct CodexSourceOptions<'a> {
    pub targets: TargetSelection<'a>,
    pub add_defaults: &'a [TargetSurface],
    pub profile: TargetProfile,
    pub execution: ResourceExecutionPolicy,
}

/// Prepare selected Codex instruction sources and expose their complete readers.
/// A qualified declared user root does not certify native HOME/CODEX_HOME loading.
pub fn with_prepared_codex<T>(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    root: InstallationRoot,
    request: SourceRequest,
    options: CodexSourceOptions<'_>,
    use_prepared: impl FnOnce(SourcePrepared<'_>) -> Result<T>,
) -> Result<OrchestrationOutcome<T>> {
    with_prepared_route(
        scope,
        guard,
        root,
        request,
        RouteOptions {
            targets: options.targets,
            add_defaults: options.add_defaults,
            route: SourceRoute::Codex {
                profile: options.profile,
                execution: options.execution,
            },
        },
        use_prepared,
    )
}

#[derive(Clone, Copy)]
enum SourceRoute<'a> {
    Historical(&'a CompilerContext),
    Codex {
        profile: TargetProfile,
        execution: ResourceExecutionPolicy,
    },
}

struct RouteOptions<'a> {
    targets: TargetSelection<'a>,
    add_defaults: &'a [TargetSurface],
    route: SourceRoute<'a>,
}

fn with_prepared_route<T>(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    root: InstallationRoot,
    request: SourceRequest,
    options: RouteOptions<'_>,
    use_prepared: impl FnOnce(SourcePrepared<'_>) -> Result<T>,
) -> Result<OrchestrationOutcome<T>> {
    let RouteOptions {
        targets,
        add_defaults,
        route,
    } = options;
    let session = match LifecycleSession::open(scope, guard, root.clone())? {
        SessionOutcome::Recovery(report) => return Ok(OrchestrationOutcome::Recovery(report)),
        SessionOutcome::Ready(session) => session,
    };
    if let SourceRequest::Remove(selection) = &request {
        return match session.prepare_remove(selection)? {
            PrepareOutcome::Blocked(diagnostics) => Ok(OrchestrationOutcome::Blocked(diagnostics)),
            PrepareOutcome::Ready(prepared) => {
                Ok(OrchestrationOutcome::Ready(use_prepared(SourcePrepared {
                    instruction_readers: vec![],
                    apply: Box::new(move || prepared.apply()),
                })?))
            }
        };
    }
    if let SourceRoute::Codex { profile, .. } = route {
        if profile != TargetSurface::CodexCli.profile() {
            return Err(codex_capability(
                profile.surface,
                "profile",
                "only Codex CLI contract revision 1 is supported",
            ));
        }
    }
    let execution = match route {
        SourceRoute::Historical(compiler) => compiler.execution,
        SourceRoute::Codex { execution, .. } => execution,
    };
    let is_add = matches!(&request, SourceRequest::Add(_));
    let mut report = PreflightReport::default();
    let mut specs = Vec::new();
    let mut bindings = Vec::new();
    let selection = match &request {
        SourceRequest::Update(selection) => {
            let selected = match select_owned_nodes(session.state(), selection)? {
                SelectionOutcome::Blocked { diagnostics, .. } => {
                    return Ok(OrchestrationOutcome::Blocked(
                        diagnostics
                            .into_iter()
                            .map(CommandDiagnostic::Selection)
                            .collect(),
                    ));
                }
                SelectionOutcome::Ready(selected) => selected,
            };
            for id in &selected.installation_ids {
                ensure_source_rule_owner(session.state(), id, route, &root)?;
            }
            // Selection and representation preflight is complete; validate targets
            // and parse the recorded binding for each installation.
            for id in selected.installation_ids {
                let item = session
                    .state()
                    .installations
                    .iter()
                    .find(|item| item.owner.installation_id == id)
                    .context("selected installation is missing")?;
                bindings.push(Binding {
                    id,
                    source_name: item.owner.source_name.clone(),
                    installed_name: item.owner.installed_name.clone(),
                    root: item.root.clone(),
                    targets: route_targets(route, targets, &item.requested)?,
                });
                match parse_install_source_label(&item.owner.source) {
                    Ok(source) => specs.push(Some(source)),
                    Err(error) => {
                        report.diagnostics.push(item_diagnostic(
                            &item.owner.installation_id,
                            PreflightCategory::Resolution,
                            error.into(),
                        ));
                        specs.push(None);
                    }
                }
            }
            Some(selection.clone())
        }
        SourceRequest::Add(items) => {
            ensure!(
                !items.is_empty(),
                "add requires at least one selected rule root"
            );
            let requested = route_targets(route, targets, add_defaults)?;
            // Retained representation/root authority is an operation-level boundary:
            // inspect every selected recorded owner before resolving new sources.
            for item in items {
                for prior in session
                    .state()
                    .installations
                    .iter()
                    .filter(|prior| prior.owner.installed_name == item.installed_name)
                {
                    ensure_source_rule_owner(
                        session.state(),
                        &prior.owner.installation_id,
                        route,
                        &root,
                    )?;
                }
            }
            for item in items {
                let mut binding = Binding {
                    id: String::new(),
                    source_name: item.source_name.clone(),
                    installed_name: item.installed_name.clone(),
                    root: root.clone(),
                    targets: requested.clone(),
                };
                // A fallback identity is diagnostic-only: normalization failure cannot
                // establish source evidence or authorize an installation.
                let label = normalized_source_label(&item.source);
                binding.id = new_installation_id(&binding, &item.source.to_string())?;
                let checked = (|| -> Result<()> {
                    let label = label?;
                    binding.id = installation_id(session.state(), &binding, &label)?;
                    ensure!(
                        !item.source_name.is_empty() && !item.installed_name.is_empty(),
                        "source name and installed alias must be nonempty"
                    );
                    ensure!(
                        !session
                            .state()
                            .bundles
                            .iter()
                            .any(|b| b.name == item.installed_name),
                        "installed alias already belongs to an owned bundle"
                    );
                    for prior in session
                        .state()
                        .installations
                        .iter()
                        .filter(|p| p.owner.installed_name == item.installed_name)
                    {
                        ensure_source_rule_owner(
                            session.state(),
                            &prior.owner.installation_id,
                            route,
                            &root,
                        )?;
                        ensure!(
                            same_binding(prior, &binding, &label),
                            "installed alias already belongs to a different binding"
                        );
                    }
                    Ok(())
                })();
                if let Err(error) = checked {
                    report.diagnostics.push(item_diagnostic(
                        &binding.id,
                        PreflightCategory::Binding,
                        error,
                    ));
                    specs.push(None);
                } else {
                    specs.push(Some(item.source.clone()));
                }
                bindings.push(binding);
            }
            // Duplicate aliases are request conflicts, even when they describe the
            // same binding. Do not silently replace either selected request.
            for index in 0..bindings.len() {
                for previous in 0..index {
                    if bindings[previous].installed_name == bindings[index].installed_name {
                        report.diagnostics.push(PreflightDiagnostic {
                            installation_ids: vec![
                                bindings[previous].id.clone(),
                                bindings[index].id.clone(),
                            ],
                            category: PreflightCategory::Binding,
                            location: None,
                            message: "duplicate installed alias in add request".into(),
                        });
                        specs[previous] = None;
                        specs[index] = None;
                    }
                }
            }
            let names = items
                .iter()
                .map(|item| item.installed_name.clone())
                .filter(|name| !name.is_empty())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if let SelectionOutcome::Blocked { diagnostics, .. } =
                select_owned_nodes(session.state(), &Selection::Names(names.clone()))?
            {
                let diagnostics = diagnostics
                    .into_iter()
                    .filter(|value| !matches!(value, SelectionDiagnostic::UnknownName(_)))
                    .map(CommandDiagnostic::Selection)
                    .collect::<Vec<_>>();
                if !diagnostics.is_empty() {
                    return Ok(OrchestrationOutcome::Blocked(diagnostics));
                }
            }
            let existing = names
                .into_iter()
                .filter(|name| {
                    session
                        .state()
                        .installations
                        .iter()
                        .any(|item| &item.owner.installed_name == name)
                })
                .collect::<Vec<_>>();
            (!existing.is_empty()).then_some(Selection::Names(existing))
        }
        SourceRequest::Remove(_) => unreachable!(),
    };
    let resolved = specs
        .iter()
        .zip(&bindings)
        .map(|(spec, binding)| {
            let source = spec.as_ref()?;
            match ResolvedRuleSource::resolve(source) {
                Ok(source) => Some(source),
                Err(error) => {
                    report.diagnostics.push(item_diagnostic(
                        &binding.id,
                        PreflightCategory::Resolution,
                        error,
                    ));
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    let captures = resolved
        .iter()
        .zip(&bindings)
        .map(|(source, binding)| {
            let source = source.as_ref()?;
            if !is_add {
                let item = session
                    .state()
                    .installations
                    .iter()
                    .find(|i| i.owner.installation_id == binding.id)
                    .expect("selected recorded binding");
                if item.owner.source != source.source_label() {
                    report.diagnostics.push(item_diagnostic(
                        &binding.id,
                        PreflightCategory::Binding,
                        anyhow::anyhow!(
                            "resolved source binding differs from recorded installation"
                        ),
                    ));
                    return None;
                }
            }
            match source.capture() {
                Ok(capture) => Some(capture),
                Err(error) => {
                    report.diagnostics.push(item_diagnostic(
                        &binding.id,
                        PreflightCategory::Capture,
                        error,
                    ));
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    let mut request = CapturedUpsert::default();
    let mut instruction_readers = Vec::new();
    for (binding, capture) in bindings.iter().zip(&captures) {
        let Some(capture) = capture else { continue };
        let compiled = (|| -> Result<_> {
            match route {
                SourceRoute::Historical(compiler) => {
                    let compiler_bindings = compiler
                        .bindings
                        .iter()
                        .filter_map(|value| {
                            let consumers = value
                                .consumers
                                .iter()
                                .copied()
                                .filter(|surface| binding.targets.contains(surface))
                                .collect::<Vec<_>>();
                            (!consumers.is_empty()).then_some(CompilerBinding {
                                client: value.client,
                                consumers,
                            })
                        })
                        .collect::<Vec<_>>();
                    capture.compile(RuleSourceBinding {
                        installation_id: &binding.id,
                        expected_source_name: &binding.source_name,
                        installed_name: &binding.installed_name,
                        root: &binding.root,
                        requested: &binding.targets,
                        bindings: &compiler_bindings,
                        profiles: &compiler.profiles,
                    })
                }
                SourceRoute::Codex { profile, .. } => {
                    let location = instruction_location(&root);
                    let compiled = capture.compile_codex(CodexSourceBinding {
                        installation_id: &binding.id,
                        expected_source_name: &binding.source_name,
                        installed_name: &binding.installed_name,
                        root: &binding.root,
                        requested: &binding.targets,
                        profile,
                        location: &location,
                    })?;
                    instruction_readers.push(SourceInstructionReaders {
                        installation_id: binding.id.clone(),
                        report: compiled.readers,
                    });
                    Ok(compiled.desired)
                }
            }
        })();
        match compiled {
            Ok(compiled) => {
                request.sets.push(compiled.set);
                request.contracts.extend(compiled.contracts);
                if is_add {
                    let edge = Reference::DirectInstallation {
                        installation_id: binding.id.clone(),
                    };
                    if !session.state().references.contains(&edge) {
                        request.add_references.push(edge);
                    }
                }
            }
            Err(error) => {
                let category = if error.downcast_ref::<CapabilityError>().is_some() {
                    PreflightCategory::Capability
                } else {
                    PreflightCategory::Compilation
                };
                report
                    .diagnostics
                    .push(item_diagnostic(&binding.id, category, error));
            }
        }
    }
    report.instruction_readers = instruction_readers.clone();
    if !report.diagnostics.is_empty() {
        session.collect_preflight(&mut request, execution, &[], &mut report)?;
        normalize_report(&mut report);
        return Ok(OrchestrationOutcome::Blocked(vec![
            CommandDiagnostic::Preflight(report),
        ]));
    }
    let snapshots = captures
        .iter()
        .flatten()
        .map(|c| c.snapshot())
        .collect::<Vec<_>>();
    let prepared =
        session.prepare_upsert(selection.as_ref(), &snapshots, execution, |_| Ok(request))?;
    instruction_readers.sort_by(|a, b| a.installation_id.cmp(&b.installation_id));
    let captures = &captures;
    match prepared {
        PrepareOutcome::Blocked(mut diagnostics) => {
            for diagnostic in &mut diagnostics {
                if let CommandDiagnostic::Preflight(report) = diagnostic {
                    report.instruction_readers = instruction_readers.clone();
                    normalize_report(report);
                }
            }
            Ok(OrchestrationOutcome::Blocked(diagnostics))
        }
        PrepareOutcome::Ready(prepared) => {
            Ok(OrchestrationOutcome::Ready(use_prepared(SourcePrepared {
                instruction_readers,
                apply: Box::new(move || {
                    for capture in captures.iter().flatten() {
                        capture.verify_unchanged()?;
                    }
                    prepared.apply()
                }),
            })?))
        }
    }
}

fn item_diagnostic(
    id: &str,
    category: PreflightCategory,
    error: anyhow::Error,
) -> PreflightDiagnostic {
    PreflightDiagnostic {
        installation_ids: vec![id.into()],
        category,
        location: None,
        message: format!("{error:#}"),
    }
}

fn codex_capability(
    surface: TargetSurface,
    name: &'static str,
    reason: &'static str,
) -> anyhow::Error {
    CapabilityError {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Instructions, name),
        reason,
    }
    .into()
}

fn route_targets(
    route: SourceRoute<'_>,
    selection: TargetSelection<'_>,
    fallback: &[TargetSurface],
) -> Result<Vec<TargetSurface>> {
    let requested = effective_targets(selection, fallback);
    if matches!(route, SourceRoute::Codex { .. })
        && !matches!(&requested, Ok(values) if values == &[TargetSurface::CodexCli])
    {
        return Err(codex_capability(
            TargetSurface::CodexCli,
            "selection",
            "exactly one intentional Codex CLI surface is required",
        ));
    }
    requested
}

fn instruction_location(root: &InstallationRoot) -> InstructionLocation {
    InstructionLocation {
        root: root.clone(),
        path: match root.scope {
            OwnershipScope::Project => "AGENTS.md",
            OwnershipScope::User => ".codex/AGENTS.md",
        }
        .into(),
    }
}

fn ensure_source_rule_owner(
    state: &CombinedState,
    id: &str,
    route: SourceRoute<'_>,
    root: &InstallationRoot,
) -> Result<()> {
    let item = state
        .installations
        .iter()
        .find(|item| item.owner.installation_id == id)
        .context("selected installation is missing")?;
    let artifacts = state
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact
                .owners
                .iter()
                .any(|owner| owner.installation_id == id)
        })
        .collect::<Vec<_>>();
    if matches!(route, SourceRoute::Historical(_)) {
        ensure!(
            !item
                .requested
                .iter()
                .any(|target| target.family() == TargetSurface::CodexCli.family())
                && !artifacts.iter().any(|artifact| matches!(
                    artifact.content,
                    ArtifactContent::MarkdownContribution { .. }
                )),
            "instruction-backed installation requires the captured instruction adapter"
        );
        return Ok(());
    }
    ensure!(
        item.root.scope == root.scope && item.root.path.as_os_str() == root.path.as_os_str(),
        "retained Codex installation root differs from declared session root"
    );
    ensure!(
        item.owner.kind == ItemKind::Rule && item.requested == [TargetSurface::CodexCli],
        "retained installation requires an exact Codex CLI rule binding"
    );
    ensure!(
        artifacts.len() <= 1,
        "retained Codex installation requires zero or one canonical contribution"
    );
    let location = instruction_location(root);
    for artifact in artifacts {
        let ArtifactContent::MarkdownContribution { contribution } = &artifact.content else {
            anyhow::bail!("retained Codex installation cannot convert whole-file ownership");
        };
        ensure!(
            artifact.root.scope == root.scope
                && artifact.root.path.as_os_str() == root.path.as_os_str()
                && artifact.path == location.path,
            "retained Codex contribution requires the canonical destination"
        );
        ensure!(
            artifact.owners.len() == 1 && contribution.id == contribution_id(id)?,
            "retained Codex contribution requires its producer identity and sole installation ownership"
        );
        let association = state
            .owner_consumers
            .iter()
            .find(|value| {
                value.installation_id == id
                    && value.root.scope == root.scope
                    && value.root.path.as_os_str() == root.path.as_os_str()
                    && value.path == location.path
                    && value.contribution_id.as_deref() == Some(&contribution.id)
            })
            .context("retained Codex contribution is missing owner consumers")?;
        ensure!(
            association.consumers == [TargetSurface::CodexCli],
            "retained Codex owner consumers must be exactly Codex CLI"
        );
        let contract = state
            .owner_contracts
            .iter()
            .find(|value| {
                value.installation_id == id
                    && value.root.scope == root.scope
                    && value.root.path.as_os_str() == root.path.as_os_str()
                    && value.path == location.path
                    && value.contribution_id.as_deref() == Some(&contribution.id)
            })
            .context("retained Codex contribution is missing its owner contract")?;
        ensure!(
            contract.profiles
                == [ConsumerContract {
                    surface: TargetSurface::CodexCli,
                    contract_revision: 1
                }]
                && matches!(&contract.content, ContractContent::Instruction { content_sha256 } if content_sha256 == &contribution.content_sha256),
            "retained Codex owner contract must match revision 1"
        );
    }
    Ok(())
}

fn same_binding(item: &Installation, binding: &Binding, source: &str) -> bool {
    item.root.scope == binding.root.scope
        && item.root.path.as_os_str() == binding.root.path.as_os_str()
        && item.owner.source == source
        && item.owner.kind == ItemKind::Rule
        && item.owner.source_name == binding.source_name
        && item.owner.installed_name == binding.installed_name
}

fn new_installation_id(binding: &Binding, source: &str) -> Result<String> {
    // JSON tuple framing preserves raw root bytes and component boundaries.
    let bytes = serde_json::to_vec(&(
        "upskill-installation-v1",
        binding.root.scope,
        binding.root.path.as_os_str().as_encoded_bytes(),
        source,
        ItemKind::Rule,
        &binding.source_name,
        &binding.installed_name,
    ))?;
    let id = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(id)
}

fn installation_id(prior: &CombinedState, binding: &Binding, source: &str) -> Result<String> {
    let id = new_installation_id(binding, source)?;
    ensure!(
        !prior
            .installations
            .iter()
            .any(|item| item.owner.installation_id == id && !same_binding(item, binding, source)),
        "installation identity collides with a different binding"
    );
    let matching = prior
        .installations
        .iter()
        .filter(|item| same_binding(item, binding, source))
        .collect::<Vec<_>>();
    ensure!(
        matching.len() <= 1,
        "multiple recorded installations share the same source binding"
    );
    Ok(matching
        .first()
        .map_or(id, |item| item.owner.installation_id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::state::SourceEvidence;
    use crate::ownership::{ArtifactOwner, OwnershipScope};

    fn binding() -> Binding {
        Binding {
            id: String::new(),
            source_name: "source".into(),
            installed_name: "alias".into(),
            root: InstallationRoot {
                scope: OwnershipScope::Project,
                path: "/scope".into(),
            },
            targets: vec![TargetSurface::ClaudeCli],
        }
    }
    fn prior() -> CombinedState {
        CombinedState {
            schema: 2,
            origin: None,
            installations: vec![],
            bundles: vec![],
            references: vec![],
            artifacts: vec![],
            owner_consumers: vec![],
            owner_contracts: vec![],
            legacy: None,
        }
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn codex_pending_recovery_stops_invalid_options_and_missing_source_request() {
        use crate::filesystem::execution::{ExecutionOutcome, TEST_HOOK};
        for stale in [false, true] {
            for defect in ["profile", "selection", "missing-source"] {
                let dest = tempfile::tempdir().unwrap();
                let root = InstallationRoot {
                    scope: OwnershipScope::Project,
                    path: dest.path().into(),
                };
                std::fs::write(dest.path().join("AGENTS.md"), b"Manual instructions.\n").unwrap();
                let scope = QualifiedScope::open(&root).unwrap();
                let guard = scope.try_lock_writer().unwrap();
                let SessionOutcome::Ready(session) =
                    LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
                else {
                    panic!("expected initial session")
                };
                let PrepareOutcome::Ready(operation) = session
                    .prepare_upsert(None, &[], ResourceExecutionPolicy::Disabled, |_| {
                        Ok(CapturedUpsert::default())
                    })
                    .unwrap()
                else {
                    panic!("expected initial preparation")
                };
                TEST_HOOK.with(|hook| {
                    *hook.borrow_mut() = Some(Box::new(|point, _| {
                        if point == "after_initial_journal" {
                            anyhow::bail!("interrupt before effects");
                        }
                        Ok(())
                    }));
                });
                let interrupted = operation.apply();
                TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
                assert!(
                    interrupted
                        .unwrap_err()
                        .to_string()
                        .contains("interrupt before effects")
                );
                assert!(scope.has_pending_operation().unwrap());
                if stale {
                    std::fs::write(
                        dest.path().join(crate::lockfile::LOCKFILE_NAME),
                        b"late unrelated state",
                    )
                    .unwrap();
                }
                let outcome = with_prepared_codex(
                    &scope,
                    &guard,
                    root.clone(),
                    SourceRequest::Add(vec![AddRule {
                        source: InstallSource::LocalPath(dest.path().join("missing-source")),
                        source_name: "source".into(),
                        installed_name: "alias".into(),
                    }]),
                    CodexSourceOptions {
                        targets: TargetSelection {
                            invocation: if defect == "selection" {
                                &[TargetSurface::ClaudeCli]
                            } else {
                                &[]
                            },
                            ..Default::default()
                        },
                        add_defaults: &[TargetSurface::CodexCli],
                        profile: TargetProfile {
                            contract_revision: if defect == "profile" { 2 } else { 1 },
                            ..TargetSurface::CodexCli.profile()
                        },
                        execution: ResourceExecutionPolicy::Disabled,
                    },
                    |_| -> Result<()> { panic!("recovery must stop the new Codex request") },
                )
                .unwrap();
                let OrchestrationOutcome::Recovery(report) = outcome else {
                    panic!("pending recovery must precede {defect}")
                };
                if stale {
                    assert!(matches!(report.outcome, ExecutionOutcome::Reconcile { .. }));
                    assert_eq!(
                        std::fs::read(dest.path().join(crate::lockfile::LOCKFILE_NAME)).unwrap(),
                        b"late unrelated state"
                    );
                    assert!(scope.has_pending_operation().unwrap());
                } else {
                    assert_eq!(report.outcome, ExecutionOutcome::Complete);
                    assert!(!scope.has_pending_operation().unwrap());
                    let recovered = super::super::state::parse_state(
                        &std::fs::read_to_string(dest.path().join(crate::lockfile::LOCKFILE_NAME))
                            .unwrap(),
                    )
                    .unwrap();
                    assert!(recovered.installations.is_empty());
                    assert!(recovered.artifacts.is_empty());
                }
                assert_eq!(
                    std::fs::read(dest.path().join("AGENTS.md")).unwrap(),
                    b"Manual instructions.\n"
                );
                assert!(!dest.path().join("missing-source").exists());
            }
        }
    }

    #[test]
    fn identity_framing_distinguishes_raw_roots_and_rejects_different_binding_collision() {
        let binding = binding();
        let mut prior = prior();
        let id = installation_id(&prior, &binding, "local:/source").unwrap();
        let alias = Binding {
            installed_name: "other-alias".into(),
            ..self::binding()
        };
        assert_ne!(
            id,
            installation_id(&prior, &alias, "local:/source").unwrap()
        );
        let lexical = Binding {
            root: InstallationRoot {
                path: "/scope/.".into(),
                ..binding.root.clone()
            },
            ..self::binding()
        };
        assert_ne!(
            id,
            installation_id(&prior, &lexical, "local:/source").unwrap()
        );
        prior.installations.push(Installation {
            owner: ArtifactOwner {
                installation_id: id.clone(),
                source: "local:/other".into(),
                source_revision: None,
                kind: ItemKind::Rule,
                source_name: binding.source_name.clone(),
                installed_name: binding.installed_name.clone(),
            },
            root: binding.root.clone(),
            requested: binding.targets.clone(),
            source_evidence: SourceEvidence::Local {
                manifest_sha256: "0".repeat(64),
            },
        });
        assert!(installation_id(&prior, &binding, "local:/source").is_err());
        prior.installations[0].owner.source = "local:/source".into();
        prior.installations[0].owner.installation_id = "existing-caller-id".into();
        prior.installations[0].owner.source_revision = Some("actual-commit-evidence".into());
        assert_eq!(
            installation_id(&prior, &binding, "local:/source").unwrap(),
            "existing-caller-id"
        );
    }
}
