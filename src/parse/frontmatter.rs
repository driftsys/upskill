//! YAML frontmatter parsing for portable-format markdown files.

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

/// Split a markdown file into its YAML frontmatter and body.
///
/// Returns `Some((yaml, body))` when the input begins with a `---` fence on
/// its own line and a closing `---` fence is found later, also on its own
/// line. Returns `None` otherwise.
///
/// Strips a leading UTF-8 BOM. Accepts both LF and CRLF line endings.
pub fn split(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let (first, rest) = split_first_line(text)?;
    if first.trim_end_matches('\r') != "---" {
        return None;
    }

    let mut consumed = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let yaml = &rest[..consumed];
            let body = &rest[consumed + line.len()..];
            return Some((yaml, body));
        }
        consumed += line.len();
    }
    None
}

fn split_first_line(s: &str) -> Option<(&str, &str)> {
    s.find('\n').map(|i| (&s[..i], &s[i + 1..]))
}

/// Parse YAML frontmatter into a typed value, returning the value and body.
///
/// Errors when frontmatter is missing or YAML is invalid (which includes
/// schema-version rejection per §8.1, surfaced via the `SchemaVersion`
/// custom deserializer).
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<(T, &str)> {
    let (yaml, body) =
        split(text).context("missing YAML frontmatter (file must begin with `---` fence)")?;
    let value: T = serde_yaml_ng::from_str(yaml).context("parsing YAML frontmatter")?;
    Ok((value, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Agent, License, Mode, Rule, Skill, ToolCap};

    #[test]
    fn split_basic() {
        let input = "---\nname: foo\n---\nbody text\n";
        let (yaml, body) = split(input).unwrap();
        assert_eq!(yaml, "name: foo\n");
        assert_eq!(body, "body text\n");
    }

    #[test]
    fn split_strips_bom() {
        let input = "\u{FEFF}---\nname: foo\n---\nbody\n";
        let (yaml, body) = split(input).unwrap();
        assert_eq!(yaml, "name: foo\n");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn split_empty_frontmatter() {
        let input = "---\n---\nbody\n";
        let (yaml, body) = split(input).unwrap();
        assert_eq!(yaml, "");
        assert_eq!(body, "body\n");
    }

    #[test]
    fn split_no_fence_returns_none() {
        assert!(split("# just markdown\n").is_none());
        assert!(split("not a fence\n---\n").is_none());
        // 4-dash line is not a fence
        assert!(split("----\nname: foo\n----\n").is_none());
    }

    #[test]
    fn parse_rejects_missing_frontmatter() {
        let result = parse::<Skill>("# just markdown\n");
        assert!(result.is_err(), "expected error for missing frontmatter");
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("missing YAML frontmatter"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn parse_rejects_schema_2() {
        let input = concat!(
            "---\n",
            "schema: 2\n",
            "name: too-new\n",
            "description: from a future version\n",
            "---\nbody\n"
        );
        let result = parse::<Skill>(input);
        assert!(result.is_err(), "expected error for schema 2");
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("schema version 2") && msg.contains("not supported"),
            "unexpected message: {msg}"
        );
    }

    #[test]
    fn skill_appendix_a3_round_trip() {
        // From docs/format-spec.md Appendix A.3.
        let input = concat!(
            "---\n",
            "schema: 1\n",
            "name: create-api-endpoint\n",
            "description: Use when scaffolding new REST API endpoints with Zod validation\n",
            "license: proprietary\n",
            "metadata:\n",
            "  version: \"1.0.0\"\n",
            "  author: platform-api\n",
            "---\n",
            "body\n",
        );
        let (skill, body) = parse::<Skill>(input).unwrap();
        assert_eq!(skill.schema.get(), 1);
        assert_eq!(skill.name, "create-api-endpoint");
        assert_eq!(skill.license, Some(License("proprietary".into())));
        assert_eq!(body, "body\n");
        let metadata = skill.metadata.as_ref().unwrap();
        assert_eq!(metadata.version.as_deref(), Some("1.0.0"));
        round_trip(&skill);
    }

    #[test]
    fn rule_appendix_a2_round_trip() {
        // From docs/format-spec.md Appendix A.2.
        let input = concat!(
            "---\n",
            "schema: 1\n",
            "name: api-conventions\n",
            "description: Use when writing or modifying API handler files\n",
            "license: proprietary\n",
            "scope:\n",
            "  paths:\n",
            "    - \"src/api/**/*.ts\"\n",
            "    - \"src/handlers/**/*.ts\"\n",
            "copilot:\n",
            "  excludeAgent: code-review\n",
            "metadata:\n",
            "  version: \"2.1.0\"\n",
            "  author: platform-api\n",
            "---\n",
            "body\n",
        );
        let (rule, _body) = parse::<Rule>(input).unwrap();
        assert_eq!(rule.name, "api-conventions");
        let scope = rule.scope.as_ref().unwrap();
        assert_eq!(scope.paths, vec!["src/api/**/*.ts", "src/handlers/**/*.ts"]);
        assert!(rule.copilot.is_some(), "copilot passthrough must be parsed");
        round_trip(&rule);
    }

    #[test]
    fn agent_appendix_a4_round_trip() {
        // From docs/format-spec.md Appendix A.4.
        let input = concat!(
            "---\n",
            "schema: 1\n",
            "name: security-reviewer\n",
            "description: Use when reviewing code for injection flaws\n",
            "license: proprietary\n",
            "mode: subagent\n",
            "model: sonnet\n",
            "tools:\n",
            "  - read\n",
            "  - grep\n",
            "  - glob\n",
            "  - bash\n",
            "preload-skills:\n",
            "  - security-baseline\n",
            "opencode:\n",
            "  temperature: 0.2\n",
            "metadata:\n",
            "  version: \"1.0.0\"\n",
            "  author: platform-security\n",
            "  audience:\n",
            "    - claude\n",
            "    - copilot\n",
            "    - opencode\n",
            "---\n",
            "body\n",
        );
        let (agent, _body) = parse::<Agent>(input).unwrap();
        assert_eq!(agent.name, "security-reviewer");
        assert_eq!(agent.mode, Some(Mode::Subagent));
        assert_eq!(agent.model.as_deref(), Some("sonnet"));
        assert_eq!(
            agent.tools,
            vec![ToolCap::Read, ToolCap::Grep, ToolCap::Glob, ToolCap::Bash]
        );
        assert_eq!(agent.preload_skills, vec!["security-baseline".to_string()]);
        assert!(
            agent.opencode.is_some(),
            "opencode passthrough must be parsed"
        );
        round_trip(&agent);
    }

    #[test]
    fn metadata_unknown_keys_preserved() {
        let input = concat!(
            "---\n",
            "schema: 1\n",
            "name: foo\n",
            "description: test unknown keys\n",
            "metadata:\n",
            "  version: \"1.0.0\"\n",
            "  author: bar\n",
            "  custom-key: custom-value\n",
            "  team:\n",
            "    name: alpha\n",
            "    lead: jdoe\n",
            "---\n",
        );
        let (skill, _) = parse::<Skill>(input).unwrap();
        let metadata = skill.metadata.as_ref().unwrap();
        assert_eq!(metadata.version.as_deref(), Some("1.0.0"));
        assert!(metadata.extra.contains_key("custom-key"));
        assert!(metadata.extra.contains_key("team"));
        round_trip(&skill);
    }

    fn round_trip<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let yaml = serde_yaml_ng::to_string(value).expect("serialize");
        let back: T = serde_yaml_ng::from_str(&yaml).expect("deserialize");
        assert_eq!(value, &back, "round-trip mismatch");
    }
}
