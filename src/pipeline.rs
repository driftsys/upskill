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
//! Audience filter: prefers the top-level `audience` field (per
//! format-spec §3.1) and falls back to `metadata.audience` when the
//! top-level is absent — accepts both shapes for back-compat.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

use crate::fetch;
use crate::generate::{self, Client};
use crate::model::{Agent, Audience, Rule, Skill};
use crate::parse::frontmatter;
use crate::source::{GithubRepo, GitlabRepo, InstallSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    /// SHA-256 of the SSOT item directory at install time. Used by the
    /// lockfile (#schema-2) for drift detection. Repeated across the per-
    /// client entries for the same item — they share one SSOT input.
    pub source_hash: Option<String>,
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

/// Install + write lockfile. Consumer-facing entry point.
///
/// Calls [`install_from_source`] then merges the resulting [`InstallReport`]
/// into `<target>/.upskill-lock.json` via [`crate::lockfile_v2`]. Existing
/// lockfile entries for the same `(kind, name)` are replaced; entries
/// installed from a different source are left in place.
///
/// `git_ref` recorded per item is taken from the source variant when one
/// is pinned (Github/Gitlab `git_ref`); local-path sources record `None`.
/// `source` label is the [`InstallSource`] `Display` form.
pub fn install_with_lockfile(source: &InstallSource, target: &Path) -> Result<InstallReport> {
    let report = install_from_source(source, target)?;

    let label = source.to_string();
    let git_ref = match source {
        InstallSource::Github(r) => r.git_ref.as_deref(),
        InstallSource::Gitlab(r) => r.git_ref.as_deref(),
        InstallSource::LocalPath(_) => None,
    };
    let hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> = report
        .items
        .iter()
        .map(|it| ((it.kind, it.name.clone()), it.source_hash.clone()))
        .collect();
    let new_items = crate::lockfile_v2::items_from_report(&report, &label, git_ref, |k, n| {
        hashes.get(&(k, n.to_string())).cloned().flatten()
    });

    let mut lock = crate::lockfile_v2::LockfileV2::load(target)?;
    for item in new_items {
        lock.upsert(item);
    }
    lock.save(target)?;

    // Per ADR-0003 / format-spec §7.4: ensure the Claude Code bridge file
    // exists at the consumer-project root. Created once with `@AGENTS.md`
    // content, never overwritten — protects user customisations.
    crate::ancillary::ensure_claude_bridge(target)?;

    // Per ADR-0003 / format-spec §7.4: when the install includes any rule,
    // register the opencode.json `instructions[]` glob so opencode picks up
    // generated rules under `.agents/rules/`. Idempotent; preserves other
    // keys.
    let has_rules = report.items.iter().any(|i| i.kind == ItemKind::Rule);
    crate::ancillary::ensure_opencode_rules_registered(target, has_rules)?;

    Ok(report)
}

/// Install items from any supported source into `target`.
///
/// Dispatches on the source variant. All git-backed variants funnel
/// through [`install_from_git_url`]; the only difference is URL
/// construction.
///
/// - `LocalPath` — installs directly from the path on disk.
/// - `Github` — `https://github.com/<owner>/<repo>.git`.
/// - `Gitlab` — `https://<host>/<owner>/<repo>.git`. Self-hosted GitLab
///   works through the `host` field on `GitlabRepo`.
///
/// Authentication: clone is plain HTTPS — git resolves credentials via
/// its own helpers (`gh`/`glab`/keychain). Token-based URL injection
/// (per ADR-0005 / format-spec §5.3) is a separate slice.
pub fn install_from_source(source: &InstallSource, target: &Path) -> Result<InstallReport> {
    match source {
        InstallSource::LocalPath(path) => install_from_local_path(path, target),
        InstallSource::Github(repo) => install_from_github(repo, target),
        InstallSource::Gitlab(repo) => install_from_gitlab(repo, target),
    }
}

fn install_from_github(repo: &GithubRepo, target: &Path) -> Result<InstallReport> {
    install_from_git_url(
        &github_clone_url(repo),
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
    )
}

fn install_from_gitlab(repo: &GitlabRepo, target: &Path) -> Result<InstallReport> {
    install_from_git_url(
        &gitlab_clone_url(repo),
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
    )
}

fn github_clone_url(repo: &GithubRepo) -> String {
    format!("https://github.com/{}/{}.git", repo.owner, repo.name)
}

fn gitlab_clone_url(repo: &GitlabRepo) -> String {
    format!("https://{}/{}/{}.git", repo.host, repo.owner, repo.name)
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
        let audience = audience_of(skill.audience.as_ref(), skill.metadata.as_ref());
        let source_hash = crate::lockfile::hash_skill_dir(&dir);

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
                source_hash: source_hash.clone(),
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
        let audience = audience_of(rule.audience.as_ref(), rule.metadata.as_ref());
        let source_hash = crate::lockfile::hash_skill_dir(&dir);

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
                source_hash: source_hash.clone(),
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
        let audience = audience_of(agent.audience.as_ref(), agent.metadata.as_ref());
        let source_hash = crate::lockfile::hash_skill_dir(&dir);

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
                source_hash: source_hash.clone(),
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

/// Resolve the effective audience for an item. Top-level (§3.1) wins; falls
/// back to `metadata.audience` for back-compat with v0.1-draft fixtures.
fn audience_of(
    top_level: Option<&Vec<Audience>>,
    metadata: Option<&crate::model::Metadata>,
) -> Option<Vec<Audience>> {
    if let Some(list) = top_level {
        return Some(list.clone());
    }
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

    #[test]
    fn github_clone_url_is_https_dot_git() {
        let repo = GithubRepo {
            owner: "driftsys".into(),
            name: "skills".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            github_clone_url(&repo),
            "https://github.com/driftsys/skills.git"
        );
    }

    #[test]
    fn gitlab_clone_url_uses_repo_host() {
        // gitlab.com
        let repo = GitlabRepo {
            host: "gitlab.com".into(),
            owner: "driftsys".into(),
            name: "skills".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            gitlab_clone_url(&repo),
            "https://gitlab.com/driftsys/skills.git"
        );

        // self-hosted GitLab
        let self_hosted = GitlabRepo {
            host: "gitlab.example.com".into(),
            owner: "team".into(),
            name: "rules".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            gitlab_clone_url(&self_hosted),
            "https://gitlab.example.com/team/rules.git"
        );
    }
}
