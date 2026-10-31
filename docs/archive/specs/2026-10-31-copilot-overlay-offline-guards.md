# Copilot overlay offline planning guards

Task #448. Implements only the first pure planning layer of the
[profile](../../technotes/copilot-project-plugin-overlay-profile.md). No native
orchestration, evidence classifier or filesystem application is added.

The real CLI accepts `--plan-copilot-overlay PACKET.json` without `--client` or
`--root`. It reads captured JSON, prints a plan and never invokes a client or
writes a destination. Invalid input exits 2. Every output includes
`native_run_authorized: false`; no output is an isolation PASS.

Edit packets contain exactly kind=edit, baseline_utf8, current_utf8, plugin_key,
action and mode. The captured baseline is canonical two-space UTF-8 JSON plus a
final newline, with exactly companyAnnouncements (nonempty string list) and an
empty enabledPlugins map. Mode is exactly integer 0600. The owned identity uses
two lower-case alphanumeric/hyphen components starting with an alphanumeric
character, each 1–64 characters, separated by one @. Reject duplicate/nonfinite
JSON, unsupported formatting/content, invalid keys, mode/presence drift and
stale current bytes. Mask/remask require baseline current bytes; unmask/release
require the exact planned masked bytes. A plan changes only the owned empty-map
content; it does not apply it to disk.

Phase packets contain exactly kind=phase, prefix, attempts and budget. Prefix
must be a prefix of observer_positive, owner_positive, baseline,
owner_candidate, masked_candidate, unmasked_reference, remasked_candidate,
post_removal. Attempts is an integer equal to prefix length and at most eight;
retry/skip/reorder/extra attempts reject. Budget has exactly process_limit=8,
copilot_max_ai_credits and claude_max_budget_usd, both explicit positive decimal
strings. No defaults or special numeric values. Emit the next planned client,
overlay condition, expected owner state and that client's declared cap; after
eight steps emit completion without another phase. Prefix is planning input, not
proof of a successfully completed native phase. Real caps/approvals and native
outcomes require independent enforcement before execution.

Keep Q11/Q11b classifiers unchanged. Full native event/resource correlation,
identity, session freshness, file/root locks, managed policy, actual credit
spending and inverse effects remain unimplemented boundaries. No real model
identifiers, settings, registrations or paid calls are needed by the tests.
