//! Complete captured rule outputs bound to explicit installation consumers.
//!
//! These pure values establish no ownership, containment or native conformance.
use crate::compile::item::{ItemCompileRequest, compile_item};
use crate::files::plan::{DesiredFile, FileLocation, FileValue};
use crate::generate::Client;
use crate::lifecycle::state::{
    CombinedState, Installation, Reference, SourceEvidence, validate_state,
};
use crate::model::Rule;
use crate::operations::plan::DesiredArtifact;
use crate::pipeline::ItemKind;
use crate::source_manifest::{SourceManifest, validate_manifest};
use crate::target::TargetSurface;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerBinding {
    pub client: Client,
    pub consumers: Vec<TargetSurface>,
}
#[derive(Debug, Clone, Copy)]
pub struct RuleDesiredSetRequest<'a> {
    pub installation: &'a Installation,
    pub source: &'a SourceManifest,
    pub folder_name: &'a str,
    pub bindings: &'a [CompilerBinding],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredInstallationSet {
    pub installation: Installation,
    pub artifacts: Vec<DesiredArtifact>,
    pub audience_excluded: Vec<TargetSurface>,
}

/// Produce a complete desired set from validated captured bytes without I/O.
///
/// Empty audience-filtered output retains installation history; it is not a remove
/// operation. Family compatibility does not qualify native behavior or physical modes.
pub fn compile_rule_desired_set(
    request: RuleDesiredSetRequest<'_>,
) -> Result<DesiredInstallationSet> {
    produce(&request).with_context(|| {
        format!(
            "compile desired rule {}",
            request.installation.owner.installation_id
        )
    })
}

fn produce(request: &RuleDesiredSetRequest<'_>) -> Result<DesiredInstallationSet> {
    let installation = request.installation;
    let owner = &installation.owner;
    ensure!(
        owner.kind == ItemKind::Rule,
        "only rule desired sets are supported"
    );
    validate_state(&CombinedState {
        schema: 2,
        installations: vec![installation.clone()],
        bundles: Vec::new(),
        references: vec![Reference::DirectInstallation {
            installation_id: owner.installation_id.clone(),
        }],
        artifacts: Vec::new(),
        owner_consumers: Vec::new(),
        legacy: None,
    })
    .context("validate installation binding")?;
    validate_manifest(request.source).context("validate captured source")?;
    let manifest_sha256 = match &installation.source_evidence {
        SourceEvidence::Git {
            manifest_sha256, ..
        }
        | SourceEvidence::Local { manifest_sha256 } => manifest_sha256,
    };
    ensure!(
        manifest_sha256 == &request.source.content_sha256,
        "captured source differs from installation manifest"
    );
    for surface in &installation.requested {
        ensure!(
            !matches!(surface, TargetSurface::CodexCli | TargetSurface::CodexIde),
            "unsupported compiler surface: {}",
            surface.name()
        );
    }
    let bindings = validate_bindings(installation, request.bindings)?;
    let compiled = compile_item(
        request.source,
        &ItemCompileRequest {
            kind: ItemKind::Rule,
            folder_name: request.folder_name.into(),
            alias: Some(owner.installed_name.clone()),
            clients: request
                .bindings
                .iter()
                .map(|binding| binding.client)
                .collect(),
        },
    )?;
    ensure!(
        compiled.source_name == owner.source_name,
        "compiled source name differs from installation binding"
    );
    ensure!(
        compiled.installed_name == owner.installed_name,
        "compiled alias differs from installation binding"
    );
    ensure!(
        compiled.source_manifest_sha256 == *manifest_sha256,
        "compiled manifest differs from installation binding"
    );
    if compiled
        .outputs
        .iter()
        .any(|output| output.client == Client::OpenCode)
    {
        let entry = request
            .source
            .files
            .iter()
            .find(|file| file.path == "RULE.md")
            .expect("compiler requires rule entrypoint");
        let (rule, _) =
            crate::parse::frontmatter::parse::<Rule>(std::str::from_utf8(&entry.bytes)?)?;
        ensure!(
            !rule
                .scope
                .as_ref()
                .is_some_and(|scope| !scope.paths.is_empty()),
            "OpenCode rule scope cannot be represented by this compiler"
        );
    }
    let mut artifacts = BTreeMap::new();
    for output in compiled.outputs {
        let mut consumers = bindings[output.client.name()].consumers.clone();
        consumers.sort_by_key(|surface| surface.name());
        for file in output.files {
            let path = file.path;
            let desired = DesiredArtifact::File(DesiredFile {
                location: FileLocation {
                    root: installation.root.clone(),
                    path: path.clone(),
                },
                value: FileValue {
                    bytes: file.bytes,
                    mode: file.mode,
                },
                consumers: consumers.clone(),
            });
            ensure!(
                artifacts.insert(path, desired).is_none(),
                "repeated desired output path"
            );
        }
    }
    let mut audience_excluded: Vec<_> = compiled
        .audience_excluded
        .iter()
        .flat_map(|client| bindings[client.name()].consumers.iter().copied())
        .collect();
    audience_excluded.sort_by_key(|surface| surface.name());
    Ok(DesiredInstallationSet {
        installation: installation.clone(),
        artifacts: artifacts.into_values().collect(),
        audience_excluded,
    })
}

fn validate_bindings<'a>(
    installation: &Installation,
    bindings: &'a [CompilerBinding],
) -> Result<BTreeMap<&'static str, &'a CompilerBinding>> {
    let mut indexed = BTreeMap::new();
    let mut consumers = BTreeSet::new();
    for binding in bindings {
        ensure!(
            indexed.insert(binding.client.name(), binding).is_none(),
            "duplicate compiler binding for {}",
            binding.client.name()
        );
        ensure!(
            !binding.consumers.is_empty(),
            "compiler binding requires explicit consumers"
        );
        for surface in &binding.consumers {
            ensure!(
                compatible(binding.client, *surface),
                "compiler {} is incompatible with {}",
                binding.client.name(),
                surface.name()
            );
            ensure!(
                consumers.insert(surface.name()),
                "duplicate compiler consumer: {}",
                surface.name()
            );
        }
    }
    ensure!(
        consumers
            == installation
                .requested
                .iter()
                .map(|surface| surface.name())
                .collect(),
        "compiler consumers must exactly account for requested surfaces"
    );
    Ok(indexed)
}

fn compatible(client: Client, surface: TargetSurface) -> bool {
    matches!(
        (client, surface),
        (Client::Claude, TargetSurface::ClaudeCli)
            | (
                Client::Copilot,
                TargetSurface::CopilotCli | TargetSurface::CopilotVscodeLocal
            )
            | (Client::OpenCode, TargetSurface::OpenCodeCli)
    )
}
