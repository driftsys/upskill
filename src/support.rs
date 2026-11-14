//! The support level of each client and kind.
//!
//! The table is compiled into the binary. A maintainer updates it after a
//! smoke run against the real client. `doctor` prints it and does not detect
//! client versions.

use crate::pipeline::ItemKind;

/// How far upskill's output for one client and kind is verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SupportLevel {
    /// The smoke test passed on the recorded client version.
    Supported,
    /// Implemented; the smoke test is missing, failing or older than the client.
    Experimental,
    /// Not implemented, or known not to work.
    Unsupported,
}

impl std::fmt::Display for SupportLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Supported => write!(f, "supported"),
            Self::Experimental => write!(f, "experimental"),
            Self::Unsupported => write!(f, "unsupported"),
        }
    }
}

/// The level of one client and kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct SupportCell {
    /// `claude` | `copilot` | `opencode` | `codex`.
    pub client: &'static str,
    pub kind: ItemKind,
    pub level: SupportLevel,
    /// Client version of the smoke run that passed. `None` unless supported.
    pub client_version: Option<&'static str>,
    /// Date of that run, `YYYY-MM-DD`. `None` unless supported.
    pub checked: Option<&'static str>,
}

const fn cell(client: &'static str, kind: ItemKind, level: SupportLevel) -> SupportCell {
    SupportCell {
        client,
        kind,
        level,
        client_version: None,
        checked: None,
    }
}

/// Twelve cells: four clients in the order `claude`, `copilot`, `opencode`,
/// `codex`, each with rule, skill and agent.
pub const SUPPORT: &[SupportCell] = &[
    cell("claude", ItemKind::Rule, SupportLevel::Experimental),
    cell("claude", ItemKind::Skill, SupportLevel::Experimental),
    cell("claude", ItemKind::Agent, SupportLevel::Experimental),
    cell("copilot", ItemKind::Rule, SupportLevel::Experimental),
    cell("copilot", ItemKind::Skill, SupportLevel::Experimental),
    cell("copilot", ItemKind::Agent, SupportLevel::Experimental),
    cell("opencode", ItemKind::Rule, SupportLevel::Experimental),
    cell("opencode", ItemKind::Skill, SupportLevel::Experimental),
    cell("opencode", ItemKind::Agent, SupportLevel::Experimental),
    cell("codex", ItemKind::Rule, SupportLevel::Unsupported),
    cell("codex", ItemKind::Skill, SupportLevel::Unsupported),
    cell("codex", ItemKind::Agent, SupportLevel::Unsupported),
];

/// One line of the `doctor` support section: the client padded to
/// `client_width`, the kind, the level, and for a cell with a version and a
/// date the suffix ` (<client> <version>, <date>)`.
pub fn format_row(cell: &SupportCell, client_width: usize) -> String {
    let mut line = format!(
        "  {:<client_width$} {:<6} {}",
        cell.client,
        cell.kind.to_string(),
        cell.level
    );
    if let (Some(version), Some(date)) = (cell.client_version, cell.checked) {
        line.push_str(&format!(" ({} {}, {})", cell.client, version, date));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_supported_row_ends_with_the_client_version_and_the_date() {
        let supported = SupportCell {
            client: "claude",
            kind: ItemKind::Skill,
            level: SupportLevel::Supported,
            client_version: Some("2.1.0"),
            checked: Some("2026-11-20"),
        };
        assert_eq!(
            format_row(&supported, 8),
            "  claude   skill  supported (claude 2.1.0, 2026-11-20)"
        );
    }

    #[test]
    fn an_experimental_row_has_no_suffix() {
        let experimental = cell("copilot", ItemKind::Agent, SupportLevel::Experimental);
        assert_eq!(
            format_row(&experimental, 8),
            "  copilot  agent  experimental"
        );
    }

    #[test]
    fn the_table_has_one_cell_for_each_client_and_kind() {
        assert_eq!(SUPPORT.len(), 12);
        let mut pairs: Vec<(&str, ItemKind)> = SUPPORT.iter().map(|c| (c.client, c.kind)).collect();
        pairs.sort();
        pairs.dedup();
        assert_eq!(pairs.len(), 12, "a client and kind pair appears twice");
        for client in ["claude", "copilot", "opencode", "codex"] {
            for kind in [ItemKind::Rule, ItemKind::Skill, ItemKind::Agent] {
                assert!(pairs.contains(&(client, kind)), "{client} {kind}");
            }
        }
    }

    #[test]
    fn only_a_supported_cell_has_a_version_and_a_date() {
        for c in SUPPORT {
            let supported = c.level == SupportLevel::Supported;
            assert_eq!(
                c.client_version.is_some(),
                supported,
                "{} {}",
                c.client,
                c.kind
            );
            assert_eq!(c.checked.is_some(), supported, "{} {}", c.client, c.kind);
        }
    }
}
