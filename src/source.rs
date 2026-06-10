use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRepo {
    pub owner: String,
    pub name: String,
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepo {
    /// Bare https remote without a trailing `.git`, e.g.
    /// `https://gitlab.example.com/group/sub/repo`. The host and path are
    /// opaque — any git host works; auth is git's job (config/helpers).
    pub url: String,
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    Github(GithubRepo),
    Git(GitRepo),
    LocalPath(PathBuf),
}

/// Stable, round-trippable string label for use in lockfiles, log lines,
/// and CLI output. Format mirrors what `parse_install_source` accepts:
///
/// - `github:<owner>/<name>[@<ref>][:<subfolder>]`
/// - `<url>[@<ref>][:<subfolder>]` — generic git source; the bare https
///   URL is its own label
/// - `local:<path>` (absolute when known, otherwise as-given)
impl fmt::Display for InstallSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallSource::Github(r) => write!(f, "github:{}/{}", r.owner, r.name)
                .and_then(|_| match &r.git_ref {
                    Some(g) => write!(f, "@{}", g),
                    None => Ok(()),
                })
                .and_then(|_| match &r.subfolder {
                    Some(s) => write!(f, ":{}", s),
                    None => Ok(()),
                }),
            InstallSource::Git(r) => {
                write!(f, "{}", r.url)?;
                if let Some(g) = &r.git_ref {
                    write!(f, "@{}", g)?;
                }
                if let Some(s) = &r.subfolder {
                    write!(f, ":{}", s)?;
                }
                Ok(())
            }
            InstallSource::LocalPath(p) => write!(f, "local:{}", p.display()),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SourceParseError {
    #[error("source must be in owner/repo format")]
    InvalidFormat,
    #[error("owner and repo must be non-empty")]
    EmptySegment,
    #[error("subfolder path must be non-empty")]
    EmptySubfolder,
    #[error("ref must be non-empty")]
    EmptyRef,
    #[error(
        "the `gitlab:` shorthand was removed; use the full https URL instead, \
         e.g. https://gitlab.com/owner/repo"
    )]
    GitlabShorthandRemoved,
}

/// Resolve the running user's home directory.
///
/// Checks `HOME` first (standard on Unix, Git Bash, and WSL), then falls
/// back to `USERPROFILE` (the canonical home-directory variable on native
/// Windows). Returns `None` when neither variable is set.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn parse_install_source(source: &str) -> Result<InstallSource, SourceParseError> {
    // Relative local path: make it absolute (lexically) so two spellings of the
    // same directory (`./reg` and `/abs/reg`) collapse to one canonical
    // `local:` label and resolve identically as a dependency source (issue
    // #212).
    if source.starts_with("./") || source.starts_with("../") {
        return Ok(InstallSource::LocalPath(absolutize_local(source)));
    }
    if source.starts_with('/') || is_windows_absolute(source) {
        return Ok(InstallSource::LocalPath(PathBuf::from(source)));
    }

    // `~/path` home expansion (`~` alone or `~/foo`). We don't support `~user`
    // — that needs platform-specific lookup and our use case is the running
    // user's own home only.
    if source == "~" || source.starts_with("~/") {
        if let Some(mut path) = home_dir() {
            if let Some(rest) = source.strip_prefix("~/") {
                path.push(rest);
            }
            return Ok(InstallSource::LocalPath(path));
        }
        // Neither HOME nor USERPROFILE set: fall through and let the path be
        // parsed verbatim. Downstream `LocalPath` handling will fail with a
        // useful filesystem error rather than a confusing parse error.
        return Ok(InstallSource::LocalPath(PathBuf::from(source)));
    }

    // The `gitlab:` / `gitlab+<host>:` shorthands were removed in favor of
    // full https URLs. Catch them explicitly so the error names the
    // replacement instead of falling through to a confusing
    // owner/repo parse error.
    if source.starts_with("gitlab:") || source.starts_with("gitlab+") {
        return Err(SourceParseError::GitlabShorthandRemoved);
    }

    // HTTPS URLs
    if let Some(rest) = source.strip_prefix("https://") {
        return parse_url_source(rest);
    }

    parse_github_source(source).map(InstallSource::Github)
}

/// Make a relative local path absolute lexically — joining the current
/// directory without touching the filesystem or resolving symlinks — so that
/// the same directory always yields one canonical `local:` label regardless of
/// how it was spelled (issue #212). Falls back to the path verbatim if the
/// current directory cannot be determined.
fn absolutize_local(path: &str) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// Returns `true` if `s` looks like a Windows absolute path (e.g. `C:\foo`
/// or `D:/bar`). Checked on all platforms so that source labels round-trip
/// correctly even when a lockfile is shared across OSes.
fn is_windows_absolute(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

/// Parse a lockfile source label produced by the [`Display`] impl back
/// into an [`InstallSource`]. The inverse of `Display::fmt` and the
/// counterpart to [`parse_install_source`], which accepts user-facing
/// shorthand instead of the canonical labels.
///
/// Recognised labels:
/// - `github:<owner>/<name>[@<ref>][:<subfolder>]`
/// - `https://<host>/<path>[@<ref>][:<subfolder>]` — generic git source
/// - `local:<path>` — absolute on round-trip; passed through verbatim
pub fn parse_install_source_label(label: &str) -> Result<InstallSource, SourceParseError> {
    if let Some(rest) = label.strip_prefix("local:") {
        return Ok(InstallSource::LocalPath(PathBuf::from(rest)));
    }
    if let Some(rest) = label.strip_prefix("github:") {
        return parse_github_source(rest).map(InstallSource::Github);
    }
    if let Some(rest) = label.strip_prefix("https://") {
        return parse_url_source(rest);
    }
    if label.starts_with("gitlab:") || label.starts_with("gitlab+") {
        return Err(SourceParseError::GitlabShorthandRemoved);
    }
    Err(SourceParseError::InvalidFormat)
}

fn parse_url_source(url_without_scheme: &str) -> Result<InstallSource, SourceParseError> {
    // Split host from path: "gitlab.com/owner/repo@ref:sub" or "github.com/owner/repo"
    let (host_part, path_part) = url_without_scheme
        .split_once('/')
        .ok_or(SourceParseError::InvalidFormat)?;

    // Strip port from host for comparison
    let host_name = host_part.split(':').next().unwrap_or(host_part);

    if host_name == "github.com" {
        return parse_github_source(path_part).map(InstallSource::Github);
    }

    // Any other https host is an opaque git remote.
    parse_git_url(host_part, path_part).map(InstallSource::Git)
}

/// Parse the path portion of a generic git URL:
/// `<path>[@<ref>][:<subfolder>]`. The host is opaque (and may carry a
/// port); the path may nest arbitrarily deep (GitLab subgroups). A
/// trailing `/` or `.git` on the repo path is stripped so browser-pasted
/// and clone URLs normalise to one canonical label.
fn parse_git_url(host: &str, path: &str) -> Result<GitRepo, SourceParseError> {
    // Split off :subfolder first. The port is on the host part, already
    // split away, so a ':' here is always a subfolder separator.
    let (before_subfolder, subfolder) = if let Some((before, sub)) = path.split_once(':') {
        if sub.trim().is_empty() {
            return Err(SourceParseError::EmptySubfolder);
        }
        (before, Some(sub.to_string()))
    } else {
        (path, None)
    };

    // Split off @ref
    let (repo_path, git_ref) = if let Some((before, r)) = before_subfolder.split_once('@') {
        if r.trim().is_empty() {
            return Err(SourceParseError::EmptyRef);
        }
        (before, Some(r.to_string()))
    } else {
        (before_subfolder, None)
    };

    let repo_path = repo_path.trim_end_matches('/');
    let repo_path = repo_path.strip_suffix(".git").unwrap_or(repo_path);
    if repo_path.trim().is_empty() {
        return Err(SourceParseError::EmptySegment);
    }

    Ok(GitRepo {
        url: format!("https://{host}/{repo_path}"),
        git_ref,
        subfolder,
    })
}

pub fn parse_github_source(source: &str) -> Result<GithubRepo, SourceParseError> {
    // Split off :subfolder first
    let (before_subfolder, subfolder) = if let Some((before, sub)) = source.split_once(':') {
        if sub.trim().is_empty() {
            return Err(SourceParseError::EmptySubfolder);
        }
        (before, Some(sub.to_string()))
    } else {
        (source, None)
    };

    // Split off @ref
    let (repo_source, git_ref) = if let Some((before, r)) = before_subfolder.split_once('@') {
        if r.trim().is_empty() {
            return Err(SourceParseError::EmptyRef);
        }
        (before, Some(r.to_string()))
    } else {
        (before_subfolder, None)
    };

    let mut repo = parse_github_repo(repo_source)?;
    repo.git_ref = git_ref;
    repo.subfolder = subfolder;
    Ok(repo)
}

pub(crate) fn parse_github_repo(source: &str) -> Result<GithubRepo, SourceParseError> {
    let Some((owner, name)) = source.split_once('/') else {
        return Err(SourceParseError::InvalidFormat);
    };

    if owner.trim().is_empty() || name.trim().is_empty() {
        return Err(SourceParseError::EmptySegment);
    }

    if source.matches('/').count() != 1 {
        return Err(SourceParseError::InvalidFormat);
    }

    Ok(GithubRepo {
        owner: owner.to_string(),
        name: name.to_string(),
        git_ref: None,
        subfolder: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise tests that mutate HOME / USERPROFILE env vars.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn parse_valid_owner_repo() {
        let repo = parse_github_repo("microsoft/skills").expect("must parse");
        assert_eq!(repo.owner, "microsoft");
        assert_eq!(repo.name, "skills");
        assert_eq!(repo.subfolder, None);
    }

    #[test]
    fn reject_missing_separator() {
        let err = parse_github_repo("microsoft-skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn reject_empty_owner() {
        let err = parse_github_repo("/skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySegment);
    }

    #[test]
    fn reject_empty_repo() {
        let err = parse_github_repo("microsoft/").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySegment);
    }

    #[test]
    fn reject_extra_slashes() {
        let err = parse_github_repo("a/b/c").expect_err("must fail");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn parse_local_path_dot_slash() {
        let source = parse_install_source("./my-skills").expect("must parse");
        let expected = std::path::absolute("./my-skills").unwrap();
        assert_eq!(source, InstallSource::LocalPath(expected));
    }

    #[test]
    fn parse_local_path_dot_dot_slash() {
        let source = parse_install_source("../shared/skills").expect("must parse");
        let expected = std::path::absolute("../shared/skills").unwrap();
        assert_eq!(source, InstallSource::LocalPath(expected));
    }

    #[test]
    fn relative_local_path_canonicalizes_to_absolute_label() {
        // Two spellings of the same directory must collapse to one canonical
        // `local:` label so cross-source resolution and conflict detection
        // treat them as the same source (issue #212).
        let cwd = std::env::current_dir().unwrap();
        let rel = parse_install_source("./some-reg").expect("relative");
        let abs = parse_install_source(cwd.join("some-reg").to_str().unwrap()).expect("absolute");
        assert_eq!(rel.to_string(), abs.to_string(), "spellings must unify");
        assert_eq!(rel, InstallSource::LocalPath(cwd.join("some-reg")));
    }

    #[test]
    fn parse_local_path_absolute() {
        let source = parse_install_source("/tmp/skills").expect("must parse");
        assert_eq!(
            source,
            InstallSource::LocalPath(PathBuf::from("/tmp/skills"))
        );
    }

    #[test]
    fn parse_local_path_windows_drive_letter() {
        let source = parse_install_source(r"C:\Users\runner\skills").expect("must parse");
        assert_eq!(
            source,
            InstallSource::LocalPath(PathBuf::from(r"C:\Users\runner\skills"))
        );
        // Forward-slash variant
        let source2 = parse_install_source("D:/projects/skills").expect("must parse");
        assert_eq!(
            source2,
            InstallSource::LocalPath(PathBuf::from("D:/projects/skills"))
        );
    }

    #[test]
    fn parse_local_path_tilde_expands_home() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", "/users/alice") };
        let result = parse_install_source("~/skills/code-review");
        match prev {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        let source = result.expect("must parse");
        assert_eq!(
            source,
            InstallSource::LocalPath(PathBuf::from("/users/alice/skills/code-review"))
        );
    }

    #[test]
    fn parse_local_path_tilde_alone_expands_to_home() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", "/users/alice") };
        let result = parse_install_source("~");
        match prev {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        let source = result.expect("must parse");
        assert_eq!(
            source,
            InstallSource::LocalPath(PathBuf::from("/users/alice"))
        );
    }

    #[test]
    fn parse_github_source_with_subfolder() {
        let source = parse_install_source("microsoft/skills:subfolder/path").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.owner, "microsoft");
        assert_eq!(repo.name, "skills");
        assert_eq!(repo.subfolder.as_deref(), Some("subfolder/path"));
    }

    #[test]
    fn parse_github_source_without_subfolder() {
        let source = parse_install_source("microsoft/skills").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.owner, "microsoft");
        assert_eq!(repo.name, "skills");
        assert_eq!(repo.subfolder, None);
    }

    #[test]
    fn reject_empty_subfolder() {
        let err = parse_install_source("microsoft/skills:").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySubfolder);
    }

    #[test]
    fn reject_whitespace_subfolder() {
        let err = parse_install_source("microsoft/skills: ").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySubfolder);
    }

    #[test]
    fn parse_ref_branch() {
        let source = parse_install_source("microsoft/skills@main").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.owner, "microsoft");
        assert_eq!(repo.name, "skills");
        assert_eq!(repo.git_ref.as_deref(), Some("main"));
        assert_eq!(repo.subfolder, None);
    }

    #[test]
    fn parse_ref_tag() {
        let source = parse_install_source("microsoft/skills@v1.0").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.git_ref.as_deref(), Some("v1.0"));
    }

    #[test]
    fn parse_ref_commit_sha() {
        let source = parse_install_source("microsoft/skills@abc1234def5678").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.git_ref.as_deref(), Some("abc1234def5678"));
    }

    #[test]
    fn parse_ref_with_subfolder() {
        let source = parse_install_source("microsoft/skills@v1.0:tools/lint").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.git_ref.as_deref(), Some("v1.0"));
        assert_eq!(repo.subfolder.as_deref(), Some("tools/lint"));
    }

    #[test]
    fn reject_empty_ref() {
        let err = parse_install_source("microsoft/skills@").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptyRef);
    }

    #[test]
    fn reject_empty_ref_with_subfolder() {
        let err = parse_install_source("microsoft/skills@:tools").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptyRef);
    }

    #[test]
    fn parse_github_url() {
        let source =
            parse_install_source("https://github.com/microsoft/skills").expect("must parse");
        let InstallSource::Github(repo) = source else {
            panic!("expected Github");
        };
        assert_eq!(repo.owner, "microsoft");
        assert_eq!(repo.name, "skills");
    }

    // Generic git-URL source tests — any https host that is not github.com
    // is an opaque git remote (GitLab, Bitbucket, Gitea, corporate git).

    #[test]
    fn parse_git_url_gitlab_com() {
        let source = parse_install_source("https://gitlab.com/team/skills").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlab.com/team/skills");
        assert_eq!(repo.git_ref, None);
        assert_eq!(repo.subfolder, None);
    }

    #[test]
    fn parse_git_url_bitbucket() {
        let source = parse_install_source("https://bitbucket.org/team/repo").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://bitbucket.org/team/repo");
    }

    #[test]
    fn parse_git_url_self_hosted_with_port() {
        let source =
            parse_install_source("https://git.company.com:8443/team/skills").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://git.company.com:8443/team/skills");
    }

    #[test]
    fn parse_git_url_subgroups_any_depth() {
        let source = parse_install_source(
            "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed",
        )
        .expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(
            repo.url,
            "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed"
        );
    }

    #[test]
    fn parse_git_url_with_ref_and_subfolder() {
        let source = parse_install_source(
            "https://gitlabee.dt.renault.com/partners/seed@v0.2.0:skills/seed.bundle.yaml",
        )
        .expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlabee.dt.renault.com/partners/seed");
        assert_eq!(repo.git_ref.as_deref(), Some("v0.2.0"));
        assert_eq!(repo.subfolder.as_deref(), Some("skills/seed.bundle.yaml"));
    }

    #[test]
    fn parse_git_url_single_path_segment() {
        // Plain git servers can host a repo directly under the root —
        // unlike GitLab there is no namespace requirement.
        let source = parse_install_source("https://git.example.com/repo").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://git.example.com/repo");
    }

    #[test]
    fn parse_git_url_strips_trailing_dot_git() {
        let source =
            parse_install_source("https://gitlab.com/team/skills.git").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlab.com/team/skills");
    }

    #[test]
    fn parse_git_url_strips_trailing_slash() {
        let source = parse_install_source("https://gitlab.com/team/skills/").expect("must parse");
        let InstallSource::Git(repo) = source else {
            panic!("expected Git");
        };
        assert_eq!(repo.url, "https://gitlab.com/team/skills");
    }

    #[test]
    fn reject_git_url_without_path() {
        let err = parse_install_source("https://gitlab.com").expect_err("must fail");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn reject_git_url_with_empty_path() {
        let err = parse_install_source("https://gitlab.com/").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySegment);
    }

    #[test]
    fn reject_removed_gitlab_shorthand() {
        let err = parse_install_source("gitlab:team/skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
    }

    #[test]
    fn reject_removed_gitlab_plus_shorthand() {
        let err =
            parse_install_source("gitlab+git.company.com:team/skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
    }

    // Lockfile source label round-trip — every shape Display can produce
    // must round-trip through parse_install_source_label so `update` can
    // reconstruct the source from a lockfile entry.

    fn assert_label_roundtrip(s: &InstallSource) {
        let label = s.to_string();
        let parsed = parse_install_source_label(&label)
            .unwrap_or_else(|e| panic!("round-trip failed for `{label}`: {e:?}"));
        assert_eq!(&parsed, s, "round-trip mismatch for `{label}`");
    }

    #[test]
    fn label_roundtrip_local_path() {
        assert_label_roundtrip(&InstallSource::LocalPath(PathBuf::from("/abs/path")));
        assert_label_roundtrip(&InstallSource::LocalPath(PathBuf::from(
            "/path with spaces/x",
        )));
    }

    #[test]
    fn label_roundtrip_github_minimal() {
        assert_label_roundtrip(&InstallSource::Github(GithubRepo {
            owner: "driftsys".into(),
            name: "skills".into(),
            git_ref: None,
            subfolder: None,
        }));
    }

    #[test]
    fn label_roundtrip_github_full() {
        assert_label_roundtrip(&InstallSource::Github(GithubRepo {
            owner: "driftsys".into(),
            name: "skills".into(),
            git_ref: Some("v1.2.3".into()),
            subfolder: Some("rules/lint".into()),
        }));
    }

    #[test]
    fn label_roundtrip_git_minimal() {
        assert_label_roundtrip(&InstallSource::Git(GitRepo {
            url: "https://gitlab.com/team/skills".into(),
            git_ref: None,
            subfolder: None,
        }));
    }

    #[test]
    fn label_roundtrip_git_full() {
        assert_label_roundtrip(&InstallSource::Git(GitRepo {
            url: "https://gitlabee.dt.renault.com/partners/alliance-car/devex/process/seed".into(),
            git_ref: Some("v0.2.0".into()),
            subfolder: Some("skills/seed.bundle.yaml".into()),
        }));
    }

    #[test]
    fn label_rejects_removed_gitlab_shorthand() {
        // Old lockfiles may still carry `gitlab:` labels — they hard-fail
        // with the same self-serve error as the CLI input form (pre-1.0,
        // no migration).
        let err = parse_install_source_label("gitlab:team/x@main").expect_err("must reject");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
        let err = parse_install_source_label("gitlab+h.com:team/x").expect_err("must reject");
        assert_eq!(err, SourceParseError::GitlabShorthandRemoved);
    }

    #[test]
    fn label_rejects_bare_string() {
        let err = parse_install_source_label("driftsys/skills").expect_err("must reject");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn home_dir_reads_home_var() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev_home = std::env::var_os("HOME");
        let prev_up = std::env::var_os("USERPROFILE");
        unsafe {
            std::env::set_var("HOME", "/home/alice");
            std::env::remove_var("USERPROFILE");
        }
        let result = home_dir();
        unsafe {
            match prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match prev_up {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
        assert_eq!(result, Some(PathBuf::from("/home/alice")));
    }

    #[test]
    fn home_dir_falls_back_to_userprofile() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev_home = std::env::var_os("HOME");
        let prev_up = std::env::var_os("USERPROFILE");
        unsafe {
            std::env::remove_var("HOME");
            std::env::set_var("USERPROFILE", r"C:\Users\alice");
        }
        let result = home_dir();
        unsafe {
            match prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match prev_up {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
        assert_eq!(result, Some(PathBuf::from(r"C:\Users\alice")));
    }

    #[test]
    fn home_dir_prefers_home_over_userprofile() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev_home = std::env::var_os("HOME");
        let prev_up = std::env::var_os("USERPROFILE");
        unsafe {
            std::env::set_var("HOME", "/home/alice");
            std::env::set_var("USERPROFILE", r"C:\Users\alice");
        }
        let result = home_dir();
        unsafe {
            match prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match prev_up {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
        assert_eq!(result, Some(PathBuf::from("/home/alice")));
    }

    #[test]
    fn home_dir_returns_none_when_neither_set() {
        let _lock = ENV_LOCK.lock().unwrap();
        let prev_home = std::env::var_os("HOME");
        let prev_up = std::env::var_os("USERPROFILE");
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("USERPROFILE");
        }
        let result = home_dir();
        unsafe {
            match prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match prev_up {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
        assert_eq!(result, None);
    }
}
