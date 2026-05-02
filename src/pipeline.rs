//! v0.2 install pipeline: SSOT (local path or git source) → per-client
//! output on disk.
//!
//! Walks a source directory laid out per format-spec §2.1, parses each
//! item's frontmatter into the model (§3), renders per-client output via
//! `crate::generate`, and writes the result to the per-client paths
//! defined in format-spec §7 / ADR-0003.
//!
//! Entry points:
//! - [`install_with_lockfile`] — consumer-facing; install + record state +
//!   create ancillary files. Backs `upskill add`.
//! - [`install_from_source`] / [`install_from_local_path`] /
//!   [`install_from_git_url`] — library-only variants without lockfile or
//!   ancillary handling.
//!
//! Authentication: when a token is resolved via [`crate::auth`], it is
//! URL-injected into the clone URL. With no token, clones fall back to
//! git's own credential helpers.
//!
//! Audience filter: prefers the top-level `audience` field (per
//! format-spec §3.1) and falls back to `metadata.audience` when the
//! top-level is absent — accepts both shapes for back-compat.

use anyhow::{Context, Result, anyhow};
use sha2::{Digest, Sha256};
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

    // Per ADR-0003: when the install includes any rule, register
    // `.github/instructions` in `.vscode/settings.json`'s
    // `chat.instructionsFilesLocations` so VS Code Copilot picks up the
    // generated `<name>.instructions.md` files.
    crate::ancillary::ensure_vscode_instructions_registered(target, has_rules)?;

    Ok(report)
}

/// What to remove. Per ADR-0004 the user must be explicit — bare
/// `upskill remove` is not allowed; either name items or pass
/// `--source <label>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveFilter {
    /// Remove every lockfile entry whose `name` matches one of these
    /// values, regardless of kind. An item name listed here that does not
    /// match any entry is an error (the caller asked to remove a thing
    /// that is not installed).
    ByNames(Vec<String>),
    /// Remove every lockfile entry whose `source` label matches this
    /// string verbatim. No-op when the lockfile contains no entry from
    /// the named source.
    BySource(String),
}

#[derive(Debug, Default, Clone)]
pub struct RemoveReport {
    pub items: Vec<RemovedItem>,
}

#[derive(Debug, Clone)]
pub struct RemovedItem {
    pub kind: ItemKind,
    pub name: String,
    /// Files actually deleted from disk (paths relative to `target`).
    /// May be empty if the lockfile knew about the item but its outputs
    /// were already gone.
    pub deleted_files: Vec<PathBuf>,
}

/// Remove installed content recorded in `<target>/.upskill-lock.json`,
/// matching `filter`. For each matching entry, deletes every per-client
/// output file (per [`output_path`]) and drops the entry from the
/// lockfile. Best-effort `rmdir` of empty per-item parent directories
/// (e.g., `.claude/skills/<name>/`) so the workspace stays clean.
///
/// Ancillary files (`CLAUDE.md`, `opencode.json`,
/// `.vscode/settings.json`) are deliberately left alone — they are
/// user-owned after creation per ADR-0003.
pub fn remove(target: &Path, filter: RemoveFilter) -> Result<RemoveReport> {
    let mut lock = crate::lockfile_v2::LockfileV2::load(target)?;

    let to_remove: Vec<crate::lockfile_v2::LockedItem> = match &filter {
        RemoveFilter::ByNames(names) => lock
            .items
            .iter()
            .filter(|i| names.iter().any(|n| n == &i.name))
            .cloned()
            .collect(),
        RemoveFilter::BySource(source) => lock
            .items
            .iter()
            .filter(|i| &i.source == source)
            .cloned()
            .collect(),
    };

    if let RemoveFilter::ByNames(names) = &filter {
        let matched: std::collections::BTreeSet<&str> =
            to_remove.iter().map(|i| i.name.as_str()).collect();
        let unknown: Vec<&str> = names
            .iter()
            .filter(|n| !matched.contains(n.as_str()))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            anyhow::bail!("not in lockfile: {}", unknown.join(", "));
        }
    }

    let mut report = RemoveReport::default();
    for entry in &to_remove {
        let kind = parse_kind(&entry.kind)
            .with_context(|| format!("lockfile entry {}: unknown kind", entry.name))?;
        let mut deleted_files = Vec::new();
        for client in ALL_CLIENTS {
            let rel = output_path(kind, client, &entry.name);
            let full = target.join(&rel);
            if full.exists() {
                fs::remove_file(&full).with_context(|| format!("delete {}", full.display()))?;
                deleted_files.push(rel);
                if let Some(parent) = full.parent() {
                    let _ = fs::remove_dir(parent);
                }
            }
        }
        lock.remove(&entry.kind, &entry.name);
        report.items.push(RemovedItem {
            kind,
            name: entry.name.clone(),
            deleted_files,
        });
    }

    lock.save(target)?;
    Ok(report)
}

fn parse_kind(s: &str) -> Result<ItemKind> {
    match s {
        "skill" => Ok(ItemKind::Skill),
        "rule" => Ok(ItemKind::Rule),
        "agent" => Ok(ItemKind::Agent),
        other => anyhow::bail!("unknown kind `{other}`"),
    }
}

/// Whether `update` writes or just reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    Apply,
    DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// SSOT hash matches the lockfile — nothing to do.
    UpToDate,
    /// `Apply` mode: the lockfile hash changed (or was previously unset
    /// and now resolved). Outputs were rewritten.
    Updated {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
    /// `DryRun` mode: SSOT hash differs from the lockfile entry; an
    /// `update` (without `--dry-run`) would rewrite outputs.
    WouldChange {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct UpdatedItem {
    pub kind: ItemKind,
    pub name: String,
    pub source: String,
    pub status: UpdateStatus,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateReport {
    pub items: Vec<UpdatedItem>,
}

/// Re-fetch every source recorded in `<target>/.upskill-lock.json` and
/// either reinstall (`Apply`) or report what would change (`DryRun`).
///
/// `names` selects which lockfile entries to update; empty means "all".
/// When names are given, the entries' source labels are used to fetch
/// — so `update foo` may also re-render other items installed from the
/// same source (the pipeline always reinstalls a source as a unit).
/// Names that match no lockfile entry are an error.
///
/// `update` always fetches per ADR-0004 — there is no `--offline`.
pub fn update(target: &Path, names: &[String], mode: UpdateMode) -> Result<UpdateReport> {
    let lock = crate::lockfile_v2::LockfileV2::load(target)?;

    let entries: Vec<crate::lockfile_v2::LockedItem> = if names.is_empty() {
        lock.items.clone()
    } else {
        let matched: Vec<crate::lockfile_v2::LockedItem> = lock
            .items
            .iter()
            .filter(|i| names.iter().any(|n| n == &i.name))
            .cloned()
            .collect();
        let matched_names: std::collections::BTreeSet<&str> =
            matched.iter().map(|i| i.name.as_str()).collect();
        let unknown: Vec<&str> = names
            .iter()
            .filter(|n| !matched_names.contains(n.as_str()))
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            anyhow::bail!("not in lockfile: {}", unknown.join(", "));
        }
        matched
    };

    // Group by source: installing or hashing once per source covers every
    // matching entry sharing it.
    let mut by_source: std::collections::BTreeMap<String, Vec<crate::lockfile_v2::LockedItem>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        by_source
            .entry(entry.source.clone())
            .or_default()
            .push(entry);
    }

    let mut report = UpdateReport::default();
    for (source_label, source_entries) in by_source {
        let source = crate::source::parse_install_source_label(&source_label)
            .with_context(|| format!("parse lockfile source label `{source_label}`"))?;

        match mode {
            UpdateMode::Apply => {
                let install_report = install_with_lockfile(&source, target)?;
                let mut new_hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> =
                    std::collections::BTreeMap::new();
                for it in &install_report.items {
                    new_hashes.insert((it.kind, it.name.clone()), it.source_hash.clone());
                }
                for entry in &source_entries {
                    let kind = parse_kind(&entry.kind)?;
                    let new_hash = new_hashes
                        .get(&(kind, entry.name.clone()))
                        .cloned()
                        .flatten();
                    let status = if new_hash == entry.hash {
                        UpdateStatus::UpToDate
                    } else {
                        UpdateStatus::Updated {
                            old_hash: entry.hash.clone(),
                            new_hash,
                        }
                    };
                    report.items.push(UpdatedItem {
                        kind,
                        name: entry.name.clone(),
                        source: source_label.clone(),
                        status,
                    });
                }
            }
            UpdateMode::DryRun => {
                let (root, _guard) = fetch_ssot(&source)?;
                let new_hashes = hash_source_items(&root);
                for entry in &source_entries {
                    let kind = parse_kind(&entry.kind)?;
                    let new_hash = new_hashes
                        .get(&(kind, entry.name.clone()))
                        .cloned()
                        .flatten();
                    let status = if new_hash == entry.hash {
                        UpdateStatus::UpToDate
                    } else {
                        UpdateStatus::WouldChange {
                            old_hash: entry.hash.clone(),
                            new_hash,
                        }
                    };
                    report.items.push(UpdatedItem {
                        kind,
                        name: entry.name.clone(),
                        source: source_label.clone(),
                        status,
                    });
                }
            }
        }
    }

    Ok(report)
}

/// Hash every item directory under a SSOT root, keyed by `(kind, name)`.
/// Used by `update --dry-run` to compute would-be hashes without
/// installing. Mirrors the per-kind walk of `install_from_local_path`.
fn hash_source_items(
    source_root: &Path,
) -> std::collections::BTreeMap<(ItemKind, String), Option<String>> {
    let mut out = std::collections::BTreeMap::new();
    for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
        let kind_dir = source_root.join(match kind {
            ItemKind::Skill => "skills",
            ItemKind::Rule => "rules",
            ItemKind::Agent => "agents",
        });
        let Ok(entries) = fs::read_dir(&kind_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                out.insert((kind, name.to_string()), hash_item_dir(&path));
            }
        }
    }
    out
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
/// Authentication: when a token is resolved via [`crate::auth`]
/// (`GITHUB_TOKEN` / `GH_TOKEN` / `gh auth token` for GitHub;
/// `GITLAB_TOKEN` / `GL_TOKEN` / `glab auth token` for GitLab), it is
/// URL-encoded and injected into the clone URL as
/// `https://<user>:<token>@<host>/...`. With no token, the clone falls
/// back to git's own credential helpers (keychain, manager, etc.) so the
/// previous behaviour is unchanged for users who rely on those.
pub fn install_from_source(source: &InstallSource, target: &Path) -> Result<InstallReport> {
    match source {
        InstallSource::LocalPath(path) => install_from_local_path(path, target),
        InstallSource::Github(repo) => install_from_github(repo, target),
        InstallSource::Gitlab(repo) => install_from_gitlab(repo, target),
    }
}

fn install_from_github(repo: &GithubRepo, target: &Path) -> Result<InstallReport> {
    install_from_git_url(
        &github_authenticated_url(repo)?,
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
    )
}

fn install_from_gitlab(repo: &GitlabRepo, target: &Path) -> Result<InstallReport> {
    install_from_git_url(
        &gitlab_authenticated_url(repo)?,
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

fn github_authenticated_url(repo: &GithubRepo) -> Result<String> {
    Ok(match crate::auth::resolve_github_token().token() {
        Some(token) => inject_basic_auth(&github_clone_url(repo), "x-access-token", token)?,
        None => github_clone_url(repo),
    })
}

fn gitlab_authenticated_url(repo: &GitlabRepo) -> Result<String> {
    Ok(match crate::auth::resolve_gitlab_token().token() {
        Some(token) => inject_basic_auth(&gitlab_clone_url(repo), "oauth2", token)?,
        None => gitlab_clone_url(repo),
    })
}

/// Resolve the SSOT root for a source, fetching when remote.
///
/// Returns `(path, guard)`:
/// - `path` is the on-disk SSOT root that callers walk for `skills/`,
///   `rules/`, `agents/` subdirectories.
/// - `guard` is `Some(TempDir)` for git sources (drop cleans up the
///   clone) and `None` for local-path sources.
///
/// Used by `update` (especially `--dry-run`) where we want the SSOT on
/// disk without committing to an install. For `install_*` the same
/// fetch happens internally inside `install_from_git_url` — we keep
/// that path independent so install can stay one tempdir-scoped pass.
pub fn fetch_ssot(source: &InstallSource) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    match source {
        InstallSource::LocalPath(p) => Ok((p.clone(), None)),
        InstallSource::Github(repo) => clone_to_tempdir(
            &github_authenticated_url(repo)?,
            repo.git_ref.as_deref(),
            repo.subfolder.as_deref(),
            &repo.owner,
            &repo.name,
        ),
        InstallSource::Gitlab(repo) => clone_to_tempdir(
            &gitlab_authenticated_url(repo)?,
            repo.git_ref.as_deref(),
            repo.subfolder.as_deref(),
            &repo.owner,
            &repo.name,
        ),
    }
}

fn clone_to_tempdir(
    url: &str,
    git_ref: Option<&str>,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    let tmp = tempfile::tempdir().context("create temp dir for clone")?;
    fetch::shallow_clone(url, git_ref, "clone", tmp.path())
        .map_err(|e| anyhow!("git clone {}: {}", url, e))?;
    let source = fetch::resolve_subfolder(&tmp.path().join("clone"), subfolder, owner, name)
        .map_err(|e| anyhow!("{}", e))?;
    Ok((source, Some(tmp)))
}

/// Inject HTTP Basic credentials into an `https://` URL so `git clone`
/// can authenticate without depending on a credential helper. The token
/// is percent-encoded against the RFC 3986 unreserved set; the user
/// segment is encoded the same way (over-aggressive but safe — typical
/// values are `oauth2` / `x-access-token`, both unreserved-only).
///
/// Returns an error if `url` does not start with `https://` or if `token`
/// is empty (callers should not invoke with an empty token).
fn inject_basic_auth(url: &str, user: &str, token: &str) -> Result<String> {
    if token.is_empty() {
        anyhow::bail!("refusing to inject empty token into URL");
    }
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| anyhow!("expected https:// URL for token injection, got: {url}"))?;
    Ok(format!(
        "https://{}:{}@{}",
        percent_encode_userinfo(user),
        percent_encode_userinfo(token),
        rest
    ))
}

/// Percent-encode `s` keeping only RFC 3986 unreserved characters
/// (`A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~`). Used for the userinfo
/// segment of an HTTPS clone URL — over-aggressive but always safe; the
/// character set covers every realistic token format
/// (`ghp_...`, `glpat-...`, etc.) without escaping.
fn percent_encode_userinfo(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
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
        let source_hash = hash_item_dir(&dir);

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
        let source_hash = hash_item_dir(&dir);

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
        let source_hash = hash_item_dir(&dir);

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

/// Where the install pipeline writes — and `remove` deletes — the per-
/// client output for a given `(kind, name)`. Path is relative to the
/// install target root and matches format-spec §7. Used by both
/// `install_*` and [`remove`] so the two stay in lockstep.
pub(crate) fn output_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    match kind {
        ItemKind::Skill => skill_output_path(client, name),
        ItemKind::Rule => rule_output_path(client, name),
        ItemKind::Agent => agent_output_path(client, name),
    }
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

/// SHA-256 hash of every file under `dir`, with each file's path-relative
/// name folded into the hash so renames register as drift. Recursive,
/// deterministic (sorted file list), and `None` when `dir` is empty or
/// unreadable. Used by the pipeline to populate `LockedItem.hash` and by
/// `doctor` (Phase B3) to detect SSOT drift.
pub(crate) fn hash_item_dir(dir: &Path) -> Option<String> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    if files.is_empty() {
        return None;
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in &files {
        let relative = file.strip_prefix(dir).unwrap_or(file);
        hasher.update(relative.to_string_lossy().as_bytes());
        if let Ok(content) = fs::read(file) {
            hasher.update(&content);
        }
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
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

    #[test]
    fn inject_basic_auth_github_oauth_user() {
        // Mirrors the call install_from_github makes when GITHUB_TOKEN is set.
        let url = inject_basic_auth(
            "https://github.com/driftsys/skills.git",
            "x-access-token",
            "ghp_AbCdEf1234567890",
        )
        .expect("inject");
        assert_eq!(
            url,
            "https://x-access-token:ghp_AbCdEf1234567890@github.com/driftsys/skills.git"
        );
    }

    #[test]
    fn inject_basic_auth_gitlab_oauth_user() {
        // Mirrors the call install_from_gitlab makes when GITLAB_TOKEN is set,
        // and exercises self-hosted GitLab (per the plan: gitlab.example.com).
        let url = inject_basic_auth(
            "https://gitlab.example.com/team/rules.git",
            "oauth2",
            "glpat-XYZ_abc-123",
        )
        .expect("inject");
        assert_eq!(
            url,
            "https://oauth2:glpat-XYZ_abc-123@gitlab.example.com/team/rules.git"
        );
    }

    #[test]
    fn inject_basic_auth_percent_encodes_special_chars() {
        // Tokens containing `:`, `@`, `/`, `%` would otherwise corrupt the URL
        // parse on the git side. Verify they're percent-encoded.
        let url = inject_basic_auth(
            "https://gitlab.com/o/r.git",
            "oauth2",
            "tok:en@with/special%chars",
        )
        .expect("inject");
        assert_eq!(
            url,
            "https://oauth2:tok%3Aen%40with%2Fspecial%25chars@gitlab.com/o/r.git"
        );
    }

    #[test]
    fn inject_basic_auth_rejects_empty_token() {
        let err = inject_basic_auth("https://github.com/o/r.git", "x-access-token", "")
            .expect_err("must reject");
        assert!(err.to_string().contains("empty token"));
    }

    #[test]
    fn inject_basic_auth_rejects_non_https() {
        let err = inject_basic_auth("http://github.com/o/r.git", "x-access-token", "tok")
            .expect_err("must reject");
        assert!(err.to_string().contains("https://"));

        let err = inject_basic_auth("git@github.com:o/r.git", "x-access-token", "tok")
            .expect_err("must reject ssh form");
        assert!(err.to_string().contains("https://"));
    }

    #[test]
    fn percent_encode_userinfo_passes_unreserved_unchanged() {
        // RFC 3986 unreserved set: A-Z a-z 0-9 - _ . ~
        assert_eq!(
            percent_encode_userinfo("Abc-_.~123"),
            "Abc-_.~123",
            "unreserved chars unchanged"
        );
    }

    #[test]
    fn percent_encode_userinfo_escapes_userinfo_separators() {
        // The chars that would actually break a URL parse if unescaped.
        assert_eq!(percent_encode_userinfo(":"), "%3A");
        assert_eq!(percent_encode_userinfo("@"), "%40");
        assert_eq!(percent_encode_userinfo("/"), "%2F");
        assert_eq!(percent_encode_userinfo("%"), "%25");
    }

    #[test]
    fn parse_kind_round_trips_lockfile_strings() {
        // The labels written by `lockfile_v2::items_from_report` MUST round-
        // trip through `parse_kind` so `remove` can dispatch on them.
        for k in [ItemKind::Rule, ItemKind::Skill, ItemKind::Agent] {
            let label = match k {
                ItemKind::Rule => "rule",
                ItemKind::Skill => "skill",
                ItemKind::Agent => "agent",
            };
            assert_eq!(parse_kind(label).unwrap(), k);
        }
    }

    #[test]
    fn parse_kind_rejects_unknown_string() {
        let err = parse_kind("bundle").expect_err("must reject");
        assert!(err.to_string().contains("bundle"));
    }

    #[test]
    fn output_path_dispatches_to_per_kind_helper() {
        // The dispatcher must produce the same path as the per-kind
        // function for the same `(kind, client, name)` tuple, otherwise
        // `install` would write to one place and `remove` would look in
        // another.
        for client in ALL_CLIENTS {
            assert_eq!(
                output_path(ItemKind::Skill, client, "x"),
                skill_output_path(client, "x")
            );
            assert_eq!(
                output_path(ItemKind::Rule, client, "x"),
                rule_output_path(client, "x")
            );
            assert_eq!(
                output_path(ItemKind::Agent, client, "x"),
                agent_output_path(client, "x")
            );
        }
    }
}
