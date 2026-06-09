//! Git-backed source resolution: clone URL construction, token
//! injection, shallow-clone-to-tempdir, and the dispatch from
//! [`InstallSource`] variants to the local install pipeline.
//!
//! Authentication: when a token is resolved via [`crate::auth`], it is
//! URL-injected into the clone URL as `https://<user>:<token>@<host>/...`.
//! With no token, the clone falls back to git's own credential helpers
//! (keychain, manager, etc.).

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

use super::{InstallReport, install_from_local_path};
use crate::fetch;
use crate::source::{GithubRepo, GitlabRepo, InstallSource};

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
pub fn install_from_source(
    source: &InstallSource,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    match source {
        InstallSource::LocalPath(path) => install_from_local_path(path, target, filter),
        InstallSource::Github(repo) => install_from_github(repo, target, filter),
        InstallSource::Gitlab(repo) => install_from_gitlab(repo, target, filter),
    }
}

fn install_from_github(
    repo: &GithubRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    install_from_git_url(
        &github_authenticated_url(repo)?,
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
        filter,
    )
}

fn install_from_gitlab(
    repo: &GitlabRepo,
    target: &Path,
    filter: Option<&crate::bundle::ResolvedItems>,
) -> Result<InstallReport> {
    install_from_git_url(
        &gitlab_authenticated_url(repo)?,
        repo.git_ref.as_deref(),
        repo.subfolder.as_deref(),
        &repo.owner,
        &repo.name,
        target,
        filter,
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
    fetch::shallow_clone(url, git_ref, "clone", tmp.path(), subfolder)
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
    fn gitlab_clone_url_with_subgroups() {
        // Subgroup namespaces carry slashes in `owner`; the clone URL is the
        // full path joined to the project name (GitLab clones the whole path).
        let repo = GitlabRepo {
            host: "gitlabee.dt.renault.com".into(),
            owner: "partners/alliance-car/devex/process".into(),
            name: "seed".into(),
            git_ref: None,
            subfolder: None,
        };
        assert_eq!(
            gitlab_clone_url(&repo),
            "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed.git"
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
}
