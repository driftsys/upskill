//! Opt-in applying orchestration for explicitly selected source-backed rule roots.
//!
//! Sessions recover before source work; opaque scoped preparation retains compiler
//! bindings and full source identity through application. Production is unchanged.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

use super::commands::{
    CapturedUpsert, CommandDiagnostic, LifecycleSession, PrepareOutcome, SessionOutcome,
};
use super::desired::CompilerBinding;
use super::selection::{Selection, SelectionDiagnostic, SelectionOutcome, select_owned_nodes};
use super::sources::{ResolvedRuleSource, RuleSourceBinding, normalized_source_label};
use super::state::{CombinedState, Installation, Reference};
use crate::files::permissions::ResourceExecutionPolicy;
use crate::filesystem::{ExecutionReport, QualifiedScope, ScopeWriterGuard};
use crate::ownership::{ArtifactContent, InstallationRoot};
use crate::pipeline::ItemKind;
use crate::source::{InstallSource, parse_install_source_label};
use crate::target::{TargetProfile, TargetSurface};

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

/// An opaque source-bound operation. No replacement state or output is accepted.
pub struct SourcePrepared<'a> {
    apply: Box<dyn FnOnce() -> Result<ExecutionReport> + 'a>,
}
impl SourcePrepared<'_> {
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
    let SourceOptions {
        targets,
        add_defaults,
        compiler,
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
                    apply: Box::new(move || prepared.apply()),
                })?))
            }
        };
    }
    let is_add = matches!(&request, SourceRequest::Add(_));
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
                ensure_source_rule_owner(session.state(), id)?;
            }
            // Finish all selection and target preflight before even parsing sources.
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
                    targets: effective_targets(targets, &item.requested)?,
                });
                specs.push(parse_install_source_label(&item.owner.source)?);
            }
            Some(selection.clone())
        }
        SourceRequest::Add(items) => {
            ensure!(
                !items.is_empty(),
                "add requires at least one selected rule root"
            );
            let requested = effective_targets(targets, add_defaults)?;
            let mut aliases = std::collections::BTreeSet::new();
            for item in items {
                ensure!(
                    !item.source_name.is_empty() && !item.installed_name.is_empty(),
                    "source name and installed alias must be nonempty"
                );
                ensure!(
                    aliases.insert(&item.installed_name),
                    "duplicate installed alias in add request"
                );
                ensure!(
                    !session
                        .state()
                        .bundles
                        .iter()
                        .any(|bundle| bundle.name == item.installed_name),
                    "installed alias already belongs to an owned bundle"
                );
                for prior in session
                    .state()
                    .installations
                    .iter()
                    .filter(|prior| prior.owner.installed_name == item.installed_name)
                {
                    ensure_source_rule_owner(session.state(), &prior.owner.installation_id)?;
                }
            }
            for item in items {
                bindings.push(Binding {
                    id: String::new(),
                    source_name: item.source_name.clone(),
                    installed_name: item.installed_name.clone(),
                    root: root.clone(),
                    targets: requested.clone(),
                });
                if session
                    .state()
                    .installations
                    .iter()
                    .any(|prior| prior.owner.installed_name == item.installed_name)
                {
                    let label = normalized_source_label(&item.source)?;
                    let binding = bindings.last().expect("just inserted binding");
                    ensure!(
                        !session.state().installations.iter().any(|prior| prior
                            .owner
                            .installed_name
                            == item.installed_name
                            && !same_binding(prior, binding, &label)),
                        "installed alias already belongs to a different binding"
                    );
                }
                specs.push(item.source.clone());
            }
            let names = items
                .iter()
                .map(|item| item.installed_name.clone())
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
        .map(ResolvedRuleSource::resolve)
        .collect::<Result<Vec<_>>>()?;
    for (binding, source) in bindings.iter_mut().zip(&resolved) {
        if is_add {
            binding.id = installation_id(session.state(), binding, source.source_label())?;
            ensure!(
                !session
                    .state()
                    .installations
                    .iter()
                    .any(|item| item.owner.installed_name == binding.installed_name
                        && item.owner.installation_id != binding.id),
                "installed alias already belongs to a different binding"
            );
        } else {
            let item = session
                .state()
                .installations
                .iter()
                .find(|item| item.owner.installation_id == binding.id)
                .context("update binding disappeared")?;
            ensure!(
                item.owner.source == source.source_label(),
                "resolved source binding differs from recorded installation"
            );
        }
    }
    let captures = resolved
        .iter()
        .map(ResolvedRuleSource::capture)
        .collect::<Result<Vec<_>>>()?;
    let snapshots = captures
        .iter()
        .map(|capture| capture.snapshot())
        .collect::<Vec<_>>();
    let prepared =
        session.prepare_upsert(selection.as_ref(), &snapshots, compiler.execution, |_| {
            let mut request = CapturedUpsert::default();
            for (binding, capture) in bindings.iter().zip(&captures) {
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
                let compiled = capture.compile(RuleSourceBinding {
                    installation_id: &binding.id,
                    expected_source_name: &binding.source_name,
                    installed_name: &binding.installed_name,
                    root: &binding.root,
                    requested: &binding.targets,
                    bindings: &compiler_bindings,
                    profiles: &compiler.profiles,
                })?;
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
            Ok(request)
        })?;
    let captures = &captures;
    match prepared {
        PrepareOutcome::Blocked(diagnostics) => Ok(OrchestrationOutcome::Blocked(diagnostics)),
        PrepareOutcome::Ready(prepared) => {
            Ok(OrchestrationOutcome::Ready(use_prepared(SourcePrepared {
                apply: Box::new(move || {
                    for capture in captures {
                        capture.verify_unchanged()?;
                    }
                    prepared.apply()
                }),
            })?))
        }
    }
}

fn ensure_source_rule_owner(state: &CombinedState, id: &str) -> Result<()> {
    ensure!(
        !state.artifacts.iter().any(|artifact| {
            matches!(
                artifact.content,
                ArtifactContent::MarkdownContribution { .. }
            ) && artifact
                .owners
                .iter()
                .any(|owner| owner.installation_id == id)
        }),
        "instruction-backed installation requires the captured instruction adapter"
    );
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

fn installation_id(prior: &CombinedState, binding: &Binding, source: &str) -> Result<String> {
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
