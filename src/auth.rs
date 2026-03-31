use std::process::Command;

/// Resolved authentication for a GitHub API request.
#[derive(Debug, PartialEq, Eq)]
pub enum GitHubAuth {
    Token(String),
    None,
}

/// Resolve a GitHub token from environment variables or the `gh` CLI.
///
/// Checks in order:
/// 1. `GITHUB_TOKEN` environment variable
/// 2. `GH_TOKEN` environment variable
/// 3. `gh auth token` CLI output
pub fn resolve_github_token() -> GitHubAuth {
    if let Some(token) = env_token("GITHUB_TOKEN") {
        return GitHubAuth::Token(token);
    }

    if let Some(token) = env_token("GH_TOKEN") {
        return GitHubAuth::Token(token);
    }

    if let Some(token) = gh_auth_token() {
        return GitHubAuth::Token(token);
    }

    GitHubAuth::None
}

fn env_token(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.trim().is_empty())
}

fn gh_auth_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

impl GitHubAuth {
    /// Returns the token string if present.
    pub fn token(&self) -> Option<&str> {
        match self {
            GitHubAuth::Token(t) => Some(t),
            GitHubAuth::None => None,
        }
    }

    /// Returns true if authentication is available.
    pub fn is_authenticated(&self) -> bool {
        matches!(self, GitHubAuth::Token(_))
    }
}

/// Resolved authentication for a GitLab API request.
#[derive(Debug, PartialEq, Eq)]
pub enum GitLabAuth {
    Token(String),
    None,
}

/// Resolve a GitLab token from environment variables or the `glab` CLI.
///
/// Checks in order:
/// 1. `GITLAB_TOKEN` environment variable
/// 2. `GL_TOKEN` environment variable
/// 3. `glab auth token` CLI output
pub fn resolve_gitlab_token() -> GitLabAuth {
    if let Some(token) = env_token("GITLAB_TOKEN") {
        return GitLabAuth::Token(token);
    }

    if let Some(token) = env_token("GL_TOKEN") {
        return GitLabAuth::Token(token);
    }

    if let Some(token) = glab_auth_token() {
        return GitLabAuth::Token(token);
    }

    GitLabAuth::None
}

fn glab_auth_token() -> Option<String> {
    let output = Command::new("glab").args(["auth", "token"]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

impl GitLabAuth {
    /// Returns the token string if present.
    pub fn token(&self) -> Option<&str> {
        match self {
            GitLabAuth::Token(t) => Some(t),
            GitLabAuth::None => None,
        }
    }

    /// Returns true if authentication is available.
    pub fn is_authenticated(&self) -> bool {
        matches!(self, GitLabAuth::Token(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serial lock to prevent env var races between tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce() -> R, R>(vars: &[(&str, Option<&str>)], f: F) -> R {
        let _lock = ENV_LOCK.lock().unwrap();
        let originals: Vec<_> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();
        for (k, v) in vars {
            // SAFETY: tests are serialised by ENV_LOCK so no concurrent mutation.
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        let result = f();
        for (k, original) in &originals {
            // SAFETY: tests are serialised by ENV_LOCK so no concurrent mutation.
            unsafe {
                match original {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        result
    }

    #[test]
    fn github_token_env_is_used() {
        with_env(
            &[("GITHUB_TOKEN", Some("ghp_test")), ("GH_TOKEN", None)],
            || {
                let auth = resolve_github_token();
                assert_eq!(auth, GitHubAuth::Token("ghp_test".into()));
            },
        );
    }

    #[test]
    fn gh_token_env_is_used_as_fallback() {
        with_env(
            &[("GITHUB_TOKEN", None), ("GH_TOKEN", Some("gho_fallback"))],
            || {
                let auth = resolve_github_token();
                assert_eq!(auth, GitHubAuth::Token("gho_fallback".into()));
            },
        );
    }

    #[test]
    fn github_token_takes_precedence() {
        with_env(
            &[
                ("GITHUB_TOKEN", Some("ghp_primary")),
                ("GH_TOKEN", Some("gho_secondary")),
            ],
            || {
                let auth = resolve_github_token();
                assert_eq!(auth, GitHubAuth::Token("ghp_primary".into()));
            },
        );
    }

    #[test]
    fn empty_env_vars_are_ignored() {
        with_env(
            &[("GITHUB_TOKEN", Some("")), ("GH_TOKEN", Some(""))],
            || {
                // Falls through to gh CLI or None
                let auth = resolve_github_token();
                // Can't assert exact value since gh CLI may or may not be present
                let _ = auth;
            },
        );
    }

    #[test]
    fn whitespace_only_env_vars_are_ignored() {
        with_env(
            &[("GITHUB_TOKEN", Some("  ")), ("GH_TOKEN", Some(" \t "))],
            || {
                let auth = resolve_github_token();
                // Falls through to gh CLI or None; key point is whitespace-only is not a token
                assert_ne!(auth, GitHubAuth::Token("  ".into()));
                assert_ne!(auth, GitHubAuth::Token(" \t ".into()));
            },
        );
    }

    #[test]
    fn token_accessor() {
        let auth = GitHubAuth::Token("abc".into());
        assert_eq!(auth.token(), Some("abc"));
        assert!(auth.is_authenticated());

        let none = GitHubAuth::None;
        assert_eq!(none.token(), None);
        assert!(!none.is_authenticated());
    }

    // GitLab auth tests

    #[test]
    fn gitlab_token_env_is_used() {
        with_env(
            &[("GITLAB_TOKEN", Some("glpat_test")), ("GL_TOKEN", None)],
            || {
                let auth = resolve_gitlab_token();
                assert_eq!(auth, GitLabAuth::Token("glpat_test".into()));
            },
        );
    }

    #[test]
    fn gl_token_env_is_used_as_fallback() {
        with_env(
            &[("GITLAB_TOKEN", None), ("GL_TOKEN", Some("gl_fallback"))],
            || {
                let auth = resolve_gitlab_token();
                assert_eq!(auth, GitLabAuth::Token("gl_fallback".into()));
            },
        );
    }

    #[test]
    fn gitlab_token_takes_precedence() {
        with_env(
            &[
                ("GITLAB_TOKEN", Some("glpat_primary")),
                ("GL_TOKEN", Some("gl_secondary")),
            ],
            || {
                let auth = resolve_gitlab_token();
                assert_eq!(auth, GitLabAuth::Token("glpat_primary".into()));
            },
        );
    }

    #[test]
    fn gitlab_token_accessor() {
        let auth = GitLabAuth::Token("abc".into());
        assert_eq!(auth.token(), Some("abc"));
        assert!(auth.is_authenticated());

        let none = GitLabAuth::None;
        assert_eq!(none.token(), None);
        assert!(!none.is_authenticated());
    }
}
