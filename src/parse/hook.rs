//! Hook body sections and validation shared by author commands.

use crate::generate::{Client, directives};
use crate::lint::ENTRYPOINTS;
use crate::model::hook::is_run_file_name;
use crate::model::{Hook, HookEvent, Matcher};
use crate::pipeline::filter_ignored;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct HookSection {
    pub event: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug)]
pub struct HookBody {
    /// Text before the first event heading, or the whole unsectioned body.
    pub preamble: String,
    pub sections: Vec<HookSection>,
    /// Other level-2 headings, collected only when event sections exist.
    pub stray_headings: Vec<(usize, String)>,
}

impl HookBody {
    /// Join the trimmed preamble and first section of this event.
    pub fn text(&self, event: &str) -> String {
        let section = self
            .sections
            .iter()
            .find(|section| section.event == event)
            .map_or("", |section| section.text.trim());
        [self.preamble.trim(), section]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

pub fn split_hook_body(body: &str, events: &[&str]) -> HookBody {
    let mut headings = Vec::new();
    let mut parser = Parser::new(body).into_offset_iter();
    while let Some((event, range)) = parser.next() {
        if !matches!(
            event,
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            })
        ) {
            continue;
        }
        let mut text = String::new();
        let mut end = range.end;
        for (event, range) in parser.by_ref() {
            match event {
                Event::Text(part) | Event::Code(part) => text.push_str(&part),
                Event::End(TagEnd::Heading(_)) => {
                    end = range.end;
                    break;
                }
                _ => {}
            }
        }
        let line = body[..range.start].bytes().filter(|&c| c == b'\n').count() + 1;
        headings.push((range.start, end, line, text.trim().to_string()));
    }
    let event_headings: Vec<_> = headings
        .iter()
        .filter(|(_, _, _, text)| events.contains(&text.as_str()))
        .collect();
    let preamble_end = event_headings
        .first()
        .map_or(body.len(), |heading| heading.0);
    let sections = event_headings
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            let end = event_headings
                .get(index + 1)
                .map_or(body.len(), |next| next.0);
            HookSection {
                event: heading.3.clone(),
                line: heading.2,
                text: body[heading.1..end].trim().to_string(),
            }
        })
        .collect();
    let stray_headings = if event_headings.is_empty() {
        Vec::new()
    } else {
        headings
            .iter()
            .filter(|(_, _, _, text)| !events.contains(&text.as_str()))
            .map(|(_, _, line, text)| (*line, text.clone()))
            .collect()
    };
    HookBody {
        preamble: body[..preamble_end].trim().to_string(),
        sections,
        stray_headings,
    }
}

/// Message for the hook name `context`, shared by lint and `upskill new hook`.
pub const RESERVED_CONTEXT_NAME_MESSAGE: &str =
    "the hook name `context` is reserved: `pack` writes the context files to hooks/context/";

#[derive(Debug)]
pub struct HookProblem {
    pub rule_id: &'static str,
    pub line: Option<usize>,
    pub message: String,
}

/// Check a parsed hook using its item directory for names and script files.
pub fn check_hook(hook: &Hook, body: &str, dir: &Path) -> Vec<HookProblem> {
    let mut problems = Vec::new();
    let mut push = |rule_id, line, message| {
        problems.push(HookProblem {
            rule_id,
            line,
            message,
        })
    };
    if hook.events.is_empty() {
        push(
            "hook-event",
            None,
            "a hook needs at least one event in `events`".into(),
        );
    }
    for (key, settings) in &hook.events {
        let Some(event) = HookEvent::parse(key) else {
            push(
                "hook-event",
                None,
                format!(
                    "unknown event `{key}`; expected one of: session-start, session-end, prompt-submit, pre-tool, post-tool, permission-request, stop, subagent-start, subagent-stop, pre-compact"
                ),
            );
            continue;
        };
        let settings = settings.clone().unwrap_or_default();
        if settings.run.is_none() && !event.accepts_context() {
            push(
                "hook-event",
                None,
                format!(
                    "event `{key}` has no `run`, and a context hook is valid only on session-start, prompt-submit, post-tool and subagent-start"
                ),
            );
        }
        if settings.run.is_none() && settings.timeout.is_some() {
            push(
                "hook-event",
                None,
                format!(
                    "event `{key}` sets `timeout` without `run`: only a command hook has a timeout"
                ),
            );
        }
        if let Some(Matcher::PerClient(matcher)) = &settings.matcher {
            if matcher.claude.is_none() && matcher.copilot.is_none() {
                push(
                    "hook-event",
                    None,
                    format!(
                        "event `{key}` has a `matcher` map with neither `claude` nor `copilot`"
                    ),
                );
            }
        }
        if settings.run.is_none()
            && settings.matcher.is_some()
            && matches!(event, HookEvent::PromptSubmit | HookEvent::Stop)
        {
            push(
                "hook-event",
                None,
                format!(
                    "a context hook on `{key}` cannot have a `matcher`: Claude Code ignores a matcher on this event, so the texts of several hooks would have no defined order"
                ),
            );
        }
    }
    let name = hook
        .name
        .as_deref()
        .unwrap_or_else(|| dir.file_name().and_then(|name| name.to_str()).unwrap_or(""));
    if !is_valid_hook_name(name) {
        push(
            "hook-name",
            None,
            format!(
                "hook name `{name}` must consist of lowercase letters, digits and hyphens, without a hyphen at the start or end, and with at most 64 characters (format-spec §2.1)"
            ),
        );
    }
    if name == "context" {
        push("hook-name", None, RESERVED_CONTEXT_NAME_MESSAGE.into());
    }
    let events: Vec<_> = hook.events.keys().map(String::as_str).collect();
    let sections = split_hook_body(body, &events);
    for (key, settings) in &hook.events {
        if HookEvent::parse(key).is_some()
            && settings.as_ref().and_then(|s| s.run.as_ref()).is_none()
            && sections.text(key).is_empty()
        {
            push(
                "hook-text",
                None,
                format!(
                    "context hook `{key}` has no text: write a `## {key}` section or text before the first event heading"
                ),
            );
        }
    }
    for (line, text) in &sections.stray_headings {
        push(
            "hook-heading",
            Some(*line),
            format!(
                "level-2 heading `{text}` is not an event of this hook; use a level-3 heading for content"
            ),
        );
    }
    let mut seen = BTreeSet::new();
    for section in &sections.sections {
        if HookEvent::parse(&section.event).is_none() {
            continue;
        }
        if !seen.insert(&section.event) {
            push(
                "hook-heading",
                Some(section.line),
                format!("event heading `{}` appears more than once", section.event),
            );
        }
    }
    for (key, settings) in &hook.events {
        if HookEvent::parse(key).is_none() {
            continue;
        }
        if let Some(run) = settings.as_ref().and_then(|s| s.run.as_deref()) {
            if !is_run_file_name(run) {
                push(
                    "hook-run",
                    None,
                    format!(
                        "`run: {run}` must be the name of one file in the item directory, without a path or arguments"
                    ),
                );
            } else if ENTRYPOINTS.contains(&run) {
                push(
                    "hook-run",
                    None,
                    format!(
                        "event `{key}`: run file `{run}` is not copied by pack: it is an entry file"
                    ),
                );
            } else if filter_ignored(vec![PathBuf::from(run)], &hook.ignore).is_empty() {
                push(
                    "hook-run",
                    None,
                    format!(
                        "event `{key}`: run file `{run}` is not copied by pack: it matches `ignore`"
                    ),
                );
            } else if std::fs::symlink_metadata(dir.join(run))
                .is_ok_and(|meta| meta.file_type().is_symlink())
            {
                push(
                    "hook-run",
                    None,
                    format!(
                        "event `{key}`: run file `{run}` is a symbolic link; pack does not copy it"
                    ),
                );
            } else if !dir.join(run).is_file() {
                push(
                    "hook-run",
                    None,
                    format!("event `{key}`: `run: {run}` names no file in the item directory"),
                );
            } else if !is_executable(&dir.join(run)) {
                push(
                    "hook-run",
                    None,
                    format!("event `{key}`: run file `{run}` is not executable"),
                );
            }
        }
    }
    // The normal body rule reports errors that already affect the whole body.
    if directives::process(body, Client::Claude).is_ok() {
        for key in hook
            .events
            .keys()
            .filter(|key| HookEvent::parse(key).is_some())
        {
            if let Err(error) = directives::process(&sections.text(key), Client::Claude) {
                push(
                    "directive",
                    None,
                    format!(
                        "event `{key}`: {error:#} (line numbers count from the first line of the text of this event, not of the file)"
                    ),
                );
            }
        }
    }
    problems
}

/// Whether the file has an executable bit. Always true off unix, where the
/// file mode has no such bit.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// The effective-name rule of format-spec §2.1.
pub fn is_valid_hook_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    const EVENTS: &[&str] = &["session-start", "prompt-submit"];

    /// Write a script file with the executable bit that `check_hook` requires.
    fn write_script(path: &Path) {
        std::fs::write(path, "exit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn split_hook_body_gives_the_whole_body_to_every_event_without_event_headings() {
        let body = split_hook_body("Text.\n\n## Overview\n\nMore.\n", EVENTS);
        assert!(body.sections.is_empty());
        assert!(body.stray_headings.is_empty());
        assert_eq!(body.text("prompt-submit"), "Text.\n\n## Overview\n\nMore.");
    }

    #[test]
    fn split_hook_body_prepends_the_preamble_to_each_event() {
        let body = split_hook_body(
            "All.\n\n## session-start\n\nStart.\n\n## prompt-submit\n\nPrompt.\n",
            EVENTS,
        );
        assert_eq!(body.preamble, "All.");
        assert_eq!(body.text("session-start"), "All.\n\nStart.");
        assert_eq!(body.text("prompt-submit"), "All.\n\nPrompt.");
        assert_eq!(body.sections[1].line, 7);
    }

    #[test]
    fn split_hook_body_keeps_level_3_headings_as_content() {
        let body = split_hook_body("## session-start\n\n### Details\n\nMore.\n", EVENTS);
        assert_eq!(body.text("session-start"), "### Details\n\nMore.");
    }

    #[test]
    fn split_hook_body_ignores_a_heading_inside_a_code_fence() {
        let body = split_hook_body(
            "## session-start\n\n```text\n## prompt-submit\n```\n",
            EVENTS,
        );
        assert_eq!(body.sections.len(), 1);
        assert_eq!(body.text("session-start"), "```text\n## prompt-submit\n```");
    }

    #[test]
    fn split_hook_body_reports_a_level_2_heading_that_is_not_an_event() {
        let body = split_hook_body("## session-start\n\nA.\n\n## Details\n\nB.\n", EVENTS);
        assert_eq!(body.stray_headings, [(5, "Details".into())]);
        assert!(body.text("session-start").contains("B."));
    }

    #[test]
    fn split_hook_body_counts_a_setext_heading() {
        let body = split_hook_body("session-start\n---\n\nA.\n", EVENTS);
        assert_eq!(body.sections.len(), 1);
        assert_eq!(body.sections[0].event, "session-start");
        assert_eq!(body.sections[0].line, 1);
        assert_eq!(body.text("session-start"), "A.");
    }

    #[test]
    fn check_hook_reports_each_rule() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("guard");
        std::fs::create_dir(&dir).unwrap();
        write_script(&dir.join("check.sh"));
        let cases = [
            (
                "{}",
                "Text.",
                "hook-event",
                None,
                "a hook needs at least one event in `events`",
            ),
            (
                "{ on-save: { run: '../x', timeout: 5 } }",
                "## on-save\n\nA.\n\n## on-save\n\nB.\n",
                "hook-event",
                None,
                "unknown event `on-save`; expected one of: session-start, session-end, prompt-submit, pre-tool, post-tool, permission-request, stop, subagent-start, subagent-stop, pre-compact",
            ),
            (
                "{ stop: }",
                "Text.",
                "hook-event",
                None,
                "event `stop` has no `run`, and a context hook is valid only on session-start, prompt-submit, post-tool and subagent-start",
            ),
            (
                "{ session-start: { timeout: 5 } }",
                "Text.",
                "hook-event",
                None,
                "event `session-start` sets `timeout` without `run`: only a command hook has a timeout",
            ),
            (
                "{ session-start: { matcher: {} } }",
                "Text.",
                "hook-event",
                None,
                "event `session-start` has a `matcher` map with neither `claude` nor `copilot`",
            ),
            (
                "{ prompt-submit: { matcher: edit } }",
                "Text.",
                "hook-event",
                None,
                "a context hook on `prompt-submit` cannot have a `matcher`: Claude Code ignores a matcher on this event, so the texts of several hooks would have no defined order",
            ),
            (
                "{ stop: { matcher: edit } }",
                "Text.",
                "hook-event",
                None,
                "a context hook on `stop` cannot have a `matcher`: Claude Code ignores a matcher on this event, so the texts of several hooks would have no defined order",
            ),
            (
                "{ session-start: }",
                "",
                "hook-text",
                None,
                "context hook `session-start` has no text: write a `## session-start` section or text before the first event heading",
            ),
            (
                "{ session-start: }",
                "## session-start\n\nA.\n\n## Details\n\nB.\n",
                "hook-heading",
                Some(5),
                "level-2 heading `Details` is not an event of this hook; use a level-3 heading for content",
            ),
            (
                "{ session-start: }",
                "## session-start\n\nA.\n\n## session-start\n\nB.\n",
                "hook-heading",
                Some(5),
                "event heading `session-start` appears more than once",
            ),
            (
                "{ post-tool: { run: ../check.sh } }",
                "",
                "hook-run",
                None,
                "`run: ../check.sh` must be the name of one file in the item directory, without a path or arguments",
            ),
            (
                "{ post-tool: { run: missing.sh } }",
                "",
                "hook-run",
                None,
                "event `post-tool`: `run: missing.sh` names no file in the item directory",
            ),
            (
                "{ session-start: , prompt-submit: }",
                "## session-start\n\n<!-- @client:claude -->\nA.\n\n## prompt-submit\n\nB.\n<!-- @endclient -->\n",
                "directive",
                None,
                "event `session-start`: unclosed `@client:` directive opened at line 1 (line numbers count from the first line of the text of this event, not of the file)",
            ),
        ];
        for (events, body, rule_id, line, message) in cases {
            let hook: Hook =
                serde_yaml_ng::from_str(&format!("description: d\nevents: {events}\n")).unwrap();
            let problems = check_hook(&hook, body, &dir);
            assert!(
                problems
                    .iter()
                    .any(|p| p.rule_id == rule_id && p.line == line && p.message == message),
                "{events}: {problems:?}"
            );
            if events.contains("on-save") {
                assert_eq!(problems.len(), 1);
            }
        }
        for (name, message) in [
            (
                "Upper",
                "hook name `Upper` must consist of lowercase letters, digits and hyphens, without a hyphen at the start or end, and with at most 64 characters (format-spec §2.1)",
            ),
            ("context", RESERVED_CONTEXT_NAME_MESSAGE),
        ] {
            let hook: Hook = serde_yaml_ng::from_str(&format!(
                "name: {name}\ndescription: d\nevents: {{ session-start: }}\n"
            ))
            .unwrap();
            let problems = check_hook(&hook, "Text.", &dir);
            assert_eq!(problems.len(), 1);
            assert_eq!(problems[0].rule_id, "hook-name");
            assert_eq!(problems[0].message, message);
        }
    }

    #[test]
    fn check_hook_refuses_a_run_file_that_pack_does_not_copy() {
        let tmp = tempfile::tempdir().unwrap();
        for file in ["HOOK.md", "skip.sh"] {
            std::fs::write(tmp.path().join(file), "x").unwrap();
        }
        for (run, ignore, message) in [
            (
                "HOOK.md",
                "[]",
                "event `session-start`: run file `HOOK.md` is not copied by pack: it is an entry file",
            ),
            (
                "skip.sh",
                "[\"*.sh\"]",
                "event `session-start`: run file `skip.sh` is not copied by pack: it matches `ignore`",
            ),
        ] {
            let hook: Hook = serde_yaml_ng::from_str(&format!(
                "name: guard\ndescription: d\nevents: {{ session-start: {{ run: {run} }} }}\nignore: {ignore}\n"
            ))
            .unwrap();
            let problems = check_hook(&hook, "", tmp.path());
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].rule_id, "hook-run");
            assert_eq!(problems[0].message, message);
        }
    }

    #[test]
    fn check_hook_accepts_a_valid_hook() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("guard");
        std::fs::create_dir(&dir).unwrap();
        write_script(&dir.join("check.sh"));
        let hook: Hook = serde_yaml_ng::from_str("description: d\nevents: { session-start: , post-tool: { matcher: { claude: Edit }, run: check.sh, timeout: 5 } }\n").unwrap();
        assert!(check_hook(&hook, "Text.", &dir).is_empty());
    }

    #[test]
    fn check_hook_accepts_a_matcher_on_a_command_hook_of_prompt_submit_and_stop() {
        let tmp = tempfile::tempdir().unwrap();
        write_script(&tmp.path().join("check.sh"));
        for event in ["prompt-submit", "stop"] {
            let hook: Hook = serde_yaml_ng::from_str(&format!(
                "name: guard\ndescription: d\nevents: {{ {event}: {{ matcher: edit, run: check.sh }} }}\n"
            ))
            .unwrap();
            assert!(check_hook(&hook, "", tmp.path()).is_empty(), "{event}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn check_hook_reports_a_run_file_without_an_executable_bit() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("check.sh");
        write_script(&script);
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o644)).unwrap();
        let hook: Hook = serde_yaml_ng::from_str(
            "name: guard\ndescription: d\nevents: { post-tool: { run: check.sh } }\n",
        )
        .unwrap();
        let problems = check_hook(&hook, "", tmp.path());
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].rule_id, "hook-run");
        assert_eq!(
            problems[0].message,
            "event `post-tool`: run file `check.sh` is not executable"
        );
    }

    #[test]
    fn check_hook_does_not_repeat_a_body_directive_error_for_each_event() {
        let tmp = tempfile::tempdir().unwrap();
        let hook: Hook = serde_yaml_ng::from_str(
            "name: guard\ndescription: d\nevents: { session-start: , prompt-submit: }\n",
        )
        .unwrap();
        let problems = check_hook(&hook, "<!-- @client:claude -->\nText.\n", tmp.path());
        assert!(
            problems.iter().all(|p| p.rule_id != "directive"),
            "{problems:?}"
        );
    }

    #[test]
    fn is_valid_hook_name_follows_the_name_rule() {
        for name in ["a", "edit-checks", "a1", &"a".repeat(64)] {
            assert!(is_valid_hook_name(name));
        }
        for name in ["", "-a", "a-", "Upper", "a_b", "a.b", &"a".repeat(65)] {
            assert!(!is_valid_hook_name(name));
        }
    }
}
