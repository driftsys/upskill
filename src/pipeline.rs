//! v0.2 install pipeline: local SSOT path → per-client output on disk.
//!
//! Walks a source directory laid out per format-spec §2.1, parses each
//! item's frontmatter into the model (§3), renders per-client output via
//! `crate::generate`, and writes the result to the per-client paths
//! defined in format-spec §7 / ADR-0003.
//!
//! Scope (Phase 3 first slice):
//! - Local path source only (no git fetch).
//! - No lockfile, no bundle resolution, no ancillary file management
//!   (`CLAUDE.md`, `.vscode/settings.json`, `opencode.json`).
//! - No CLI wiring; this is library API only.
//!
//! Audience filter: respects `metadata.audience` per the current model
//! shape. Promotion of `audience` to a top-level field (per the
//! format-spec PR #76) is a separate task.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

use crate::fetch;
use crate::generate::{self, Client};
use crate::model::{Agent, Audience, Rule, Skill};
use crate::parse::frontmatter;
use crate::source::{GithubRepo, InstallSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Rule,
    Skill,
    Agent,
}

#[derive(Debug, Clone)]
pub struct InstalledItem {
    pub kind: ItemKind,
    pub name: String,
    pub client: Client,
    /// Path relative to the install target root.
    pub output_path: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct InstallReport {
    pub items: Vec<InstalledItem>,
}

const ALL_CLIENTS: [Client; 3] = [Client::Claude, Client::Copilot, Client::OpenCode];

/// Install every item under `source` into `target`, generating per-client
/// output for each client unless filtered by `metadata.audience`.
pub fn install_from_local_path(source: &Path, target: &Path) -> Result<InstallReport> {
    let mut report = InstallReport::default();

    install_skills(source, target, &mut report)?;
    install_rules(source, target, &mut report)?;
    install_agents(source, target, &mut report)?;

    Ok(report)
}

/// Install items from any supported source into `target`.
///
/// Dispatches on the source variant:
/// - `LocalPath` — installs directly from the path on disk.
/// - `Github` — shallow-clones `https://github.com/<owner>/<repo>` into a
///   temp directory, resolves the optional subfolder, then runs the local
///   install pipeline against the resolved path. The temp clone is removed
///   on success or failure.
/// - `Gitlab` — not yet supported in this slice; tracked as a follow-up.
pub fn install_from_source(source: &InstallSource, target: &Path) -> Result<InstallReport> {
    match source {
        InstallSource::LocalPath(path) => install_from_local_path(path, target),
        InstallSource::Github(repo) => install_from_github(repo, target),
        InstallSource::Gitlab(_) => Err(anyhow!(
            "GitLab sources are not yet supported by the v0.2 pipeline; \
             use a GitHub source or a local path"
        )),
    }
}

fn install_from_github(repo: &GithubRepo, target: &Path) -> Result<InstallReport> {
    let url = format!("https://github.com/{}/{}.git", repo.owner, repo.name);
    install_from_git_url(
        &url,
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
    )
}

/// Shallow-clone `url` into a tempdir, resolve `subfolder` inside the clone,
/// and run the local install pipeline against the result. The tempdir is
/// removed on return regardless of outcome (RAII via `tempfile::TempDir`).
///
/// Public so callers can install from arbitrary git URLs (mirrors, local
/// `file://` clones, future GitLab self-hosted) without going through
/// [`InstallSource`]. The high-level [`install_from_source`] is preferred
/// when an `InstallSource` already exists.
pub fn install_from_git_url(
    url: &str,
    git_ref: Option<&str>,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
    target: &Path,
) -> Result<InstallReport> {
    let tmp = tempfile::tempdir().context("create temp dir for clone")?;
    fetch::shallow_clone(url, git_ref, "clone", tmp.path())
        .map_err(|e| anyhow!("git clone {}: {}", url, e))?;
    let source = fetch::resolve_subfolder(&tmp.path().join("clone"), subfolder, owner, name)
        .map_err(|e| anyhow!("{}", e))?;
    install_from_local_path(&source, target)
}

fn install_skills(source: &Path, target: &Path, report: &mut InstallReport) -> Result<()> {
    for (name, dir) in iter_item_dirs(&source.join("skills"))? {
        let entry_path = dir.join("SKILL.md");
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;
        let (skill, body) = frontmatter::parse::<Skill>(&raw)
            .with_context(|| format!("parse {}", entry_path.display()))?;
        let audience = audience_of(skill.metadata.as_ref());

        for client in ALL_CLIENTS {
            if !targets(client, audience.as_deref()) {
                continue;
            }
            let rendered = generate::render_skill(&skill, body, client)
                .with_context(|| format!("render skill {} for {:?}", name, client))?;
            let rel = skill_output_path(client, &name);
            write_output(target, &rel, &rendered)?;
            report.items.push(InstalledItem {
                kind: ItemKind::Skill,
                name: name.clone(),
                client,
                output_path: rel,
            });
        }
    }
    Ok(())
}

fn install_rules(source: &Path, target: &Path, report: &mut InstallReport) -> Result<()> {
    for (name, dir) in iter_item_dirs(&source.join("rules"))? {
        let entry_path = dir.join("RULE.md");
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;
        let (rule, body) = frontmatter::parse::<Rule>(&raw)
            .with_context(|| format!("parse {}", entry_path.display()))?;
        let audience = audience_of(rule.metadata.as_ref());

        for client in ALL_CLIENTS {
            if !targets(client, audience.as_deref()) {
                continue;
            }
            let rendered = generate::render_rule(&rule, body, client)
                .with_context(|| format!("render rule {} for {:?}", name, client))?;
            let rel = rule_output_path(client, &name);
            write_output(target, &rel, &rendered)?;
            report.items.push(InstalledItem {
                kind: ItemKind::Rule,
                name: name.clone(),
                client,
                output_path: rel,
            });
        }
    }
    Ok(())
}

fn install_agents(source: &Path, target: &Path, report: &mut InstallReport) -> Result<()> {
    for (name, dir) in iter_item_dirs(&source.join("agents"))? {
        let entry_path = dir.join("AGENT.md");
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;
        let (agent, body) = frontmatter::parse::<Agent>(&raw)
            .with_context(|| format!("parse {}", entry_path.display()))?;
        let audience = audience_of(agent.metadata.as_ref());

        for client in ALL_CLIENTS {
            if !targets(client, audience.as_deref()) {
                continue;
            }
            let rendered = generate::render_agent(&agent, body, client)
                .with_context(|| format!("render agent {} for {:?}", name, client))?;
            let rel = agent_output_path(client, &name);
            write_output(target, &rel, &rendered)?;
            report.items.push(InstalledItem {
                kind: ItemKind::Agent,
                name: name.clone(),
                client,
                output_path: rel,
            });
        }
    }
    Ok(())
}

/// Per format-spec §7 / ADR-0003. opencode `.agents/skills/<n>/SKILL.md`
/// is the canonical-store path; opencode walks it natively.
fn skill_output_path(client: Client, name: &str) -> PathBuf {
    match client {
        Client::Claude => PathBuf::from(format!(".claude/skills/{name}/SKILL.md")),
        Client::Copilot => PathBuf::from(format!(".github/skills/{name}/SKILL.md")),
        Client::OpenCode => PathBuf::from(format!(".agents/skills/{name}/SKILL.md")),
    }
}

/// Per format-spec §7 / ADR-0003. Copilot uses
/// `<name>.instructions.md`; opencode uses a per-rule directory under
/// `.agents/rules/`.
fn rule_output_path(client: Client, name: &str) -> PathBuf {
    match client {
        Client::Claude => PathBuf::from(format!(".claude/rules/{name}.md")),
        Client::Copilot => PathBuf::from(format!(".github/instructions/{name}.instructions.md")),
        Client::OpenCode => PathBuf::from(format!(".agents/rules/{name}/RULE.md")),
    }
}

/// Per format-spec §7 / ADR-0003. Copilot uses `<name>.agent.md`.
fn agent_output_path(client: Client, name: &str) -> PathBuf {
    match client {
        Client::Claude => PathBuf::from(format!(".claude/agents/{name}.md")),
        Client::Copilot => PathBuf::from(format!(".github/agents/{name}.agent.md")),
        Client::OpenCode => PathBuf::from(format!(".opencode/agents/{name}.md")),
    }
}

fn write_output(target: &Path, rel: &Path, content: &str) -> Result<()> {
    let full = target.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    fs::write(&full, content).with_context(|| format!("write {}", full.display()))?;
    Ok(())
}

/// Iterate `(name, dir)` for every immediate subdirectory of `kind_root`.
/// Returns an empty iterator when the kind root does not exist (treating
/// "no items of this kind" as a non-error).
fn iter_item_dirs(kind_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    if !kind_root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in
        fs::read_dir(kind_root).with_context(|| format!("read_dir {}", kind_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .to_str()
            .map(str::to_owned)
            .with_context(|| format!("non-UTF8 name in {}", kind_root.display()))?;
        out.push((name, path));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn audience_of(metadata: Option<&crate::model::Metadata>) -> Option<Vec<Audience>> {
    metadata.and_then(|m| m.audience.clone())
}

fn targets(client: Client, audience: Option<&[Audience]>) -> bool {
    match audience {
        None => true,
        Some(list) => list.iter().any(|a| audience_matches(client, *a)),
    }
}

fn audience_matches(client: Client, a: Audience) -> bool {
    matches!(
        (client, a),
        (Client::Claude, Audience::Claude)
            | (Client::Copilot, Audience::Copilot)
            | (Client::OpenCode, Audience::OpenCode)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_paths_match_spec() {
        assert_eq!(
            skill_output_path(Client::Claude, "x"),
            PathBuf::from(".claude/skills/x/SKILL.md")
        );
        assert_eq!(
            skill_output_path(Client::OpenCode, "x"),
            PathBuf::from(".agents/skills/x/SKILL.md")
        );
        assert_eq!(
            rule_output_path(Client::Copilot, "x"),
            PathBuf::from(".github/instructions/x.instructions.md")
        );
        assert_eq!(
            rule_output_path(Client::OpenCode, "x"),
            PathBuf::from(".agents/rules/x/RULE.md")
        );
        assert_eq!(
            agent_output_path(Client::Copilot, "x"),
            PathBuf::from(".github/agents/x.agent.md")
        );
        assert_eq!(
            agent_output_path(Client::OpenCode, "x"),
            PathBuf::from(".opencode/agents/x.md")
        );
    }

    #[test]
    fn audience_none_targets_all_clients() {
        for c in ALL_CLIENTS {
            assert!(targets(c, None));
        }
    }

    #[test]
    fn audience_subset_filters_other_clients() {
        let only_claude = vec![Audience::Claude];
        assert!(targets(Client::Claude, Some(&only_claude)));
        assert!(!targets(Client::Copilot, Some(&only_claude)));
        assert!(!targets(Client::OpenCode, Some(&only_claude)));
    }
}
