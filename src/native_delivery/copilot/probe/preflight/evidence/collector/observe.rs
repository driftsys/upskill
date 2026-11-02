//! Strict in-memory observations. Unknown native strings never reach records.
use crate::native_delivery::copilot::probe::{CopilotInventoryKind, summarize_inventory};
use crate::native_delivery::process::{NativeGroupState, NativeProcessOutput, NativeStopReason};
use serde_json::{Value, json};
use std::path::{Component, Path};

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
) -> Result<Value, &'static str> {
    if !transport(output) {
        return Err("transport");
    }
    if !output.stderr.is_empty() {
        return Err("stderr");
    }
    let Some(index) = index else {
        return if output.stdout.is_empty() {
            Ok(json!({"setup_complete":true}))
        } else {
            Err("unexpected_output")
        };
    };
    if index == 0 {
        return if output.stdout
            == b"GitHub Copilot CLI 1.0.105.\nRun 'copilot update' to check for updates.\n"
        {
            Ok(json!({"version_matches":true}))
        } else {
            Err("version")
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
            Err("unexpected_output")
        };
    };
    let summary = summarize_inventory("1.0.105", kind, &output.stdout).map_err(|_| "inventory")?;
    let text = std::str::from_utf8(&output.stdout).map_err(|_| "inventory")?;
    let rows = crate::parse::json::parse_unique(text).map_err(|_| "inventory")?;
    let name = plan["native_name"].as_str().ok_or("input")?;
    let selector = format!("{name}@{name}");
    let root = Path::new(plan["packet_root"].as_str().ok_or("input")?);
    let catalog = plan["catalog"].as_str().ok_or("input")?;
    let revision = Path::new(plan["home"].as_str().ok_or("input")?)
        .join(plan["revision_root"].as_str().ok_or("input")?);
    let mut matches = 0;
    for row in rows.as_array().ok_or("inventory")? {
        let source = row["source"].as_str().ok_or("inventory")?;
        let source_path = Path::new(source);
        if source.contains("://")
            || !source_path.is_absolute()
            || source_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            || !source_path.starts_with(root)
        {
            return Err("outside_packet_source");
        }
        let named = row["name"] == name
            || row["name"] == selector
            || (kind == CopilotInventoryKind::Plugins && row["marketplace"] == name);
        if !named {
            continue;
        }
        matches += 1;
        if kind == CopilotInventoryKind::Plugins
            && ((row["name"] != name && row["name"] != selector)
                || row["marketplace"] != name
                || row["version"] != plan["version"]
                || row["enabled"] != true)
        {
            return Err("fixture_conflict");
        }
        if source_path != Path::new(catalog)
            && source_path != revision
            && source_path != revision.join("skills/contract-probe")
        {
            return Err("fixture_conflict");
        }
    }
    let expected = usize::from(matches!(index, 5 | 6));
    if matches != expected {
        return Err("fixture_conflict");
    }
    Ok(json!({"inventory":summary,"fixture_named_entries":matches}))
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
                Err("transport"),
                "case {case}"
            );
        }
    }
}
