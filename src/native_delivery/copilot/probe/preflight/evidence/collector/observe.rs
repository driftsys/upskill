//! Strict in-memory observations. Unknown native strings never reach records.
use crate::native_delivery::copilot::probe::{CopilotInventoryKind, summarize_inventory};
use crate::native_delivery::process::{NativeGroupState, NativeProcessOutput, NativeStopReason};
use serde_json::{Value, json};
use std::path::{Component, Path};

/// Only fixed diagnostic labels may leave the in-memory validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Rejection {
    pub reason: &'static str,
    pub source_category: Option<&'static str>,
}
impl From<&'static str> for Rejection {
    fn from(reason: &'static str) -> Self {
        Self {
            reason,
            source_category: None,
        }
    }
}

fn source_category(source: &str) -> &'static str {
    if source.starts_with("GitHub: ") {
        "display_github"
    } else if source.starts_with("Local: ") {
        "display_local"
    } else if source.starts_with("URL: ") {
        "display_url"
    } else if source.contains("://") {
        "uri"
    } else if Path::new(source)
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        "parent_component"
    } else if Path::new(source).is_absolute() {
        "outside_absolute"
    } else {
        "relative"
    }
}

// These labels describe inventory metadata, never a trusted remote origin.
fn default_remote_display(source: &str) -> bool {
    if let Some(repository) = source.strip_prefix("GitHub: ") {
        let parts: Vec<_> = repository.split('/').collect();
        return parts.len() == 2
            && parts.iter().all(|part| {
                !part.is_empty()
                    && *part != "."
                    && *part != ".."
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
            });
    }
    let Some(url) = source.strip_prefix("URL: ") else {
        return false;
    };
    let Some(address) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = address.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.is_empty()
        && authority != "."
        && authority != ".."
        && !authority.contains('@')
        && !url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

pub(super) fn transport(output: &NativeProcessOutput) -> bool {
    output.reason == NativeStopReason::Exited
        && output.exit_code == Some(0)
        && output.group == NativeGroupState::Empty
        && output.direct_child_reaped
        && output.output_complete
        && output.scope_unchanged
        && output.executable_unchanged
}
pub(super) fn facts(output: &NativeProcessOutput) -> Value {
    let reason = match output.reason {
        NativeStopReason::Exited => "exited",
        NativeStopReason::TimedOut => "timed_out",
        NativeStopReason::Cancelled => "cancelled",
        NativeStopReason::OutputLimit => "output_limit",
        NativeStopReason::OutputReadFailed => "output_read_failed",
    };
    json!({"reason":reason,"exit_code":output.exit_code,"stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len(),
        "group_empty":output.group==NativeGroupState::Empty,"direct_child_reaped":output.direct_child_reaped,
        "output_complete":output.output_complete,"scope_unchanged":output.scope_unchanged,
        "executable_unchanged":output.executable_unchanged,"cleanup_signal_sent":output.cleanup_signal_sent})
}
pub(super) fn inspect(
    index: Option<usize>,
    output: &NativeProcessOutput,
    plan: &Value,
) -> Result<Value, Rejection> {
    if !transport(output) {
        return Err("transport".into());
    }
    if !output.stderr.is_empty() {
        return Err("stderr".into());
    }
    let Some(index) = index else {
        return if output.stdout.is_empty() {
            Ok(json!({"setup_complete":true}))
        } else {
            Err("unexpected_output".into())
        };
    };
    if index == 0 {
        return if output.stdout
            == b"GitHub Copilot CLI 1.0.107.\nRun 'copilot update' to check for updates.\n"
        {
            Ok(json!({"version_matches":true}))
        } else {
            Err("version".into())
        };
    }
    let kind = match index {
        1 | 5 | 9 => Some(CopilotInventoryKind::Plugins),
        2 | 6 | 10 => Some(CopilotInventoryKind::Marketplaces),
        _ => None,
    };
    let Some(kind) = kind else {
        return if output.stdout.is_empty() {
            Ok(json!({"unstructured_output_empty":true}))
        } else {
            Err("unexpected_output".into())
        };
    };
    let summary = summarize_inventory("1.0.107", kind, &output.stdout).map_err(|_| "inventory")?;
    let text = std::str::from_utf8(&output.stdout).map_err(|_| "inventory")?;
    let rows = crate::parse::json::parse_unique(text).map_err(|_| "inventory")?;
    let name = plan["native_name"].as_str().ok_or("input")?;
    let selector = format!("{name}@{name}");
    let root = Path::new(plan["packet_root"].as_str().ok_or("input")?);
    let catalog = plan["catalog"].as_str().ok_or("input")?;
    let revision = Path::new(plan["home"].as_str().ok_or("input")?)
        .join(plan["revision_root"].as_str().ok_or("input")?);
    let mut matches = 0;
    let mut unrelated_builtin_label_entries = 0;
    let mut unrelated_default_remote_display_entries = 0;
    for row in rows.as_array().ok_or("inventory")? {
        let source = row["source"].as_str().ok_or("inventory")?;
        let named = row["name"] == name
            || row["name"] == selector
            || (kind == CopilotInventoryKind::Plugins && row["marketplace"] == name);
        // The pinned client uses this non-filesystem marker for builtin metadata.
        // It grants no origin or binding proof and never exempts an owned reference.
        if !named && source == "builtin" {
            unrelated_builtin_label_entries += 1;
            continue;
        }
        // Marketplace JSON uses display labels, unlike plugin filesystem sources.
        // Count only unrelated default remote metadata; owned names never use this
        // exception, and a default flag establishes no origin or binding evidence.
        if kind == CopilotInventoryKind::Marketplaces
            && !named
            && row["isDefault"] == true
            && default_remote_display(source)
        {
            unrelated_default_remote_display_entries += 1;
            continue;
        }
        let local_source = if kind == CopilotInventoryKind::Marketplaces {
            source.strip_prefix("Local: ").unwrap_or(source)
        } else {
            source
        };
        let source_path = Path::new(local_source);
        if local_source.contains("://")
            || !source_path.is_absolute()
            || source_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            || !source_path.starts_with(root)
        {
            return Err(Rejection {
                reason: "outside_packet_source",
                source_category: Some(source_category(source)),
            });
        }
        if !named {
            continue;
        }
        matches += 1;
        if kind == CopilotInventoryKind::Marketplaces
            && (row["name"] != name
                || row["isDefault"] != false
                || source_path != Path::new(catalog))
        {
            return Err("fixture_conflict".into());
        }
        if kind == CopilotInventoryKind::Plugins
            && ((row["name"] != name && row["name"] != selector)
                || row["marketplace"] != name
                || row["version"] != plan["version"]
                || row["enabled"] != true)
        {
            return Err("fixture_conflict".into());
        }
        if source_path != Path::new(catalog)
            && source_path != revision
            && source_path != revision.join("skills/contract-probe")
        {
            return Err("fixture_conflict".into());
        }
    }
    let expected = usize::from(matches!(index, 5 | 6));
    if matches != expected {
        return Err("fixture_conflict".into());
    }
    Ok(json!({"inventory":summary,"fixture_named_entries":matches,
        "unrelated_builtin_label_entries":unrelated_builtin_label_entries,
        "unrelated_default_remote_display_entries":unrelated_default_remote_display_entries}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn successful() -> NativeProcessOutput {
        NativeProcessOutput {
            reason: NativeStopReason::Exited,
            exit_code: Some(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
            group: NativeGroupState::Empty,
            direct_child_reaped: true,
            cleanup_signal_sent: false,
            output_complete: true,
            scope_unchanged: true,
            executable_unchanged: true,
        }
    }

    #[test]
    fn collector_rejects_each_uncertain_transport_fact() {
        assert!(inspect(None, &successful(), &Value::Null).is_ok());
        for case in 0..11 {
            let mut output = successful();
            match case {
                0 => output.reason = NativeStopReason::TimedOut,
                1 => output.reason = NativeStopReason::Cancelled,
                2 => output.reason = NativeStopReason::OutputLimit,
                3 => output.reason = NativeStopReason::OutputReadFailed,
                4 => output.exit_code = Some(1),
                5 => output.exit_code = None,
                6 => output.group = NativeGroupState::Unresolved,
                7 => output.direct_child_reaped = false,
                8 => output.output_complete = false,
                9 => output.scope_unchanged = false,
                10 => output.executable_unchanged = false,
                _ => unreachable!(),
            }
            assert_eq!(
                inspect(None, &output, &Value::Null),
                Err("transport".into()),
                "case {case}"
            );
        }
    }
    #[test]
    fn source_categories_cannot_mask_transport_stderr_or_later_malformed_rows() {
        let plan = json!({"native_name":"owned", "packet_root":"/packet", "catalog":"/packet/catalog", "home":"/packet/home", "revision_root":"revision", "version":"1"});
        for (index, row) in [
            (
                1,
                json!({"name":"unrelated", "marketplace":"unrelated", "source":"GitHub: secret-canary", "version":"1", "enabled":true}),
            ),
            (
                2,
                json!({"name":"unrelated", "source":"GitHub: secret-canary", "isDefault":true}),
            ),
        ] {
            let mut output = successful();
            output.stdout = serde_json::to_vec(&json!([row.clone()])).unwrap();
            assert_eq!(
                inspect(Some(index), &output, &plan)
                    .unwrap_err()
                    .source_category,
                Some("display_github")
            );
            output.exit_code = Some(1);
            assert_eq!(
                inspect(Some(index), &output, &plan),
                Err("transport".into())
            );
            output.exit_code = Some(0);
            output.stderr = b"secret-canary".to_vec();
            assert_eq!(inspect(Some(index), &output, &plan), Err("stderr".into()));
            output.stderr.clear();
            output.stdout = serde_json::to_vec(&json!([row, {"unknown":"secret-canary"}])).unwrap();
            assert_eq!(
                inspect(Some(index), &output, &plan),
                Err("inventory".into())
            );
        }
    }

    #[test]
    fn source_category_precedence_describes_only_lexical_shape() {
        for (source, expected) in [
            ("/secret-canary/../outside", "parent_component"),
            ("https://secret-canary.invalid/../path", "uri"),
            ("GitHub: https://secret-canary.invalid", "display_github"),
            ("Local: https://secret-canary.invalid", "display_local"),
            ("URL: https://secret-canary.invalid/../path", "display_url"),
        ] {
            assert_eq!(source_category(source), expected);
        }
    }
    #[test]
    fn marketplace_display_sources_keep_owned_and_scope_checks() {
        let plan = json!({"native_name":"owned", "packet_root":"/packet", "catalog":"/packet/catalog", "home":"/packet/home", "revision_root":"revision", "version":"1"});
        for (name, source, default, expected) in [
            (
                "owned",
                "GitHub: secret-canary/repo",
                true,
                "outside_packet_source",
            ),
            (
                "owned@owned",
                "GitHub: secret-canary/repo",
                true,
                "outside_packet_source",
            ),
            (
                "unrelated",
                "GitHub: secret-canary/repo",
                false,
                "outside_packet_source",
            ),
            (
                "unrelated",
                "URL: https://secret-canary.invalid",
                false,
                "outside_packet_source",
            ),
            ("unrelated", "GitHub: ", true, "outside_packet_source"),
            ("unrelated", "URL: ", true, "outside_packet_source"),
            (
                "unrelated",
                "GitHub: ../secret-canary",
                true,
                "outside_packet_source",
            ),
            (
                "unrelated",
                "Local: /outside/catalog",
                false,
                "outside_packet_source",
            ),
            (
                "owned",
                "Local: /packet/catalog/../wrong",
                false,
                "outside_packet_source",
            ),
            ("owned", "Local: /packet/wrong", false, "fixture_conflict"),
            (
                "owned",
                "Local: /packet/home/revision",
                false,
                "fixture_conflict",
            ),
            ("owned", "Local: /packet/catalog", true, "fixture_conflict"),
        ] {
            let mut output = successful();
            output.stdout =
                serde_json::to_vec(&json!([{"name":name,"source":source,"isDefault":default}]))
                    .unwrap();
            assert_eq!(
                inspect(Some(6), &output, &plan).unwrap_err().reason,
                expected,
                "{source}"
            );
        }
        let mut valid = successful();
        valid.stdout = serde_json::to_vec(
            &json!([{"name":"owned","source":"Local: /packet/catalog","isDefault":false}]),
        )
        .unwrap();
        assert_eq!(
            inspect(Some(6), &valid, &plan).unwrap()["fixture_named_entries"],
            1
        );
        let mut output = successful();
        output.stdout = serde_json::to_vec(&json!([{"name":"unrelated","marketplace":"unrelated","source":"Local: /packet/catalog","version":"1","enabled":true}])).unwrap();
        assert_eq!(
            inspect(Some(1), &output, &plan).unwrap_err().reason,
            "outside_packet_source"
        );
    }

    #[test]
    fn default_display_metadata_needs_complete_inventory_validation() {
        let plan = json!({"native_name":"owned", "packet_root":"/packet", "catalog":"/packet/catalog", "home":"/packet/home", "revision_root":"revision", "version":"1"});
        let row =
            json!({"name":"unrelated","source":"GitHub: secret-canary/repo","isDefault":true});
        let mut output = successful();
        output.stdout = serde_json::to_vec(&json!([row.clone()])).unwrap();
        assert_eq!(
            inspect(Some(2), &output, &plan).unwrap()["unrelated_default_remote_display_entries"],
            1
        );
        output.stdout = serde_json::to_vec(&json!([row, {"unknown":"secret-canary"}])).unwrap();
        assert_eq!(inspect(Some(2), &output, &plan), Err("inventory".into()));
    }
    fn default_marketplace(source: &str) -> Result<Value, Rejection> {
        let plan = json!({"native_name":"owned", "packet_root":"/packet", "catalog":"/packet/catalog", "home":"/packet/home", "revision_root":"revision", "version":"1"});
        let mut output = successful();
        output.stdout =
            serde_json::to_vec(&json!([{"name":"unrelated","source":source,"isDefault":true}]))
                .unwrap();
        inspect(Some(2), &output, &plan)
    }

    #[test]
    fn default_url_display_rejects_each_malformed_authority_and_character_class() {
        for source in [
            "URL: ftp://secret-canary.invalid/catalog",
            "URL: file:///catalog",
            "URL: unknown",
            "URL: https://",
            "URL: http://",
            "URL: https:///catalog",
            "URL: https://?query",
            "URL: https://#fragment",
            "URL: https://.",
            "URL: http://..",
            "URL: https://user@secret-canary.invalid/catalog",
            "URL: https://user:password@secret-canary.invalid/catalog",
            "URL: https://secret-canary.invalid/with space",
            "URL: https://secret-canary.invalid/with\tcontrol",
            "URL: https://secret-canary.invalid/with\u{7f}control",
            "URL: https://secret-canary.invalid/with\0control",
        ] {
            let error = default_marketplace(source).unwrap_err();
            assert_eq!(error.reason, "outside_packet_source", "{source:?}");
            assert_eq!(error.source_category, Some("display_url"));
        }
        for source in [
            "URL: https://secret-canary.invalid/catalog",
            "URL: http://secret-canary.invalid/catalog",
            "URL: https://secret-canary.invalid/catalog?query=value#fragment",
        ] {
            let result = default_marketplace(source).unwrap();
            assert_eq!(result["unrelated_default_remote_display_entries"], 1);
            assert_eq!(result["fixture_named_entries"], 0);
            assert!(!result.to_string().contains("secret-canary"));
        }
    }

    #[test]
    fn default_github_display_rejects_each_malformed_component() {
        for source in [
            "GitHub: /repository",
            "GitHub: owner/",
            "GitHub: ./repository",
            "GitHub: owner/.",
            "GitHub: ../repository",
            "GitHub: owner/..",
            "GitHub: invalid owner/repository",
            "GitHub: owner/invalid repository",
            "GitHub: invalidé/repository",
            "GitHub: owner/invalidé",
            "GitHub: owner/repository/extra",
        ] {
            let error = default_marketplace(source).unwrap_err();
            assert_eq!(error.reason, "outside_packet_source", "{source:?}");
            assert_eq!(error.source_category, Some("display_github"));
        }
        for source in [
            "GitHub: owner/repository",
            "GitHub: owner-name/repo.name_1-2",
        ] {
            let result = default_marketplace(source).unwrap();
            assert_eq!(result["unrelated_default_remote_display_entries"], 1);
            assert_eq!(result["fixture_named_entries"], 0);
        }
    }
}
