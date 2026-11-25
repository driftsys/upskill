//! Hook items (§3.10).

use crate::model::common::{License, Metadata, SchemaVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    PromptSubmit,
    PreTool,
    PostTool,
    PermissionRequest,
    Stop,
    SubagentStart,
    SubagentStop,
    PreCompact,
}

impl HookEvent {
    pub const ALL: [HookEvent; 10] = [
        Self::SessionStart,
        Self::SessionEnd,
        Self::PromptSubmit,
        Self::PreTool,
        Self::PostTool,
        Self::PermissionRequest,
        Self::Stop,
        Self::SubagentStart,
        Self::SubagentStop,
        Self::PreCompact,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|event| event.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::SessionStart => "session-start",
            Self::SessionEnd => "session-end",
            Self::PromptSubmit => "prompt-submit",
            Self::PreTool => "pre-tool",
            Self::PostTool => "post-tool",
            Self::PermissionRequest => "permission-request",
            Self::Stop => "stop",
            Self::SubagentStart => "subagent-start",
            Self::SubagentStop => "subagent-stop",
            Self::PreCompact => "pre-compact",
        }
    }

    pub fn claude_name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::PromptSubmit => "UserPromptSubmit",
            Self::PreTool => "PreToolUse",
            Self::PostTool => "PostToolUse",
            Self::PermissionRequest => "PermissionRequest",
            Self::Stop => "Stop",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::PreCompact => "PreCompact",
        }
    }

    pub fn copilot_name(self) -> &'static str {
        match self {
            Self::SessionStart => "sessionStart",
            Self::SessionEnd => "sessionEnd",
            Self::PromptSubmit => "userPromptSubmitted",
            Self::PreTool => "preToolUse",
            Self::PostTool => "postToolUse",
            Self::PermissionRequest => "permissionRequest",
            Self::Stop => "agentStop",
            Self::SubagentStart => "subagentStart",
            Self::SubagentStop => "subagentStop",
            Self::PreCompact => "preCompact",
        }
    }

    pub fn accepts_context(self) -> bool {
        matches!(
            self,
            Self::SessionStart | Self::PromptSubmit | Self::PostTool | Self::SubagentStart
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    #[serde(default)]
    pub schema: SchemaVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub description: String,
    pub events: BTreeMap<String, Option<HookSettings>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<Matcher>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Matcher {
    All(String),
    PerClient(ClientMatcher),
}

impl<'de> Deserialize<'de> for Matcher {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MatcherVisitor;

        impl<'de> serde::de::Visitor<'de> for MatcherVisitor {
            type Value = Matcher;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a matcher string or a map with `claude` or `copilot`")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Matcher, E> {
                Ok(Matcher::All(value.to_string()))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Matcher, A::Error> {
                let mut matcher = ClientMatcher {
                    claude: None,
                    copilot: None,
                };
                while let Some(key) = map.next_key::<String>()? {
                    let slot = match key.as_str() {
                        "claude" => &mut matcher.claude,
                        "copilot" => &mut matcher.copilot,
                        other => {
                            return Err(serde::de::Error::custom(format!(
                                "unknown matcher client `{other}`; expected `claude` or `copilot`"
                            )));
                        }
                    };
                    if slot.is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate matcher client `{key}`"
                        )));
                    }
                    *slot = Some(map.next_value()?);
                }
                Ok(Matcher::PerClient(matcher))
            }
        }

        deserializer.deserialize_any(MatcherVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientMatcher {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copilot: Option<String>,
}

/// A single file name that is safe to include in a generated shell command.
pub fn is_run_file_name(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_event_maps_each_portable_name_to_both_clients() {
        let rows = [
            ("session-start", "SessionStart", "sessionStart"),
            ("session-end", "SessionEnd", "sessionEnd"),
            ("prompt-submit", "UserPromptSubmit", "userPromptSubmitted"),
            ("pre-tool", "PreToolUse", "preToolUse"),
            ("post-tool", "PostToolUse", "postToolUse"),
            (
                "permission-request",
                "PermissionRequest",
                "permissionRequest",
            ),
            ("stop", "Stop", "agentStop"),
            ("subagent-start", "SubagentStart", "subagentStart"),
            ("subagent-stop", "SubagentStop", "subagentStop"),
            ("pre-compact", "PreCompact", "preCompact"),
        ];
        assert_eq!(HookEvent::ALL.len(), 10);
        for (event, (portable, claude, copilot)) in HookEvent::ALL.into_iter().zip(rows) {
            assert_eq!(HookEvent::parse(portable), Some(event));
            assert_eq!(event.name(), portable);
            assert_eq!(event.claude_name(), claude);
            assert_eq!(event.copilot_name(), copilot);
        }
        assert_eq!(HookEvent::parse("on-save"), None);
    }

    #[test]
    fn hook_event_accepts_context_only_on_four_events() {
        assert_eq!(
            HookEvent::ALL
                .into_iter()
                .filter(|e| e.accepts_context())
                .collect::<Vec<_>>(),
            [
                HookEvent::SessionStart,
                HookEvent::PromptSubmit,
                HookEvent::PostTool,
                HookEvent::SubagentStart
            ]
        );
    }

    #[test]
    fn hook_parses_empty_and_map_event_settings() {
        let hook: Hook = serde_yaml_ng::from_str("description: d\nevents:\n  session-start:\n  post-tool: { matcher: edit, run: check.sh, timeout: 10 }\n").unwrap();
        assert_eq!(hook.name, None);
        assert_eq!(hook.events["session-start"], None);
        let settings = hook.events["post-tool"].as_ref().unwrap();
        assert_eq!(settings.matcher, Some(Matcher::All("edit".into())));
        assert_eq!(settings.run.as_deref(), Some("check.sh"));
        assert_eq!(settings.timeout, Some(10));
    }

    #[test]
    fn matcher_accepts_a_client_map() {
        assert_eq!(
            serde_yaml_ng::from_str::<Matcher>("{ claude: 'Edit|Write', copilot: edit }").unwrap(),
            Matcher::PerClient(ClientMatcher {
                claude: Some("Edit|Write".into()),
                copilot: Some("edit".into())
            })
        );
    }

    #[test]
    fn matcher_map_refuses_an_unknown_client_key() {
        let err = serde_yaml_ng::from_str::<Matcher>("{ codex: x }").unwrap_err();
        assert!(
            err.to_string()
                .contains("unknown matcher client `codex`; expected `claude` or `copilot`"),
            "{err}"
        );
    }

    #[test]
    fn matcher_map_refuses_a_repeated_client_key() {
        let err = serde_yaml_ng::from_str::<Matcher>("{ claude: a, claude: b }").unwrap_err();
        assert!(
            err.to_string()
                .contains("duplicate matcher client `claude`"),
            "{err}"
        );
    }

    #[test]
    fn hook_refuses_an_unknown_field() {
        let err =
            serde_yaml_ng::from_str::<Hook>("description: d\nevents: {}\naudience: [claude]\n")
                .unwrap_err();
        assert!(err.to_string().contains("unknown field `audience`"));
        let err = serde_yaml_ng::from_str::<Hook>("description: d\n").unwrap_err();
        assert!(err.to_string().contains("missing field `events`"));
        assert!(serde_yaml_ng::from_str::<Hook>("events: {}\n").is_err());
        assert!(
            serde_yaml_ng::from_str::<Hook>(
                "description: d\nevents: { session-start: { extra: x } }\n"
            )
            .is_err()
        );
    }

    #[test]
    fn is_run_file_name_accepts_one_file_name_only() {
        for name in ["check.sh", "run_1-a.py"] {
            assert!(is_run_file_name(name));
        }
        for name in ["", ".hidden", "-x", "a/b.sh", "../a.sh", "a b.sh", "a;b"] {
            assert!(!is_run_file_name(name), "{name}");
        }
    }
}
