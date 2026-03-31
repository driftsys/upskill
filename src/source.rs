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

pub fn parse_install_source(source: &str) -> Result<InstallSource, SourceParseError> {
    if source.starts_with("./") || source.starts_with("../") || source.starts_with('/') {
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
}
