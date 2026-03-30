//! # upskill
//!
//! Upskill your coding agents.
//!
//! Ultra-lightweight [Agent Skills](https://agentskills.io/) package manager
//! in Rust. Install, list, update, and remove SKILL.md packages across
//! coding agents (Claude Code, Copilot, Codex, Cursor, OpenCode).
//!
//! No Node.js. No npm. Single static binary.
//!
//! ## Status
//!
//! This crate is under active development. v0.1.0 is a name reservation.
//! See the repository for progress.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRepo {
    pub owner: String,
    pub name: String,
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
}

pub fn parse_install_source(source: &str) -> Result<InstallSource, SourceParseError> {
    if source.starts_with("./") || source.starts_with("../") || source.starts_with('/') {
        return Ok(InstallSource::LocalPath(source.to_string()));
    }

    parse_github_repo(source).map(InstallSource::Github)
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
    }

    #[test]
    fn reject_missing_separator() {
        let err = parse_github_repo("microsoft-skills").expect_err("must fail");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn reject_empty_segments() {
        let err = parse_github_repo("microsoft/").expect_err("must fail");
        assert_eq!(err, SourceParseError::EmptySegment);
    }

    #[test]
    fn reject_multiple_slashes() {
        let err = parse_github_repo("owner/repo/extra").expect_err("must fail");
        assert_eq!(err, SourceParseError::InvalidFormat);
    }

    #[test]
    fn parse_local_relative_source() {
        let source = parse_install_source("./skills").expect("must parse");
        assert_eq!(source, InstallSource::LocalPath("./skills".to_string()));
    }

    #[test]
    fn parse_local_absolute_source() {
        let source = parse_install_source("/tmp/skills").expect("must parse");
        assert_eq!(source, InstallSource::LocalPath("/tmp/skills".to_string()));
    }

    #[test]
    fn parse_github_source_from_install_parser() {
        let source = parse_install_source("owner/repo").expect("must parse");
        assert_eq!(
            source,
            InstallSource::Github(GithubRepo {
                owner: "owner".to_string(),
                name: "repo".to_string(),
            })
        );
    }
}
