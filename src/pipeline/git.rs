//! Git-backed source resolution: clone URL construction,
//! shallow-clone-to-tempdir, and the dispatch from [`InstallSource`]
//! variants to the local install pipeline.
//!
//! Authentication: upskill never injects credentials into clone URLs.
//! Clones use the bare `https://<host>/...` URL and rely entirely on
//! git's own configuration — credential helpers (keychain, manager),
//! `url.<base>.insteadOf` rewrites, and SSH. CI must supply credentials
//! through those same git mechanisms rather than an env-var token.

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

use super::{InstallReport, install_from_local_path};
use crate::fetch;
use crate::source::{GitRepo, GithubRepo, InstallSource};

/// Install items from any supported source into `target`.
///
/// Dispatches on the source variant. All git-backed variants funnel
/// through [`install_from_git_url`]; the only difference is URL
/// construction.
///
/// - `LocalPath` — installs directly from the path on disk.
/// - `Github` — `https://github.com/<owner>/<repo>.git`.
/// - `Git` — `<url>.git`; the URL is stored verbatim on [`GitRepo`], so
///   any https git host (GitLab incl. self-hosted/subgroups, Bitbucket,
///   Gitea, …) works identically.
///
/// Authentication: clones use the bare URL and rely on git's own
/// configuration (credential helpers, `insteadOf` rewrites, SSH).
/// upskill does not resolve or inject tokens.
pub fn install_from_source(
    source: &InstallSource,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    match source {
        InstallSource::LocalPath(path) => install_from_local_path(path, target, filter),
        InstallSource::Github(repo) => install_from_github(repo, target, filter),
        InstallSource::Git(repo) => install_from_git(repo, target, filter),
    }
}

fn install_from_github(
    repo: &GithubRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    install_from_git_url(
        &github_clone_url(repo),
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
        filter,
    )
}

fn install_from_git(
    repo: &GitRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    let (owner, name) = repo_display_parts(&repo.url);
    install_from_git_url(
        &git_clone_url(repo),
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        owner,
        name,
        target,
        filter,
    )
}

fn github_clone_url(repo: &GithubRepo) -> String {
    format!("https://github.com/{}/{}.git", repo.owner, repo.name)
}

fn git_clone_url(repo: &GitRepo) -> String {
    format!("{}.git", repo.url)
}

/// Split a bare remote URL into (prefix, last segment) for the
/// human-facing error messages in `fetch::resolve_subfolder`, which
/// formats them back as `{owner}/{name}` — reproducing the full URL.
fn repo_display_parts(url: &str) -> (&str, &str) {
    url.rsplit_once('/').unwrap_or(("", url))
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
            &github_clone_url(repo),
            repo.git_ref.as_deref(),
            repo.subfolder.as_deref(),
            &repo.owner,
            &repo.name,
        ),
        InstallSource::Git(repo) => {
            let (owner, name) = repo_display_parts(&repo.url);
            clone_to_tempdir(
                &git_clone_url(repo),
                repo.git_ref.as_deref(),
                repo.subfolder.as_deref(),
                owner,
                name,
            )
        }
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
    fetch::shallow_clone(url, git_ref, "clone", tmp.path(), subfolder)
        .map_err(|e| anyhow!("git clone {}: {}", url, e))?;
    let source = fetch::resolve_subfolder(&tmp.path().join("clone"), subfolder, owner, name)
        .map_err(|e| anyhow!("{}", e))?;
    Ok((source, Some(tmp)))
}

/// Shallow-clone `url` into a tempdir, resolve `subfolder` inside the clone,
/// and run the local install pipeline against the result. The tempdir is
/// removed on return regardless of outcome (RAII via `tempfile::TempDir`).
///
/// Public so callers can install from arbitrary git URLs (mirrors, local
/// `file://` clones) without going through
/// [`InstallSource`]. The high-level [`install_from_source`] is preferred
/// when an `InstallSource` already exists.
pub fn install_from_git_url(
    url: &str,
    git_ref: Option<&str>,
    subfolder: Option<&str>,
    owner: &str,
    name: &str,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    let tmp = tempfile::tempdir().context("create temp dir for clone")?;
    fetch::shallow_clone(url, git_ref, "clone", tmp.path(), subfolder)
        .map_err(|e| anyhow!("git clone {}: {}", url, e))?;
    let source = fetch::resolve_subfolder(&tmp.path().join("clone"), subfolder, owner, name)
        .map_err(|e| anyhow!("{}", e))?;
    install_from_local_path(&source, target, filter)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn git_clone_url_appends_dot_git() {
        let repo = GitRepo {
            url: "https://gitlab.example.com/team/rules".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            git_clone_url(&repo),
            "https://gitlab.example.com/team/rules.git"
        );
    }

    #[test]
    fn git_clone_url_preserves_deep_paths_and_ports() {
        let repo = GitRepo {
            url: "https://git.company.com:8443/partners/devex/process/seed".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            git_clone_url(&repo),
            "https://git.company.com:8443/partners/devex/process/seed.git"
        );
    }

    #[test]
    fn repo_display_parts_splits_last_segment() {
        assert_eq!(
            repo_display_parts("https://gitlab.com/team/skills"),
            ("https://gitlab.com/team", "skills")
        );
    }
}
