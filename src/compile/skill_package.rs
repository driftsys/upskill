//! Pure inactive package plans; no native registration or qualification.

use crate::compile::item::CompiledItemFile;
use crate::compile::skill_selection::{
    SelectedSkill, SkillBodyOrigin, render_entrypoint, validate_name,
};
use crate::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use crate::pipeline::{ItemKind, is_entrypoint_or_override};
use crate::target::TargetSurface;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Component, PathBuf};

/// Scope must be canonicalized and qualified by the observation/application layer.
#[derive(Debug, Clone)]
pub struct SkillPackageRequest<'a> {
    pub selected: &'a SelectedSkill,
    pub surface: TargetSurface,
    pub scope: &'a InstallationRoot,
    pub owner: &'a ArtifactOwner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SkillPackageBinding {
    LocalMarketplace { directory: PathBuf },
    ExplicitSkillSource { directory: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPackage {
    pub item_root: String,
    pub revision_root: String,
    pub native_name: String,
    pub version: String,
    pub payload_sha256: String,
    pub package_sha256: String,
    /// Paths are relative to the selected scope, including the revision prefix.
    pub revision_files: Vec<CompiledItemFile>,
    /// A separately owned mutable selector; not part of the immutable package.
    pub catalog_file: Option<CompiledItemFile>,
    pub binding: SkillPackageBinding,
}

/// Compile an inactive plan without reading sources, qualifying roots or writing files.
/// Public selected records are revalidated for consistency, not authenticated.
pub fn compile_skill_package(request: &SkillPackageRequest<'_>) -> Result<SkillPackage> {
    let profile = profile(request.surface)?;
    let (scope_kind, scope_path) = scope_identity(request.scope)?;
    validate_selection(request)?;
    let mut identity = Sha256::new();
    identity.update(b"upskill:skill-owner:v1\0");
    identity_fields(&mut identity, request, scope_kind, scope_path);
    let native_name = format!("upskill-{}", hex(identity.finalize()));
    let item_root = format!(
        ".upskill/exports/{}/{}",
        profile.client, request.owner.installation_id
    );
    let mut files = content_files(request.selected)?;
    if let Some(path) = profile.plugin_manifest {
        files.push(json_file(
            path,
            plugin_manifest(request, &native_name, "0.0.0-payload"),
        )?);
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    super::item::validate_topology(files.iter().map(|file| file.path.as_str()))?;
    let payload_sha256 = payload_hash(request, scope_kind, scope_path, &files);
    let version = format!("0.0.0-r{payload_sha256}");
    if let Some(path) = profile.plugin_manifest {
        let file = files
            .iter_mut()
            .find(|file| file.path == path)
            .expect("inserted envelope");
        *file = json_file(path, plugin_manifest(request, &native_name, &version))?;
    }
    let mut digest = Sha256::new();
    digest.update(b"upskill:skill-package-files:v1\0");
    hash_files(&mut digest, &files);
    let package_sha256 = hex(digest.finalize());
    let revision_root = format!("{item_root}/revisions/{payload_sha256}");
    for file in &mut files {
        file.path = format!("{revision_root}/{}", file.path);
    }
    let (catalog_file, binding) = if let Some(path) = profile.marketplace_manifest {
        let source = format!("./revisions/{payload_sha256}");
        let entry = if request.surface == TargetSurface::CodexCli {
            json!({"name": native_name, "source": {"source": "local", "path": source},
                "policy": {"installation": "AVAILABLE", "authentication": "ON_INSTALL"}, "category": "Productivity"})
        } else {
            json!({"name": native_name, "source": source})
        };
        let mut catalog = json!({"name": native_name, "plugins": [entry]});
        if request.surface != TargetSurface::CodexCli {
            catalog["owner"] = json!({"name": "upskill"});
        }
        (
            Some(json_file(&format!("{item_root}/{path}"), catalog)?),
            SkillPackageBinding::LocalMarketplace {
                directory: request.scope.path.join(&item_root),
            },
        )
    } else {
        (
            None,
            SkillPackageBinding::ExplicitSkillSource {
                directory: request.scope.path.join(&revision_root).join("skills"),
            },
        )
    };
    Ok(SkillPackage {
        item_root,
        revision_root,
        native_name,
        version,
        payload_sha256,
        package_sha256,
        revision_files: files,
        catalog_file,
        binding,
    })
}

struct PackageProfile {
    client: &'static str,
    plugin_manifest: Option<&'static str>,
    marketplace_manifest: Option<&'static str>,
}

fn profile(surface: TargetSurface) -> Result<PackageProfile> {
    let (client, plugin_manifest, marketplace_manifest) = match surface {
        TargetSurface::ClaudeCli => (
            "claude",
            Some(".claude-plugin/plugin.json"),
            Some(".claude-plugin/marketplace.json"),
        ),
        TargetSurface::CopilotCli => (
            "copilot",
            Some(".plugin/plugin.json"),
            Some(".plugin/marketplace.json"),
        ),
        TargetSurface::CodexCli => (
            "codex",
            Some(".codex-plugin/plugin.json"),
            Some(".agents/plugins/marketplace.json"),
        ),
        TargetSurface::OpenCodeCli => ("opencode", None, None),
        _ => bail!("skill package planning does not support {}", surface.name()),
    };
    Ok(PackageProfile {
        client,
        plugin_manifest,
        marketplace_manifest,
    })
}

pub(crate) fn scope_identity(root: &InstallationRoot) -> Result<(&'static str, &str)> {
    let path = root
        .path
        .to_str()
        .context("skill package scope must be UTF-8")?;
    let normalized: PathBuf = root.path.components().collect();
    if !root.path.is_absolute()
        || path.chars().any(char::is_control)
        || root
            .path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        || normalized.as_os_str() != root.path.as_os_str()
    {
        bail!("skill package scope must be an absolute normalized path");
    }
    let kind = match root.scope {
        OwnershipScope::Project => "project",
        OwnershipScope::User => "user",
    };
    Ok((kind, path))
}

fn validate_selection(request: &SkillPackageRequest<'_>) -> Result<()> {
    let skill = request.selected;
    let owner = request.owner;
    if skill.surface != request.surface {
        bail!("selected skill surface does not match package target");
    }
    if owner.installation_id.is_empty()
        || !owner
            .installation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("invalid skill package installation id");
    }
    if owner.kind != ItemKind::Skill
        || owner.source.is_empty()
        || owner.source_revision.as_ref().is_some_and(String::is_empty)
        || owner.source_name != skill.source_name
        || owner.installed_name != skill.installed_name
    {
        bail!("skill package owner does not match selected skill identity");
    }
    validate_name(&skill.source_name)?;
    validate_name(&skill.installed_name)?;
    if skill.metadata.name != skill.installed_name
        || skill.metadata.description.chars().count() > 1024
    {
        bail!("invalid selected skill metadata binding");
    }
    if skill.source_manifest_sha256.len() != 64
        || !skill
            .source_manifest_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("invalid selected source manifest SHA-256");
    }
    if skill.entrypoint.path != format!("skills/{}/SKILL.md", skill.installed_name)
        || skill.entrypoint.mode.is_some()
        || skill.entrypoint.bytes != render_entrypoint(&skill.metadata, &skill.body)?.as_bytes()
    {
        bail!("selected skill entrypoint does not match its identity, metadata or body");
    }
    if let SkillBodyOrigin::Override { path } = &skill.body_origin {
        let suffix = match request.surface {
            TargetSurface::ClaudeCli => "claude",
            TargetSurface::CopilotCli => "copilot",
            TargetSurface::OpenCodeCli => "opencode",
            _ => bail!("selected target has no override form"),
        };
        if path != &format!("SKILL.{suffix}.md") {
            bail!("selected override origin does not match target");
        }
    }
    Ok(())
}

fn content_files(skill: &SelectedSkill) -> Result<Vec<CompiledItemFile>> {
    let mut files = vec![skill.entrypoint.clone()];
    let mut previous: Option<&str> = None;
    for resource in &skill.resources {
        crate::ownership::validate_relative_path(&resource.path)?;
        if previous.is_some_and(|path| path >= resource.path.as_str()) {
            bail!("selected resources must be strictly ordered and unique");
        }
        previous = Some(&resource.path);
        if !resource.path.contains('/')
            && (is_entrypoint_or_override(&resource.path) || resource.path == "SKILL.codex.md")
        {
            bail!(
                "source-only file cannot be a packaged skill resource: {}",
                resource.path
            );
        }
        if resource.mode.is_some_and(|mode| mode > 0o7777)
            || resource.content_sha256 != hex(Sha256::digest(&resource.bytes))
        {
            bail!("invalid selected resource evidence: {}", resource.path);
        }
        files.push(CompiledItemFile {
            path: format!("skills/{}/{}", skill.installed_name, resource.path),
            bytes: resource.bytes.clone(),
            mode: resource.mode,
        });
    }
    Ok(files)
}

fn plugin_manifest(
    request: &SkillPackageRequest<'_>,
    name: &str,
    version: &str,
) -> serde_json::Value {
    let mut manifest = json!({"name": name, "version": version, "description": request.selected.metadata.description});
    if request.surface == TargetSurface::CodexCli {
        manifest["skills"] = json!("./skills/");
    }
    manifest
}

fn json_file(path: &str, value: serde_json::Value) -> Result<CompiledItemFile> {
    let mut bytes =
        serde_json::to_vec_pretty(&value).context("serialize native skill package metadata")?;
    bytes.push(b'\n');
    Ok(CompiledItemFile {
        path: path.into(),
        bytes,
        mode: None,
    })
}

fn identity_fields(hash: &mut Sha256, request: &SkillPackageRequest<'_>, kind: &str, path: &str) {
    for value in [
        kind,
        path,
        &request.owner.installation_id,
        request.surface.name(),
    ] {
        frame(hash, value.as_bytes());
    }
}

fn payload_hash(
    request: &SkillPackageRequest<'_>,
    kind: &str,
    path: &str,
    files: &[CompiledItemFile],
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"upskill:skill-payload:v1\0");
    identity_fields(&mut hash, request, kind, path);
    frame(&mut hash, request.owner.source.as_bytes());
    match &request.owner.source_revision {
        None => hash.update([0]),
        Some(revision) => {
            hash.update([1]);
            frame(&mut hash, revision.as_bytes());
        }
    }
    for value in [
        &request.selected.source_name,
        &request.selected.installed_name,
        &request.selected.source_manifest_sha256,
    ] {
        frame(&mut hash, value.as_bytes());
    }
    match &request.selected.body_origin {
        SkillBodyOrigin::Canonical => frame(&mut hash, b"canonical"),
        SkillBodyOrigin::Override { path } => {
            frame(&mut hash, b"override");
            frame(&mut hash, path.as_bytes());
        }
    }
    hash_files(&mut hash, files);
    hex(hash.finalize())
}

fn hash_files(hash: &mut Sha256, files: &[CompiledItemFile]) {
    hash.update((files.len() as u64).to_be_bytes());
    for file in files {
        frame(hash, file.path.as_bytes());
        frame(hash, &file.bytes);
        match file.mode {
            None => hash.update([0]),
            Some(mode) => {
                hash.update([1]);
                hash.update(mode.to_be_bytes());
            }
        }
    }
}

fn frame(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}
fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
