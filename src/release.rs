//! The CLI release check: version parsing and tag selection for
//! `update --notice`. This module does no I/O.

use std::fmt;
use std::time::Duration;

/// The repository whose tags give the latest CLI release.
pub const RELEASE_URL: &str = "https://github.com/driftsys/upskill";
/// The time that the tag lookup may take.
pub const TIME_LIMIT: Duration = Duration::from_secs(3);
/// The environment variable that turns the release check off.
pub const OPT_OUT_ENV: &str = "UPSKILL_NO_UPDATE_CHECK";

/// A release version `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parse `X.Y.Z` with an optional leading `v`. A pre-release suffix, a
/// version with fewer or more parts, and any other text give `None`.
pub fn parse_version(s: &str) -> Option<Version> {
    let mut parts = s.strip_prefix('v').unwrap_or(s).split('.');
    let mut next = || {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse::<u64>().ok()
    };
    let version = Version {
        major: next()?,
        minor: next()?,
        patch: next()?,
    };
    parts.next().is_none().then_some(version)
}

/// The greatest version among the tags in `git ls-remote --tags` output
/// (lines of `<sha>\trefs/tags/<tag>`). Peeled `^{}` lines and tags that are
/// not a plain version are skipped.
pub fn latest_tag(ls_remote_output: &str) -> Option<Version> {
    ls_remote_output
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter_map(|(_, name)| name.strip_prefix("refs/tags/"))
        .filter(|tag| !tag.ends_with("^{}"))
        .filter_map(parse_version)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    #[test]
    fn parse_accepts_v_prefix_and_plain() {
        assert_eq!(parse_version("v0.9.4"), Some(v(0, 9, 4)));
        assert_eq!(parse_version("0.9.4"), Some(v(0, 9, 4)));
    }

    #[test]
    fn parse_rejects_prerelease_and_short() {
        assert_eq!(parse_version("v1.0.0-rc1"), None);
        assert_eq!(parse_version("v1.2"), None);
        assert_eq!(parse_version("latest"), None);
    }

    #[test]
    fn latest_tag_ignores_peeled_and_odd_tags() {
        let out = "aaa\trefs/tags/v0.9.3\n\
                   bbb\trefs/tags/v0.9.4\n\
                   ccc\trefs/tags/v0.9.4^{}\n\
                   ddd\trefs/tags/v1.0.0-rc1\n\
                   eee\trefs/tags/latest\n";
        assert_eq!(latest_tag(out), Some(v(0, 9, 4)));
    }

    #[test]
    fn latest_tag_orders_numerically() {
        let out = "a\trefs/tags/v0.9.9\nb\trefs/tags/v0.9.10\n";
        assert_eq!(latest_tag(out), Some(v(0, 9, 10)));
    }

    #[test]
    fn latest_tag_of_empty_output_is_none() {
        assert_eq!(latest_tag(""), None);
    }
}
