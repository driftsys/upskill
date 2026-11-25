//! Bundle output planning without consumer state or filesystem writes.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::discovery::{find_command_dir, iter_item_resources};
use super::ignore::filter_ignored;
use crate::model::Command;
use crate::parse::frontmatter;

use super::ItemKind;
use super::install::{install_from_local_path_selective, resolve_bundle_closure, source_mode};
use super::plan::PlanNames;
use crate::generate::Client;
use crate::scope::Scope;
use crate::select::{ClientSelection, SelectedClient};
use crate::source::InstallSource;

pub struct BundleFile {
    pub kind: ItemKind,
    pub name: String,
    /// Relative to the client root: the planned path without its first
    /// component (`.claude/skills/a/SKILL.md` -> `skills/a/SKILL.md`).
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

/// A command declaration with the command read from its bundle's source.
pub struct BundleCommand {
    pub name: String,
    pub bundle: String,
    pub source: CommandSource,
}

/// A resource relative to the command directory.
pub struct CommandResource {
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}

/// The parsed command, its body and the resources that its ignore list keeps.
pub struct CommandSource {
    pub command: Command,
    pub body: String,
    pub resources: Vec<CommandResource>,
}

/// Read a named command without writing consumer state or generated files.
pub fn read_command(root: &Path, name: &str) -> Result<CommandSource> {
    let dir = find_command_dir(root, name)?.with_context(|| {
        format!(
            "no directory `{name}` with a COMMAND.md in {}",
            root.display()
        )
    })?;
    let path = dir.join("COMMAND.md");
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let (command, body) =
        frontmatter::parse::<Command>(&raw).with_context(|| format!("parse {}", path.display()))?;
    if command.name != name {
        bail!(
            "{}: `name: {}` must equal the directory name `{name}`",
            path.display(),
            command.name
        );
    }
    let mut resources = Vec::new();
    for rel in filter_ignored(iter_item_resources(&dir), &command.ignore) {
        let from = dir.join(&rel);
        resources.push(CommandResource {
            rel,
            bytes: std::fs::read(&from).with_context(|| format!("read {}", from.display()))?,
            mode: source_mode(&from)?,
        });
    }
    Ok(CommandSource {
        command,
        body: body.to_string(),
        resources,
    })
}

pub struct BundleOutput {
    /// The closure, dependencies before dependents; the last is the entry.
    pub bundles: Vec<crate::model::Bundle>,
    /// Skill and agent files for the client. No rule or command file.
    pub files: Vec<BundleFile>,
    /// Commands in closure order, then in each bundle list order.
    pub commands: Vec<BundleCommand>,
    /// Names of the rules that generate output for the client, sorted.
    pub rules: Vec<String>,
}

pub fn plan_bundle_output(
    registry_root: &Path,
    bundle: &str,
    client: Client,
) -> Result<BundleOutput> {
    if !crate::parse::bundle::discover(registry_root)?
        .iter()
        .any(|(_, found)| found.name == bundle)
    {
        bail!("no bundle named '{bundle}' in {}", registry_root.display());
    }
    let label = InstallSource::LocalPath(registry_root.to_path_buf()).to_string();
    let (closure, bundles) = resolve_bundle_closure(
        &label,
        registry_root,
        None,
        None,
        &[bundle.to_string()],
        &[],
    )?;
    let selected = match client {
        Client::Claude => SelectedClient::Claude,
        Client::Copilot => SelectedClient::Copilot,
        Client::OpenCode => SelectedClient::OpenCode,
    };
    let selection = ClientSelection::restrict(&[selected]);
    let mut files = Vec::new();
    let mut rules = BTreeSet::new();
    let mut planned = Vec::new();
    for si in closure.by_source.values() {
        let report = install_from_local_path_selective(
            &si.root,
            Some(&si.items),
            &selection,
            &PlanNames::default(),
            Scope::Project,
        )?;
        planned.extend(report.planned);
    }
    super::apply::check_installed_names(&planned)?;
    for item in planned {
        for file in item.files {
            if file.client != client {
                continue;
            }
            if item.kind == ItemKind::Rule {
                rules.insert(item.name.clone());
            } else {
                let rel = plugin_relative_path(&file.rel)?;
                files.push(BundleFile {
                    kind: item.kind,
                    name: item.name.clone(),
                    rel,
                    bytes: file.bytes,
                    mode: file.mode,
                });
            }
        }
    }
    // Read each command now: a fetched source is deleted when `closure` drops.
    let mut commands = Vec::new();
    for resolved in &bundles {
        for name in &resolved.bundle.items.commands {
            let root = &closure.source_roots[&resolved.source_label];
            let source = read_command(root, name).with_context(|| {
                format!("command `{name}` of bundle `{}`", resolved.bundle.name)
            })?;
            commands.push(BundleCommand {
                name: name.clone(),
                bundle: resolved.bundle.name.clone(),
                source,
            });
        }
    }
    Ok(BundleOutput {
        bundles: bundles
            .into_iter()
            .map(|resolved| resolved.bundle)
            .collect(),
        files,
        commands,
        rules: rules.into_iter().collect(),
    })
}

fn plugin_relative_path(path: &Path) -> Result<PathBuf> {
    let rel: PathBuf = path.components().skip(1).collect();
    if rel
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("invalid plugin relative path: {}", rel.display());
    }
    Ok(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_relative_path_refuses_parent_components_after_the_client_root() {
        for path in [
            ".claude/../outside.md",
            ".claude/skills/a/../../outside.md",
            ".github/agents/../outside.md",
        ] {
            let error = plugin_relative_path(Path::new(path)).unwrap_err();
            assert!(error.to_string().contains("invalid plugin relative path"));
        }
    }

    #[test]
    fn plugin_relative_path_keeps_normal_components_after_the_client_root() {
        for (path, relative) in [
            (".claude/skills/a/SKILL.md", "skills/a/SKILL.md"),
            (
                ".github/agents/reviewer.agent.md",
                "agents/reviewer.agent.md",
            ),
        ] {
            assert_eq!(
                plugin_relative_path(Path::new(path)).unwrap(),
                Path::new(relative)
            );
        }
    }
}
