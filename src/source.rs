use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRepo {
    pub owner: String,
    pub name: String,
    pub subfolder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    Github(GithubRepo),
    LocalPath(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SourceParseError {
    #[error("source must be in owner/repo format")]
    InvalidFormat,
    #[error("owner and repo must be non-empty")]
    EmptySegment,
    #[error("subfolder path must be non-empty")]
    EmptySubfolder,
}

pub fn parse_install_source(source: &str) -> Result<InstallSource, SourceParseError> {
    if source.starts_with("./") || source.starts_with("../") || source.starts_with('/') {
        return Ok(InstallSource::LocalPath(source.to_string()));
    }

    parse_github_source(source).map(InstallSource::Github)
}

pub fn parse_github_source(source: &str) -> Result<GithubRepo, SourceParseError> {
    let (repo_source, subfolder) = if let Some((repo_source, subfolder)) = source.split_once(':') {
        if subfolder.trim().is_empty() {
            return Err(SourceParseError::EmptySubfolder);
        }
        (repo_source, Some(subfolder.to_string()))
    } else {
        (source, None)
    };

    let mut repo = parse_github_repo(repo_source)?;
    repo.subfolder = subfolder;
    Ok(repo)
}

pub fn parse_github_repo(source: &str) -> Result<GithubRepo, SourceParseError> {
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
        assert_eq!(source, InstallSource::LocalPath("./my-skills".to_string()));
    }

    #[test]
    fn parse_local_path_dot_dot_slash() {
        let source = parse_install_source("../shared/skills").expect("must parse");
        assert_eq!(
            source,
            InstallSource::LocalPath("../shared/skills".to_string())
        );
    }

    #[test]
    fn parse_local_path_absolute() {
        let source = parse_install_source("/tmp/skills").expect("must parse");
        assert_eq!(source, InstallSource::LocalPath("/tmp/skills".to_string()));
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
}
