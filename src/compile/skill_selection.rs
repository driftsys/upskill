//! Opt-in pure selection of captured skills for four CLI surfaces.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::compile::item::CompiledItemFile;
use crate::generate::directives::process_for_family;
use crate::model::{ItemRequires, License, Metadata, Skill};
use crate::parse::frontmatter;
use crate::pipeline::{filter_ignored, is_entrypoint_or_override};
use crate::source_manifest::{CapturedSourceFile, SourceManifest, validate_manifest};
use crate::target::{
    CapabilityCategory, CapabilityDeclaration, CapabilityError, CapabilityKey, TargetFamily,
    TargetSurface,
};
use anyhow::{Context, Result, bail};
use serde_yaml_ng::{Mapping, Value};

/// Projection declarations describe offline representability, not native qualification.
#[derive(Debug, Clone)]
pub struct SkillSelectionRequest<'a> {
    pub source: &'a SourceManifest,
    pub folder_name: &'a str,
    pub alias: Option<&'a str>,
    pub surface: TargetSurface,
    pub capabilities: &'a [CapabilityDeclaration],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillBodyOrigin {
    Canonical,
    Override { path: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub license: Option<License>,
    pub metadata: Option<Metadata>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedSkill {
    pub surface: TargetSurface,
    pub source_name: String,
    pub installed_name: String,
    pub source_manifest_sha256: String,
    pub metadata: SkillMetadata,
    pub requires: ItemRequires,
    pub body: String,
    pub body_origin: SkillBodyOrigin,
    pub entrypoint: CompiledItemFile,
    /// Paths remain relative to the authored skill resource tree.
    pub resources: Vec<CapturedSourceFile>,
}

/// Select without I/O; `None` means the canonical audience excludes this target.
pub fn select_skill(request: &SkillSelectionRequest<'_>) -> Result<Option<SelectedSkill>> {
    let source = request.source;
    validate_manifest(source).context("validate selected skill source manifest")?;
    super::item::validate_topology(source.files.iter().map(|file| file.path.as_str()))
        .context("validate selected skill source topology")?;
    if !matches!(
        request.surface,
        TargetSurface::ClaudeCli
            | TargetSurface::CopilotCli
            | TargetSurface::CodexCli
            | TargetSurface::OpenCodeCli
    ) {
        bail!(
            "selected skills do not support surface {}",
            request.surface.name()
        );
    }
    validate_name(request.folder_name).context("invalid skill folder name")?;
    let installed_name = request.alias.unwrap_or(request.folder_name);
    validate_name(installed_name).context("invalid installed skill name")?;
    let entry = source
        .files
        .iter()
        .find(|file| file.path == "SKILL.md")
        .context("missing canonical SKILL.md entrypoint")?;
    let text = std::str::from_utf8(&entry.bytes).context("decode canonical SKILL.md as UTF-8")?;
    let (skill, canonical_body) =
        frontmatter::parse::<Skill>(text).context("parse canonical SKILL.md")?;
    if skill
        .name
        .as_deref()
        .is_some_and(|name| name != request.folder_name)
    {
        bail!(
            "canonical skill name must equal folder name {:?}",
            request.folder_name
        );
    }
    if skill.description.chars().count() > 1024 {
        bail!("skill description must be at most 1024 characters");
    }
    if skill.audience.as_ref().is_some_and(|audience| {
        !audience
            .iter()
            .any(|target| TargetFamily::from(*target) == request.surface.family())
    }) {
        return Ok(None);
    }
    if source
        .files
        .iter()
        .any(|file| file.path == "SKILL.codex.md")
    {
        bail!("unsupported SKILL.codex.md override; use canonical Codex directives instead");
    }
    validate_projection(&skill, request)?;
    let (body, body_origin) = select_body(source, canonical_body, request.surface)?;
    let metadata = SkillMetadata {
        name: installed_name.into(),
        description: skill.description,
        license: skill.license,
        metadata: skill.metadata,
    };
    let rendered = render_entrypoint(&metadata, &body)?;
    let resources = select_resources(source, &skill.ignore);
    Ok(Some(SelectedSkill {
        surface: request.surface,
        source_name: request.folder_name.into(),
        installed_name: installed_name.into(),
        source_manifest_sha256: source.content_sha256.clone(),
        metadata,
        requires: skill.requires,
        body,
        body_origin,
        entrypoint: CompiledItemFile {
            path: format!("skills/{installed_name}/SKILL.md"),
            bytes: rendered.into_bytes(),
            mode: None,
        },
        resources,
    }))
}

pub(super) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('-')
        || name.ends_with('-')
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!(
            "skill name must be 1..=64 lowercase ASCII letters, digits or hyphens, without a leading or trailing hyphen"
        );
    }
    Ok(())
}

fn validate_projection(skill: &Skill, request: &SkillSelectionRequest<'_>) -> Result<()> {
    for (field, value) in [
        ("claude", &skill.claude),
        ("copilot", &skill.copilot),
        ("opencode", &skill.opencode),
    ] {
        if let Some(value) = value {
            if !matches!(value, Value::Mapping(map) if map.is_empty()) {
                return Err(unmapped_field(request.surface))
                    .with_context(|| format!("unsupported skill field {field}"));
            }
        }
    }
    if let Some(field) = skill.extra.keys().next() {
        return Err(unmapped_field(request.surface))
            .with_context(|| format!("unsupported skill field {field}"));
    }
    for (field, present) in [
        ("name", true),
        ("description", true),
        ("license", skill.license.is_some()),
        ("metadata", skill.metadata.is_some()),
    ] {
        if present {
            request
                .surface
                .profile()
                .validate_capability(
                    CapabilityKey::new(CapabilityCategory::Skills, field),
                    request.capabilities,
                )
                .with_context(|| format!("validate skill {field} projection"))?;
        }
    }
    Ok(())
}

fn unmapped_field(surface: TargetSurface) -> CapabilityError {
    CapabilityError {
        surface,
        key: CapabilityKey::new(CapabilityCategory::Skills, "native-frontmatter"),
        reason: "field has no reviewed selected-skill mapping",
    }
}

fn select_body(
    source: &SourceManifest,
    canonical: &str,
    surface: TargetSurface,
) -> Result<(String, SkillBodyOrigin)> {
    let suffix = match surface.family() {
        TargetFamily::Claude => Some("claude"),
        TargetFamily::Copilot => Some("copilot"),
        TargetFamily::OpenCode => Some("opencode"),
        _ => None,
    };
    if let Some(path) = suffix.map(|suffix| format!("SKILL.{suffix}.md")) {
        if let Some(file) = source.files.iter().find(|file| file.path == path) {
            let body = std::str::from_utf8(&file.bytes)
                .with_context(|| format!("decode override {path} as UTF-8"))?;
            if frontmatter::split(body).is_some() {
                bail!("override {path} must contain body content only, without YAML frontmatter");
            }
            return Ok((body.into(), SkillBodyOrigin::Override { path }));
        }
    }
    let body = process_for_family(canonical, surface.family())
        .context("process canonical skill directives")?;
    Ok((body, SkillBodyOrigin::Canonical))
}

pub(super) fn render_entrypoint(metadata: &SkillMetadata, body: &str) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(metadata.name.clone()));
    map.insert(
        Value::from("description"),
        Value::from(metadata.description.clone()),
    );
    if let Some(license) = &metadata.license {
        map.insert(Value::from("license"), serde_yaml_ng::to_value(license)?);
    }
    if let Some(extra) = &metadata.metadata {
        map.insert(Value::from("metadata"), serde_yaml_ng::to_value(extra)?);
    }
    let yaml = serde_yaml_ng::to_string(&map).context("serialize selected skill metadata")?;
    // A separator is generated syntax; the selected body's bytes stay intact.
    let separator = if body.is_empty() || body.starts_with('\n') || body.starts_with("\r\n") {
        ""
    } else {
        "\n"
    };
    Ok(format!("---\n{yaml}---\n{separator}{body}"))
}

fn select_resources(source: &SourceManifest, ignore: &[String]) -> Vec<CapturedSourceFile> {
    let paths = source
        .files
        .iter()
        .filter(|file| file.path.contains('/') || !is_entrypoint_or_override(&file.path))
        .map(|file| PathBuf::from(&file.path))
        .collect();
    let kept: HashSet<_> = filter_ignored(paths, ignore).into_iter().collect();
    source
        .files
        .iter()
        .filter(|file| kept.contains(&PathBuf::from(&file.path)))
        .cloned()
        .collect()
}
