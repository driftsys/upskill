//! Ancillary file management for the v0.2 install pipeline.
//!
//! Per [ADR-0003](../../docs/adr/0003-generation-pipeline.md) §"Ancillary
//! file handling" and format-spec §7.4. The pipeline writes per-client item
//! output (rules / skills / agents) to the paths in §7; some clients also
//! need a single one-time hand-shake file at the consumer-project root for
//! discovery to work. Those files are managed here, separately from the
//! per-item generation pipeline:
//!
//! - **`CLAUDE.md`** — created once with `@AGENTS.md` content if absent.
//!   Bridges Claude Code (which does not natively read `AGENTS.md`) to the
//!   project-level instructions Claude users expect to live in `AGENTS.md`.
//!   Never overwritten — protects user customisations.
//!
//! - `.vscode/settings.json` and `opencode.json` — separate slice. Stubs are
//!   not provided here; this module is intentionally focused on the
//!   non-mutating CLAUDE.md case until the in-place JSON edit story
//!   (preserving unknown keys) lands.

use anyhow::{Context, Result};
use std::path::Path;

/// Filename written at the consumer-project root.
const CLAUDE_MD: &str = "CLAUDE.md";

/// Content the bridge file ships with. The single `@AGENTS.md` line is the
/// Claude Code "load this file" directive; everything Claude Code needs at
/// the project level is then expected to live in `AGENTS.md`.
const CLAUDE_MD_BRIDGE: &str = "@AGENTS.md\n";

/// Outcome of a single ancillary write, surfaced for callers that want to
/// log or report what happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AncillaryAction {
    /// File did not exist; we created it.
    Created,
    /// File already existed; we left it alone.
    Preserved,
}

/// Ensure `<target>/CLAUDE.md` exists with the bridge content.
///
/// - Absent → created with `CLAUDE_MD_BRIDGE`.
/// - Present → never modified, regardless of content. A user (or a previous
///   `upskill` run) may have customised the file; preserving it is part of
///   the contract per ADR-0003.
pub fn ensure_claude_bridge(target: &Path) -> Result<AncillaryAction> {
    let path = target.join(CLAUDE_MD);
    if path.exists() {
        return Ok(AncillaryAction::Preserved);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create dir {}", parent.display()))?;
    }
    std::fs::write(&path, CLAUDE_MD_BRIDGE).with_context(|| format!("write {}", path.display()))?;
    Ok(AncillaryAction::Created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_claude_md_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let action = ensure_claude_bridge(tmp.path()).expect("ensure");

        assert_eq!(action, AncillaryAction::Created);
        let content = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
        assert_eq!(content, CLAUDE_MD_BRIDGE);
    }

    #[test]
    fn preserves_existing_claude_md_verbatim() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        let user_content = "# My CLAUDE.md\n\nUser customisations here.\n";
        std::fs::write(&path, user_content).unwrap();

        let action = ensure_claude_bridge(tmp.path()).expect("ensure");

        assert_eq!(action, AncillaryAction::Preserved);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, user_content, "must not overwrite user content");
    }

    #[test]
    fn second_call_after_create_is_preserve() {
        let tmp = tempfile::tempdir().unwrap();
        let first = ensure_claude_bridge(tmp.path()).expect("ensure 1");
        let second = ensure_claude_bridge(tmp.path()).expect("ensure 2");
        assert_eq!(first, AncillaryAction::Created);
        assert_eq!(second, AncillaryAction::Preserved);
    }
}
