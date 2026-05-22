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
pub struct GitlabRepo {
    pub host: String,
    pub owner: String,
    pub name: String,
    pub git_ref: Option<String>,
    pub subfolder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    Github(GithubRepo),
    Gitlab(GitlabRepo),
    LocalPath(PathBuf),
}

/// Stable, round-trippable string label for use in lockfiles, log lines,
/// and CLI output. Format mirrors what `parse_install_source` accepts:
///
/// - `github:<owner>/<name>[@<ref>][:<subfolder>]`
/// - `gitlab:<owner>/<name>[@<ref>][:<subfolder>]` (host omitted when
///   `gitlab.com`; otherwise `gitlab+<host>:...`)
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
            InstallSource::Gitlab(r) => {
                if r.host == "gitlab.com" {
                    write!(f, "gitlab:{}/{}", r.owner, r.name)?;
                } else {
                    write!(f, "gitlab+{}:{}/{}", r.host, r.owner, r.name)?;
                }
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
    if source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with('/')
        || is_windows_absolute(source)
    {
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

    // gitlab: prefix
    if let Some(rest) = source.strip_prefix("gitlab:") {
        return parse_gitlab_source(rest, "gitlab.com").map(InstallSource::Gitlab);
    }

    // HTTPS URLs
    if let Some(rest) = source.strip_prefix("https://") {
        return parse_url_source(rest);
    }

    parse_github_source(source).map(InstallSource::Github)
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
/// - `gitlab:<owner>/<name>[@<ref>][:<subfolder>]` — host is `gitlab.com`
/// - `gitlab+<host>:<owner>/<name>[@<ref>][:<subfolder>]` — self-hosted
/// - `local:<path>` — absolute on round-trip; passed through verbatim
pub fn parse_install_source_label(label: &str) -> Result<InstallSource, SourceParseError> {
    if let Some(rest) = label.strip_prefix("local:") {
        return Ok(InstallSource::LocalPath(PathBuf::from(rest)));
    }
    if let Some(rest) = label.strip_prefix("github:") {
        return parse_github_source(rest).map(InstallSource::Github);
    }
    if let Some(rest) = label.strip_prefix("gitlab+") {
        let (host, after) = rest
            .split_once(':')
            .ok_or(SourceParseError::InvalidFormat)?;
        if host.is_empty() {
            return Err(SourceParseError::EmptySegment);
        }
        return parse_gitlab_source(after, host).map(InstallSource::Gitlab);
    }
    if let Some(rest) = label.strip_prefix("gitlab:") {
        return parse_gitlab_source(rest, "gitlab.com").map(InstallSource::Gitlab);
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

    // Everything else (gitlab.com, self-hosted) treated as GitLab-compatible
    parse_gitlab_source(path_part, host_part).map(InstallSource::Gitlab)
}

fn parse_gitlab_source(source: &str, host: &str) -> Result<GitlabRepo, SourceParseError> {
    // Split off :subfolder first
    let (before_subfolder, subfolder) = if let Some((before, sub)) = source.split_once(':') {
        // Avoid confusing port numbers with subfolders — port is on the host, not here
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

    let (owner, name) = repo_source
        .split_once('/')
        .ok_or(SourceParseError::InvalidFormat)?;

    if owner.trim().is_empty() || name.trim().is_empty() {
        return Err(SourceParseError::EmptySegment);
    }

    if repo_source.matches('/').count() != 1 {
        return Err(SourceParseError::InvalidFormat);
    }

    Ok(GitlabRepo {
        host: host.to_string(),
        owner: owner.to_string(),
        name: name.to_string(),
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
        assert_eq!(
            source,
            InstallSource::LocalPath(PathBuf::from("./my-skills"))
        );
    }

    #[test]
    fn parse_local_path_dot_dot_slash() {
        let source = parse_install_source("../shared/skills").expect("must parse");
        assert_eq!(
            source,
            InstallSource::LocalPath(PathBuf::from("../shared/skills"))
        );
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
        // SAFETY: tests are not parallel for env mutation here because
        // the helper acquires a process-wide mutex via a static. We
        // accept the small risk of cross-test interference for this
        // narrow case — `parse_install_source` only reads HOME, no other
        // test in this module touches it.
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

    // GitLab source tests

    #[test]
    fn parse_gitlab_prefix() {
        let source = parse_install_source("gitlab:team/skills").expect("must parse");
        let InstallSource::Gitlab(repo) = source else {
            panic!("expected Gitlab");
        };
        assert_eq!(repo.host, "gitlab.com");
        assert_eq!(repo.owner, "team");
        assert_eq!(repo.name, "skills");
    }

    #[test]
    fn parse_gitlab_prefix_with_ref() {
        let source = parse_install_source("gitlab:team/skills@v2.0").expect("must parse");
        let InstallSource::Gitlab(repo) = source else {
            panic!("expected Gitlab");
        };
        assert_eq!(repo.git_ref.as_deref(), Some("v2.0"));
    }

    #[test]
    fn parse_gitlab_prefix_with_subfolder() {
        let source =
            parse_install_source("gitlab:team/skills@v1.0:tools/lint").expect("must parse");
        let InstallSource::Gitlab(repo) = source else {
            panic!("expected Gitlab");
        };
        assert_eq!(repo.git_ref.as_deref(), Some("v1.0"));
        assert_eq!(repo.subfolder.as_deref(), Some("tools/lint"));
    }

    #[test]
    fn parse_gitlab_url() {
        let source = parse_install_source("https://gitlab.com/team/skills").expect("must parse");
        let InstallSource::Gitlab(repo) = source else {
            panic!("expected Gitlab");
        };
        assert_eq!(repo.host, "gitlab.com");
        assert_eq!(repo.owner, "team");
        assert_eq!(repo.name, "skills");
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

    #[test]
    fn parse_selfhosted_gitlab_url() {
        let source =
            parse_install_source("https://git.company.com/team/skills").expect("must parse");
        let InstallSource::Gitlab(repo) = source else {
            panic!("expected Gitlab");
        };
        assert_eq!(repo.host, "git.company.com");
        assert_eq!(repo.owner, "team");
        assert_eq!(repo.name, "skills");
    }

    #[test]
    fn parse_selfhosted_gitlab_with_port() {
        let source =
            parse_install_source("https://git.company.com:8443/team/skills").expect("must parse");
        let InstallSource::Gitlab(repo) = source else {
            panic!("expected Gitlab");
        };
        assert_eq!(repo.host, "git.company.com:8443");
        assert_eq!(repo.owner, "team");
        assert_eq!(repo.name, "skills");
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
    fn label_roundtrip_gitlab_dot_com() {
        assert_label_roundtrip(&InstallSource::Gitlab(GitlabRepo {
            host: "gitlab.com".into(),
            owner: "team".into(),
            name: "skills".into(),
            git_ref: Some("main".into()),
            subfolder: None,
        }));
    }

    #[test]
    fn label_roundtrip_gitlab_self_hosted() {
        assert_label_roundtrip(&InstallSource::Gitlab(GitlabRepo {
            host: "gitlab.example.com".into(),
            owner: "team".into(),
            name: "rules".into(),
            git_ref: None,
            subfolder: Some("a/b".into()),
        }));
    }

    #[test]
    fn label_rejects_bare_string() {
        let err = parse_install_source_label("driftsys/skills").expect_err("must reject");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn label_rejects_gitlab_plus_without_host() {
        let err = parse_install_source_label("gitlab+:team/x").expect_err("must reject");
        assert_eq!(err, SourceParseError::EmptySegment);
    }

    #[test]
    fn home_dir_reads_home_var() {
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
