//! Pure captured Codex CLI instructions and a separate location reader report.
//!
//! Reader compatibility is structural. It grants no incidental ownership, physical
//! containment, native home authority or runtime qualification.
use super::{ContractedDesiredSet, DesiredInstallationSet};
use crate::generate::{directives::process_for_family, format::format_markdown};
use crate::instructions::aggregate::{Contribution, validate_contribution};
use crate::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use crate::lifecycle::contracts::{ConsumerContract, ContractContent, OwnerContract};
use crate::lifecycle::state::{
    CombinedState, Installation, Reference, SourceEvidence, validate_state,
};
use crate::model::{Audience, Rule};
use crate::operations::plan::DesiredArtifact;
use crate::ownership::{OwnershipScope, validate_relative_path};
use crate::pipeline::{ItemKind, effective_name, filter_ignored, is_entrypoint_or_override};
use crate::source_manifest::{SourceManifest, content_hash, validate_manifest};
use crate::target::{
    CapabilityCategory, CapabilityError, CapabilityKey, TargetProfile, TargetSurface,
};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct CodexInstructionRequest<'a> {
    pub installation: &'a Installation,
    pub source: &'a SourceManifest,
    pub folder_name: &'a str,
    pub profile: TargetProfile,
    pub location: &'a InstructionLocation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionReaderRole {
    Requested,
    PotentialIncidental,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionReader {
    pub profile: TargetProfile,
    pub role: InstructionReaderRole,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionCompatibility {
    NotEvaluated,
    UnconditionalMarkdown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionReaderReport {
    pub location: InstructionLocation,
    pub contract_revision: u32,
    pub readers: Vec<InstructionReader>,
    pub compatibility: InstructionCompatibility,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCodexInstructions {
    pub desired: ContractedDesiredSet,
    pub readers: InstructionReaderReport,
}

/// Compile one explicit revision-1 Codex CLI rule without I/O or input mutation.
///
/// The reviewed location contract supplies every potential reader, including
/// conditional VS Code Local AGENTS discovery through `chat.useAgentsMdFile`.
/// Callers must separately qualify the real project root or default native home.
pub fn compile_codex_instructions(
    request: CodexInstructionRequest<'_>,
) -> Result<CompiledCodexInstructions> {
    produce(request).with_context(|| {
        format!(
            "compile Codex instructions {}",
            request.installation.owner.installation_id
        )
    })
}

fn unsupported(surface: TargetSurface, name: &'static str, reason: &'static str) -> anyhow::Error {
    CapabilityError {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Instructions, name),
        reason,
    }
    .into()
}

fn produce(request: CodexInstructionRequest<'_>) -> Result<CompiledCodexInstructions> {
    let item = request.installation;
    let owner = &item.owner;
    ensure!(
        owner.kind == ItemKind::Rule,
        "Codex instructions require a rule"
    );
    if request.profile != TargetSurface::CodexCli.profile() {
        return Err(unsupported(
            request.profile.surface,
            "profile",
            "only Codex CLI contract revision 1 is supported",
        ));
    }
    if item.requested != [TargetSurface::CodexCli] {
        return Err(unsupported(
            TargetSurface::CodexCli,
            "selection",
            "exactly one intentional Codex CLI surface is required",
        ));
    }
    validate_state(&CombinedState {
        schema: 2,
        origin: None,
        installations: vec![item.clone()],
        bundles: vec![],
        references: vec![Reference::DirectInstallation {
            installation_id: owner.installation_id.clone(),
        }],
        artifacts: vec![],
        owner_consumers: vec![],
        owner_contracts: vec![],
        legacy: None,
    })
    .context("validate installation binding")?;
    validate_manifest(request.source).context("validate captured source")?;
    crate::compile::item::validate_topology(
        request.source.files.iter().map(|file| file.path.as_str()),
    )
    .context("validate captured source topology")?;
    let manifest_sha256 = match &item.source_evidence {
        SourceEvidence::Local { manifest_sha256 }
        | SourceEvidence::Git {
            manifest_sha256, ..
        } => manifest_sha256,
    };
    ensure!(
        manifest_sha256 == &request.source.content_sha256,
        "captured source differs from installation manifest"
    );
    ensure!(
        request.location.root.scope == item.root.scope
            && request.location.root.path.as_os_str() == item.root.path.as_os_str(),
        "instruction destination differs from installation root"
    );
    let expected_path = match item.root.scope {
        OwnershipScope::Project => "AGENTS.md",
        OwnershipScope::User => ".codex/AGENTS.md",
    };
    if request.location.path != expected_path {
        return Err(unsupported(
            TargetSurface::CodexCli,
            "destination",
            "only project AGENTS.md or default global .codex/AGENTS.md is supported",
        ));
    }
    validate_name(request.folder_name).context("invalid folder name")?;
    validate_name(&owner.installed_name).context("invalid alias")?;
    let entry = request
        .source
        .files
        .iter()
        .find(|f| f.path == "RULE.md")
        .context("missing canonical RULE.md entrypoint")?;
    let (rule, body) = crate::parse::frontmatter::parse::<Rule>(
        std::str::from_utf8(&entry.bytes).context("decode canonical RULE.md")?,
    )
    .context("parse canonical RULE.md")?;
    let source_name = effective_name(ItemKind::Rule, rule.name.as_deref(), request.folder_name);
    validate_name(&source_name).context("invalid resolved source name")?;
    ensure!(
        source_name == owner.source_name,
        "source name differs from installation binding"
    );
    if rule
        .scope
        .as_ref()
        .is_some_and(|scope| !scope.paths.is_empty())
    {
        return Err(unsupported(
            TargetSurface::CodexCli,
            "scope",
            "nonempty rule scope cannot be represented",
        ));
    }
    if rule.claude.is_some()
        || rule.copilot.is_some()
        || rule.opencode.is_some()
        || !rule.extra.is_empty()
    {
        return Err(unsupported(
            TargetSurface::CodexCli,
            "extensions",
            "native extension maps and unknown rule fields cannot be represented",
        ));
    }
    // Overrides are required behavior, even when the resource ignore list matches them.
    if request.source.files.iter().any(|file| {
        file.path != "RULE.md"
            && file.path.starts_with("RULE.")
            && file.path.ends_with(".md")
            && !file.path.contains('/')
    }) {
        return Err(unsupported(
            TargetSurface::CodexCli,
            "override",
            "rule override entrypoints cannot be represented",
        ));
    }
    let resources = request
        .source
        .files
        .iter()
        .filter(|file| file.path.contains('/') || !is_entrypoint_or_override(&file.path))
        .map(|file| PathBuf::from(&file.path))
        .collect();
    if !filter_ignored(resources, &rule.ignore).is_empty() {
        return Err(unsupported(
            TargetSurface::CodexCli,
            "resources",
            "effective resources cannot be represented",
        ));
    }
    let mut readers = reader_report(request.location);
    let mut desired = ContractedDesiredSet {
        set: DesiredInstallationSet {
            installation: item.clone(),
            artifacts: vec![],
            audience_excluded: vec![],
        },
        contracts: vec![],
    };
    if !eligible(&rule, request.profile)? {
        desired.set.audience_excluded.push(TargetSurface::CodexCli);
        return Ok(CompiledCodexInstructions { desired, readers });
    }
    let markdown = canonical_body(body, request.profile)?;
    ensure!(
        !markdown.trim().is_empty(),
        "instruction body must be nonempty"
    );
    for reader in &readers.readers {
        if !eligible(&rule, reader.profile)? {
            return Err(unsupported(
                reader.profile.surface,
                "audience",
                "source audience excludes a declared instruction reader",
            ));
        }
        if canonical_body(body, reader.profile)? != markdown {
            return Err(unsupported(
                reader.profile.surface,
                "directives",
                "declared instruction readers receive different canonical Markdown",
            ));
        }
    }
    let id = contribution_id(&owner.installation_id)?;
    validate_contribution(&Contribution::new(&id, &markdown))
        .context("validate canonical instruction contribution")?;
    desired.contracts.push(OwnerContract {
        root: item.root.clone(),
        path: request.location.path.clone(),
        contribution_id: Some(id.clone()),
        installation_id: owner.installation_id.clone(),
        source_manifest_sha256: request.source.content_sha256.clone(),
        content: ContractContent::Instruction {
            content_sha256: content_hash(markdown.as_bytes()),
        },
        profiles: vec![ConsumerContract {
            surface: TargetSurface::CodexCli,
            contract_revision: request.profile.contract_revision,
        }],
    });
    desired.set.artifacts.push(DesiredArtifact::Instruction(
        DesiredInstructionContribution {
            location: request.location.clone(),
            id,
            markdown,
            consumers: vec![TargetSurface::CodexCli],
        },
    ));
    readers.compatibility = InstructionCompatibility::UnconditionalMarkdown;
    Ok(CompiledCodexInstructions { desired, readers })
}

/// Stable identity shared by production and retained representation validation.
pub(crate) fn contribution_id(installation_id: &str) -> Result<String> {
    Ok(content_hash(&serde_json::to_vec(&(
        "upskill-codex-contribution-v1",
        installation_id,
    ))?))
}

fn validate_name(name: &str) -> Result<()> {
    validate_relative_path(name)?;
    ensure!(
        !name.contains('/'),
        "item name must be a single portable component"
    );
    Ok(())
}
fn eligible(rule: &Rule, profile: TargetProfile) -> Result<bool> {
    let audience = Audience::try_from(profile.family)?;
    Ok(rule
        .audience
        .as_ref()
        .is_none_or(|list| list.contains(&audience)))
}
fn canonical_body(body: &str, profile: TargetProfile) -> Result<String> {
    ensure!(
        !body
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')),
        "instruction body contains unrepresentable control characters"
    );
    format_markdown(&process_for_family(body, profile.family)?)
}
fn reader_report(location: &InstructionLocation) -> InstructionReaderReport {
    let surfaces: &[TargetSurface] = match location.root.scope {
        OwnershipScope::Project => &[
            TargetSurface::CodexCli,
            TargetSurface::CodexIde,
            TargetSurface::CopilotCli,
            TargetSurface::CopilotVscodeLocal,
            TargetSurface::OpenCodeCli,
        ],
        OwnershipScope::User => &[TargetSurface::CodexCli, TargetSurface::CodexIde],
    };
    let mut readers: Vec<_> = surfaces
        .iter()
        .map(|surface| InstructionReader {
            profile: surface.profile(),
            role: if *surface == TargetSurface::CodexCli {
                InstructionReaderRole::Requested
            } else {
                InstructionReaderRole::PotentialIncidental
            },
        })
        .collect();
    readers.sort_by_key(|reader| reader.profile.surface.name());
    InstructionReaderReport {
        location: location.clone(),
        contract_revision: 1,
        readers,
        compatibility: InstructionCompatibility::NotEvaluated,
    }
}
