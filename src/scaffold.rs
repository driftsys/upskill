//! `upskill new <kind> <name>` — scaffold a starter SSOT item.
//!
//! Author command per ADR-0004 — runs only inside a source-registry
//! tree. Writes one file at `<cwd>/<kind>s/<name>/<KIND>.md` (per
//! format-spec §2.1) with the minimum frontmatter the spec requires
//! plus kind-specific defaults: `mode: subagent` / `model: sonnet`
//! for agents.
//!
//! Refuses if:
//!
//! - `cwd/.upskill-lock.json` exists (consumer project, not a source
//!   registry).
//! - The target item directory already exists.
//! - `<name>` does not satisfy format-spec §2.1
//!   (`[a-z0-9-]{1,64}`, no leading/trailing hyphen).

use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};

use crate::lint::is_consumer_project;

/// Item kind to scaffold. Maps 1:1 to format-spec entrypoint files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewKind {
    Rule,
    Skill,
    Agent,
}

impl NewKind {
    /// Parse the CLI value (`rule` / `skill` / `agent`).
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "rule" => Ok(Self::Rule),
            "skill" => Ok(Self::Skill),
            "agent" => Ok(Self::Agent),
            other => bail!("unknown kind `{other}` — expected one of: rule, skill, agent"),
        }
    }

    fn dir(self) -> &'static str {
        match self {
            Self::Rule => "rules",
            Self::Skill => "skills",
            Self::Agent => "agents",
        }
    }

    fn entry(self) -> &'static str {
        match self {
            Self::Rule => "RULE.md",
            Self::Skill => "SKILL.md",
            Self::Agent => "AGENT.md",
        }
    }
}

/// Outcome of one scaffold call — the path written, for the CLI to
/// echo back.
#[derive(Debug, Clone)]
pub struct ScaffoldReport {
    pub written: PathBuf,
    pub kind: NewKind,
    pub name: String,
}

/// Scaffold a new item under `root`. `root` is typically
/// `std::env::current_dir()`; it is also the consumer-project guard
/// boundary.
pub fn scaffold(root: &Path, kind: NewKind, name: &str) -> Result<ScaffoldReport> {
    if is_consumer_project(root) {
        return Err(anyhow!(
            "{}: refusing to scaffold — `.upskill-lock.json` indicates this is a consumer \
             project, not a source registry. Run `upskill new` inside the SSOT tree instead.",
            root.display()
        ));
    }
    validate_name(name)?;

    let item_dir = root.join(kind.dir()).join(name);
    let entry_path = item_dir.join(kind.entry());
    if item_dir.exists() {
        bail!(
            "{}: target directory already exists — refusing to overwrite",
            item_dir.display()
        );
    }

    std::fs::create_dir_all(&item_dir).with_context(|| format!("create {}", item_dir.display()))?;
    std::fs::write(&entry_path, render(kind, name))
        .with_context(|| format!("write {}", entry_path.display()))?;

    Ok(ScaffoldReport {
        written: entry_path,
        kind,
        name: name.to_string(),
    })
}

/// Format-spec §2.1: `[a-z0-9-]{1,64}`, no leading or trailing hyphen.
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("invalid name: empty");
    }
    if name.len() > 64 {
        bail!("invalid name `{name}`: must be ≤ 64 characters");
    }
    if name.starts_with('-') || name.ends_with('-') {
        bail!("invalid name `{name}`: must not start or end with `-`");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!("invalid name `{name}`: only lowercase letters, digits, and `-` are allowed");
    }
    Ok(())
}

/// Render the starter file body for `kind`.
///
/// The output is deliberately canonical — it parses into the model
/// without warnings and round-trips through `upskill fmt` as a no-op.
fn render(kind: NewKind, name: &str) -> String {
    match kind {
        NewKind::Rule => format!(
            "---\n\
             schema: 1\n\
             name: {name}\n\
             description: TODO describe what this rule enforces and when it applies.\n\
             ---\n\
             \n\
             ## Overview\n\
             \n\
             TODO write the rule body. Replace this scaffold before publishing.\n",
        ),
        NewKind::Skill => format!(
            "---\n\
             schema: 1\n\
             name: {name}\n\
             description: TODO describe when to use this skill — start with `Use when ...`.\n\
             ---\n\
             \n\
             ## Overview\n\
             \n\
             TODO write the skill body. Replace this scaffold before publishing.\n",
        ),
        NewKind::Agent => format!(
            "---\n\
             schema: 1\n\
             name: {name}\n\
             description: TODO describe when to invoke this agent — start with `Use when ...`.\n\
             mode: subagent\n\
             model: sonnet\n\
             ---\n\
             \n\
             ## Overview\n\
             \n\
             TODO write the agent system prompt. Replace this scaffold before publishing.\n",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_skill_writes_canonical_skill_md() {
        let tmp = tempfile::tempdir().unwrap();
        let report = scaffold(tmp.path(), NewKind::Skill, "code-review").unwrap();
        assert_eq!(
            report.written,
            tmp.path().join("skills/code-review/SKILL.md")
        );
        let content = std::fs::read_to_string(&report.written).unwrap();
        // Frontmatter parses against the model — guarantees the
        // scaffold is shape-correct for a brand-new author.
        let (skill, _body) =
            crate::parse::frontmatter::parse::<crate::model::Skill>(&content).unwrap();
        assert_eq!(skill.name, "code-review");
    }

    #[test]
    fn scaffold_agent_emits_subagent_sonnet_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let report = scaffold(tmp.path(), NewKind::Agent, "reviewer").unwrap();
        let content = std::fs::read_to_string(&report.written).unwrap();
        let (agent, _) = crate::parse::frontmatter::parse::<crate::model::Agent>(&content).unwrap();
        assert_eq!(agent.mode, Some(crate::model::Mode::Subagent));
        assert_eq!(agent.model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn scaffold_refuses_existing_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("skills/dup")).unwrap();
        let err = scaffold(tmp.path(), NewKind::Skill, "dup").expect_err("must refuse");
        assert!(format!("{err:#}").contains("already exists"));
    }

    #[test]
    fn scaffold_refuses_consumer_project() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".upskill-lock.json"),
            r#"{"schema":2,"items":[]}"#,
        )
        .unwrap();
        let err = scaffold(tmp.path(), NewKind::Skill, "x").expect_err("must refuse");
        assert!(format!("{err:#}").contains("consumer project"));
    }

    #[test]
    fn validate_name_accepts_canonical_forms() {
        for ok in ["a", "abc", "abc-123", "a1", "long-name-with-hyphens"] {
            assert!(validate_name(ok).is_ok(), "{ok} should be valid");
        }
    }

    #[test]
    fn validate_name_rejects_invalid_forms() {
        for bad in [
            "",
            "Bad-Case",
            "trailing-",
            "-leading",
            "snake_case",
            "with space",
            "dot.dot",
        ] {
            assert!(validate_name(bad).is_err(), "{bad} should be invalid");
        }
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let long = "a".repeat(65);
        assert!(validate_name(&long).is_err());
    }

    #[test]
    fn parse_kind_round_trips_strings() {
        assert_eq!(NewKind::parse("skill").unwrap(), NewKind::Skill);
        assert_eq!(NewKind::parse("rule").unwrap(), NewKind::Rule);
        assert_eq!(NewKind::parse("agent").unwrap(), NewKind::Agent);
        assert!(NewKind::parse("bundle").is_err());
    }
}
