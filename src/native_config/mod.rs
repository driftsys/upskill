//! Pure owned contributions to captured native configuration documents.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ops::Range;

mod jsonc;
mod toml;
use crate::files::plan::{FileLocation, FileValue, ObservedFile, PlannedFile};
use crate::ownership::InstallationRoot;
use crate::target::TargetSurface;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ConfigFormat {
    Jsonc,
    Toml,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ConfigSelector {
    Key(Vec<String>),
    ArrayEntry { path: Vec<String>, value: Value },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One captured layer. Tracking/ignore flags are caller-supplied provenance.
pub struct ConfigLayer {
    pub format: ConfigFormat,
    pub file: ObservedFile,
    pub tracked: bool,
    pub ignored: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedConfigEntry {
    pub id: String,
    pub selector: ConfigSelector,
    pub value: Value,
    pub raw_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Exact baseline restoration remains eligible only while `expected` matches
/// the complete current file, including mode. External edits disable it.
pub struct ConfigRollback {
    pub baseline: Option<FileValue>,
    pub expected: FileValue,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigOwnership {
    pub location: FileLocation,
    pub surface: TargetSurface,
    pub format: ConfigFormat,
    pub entries: Vec<OwnedConfigEntry>,
    pub rollback: Option<ConfigRollback>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Complete effective native layers in caller-established precedence order.
/// These records do not prove authenticity, qualification or native support.
pub struct CapturedConfig {
    pub surface: TargetSurface,
    pub scope: InstallationRoot,
    pub target: ConfigLayer,
    pub lower_layers: Vec<ConfigLayer>,
    pub higher_layers: Vec<ConfigLayer>,
    pub ownership: Option<ConfigOwnership>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigContribution {
    pub id: String,
    pub selector: ConfigSelector,
    pub expected: Option<OwnedConfigEntry>,
    pub desired: Option<Value>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// Apply the file precondition and resulting ownership together through the
/// separately admitted coordinator; this preview is not an execution receipt.
pub struct ConfigPatch {
    pub file: PlannedFile,
    pub ownership_after: Option<ConfigOwnership>,
}

#[derive(Debug)]
struct Node {
    span: Range<usize>,
    value: Option<Value>,
    children: Children,
    insert: Option<usize>,
    is_table: bool,
}
#[derive(Debug)]
enum Children {
    Object(Vec<Member>),
    Array(Vec<Node>),
    Scalar,
}
#[derive(Debug)]
struct Member {
    key: String,
    key_span: Range<usize>,
    separator: Range<usize>,
    node: Node,
}
impl Node {
    fn lookup(&self, path: &[String]) -> Result<Option<&Node>> {
        let mut current = self;
        for key in path {
            let Children::Object(members) = &current.children else {
                bail!("configuration selector crosses a non-object value");
            };
            let Some(member) = members.iter().find(|m| &m.key == key) else {
                return Ok(None);
            };
            current = &member.node;
        }
        Ok(Some(current))
    }
}
impl ConfigSelector {
    fn path(&self) -> &[String] {
        match self {
            Self::Key(path) | Self::ArrayEntry { path, .. } => path,
        }
    }
    fn selected<'a>(&self, root: &'a Node) -> Result<Option<&'a Node>> {
        let Some(node) = root.lookup(self.path())? else {
            return Ok(None);
        };
        match self {
            Self::Key(_) => Ok(Some(node)),
            Self::ArrayEntry { value, .. } => {
                let Children::Array(nodes) = &node.children else {
                    bail!("configuration selector is not an array");
                };
                let matching = nodes
                    .iter()
                    .filter(|node| node.value.as_ref() == Some(value))
                    .collect::<Vec<_>>();
                ensure!(matching.len() <= 1, "duplicate configuration array entry");
                Ok(matching.first().copied())
            }
        }
    }
    fn with_value(&self, value: &Value) -> Self {
        match self {
            Self::Key(path) => Self::Key(path.clone()),
            Self::ArrayEntry { path, .. } => Self::ArrayEntry {
                path: path.clone(),
                value: value.clone(),
            },
        }
    }
}
fn parse(format: ConfigFormat, source: &str) -> Result<Node> {
    match format {
        ConfigFormat::Jsonc => jsonc::parse(source),
        ConfigFormat::Toml => toml::parse(source),
    }
}
fn source(layer: &ConfigLayer) -> Result<&str> {
    layer.file.value.as_ref().map_or(
        Ok(match layer.format {
            ConfigFormat::Jsonc => "{}",
            ConfigFormat::Toml => "",
        }),
        |file| std::str::from_utf8(&file.bytes).context("native configuration must be UTF-8"),
    )
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn overlaps(a: &ConfigSelector, b: &ConfigSelector) -> bool {
    match (a, b) {
        (
            ConfigSelector::ArrayEntry { path: a, value: x },
            ConfigSelector::ArrayEntry { path: b, value: y },
        ) if a == b => x == y,
        _ => a.path().starts_with(b.path()) || b.path().starts_with(a.path()),
    }
}
fn validate_capture(config: &CapturedConfig) -> Result<()> {
    crate::compile::skill_package::scope_identity(&config.scope)?;
    ensure!(
        config.target.file.location.root == config.scope,
        "configuration write root differs from selected installation root"
    );
    crate::ownership::validate_relative_path(&config.target.file.location.path)?;
    ensure!(
        matches!(
            config.surface,
            TargetSurface::ClaudeCli
                | TargetSurface::CopilotCli
                | TargetSurface::CodexCli
                | TargetSurface::OpenCodeCli
        ),
        "unsupported native configuration surface"
    );
    ensure!(
        (config.surface == TargetSurface::CodexCli) == (config.target.format == ConfigFormat::Toml),
        "configuration format does not match surface"
    );
    if config.surface == TargetSurface::OpenCodeCli
        && config.scope.scope == crate::ownership::OwnershipScope::Project
    {
        ensure!(
            !config.target.tracked && config.target.ignored,
            "OpenCode project overlay must be ignored and untracked"
        );
        ensure!(
            matches!(
                config.target.file.location.path.as_str(),
                ".opencode/opencode.json" | ".opencode/opencode.jsonc"
            ),
            "OpenCode project configuration must use the local overlay"
        );
    }
    let mut locations = vec![&config.target.file.location];
    for layer in config.lower_layers.iter().chain(&config.higher_layers) {
        crate::compile::skill_package::scope_identity(&layer.file.location.root)?;
        crate::ownership::validate_relative_path(&layer.file.location.path)?;
        ensure!(
            layer.format == config.target.format,
            "inherited configuration format differs from target"
        );
        ensure!(
            !locations.contains(&&layer.file.location),
            "duplicate effective configuration layer"
        );
        locations.push(&layer.file.location);
    }
    Ok(())
}
fn validate_ownership(
    config: &CapturedConfig,
    root: &Node,
    text: &str,
) -> Result<Vec<OwnedConfigEntry>> {
    let Some(ownership) = &config.ownership else {
        return Ok(vec![]);
    };
    ensure!(
        ownership.location == config.target.file.location
            && ownership.surface == config.surface
            && ownership.format == config.target.format,
        "configuration ownership belongs to another document"
    );
    ensure!(
        !ownership.entries.is_empty(),
        "empty configuration ownership record"
    );
    if let Some(rollback) = &ownership.rollback {
        if let Some(baseline) = &rollback.baseline {
            parse(config.target.format, std::str::from_utf8(&baseline.bytes)?)
                .context("invalid configuration rollback baseline")?;
        }
        parse(
            config.target.format,
            std::str::from_utf8(&rollback.expected.bytes)?,
        )
        .context("invalid expected rollback document")?;
    }
    for (i, entry) in ownership.entries.iter().enumerate() {
        validate_selector(&entry.selector)?;
        ensure!(!entry.id.is_empty(), "empty configuration owner identity");
        for other in &ownership.entries[..i] {
            ensure!(
                entry.id != other.id && !overlaps(&entry.selector, &other.selector),
                "conflicting configuration ownership selectors"
            );
        }
        let selected = entry
            .selector
            .selected(root)?
            .context("owned configuration entry is missing")?;
        ensure!(
            selected.value.as_ref() == Some(&entry.value)
                && hash(text[selected.span.clone()].as_bytes()) == entry.raw_sha256,
            "owned configuration entry has changed"
        );
    }
    Ok(ownership.entries.clone())
}
fn validate_selector(selector: &ConfigSelector) -> Result<()> {
    ensure!(
        !selector.path().is_empty()
            && selector
                .path()
                .iter()
                .all(|s| !s.is_empty() && !s.chars().any(char::is_control)),
        "configuration selector must contain nonempty keys"
    );
    Ok(())
}
fn validate_layers(config: &CapturedConfig, change: &ConfigContribution) -> Result<()> {
    for layer in config.lower_layers.iter().chain(&config.higher_layers) {
        let root = parse(layer.format, source(layer)?)?;
        // Scalar selectors are exclusively managed. Array contributions can
        // coexist with lower manual entries, but never with the same identity.
        match &change.selector {
            ConfigSelector::Key(_) => ensure!(
                root.lookup(change.selector.path())?.is_none(),
                "configuration selector conflicts with an inherited layer"
            ),
            ConfigSelector::ArrayEntry { .. } => {
                ensure!(
                    change.selector.selected(&root)?.is_none(),
                    "inherited array source conflicts with owned selector"
                );
                if let Some(desired) = &change.desired {
                    ensure!(
                        change
                            .selector
                            .with_value(desired)
                            .selected(&root)?
                            .is_none(),
                        "desired source already occurs in another configuration layer"
                    );
                }
            }
        }
    }
    // Higher arrays might replace rather than append on an unqualified profile.
    // Reject them conservatively instead of inferring effective native exposure.
    for layer in &config.higher_layers {
        ensure!(
            parse(layer.format, source(layer)?)?
                .lookup(change.selector.path())?
                .is_none(),
            "configuration contribution is masked by a higher layer"
        );
    }
    Ok(())
}

/// Plan one contribution using captured evidence only. No filesystem or native
/// effect occurs. Callers must qualify the root and enforce the complete file
/// precondition before applying; captured layer completeness is caller evidence.
pub fn plan_config_patch(
    config: &CapturedConfig,
    change: &ConfigContribution,
) -> Result<ConfigPatch> {
    validate_capture(config)?;
    validate_selector(&change.selector)?;
    ensure!(
        !change.id.is_empty(),
        "configuration contribution identity is empty"
    );
    let text = source(&config.target)?;
    let root = parse(config.target.format, text)?;
    let mut entries = validate_ownership(config, &root, text)?;
    let old = entries.iter().find(|e| e.id == change.id).cloned();
    ensure!(
        old == change.expected,
        "configuration expected ownership differs from current evidence"
    );
    if let Some(old) = &old {
        ensure!(
            old.selector == change.selector,
            "configuration selector differs from owned selector"
        );
    }
    for entry in entries.iter().filter(|e| e.id != change.id) {
        ensure!(
            !overlaps(&entry.selector, &change.selector),
            "configuration selector overlaps another owner"
        );
        if let Some(desired) = &change.desired {
            ensure!(
                !overlaps(&entry.selector, &change.selector.with_value(desired)),
                "desired selector overlaps another owner"
            );
        }
    }
    validate_layers(config, change)?;
    let selected = change.selector.selected(&root)?;
    if old.is_none() {
        ensure!(
            selected.is_none(),
            "existing manual configuration entry cannot be adopted"
        );
        ensure!(
            change.desired.is_some(),
            "no owned configuration entry to remove"
        );
    }
    if let Some(desired) = &change.desired {
        if matches!(change.selector, ConfigSelector::ArrayEntry { .. })
            && old.as_ref().map(|e| &e.value) != Some(desired)
        {
            ensure!(
                change
                    .selector
                    .with_value(desired)
                    .selected(&root)?
                    .is_none(),
                "desired array source already exists"
            );
        }
    }
    let result = edit(
        config.target.format,
        text,
        &root,
        &change.selector,
        change.desired.as_ref(),
    )?;
    let result_root = parse(config.target.format, &result)
        .context("planned native configuration failed validation")?;
    entries.retain(|e| e.id != change.id);
    if let Some(value) = &change.desired {
        let selector = change.selector.with_value(value);
        let node = selector
            .selected(&result_root)?
            .context("planned configuration contribution is absent")?;
        ensure!(
            node.value.as_ref() == Some(value),
            "planned configuration value differs from desired value"
        );
        entries.push(OwnedConfigEntry {
            id: change.id.clone(),
            selector,
            value: value.clone(),
            raw_sha256: hash(result[node.span.clone()].as_bytes()),
        });
    }
    let before = config.target.file.value.clone();
    let after = FileValue {
        bytes: result.into_bytes(),
        mode: before.as_ref().map_or(Some(0o600), |v| v.mode),
    };
    let rollback = match &config.ownership {
        None => Some(ConfigRollback {
            baseline: before.clone(),
            expected: after.clone(),
        }),
        Some(ownership) => ownership
            .rollback
            .as_ref()
            .filter(|r| before.as_ref() == Some(&r.expected))
            .map(|r| ConfigRollback {
                baseline: r.baseline.clone(),
                expected: after.clone(),
            }),
    };
    if entries.is_empty() {
        return Ok(ConfigPatch {
            file: PlannedFile {
                location: config.target.file.location.clone(),
                before,
                after: rollback.map_or(Some(after), |r| r.baseline),
            },
            ownership_after: None,
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let ownership = ConfigOwnership {
        location: config.target.file.location.clone(),
        surface: config.surface,
        format: config.target.format,
        entries,
        rollback,
    };
    Ok(ConfigPatch {
        file: PlannedFile {
            location: config.target.file.location.clone(),
            before,
            after: Some(after),
        },
        ownership_after: Some(ownership),
    })
}

fn edit(
    format: ConfigFormat,
    text: &str,
    root: &Node,
    selector: &ConfigSelector,
    desired: Option<&Value>,
) -> Result<String> {
    if let Some(node) = selector.selected(root)? {
        if let Some(value) = desired {
            return match format {
                ConfigFormat::Jsonc => jsonc::replace(text, node.span.clone(), value),
                ConfigFormat::Toml => {
                    ensure!(
                        !node.is_table,
                        "replacing TOML table selectors is unsupported"
                    );
                    Ok(splice(
                        text,
                        vec![(node.span.clone(), toml::encode(value)?)],
                    ))
                }
            };
        }
        let (parent, member) = match selector {
            ConfigSelector::Key(path) => {
                let parent = root.lookup(&path[..path.len() - 1])?.unwrap();
                let Children::Object(members) = &parent.children else {
                    unreachable!()
                };
                (
                    parent,
                    members.iter().find(|m| m.key == path[path.len() - 1]),
                )
            }
            ConfigSelector::ArrayEntry { path, .. } => (root.lookup(path)?.unwrap(), None),
        };
        return match format {
            ConfigFormat::Jsonc => jsonc::remove(text, parent, node, member),
            ConfigFormat::Toml => toml::remove(text, parent, node, member),
        };
    }
    let value = desired.context("no owned configuration value to remove")?;
    if let ConfigSelector::ArrayEntry { path, .. } = selector {
        if let Some(container) = root.lookup(path)? {
            return match format {
                ConfigFormat::Jsonc => jsonc::insert(text, container, None, value),
                ConfigFormat::Toml => toml::insert_array(text, container, value),
            };
        }
    }
    let value = if matches!(selector, ConfigSelector::ArrayEntry { .. }) {
        Value::Array(vec![value.clone()])
    } else {
        value.clone()
    };
    match format {
        ConfigFormat::Toml => toml::insert(text, root, selector.path(), &value),
        ConfigFormat::Jsonc => {
            let path = selector.path();
            for i in 0..path.len() {
                let parent = root.lookup(&path[..i])?.unwrap();
                if root.lookup(&path[..i + 1])?.is_none() {
                    let mut nested = value;
                    for key in path[i + 1..].iter().rev() {
                        nested = serde_json::json!({key:nested});
                    }
                    ensure!(
                        matches!(parent.children, Children::Object(_)),
                        "configuration parent is not an object"
                    );
                    return jsonc::insert(text, parent, Some(&path[i]), &nested);
                }
            }
            bail!("configuration entry already exists")
        }
    }
}
fn splice(source: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0.start));
    let mut result = source.to_owned();
    for (range, value) in edits {
        result.replace_range(range, &value);
    }
    result
}
