//! Complete per-client item outputs from captured source values.

use crate::generate::link_rewrite::{reprefix_resource_links, rewrite_resource_links};
use crate::generate::{render_agent, render_rule, render_skill};
use crate::legacy::layout;
use crate::model::{Agent, Audience, Rule, Skill};
use crate::parse::frontmatter;
use crate::pipeline::{effective_name, filter_ignored, is_entrypoint_or_override};
use crate::source_manifest::validate_manifest;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeSet, HashSet};
use std::path::{Component, Path, PathBuf};

use crate::generate::Client;
use crate::pipeline::ItemKind;
use crate::source_manifest::SourceManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemCompileRequest {
    pub kind: ItemKind,
    pub folder_name: String,
    pub alias: Option<String>,
    pub clients: Vec<Client>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledItemFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledClientItem {
    pub client: Client,
    pub files: Vec<CompiledItemFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledItem {
    pub kind: ItemKind,
    pub source_name: String,
    pub installed_name: String,
    pub source_manifest_sha256: String,
    pub outputs: Vec<CompiledClientItem>,
    pub audience_excluded: Vec<Client>,
}

/// Compile a complete item without source reads, output writes or input mutation.
///
/// The supplied manifest is validated in full before parsing. Returned files do
/// not establish native support, ownership or permission application policy.
pub fn compile_item(source: &SourceManifest, request: &ItemCompileRequest) -> Result<CompiledItem> {
    validate_manifest(source).context("validate source manifest for item compilation")?;
    validate_topology(source.files.iter().map(|file| file.path.as_str()))
        .context("validate source topology")?;
    validate_name(&request.folder_name).context("invalid folder name")?;
    if let Some(alias) = &request.alias {
        validate_name(alias).context("invalid alias")?;
    }
    if request.clients.is_empty() {
        bail!("requested client selection must be nonempty");
    }
    let requested: HashSet<_> = request.clients.iter().copied().collect();
    if requested.len() != request.clients.len() {
        bail!("requested client selection contains duplicate clients");
    }

    let entrypoint = request.kind.entrypoint_filename();
    let entry = source
        .files
        .iter()
        .find(|file| file.path == entrypoint)
        .with_context(|| format!("missing canonical {entrypoint} entrypoint"))?;
    let text = std::str::from_utf8(&entry.bytes)
        .with_context(|| format!("decode canonical {entrypoint} as UTF-8"))?;
    let (item, body) =
        match request.kind {
            ItemKind::Skill => frontmatter::parse::<Skill>(text)
                .map(|(item, body)| (ParsedItem::Skill(item), body)),
            ItemKind::Rule => {
                frontmatter::parse::<Rule>(text).map(|(item, body)| (ParsedItem::Rule(item), body))
            }
            ItemKind::Agent => frontmatter::parse::<Agent>(text)
                .map(|(item, body)| (ParsedItem::Agent(item), body)),
        }
        .with_context(|| format!("parse canonical {entrypoint}"))?;
    let source_name = effective_name(request.kind, item.declared_name(), &request.folder_name);
    validate_name(&source_name).context("invalid resolved source name")?;
    let installed_name = request.alias.as_ref().unwrap_or(&source_name).clone();

    let resource_paths = source
        .files
        .iter()
        .filter(|file| file.path.contains('/') || !is_entrypoint_or_override(&file.path))
        .map(|file| PathBuf::from(&file.path))
        .collect();
    let copied: HashSet<PathBuf> = filter_ignored(resource_paths, item.ignore())
        .into_iter()
        .collect();
    let resources: Vec<_> = source
        .files
        .iter()
        .filter(|file| copied.contains(&PathBuf::from(&file.path)))
        .collect();
    let mut outputs = Vec::new();
    let mut audience_excluded = Vec::new();
    for client in Client::ALL {
        if !requested.contains(&client) {
            continue;
        }
        if !item.targets(client) {
            audience_excluded.push(client);
            continue;
        }
        let override_name = format!(
            "{}.{}.md",
            entrypoint
                .strip_suffix(".md")
                .expect("canonical Markdown entrypoint"),
            client.name()
        );
        if source.files.iter().any(|file| file.path == override_name) {
            bail!(
                "unsupported override {override_name} for {} {source_name} ({})",
                request.kind,
                client.name()
            );
        }
        let mut rendered = item.render(&source_name, body, client).with_context(|| {
            format!(
                "render {} {source_name} for {}",
                request.kind,
                client.name()
            )
        })?;
        if !layout::is_dir_backed(request.kind, client.into()) {
            rendered =
                rewrite_resource_links(&rendered, &source_name, &copied).with_context(|| {
                    format!(
                        "rewrite resource links for {source_name} ({})",
                        client.name()
                    )
                })?;
            if installed_name != source_name {
                rendered =
                    reprefix_resource_links(&rendered, &source_name, &installed_name, &copied)
                        .with_context(|| {
                            format!(
                                "re-prefix resource links for {installed_name} ({})",
                                client.name()
                            )
                        })?;
            }
        }
        let mut files = vec![CompiledItemFile {
            path: portable_path(&layout::output_path(
                request.kind,
                client.into(),
                &installed_name,
            ))?,
            bytes: rendered.into_bytes(),
            mode: None,
        }];
        let resource_base =
            layout::resource_base_path(request.kind, client.into(), &installed_name);
        for resource in &resources {
            files.push(CompiledItemFile {
                path: portable_path(&resource_base.join(&resource.path))?,
                bytes: resource.bytes.clone(),
                mode: resource.mode,
            });
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        validate_topology(files.iter().map(|file| file.path.as_str()))
            .with_context(|| format!("validate generated output topology for {}", client.name()))?;
        outputs.push(CompiledClientItem { client, files });
    }
    Ok(CompiledItem {
        kind: request.kind,
        source_name,
        installed_name,
        source_manifest_sha256: source.content_sha256.clone(),
        outputs,
        audience_excluded,
    })
}

fn validate_name(name: &str) -> Result<()> {
    crate::ownership::validate_relative_path(name)?;
    if name.contains('/') {
        bail!("item name must be a single portable component");
    }
    Ok(())
}

fn portable_path(path: &Path) -> Result<String> {
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(part) => part
                .to_str()
                .context("generated path contains a non-UTF-8 component"),
            _ => bail!("generated path contains a non-normal component"),
        })
        .collect::<Result<Vec<_>>>()?;
    let portable = components.join("/");
    crate::ownership::validate_relative_path(&portable)
        .with_context(|| format!("invalid generated path {portable:?}"))?;
    Ok(portable)
}

fn validate_topology<'a>(paths: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for path in paths {
        if !seen.insert(path) {
            bail!("duplicate file location {path:?}");
        }
    }
    for path in &seen {
        for (offset, _) in path.match_indices('/') {
            if seen.contains(&path[..offset]) {
                bail!(
                    "file/descendant topology conflict between {:?} and {path:?}",
                    &path[..offset]
                );
            }
        }
    }
    Ok(())
}

enum ParsedItem {
    Skill(Skill),
    Rule(Rule),
    Agent(Agent),
}

impl ParsedItem {
    fn declared_name(&self) -> Option<&str> {
        match self {
            Self::Skill(item) => item.name.as_deref(),
            Self::Rule(item) => item.name.as_deref(),
            Self::Agent(item) => item.name.as_deref(),
        }
    }

    fn ignore(&self) -> &[String] {
        match self {
            Self::Skill(item) => &item.ignore,
            Self::Rule(item) => &item.ignore,
            Self::Agent(item) => &item.ignore,
        }
    }

    fn targets(&self, client: Client) -> bool {
        let audience = match self {
            Self::Skill(item) => item.audience.as_deref(),
            Self::Rule(item) => item.audience.as_deref(),
            Self::Agent(item) => item.audience.as_deref(),
        };
        audience.is_none_or(|list| {
            list.iter().any(|audience| {
                matches!(
                    (client, audience),
                    (Client::Claude, Audience::Claude)
                        | (Client::Copilot, Audience::Copilot)
                        | (Client::OpenCode, Audience::OpenCode)
                )
            })
        })
    }

    fn render(&self, name: &str, body: &str, client: Client) -> Result<String> {
        match self {
            Self::Skill(item) => render_skill(item, name, body, client),
            Self::Rule(item) => render_rule(item, name, body, client),
            Self::Agent(item) => render_agent(item, name, body, client),
        }
    }
}
