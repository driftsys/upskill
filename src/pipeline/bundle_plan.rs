//! Bundle output planning without consumer state or filesystem writes.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

use super::ItemKind;
use super::install::{install_from_local_path_selective, resolve_bundle_closure};
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

pub struct BundleOutput {
    /// The closure, dependencies before dependents; the last is the entry.
    pub bundles: Vec<crate::model::Bundle>,
    /// Skill and agent files for the client. No rule file.
    pub files: Vec<BundleFile>,
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
    Ok(BundleOutput {
        bundles: bundles
            .into_iter()
            .map(|resolved| resolved.bundle)
            .collect(),
        files,
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
