//! Hook files, static context files and command scripts for plugin output.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::{PackClient, PackFile};
use crate::generate::{Client, directives};
use crate::model::{Bundle, HookEvent, HookSettings, Matcher};
use crate::parse::hook::{HookBody, check_hook, split_hook_body};
use crate::pipeline::{BundleHook, HookSource, read_hook};

pub(super) struct PackHook {
    pub name: String,
    pub source: HookSource,
    pub body: HookBody,
}

/// Refuse a hook named by two bundles of the closure.
pub(super) fn check_hook_names(bundles: &[Bundle]) -> Result<()> {
    let mut names = BTreeMap::new();
    for bundle in bundles {
        for name in &bundle.items.hooks {
            if let Some(first) = names.insert(name, &bundle.name) {
                if first != &bundle.name {
                    bail!(
                        "hook `{name}` is named by both bundles `{first}` and `{}`",
                        bundle.name
                    );
                }
            }
        }
    }
    Ok(())
}

/// Read and validate every hook, with the declaration attached to each error.
/// A name listed twice in one bundle counts once; `check_hook_names` already
/// refused a name that two bundles list.
pub(super) fn read_pack_hooks(hooks: &[BundleHook]) -> Result<Vec<PackHook>> {
    let mut seen = BTreeSet::new();
    hooks
        .iter()
        .filter(|declared| seen.insert(declared.name.as_str()))
        .map(|declared| {
            let read = || {
                let source = read_hook(&declared.root, &declared.name)?;
                let problems: Vec<_> = check_hook(&source.hook, &source.body, &source.dir)
                    .into_iter()
                    .map(|problem| format!("{}: {}", source.path.display(), problem.message))
                    .collect();
                if !problems.is_empty() {
                    bail!("{}", problems.join("\n"));
                }
                // `check_hook` leaves errors of the whole body to lint. Check
                // them here before any client filtering skips an event.
                if let Err(error) = directives::process(&source.body, Client::Claude) {
                    bail!("{}: {error:#}", source.path.display());
                }
                let events: Vec<_> = source.hook.events.keys().map(String::as_str).collect();
                let body = split_hook_body(&source.body, &events);
                Ok(PackHook {
                    name: declared.name.clone(),
                    source,
                    body,
                })
            };
            read().with_context(|| {
                format!("hook `{}` of bundle `{}`", declared.name, declared.bundle)
            })
        })
        .collect()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaudeHookFile {
    hooks: BTreeMap<String, Vec<ClaudeGroup>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaudeGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    matcher: Option<String>,
    hooks: Vec<ClaudeCommand>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaudeCommand {
    r#type: String,
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CopilotHookFile {
    version: u32,
    hooks: BTreeMap<String, Vec<CopilotCommand>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CopilotCommand {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    matcher: Option<String>,
    bash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    powershell: Option<String>,
    #[serde(rename = "timeoutSec", skip_serializing_if = "Option::is_none")]
    timeout_sec: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeContext<'a> {
    hook_specific_output: ClaudeContextOutput<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeContextOutput<'a> {
    hook_event_name: &'a str,
    additional_context: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CopilotContext<'a> {
    additional_context: &'a str,
}

struct ContextGroup {
    event: HookEvent,
    matcher: Option<String>,
    texts: Vec<String>,
}

/// Write context files, command resources and a hook file when entries exist.
pub(super) fn hook_files(
    hooks: &[PackHook],
    notice: bool,
    client: PackClient,
) -> Result<Vec<PackFile>> {
    let generator = match client {
        PackClient::Claude => Client::Claude,
        PackClient::Copilot => Client::Copilot,
    };
    let mut groups: Vec<ContextGroup> = Vec::new();
    for hook in hooks {
        for (key, settings) in &hook.source.hook.events {
            let event = HookEvent::parse(key)
                .with_context(|| format!("unknown event `{key}` of hook `{}`", hook.name))?;
            let settings = settings.clone().unwrap_or_default();
            if settings.run.is_some() {
                continue;
            }
            let Some(matcher) = resolve_matcher(&settings, event, true, client) else {
                continue;
            };
            let text = directives::process(&hook.body.text(key), generator)
                .with_context(|| format!("hook `{}`, event `{key}`", hook.name))?
                .trim()
                .to_string();
            if text.is_empty() {
                continue;
            }
            if let Some(group) = groups
                .iter_mut()
                .find(|group| group.event == event && group.matcher == matcher)
            {
                group.texts.push(text);
            } else {
                groups.push(ContextGroup {
                    event,
                    matcher,
                    texts: vec![text],
                });
            }
        }
    }

    let mut claude: ClaudeHookFile = if notice && client == PackClient::Claude {
        serde_json::from_str(include_str!(
            "../../plugins/upskill-claude/hooks/hooks.json"
        ))?
    } else {
        ClaudeHookFile {
            hooks: BTreeMap::new(),
        }
    };
    let mut copilot: CopilotHookFile = if notice && client == PackClient::Copilot {
        serde_json::from_str(include_str!("../../plugins/upskill-copilot/hooks.json"))?
    } else {
        CopilotHookFile {
            version: 1,
            hooks: BTreeMap::new(),
        }
    };
    let mut files = Vec::new();
    let mut counts = BTreeMap::new();
    for group in groups {
        let count = counts.entry(group.event).or_insert(0);
        *count += 1;
        let filename = if *count == 1 {
            format!("{}.json", group.event.name())
        } else {
            format!("{}-{count}.json", group.event.name())
        };
        let path = format!("hooks/context/{filename}");
        let text = group.texts.join("\n\n");
        let bytes = match client {
            PackClient::Claude => {
                claude
                    .hooks
                    .entry(group.event.claude_name().to_string())
                    .or_default()
                    .push(ClaudeGroup {
                        matcher: group.matcher,
                        hooks: vec![ClaudeCommand {
                            r#type: "command".into(),
                            command: format!("cat \"${{CLAUDE_PLUGIN_ROOT}}/{path}\""),
                            timeout: None,
                        }],
                    });
                json_bytes(&ClaudeContext {
                    hook_specific_output: ClaudeContextOutput {
                        hook_event_name: group.event.claude_name(),
                        additional_context: &text,
                    },
                })?
            }
            PackClient::Copilot => {
                copilot
                    .hooks
                    .entry(group.event.copilot_name().to_string())
                    .or_default()
                    .push(CopilotCommand {
                        r#type: "command".into(),
                        matcher: group.matcher,
                        bash: format!("cat \"${{COPILOT_PLUGIN_ROOT}}/{path}\""),
                        powershell: Some(format!(
                            "Get-Content -Raw -Encoding UTF8 -LiteralPath \"$env:COPILOT_PLUGIN_ROOT/{path}\""
                        )),
                        timeout_sec: None,
                    });
                json_bytes(&CopilotContext {
                    additional_context: &text,
                })?
            }
        };
        files.push(PackFile {
            rel: path.into(),
            bytes: ascii_escape(&bytes),
            mode: None,
        });
    }
    for hook in hooks {
        let mut copied = false;
        for (key, settings) in &hook.source.hook.events {
            let settings = settings.clone().unwrap_or_default();
            let Some(run) = &settings.run else {
                continue;
            };
            let event = HookEvent::parse(key)
                .with_context(|| format!("unknown event `{key}` of hook `{}`", hook.name))?;
            let Some(matcher) = resolve_matcher(&settings, event, false, client) else {
                continue;
            };
            match command_entry(&hook.name, run, matcher, settings.timeout, client) {
                CommandEntry::Claude(entry) => claude
                    .hooks
                    .entry(event.claude_name().to_string())
                    .or_default()
                    .push(entry),
                CommandEntry::Copilot(entry) => copilot
                    .hooks
                    .entry(event.copilot_name().to_string())
                    .or_default()
                    .push(entry),
            }
            copied = true;
        }
        if copied {
            for resource in &hook.source.resources {
                files.push(PackFile {
                    rel: std::path::Path::new("hooks")
                        .join(&hook.name)
                        .join(&resource.rel),
                    bytes: resource.bytes.clone(),
                    mode: resource.mode,
                });
            }
        }
    }
    let hook_file = match client {
        PackClient::Claude if !claude.hooks.is_empty() => {
            Some(("hooks/hooks.json", json_bytes(&claude)?))
        }
        PackClient::Copilot if !copilot.hooks.is_empty() => {
            Some(("hooks.json", json_bytes(&copilot)?))
        }
        _ => None,
    };
    if let Some((path, bytes)) = hook_file {
        files.push(PackFile {
            rel: path.into(),
            bytes,
            mode: None,
        });
    }
    Ok(files)
}

enum CommandEntry {
    Claude(ClaudeGroup),
    Copilot(CopilotCommand),
}

fn command_entry(
    name: &str,
    run: &str,
    matcher: Option<String>,
    timeout: Option<u32>,
    client: PackClient,
) -> CommandEntry {
    match client {
        PackClient::Claude => CommandEntry::Claude(ClaudeGroup {
            matcher,
            hooks: vec![ClaudeCommand {
                r#type: "command".into(),
                command: format!(
                    "UPSKILL_CLIENT=claude \"${{CLAUDE_PLUGIN_ROOT}}/hooks/{name}/{run}\""
                ),
                timeout,
            }],
        }),
        PackClient::Copilot => CommandEntry::Copilot(CopilotCommand {
            r#type: "command".into(),
            matcher,
            bash: format!("UPSKILL_CLIENT=copilot \"${{COPILOT_PLUGIN_ROOT}}/hooks/{name}/{run}\""),
            powershell: None,
            timeout_sec: timeout,
        }),
    }
}

/// None skips this client; Some(None) writes an entry without a matcher.
fn resolve_matcher(
    settings: &HookSettings,
    event: HookEvent,
    context: bool,
    client: PackClient,
) -> Option<Option<String>> {
    match &settings.matcher {
        None if context && event == HookEvent::SessionStart && client == PackClient::Claude => {
            Some(Some("startup|clear|compact".into()))
        }
        None => Some(None),
        Some(Matcher::All(matcher)) => Some(Some(matcher.clone())),
        Some(Matcher::PerClient(matcher)) => match client {
            PackClient::Claude => matcher.claude.clone().map(Some),
            PackClient::Copilot => matcher.copilot.clone().map(Some),
        },
    }
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Escape Unicode in serialized JSON using lowercase UTF-16 code units.
fn ascii_escape(json: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(json.len());
    for character in String::from_utf8_lossy(json).chars() {
        if character.is_ascii() {
            bytes.push(character as u8);
        } else {
            let mut units = [0; 2];
            for &mut unit in character.encode_utf16(&mut units) {
                bytes.extend_from_slice(br"\u");
                for shift in [12, 8, 4, 0] {
                    bytes.push(b"0123456789abcdef"[((unit >> shift) & 0xf) as usize]);
                }
            }
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ClientMatcher;
    use crate::parse::frontmatter;
    use std::path::PathBuf;

    #[test]
    fn notice_hook_files_round_trip_byte_for_byte() {
        let claude = include_str!("../../plugins/upskill-claude/hooks/hooks.json");
        let copilot = include_str!("../../plugins/upskill-copilot/hooks.json");
        let claude_file: ClaudeHookFile = serde_json::from_str(claude).unwrap();
        let copilot_file: CopilotHookFile = serde_json::from_str(copilot).unwrap();
        for (actual, expected) in [
            (json_bytes(&claude_file).unwrap(), claude),
            (json_bytes(&copilot_file).unwrap(), copilot),
        ] {
            assert_eq!(actual, expected.as_bytes());
        }
        for (client, expected) in [(PackClient::Claude, claude), (PackClient::Copilot, copilot)] {
            let files = hook_files(&[], true, client).unwrap();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].bytes, expected.as_bytes());
        }
    }

    #[test]
    fn resolve_matcher_follows_the_design() {
        for client in [PackClient::Claude, PackClient::Copilot] {
            let settings = HookSettings::default();
            let default = match client {
                PackClient::Claude => Some(Some("startup|clear|compact".into())),
                PackClient::Copilot => Some(None),
            };
            assert_eq!(
                resolve_matcher(&settings, HookEvent::SessionStart, true, client),
                default
            );
            assert_eq!(
                resolve_matcher(&settings, HookEvent::PostTool, true, client),
                Some(None)
            );
            assert_eq!(
                resolve_matcher(&settings, HookEvent::SessionStart, false, client),
                Some(None)
            );
            let settings = HookSettings {
                matcher: Some(Matcher::All("edit".into())),
                ..Default::default()
            };
            assert_eq!(
                resolve_matcher(&settings, HookEvent::PostTool, true, client),
                Some(Some("edit".into()))
            );
            for (claude, copilot) in [
                (Some("Edit"), Some("edit")),
                (Some("Edit"), None),
                (None, Some("edit")),
            ] {
                let settings = HookSettings {
                    matcher: Some(Matcher::PerClient(ClientMatcher {
                        claude: claude.map(str::to_string),
                        copilot: copilot.map(str::to_string),
                    })),
                    ..Default::default()
                };
                let expected = match client {
                    PackClient::Claude => claude,
                    PackClient::Copilot => copilot,
                };
                assert_eq!(
                    resolve_matcher(&settings, HookEvent::PostTool, true, client),
                    expected.map(|text| Some(text.to_string()))
                );
            }
        }
    }

    #[test]
    fn hook_files_is_empty_without_notice_and_without_entries() {
        for client in [PackClient::Claude, PackClient::Copilot] {
            assert!(hook_files(&[], false, client).unwrap().is_empty());
        }
    }

    #[test]
    fn context_file_is_ascii_only() {
        let raw = "---\ndescription: Test hook\nevents:\n  session-start:\n---\n\ncafé — ✓\n";
        let (hook, text) = frontmatter::parse(raw).unwrap();
        let body = split_hook_body(text, &["session-start"]);
        let hooks = [PackHook {
            name: "test".into(),
            source: HookSource {
                hook,
                path: PathBuf::from("test/HOOK.md"),
                dir: PathBuf::from("test"),
                body: text.into(),
                resources: Vec::new(),
            },
            body,
        }];
        for client in [PackClient::Claude, PackClient::Copilot] {
            let files = hook_files(&hooks, false, client).unwrap();
            let bytes = &files
                .iter()
                .find(|file| file.rel == std::path::Path::new("hooks/context/session-start.json"))
                .unwrap()
                .bytes;
            assert!(bytes.is_ascii());
            assert!(String::from_utf8_lossy(bytes).contains(r"caf\u00e9 \u2014 \u2713"));
            let json: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            let output = match client {
                PackClient::Claude => &json["hookSpecificOutput"],
                PackClient::Copilot => &json,
            };
            assert_eq!(output["additionalContext"], "café — ✓");
        }
    }

    #[test]
    fn ascii_escape_writes_a_surrogate_pair_above_u_ffff() {
        let bytes = ascii_escape(&serde_json::to_vec("😀").unwrap());
        assert_eq!(bytes, br#""\ud83d\ude00""#);
        assert_eq!(serde_json::from_slice::<String>(&bytes).unwrap(), "😀");
    }
}
