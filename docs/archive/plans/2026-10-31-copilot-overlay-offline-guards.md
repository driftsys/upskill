# Copilot Overlay Offline Guards Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans inline with TDD.

**Goal:** Implement pure byte-edit and protocol-planning guards, not a native
runner. **Architecture:** Extend the existing qualification script with a strict
offline JSON mode and pure functions. Existing native classifiers remain
unchanged. **Tech Stack:** Python standard library and unittest. **Spec:**
../specs/2026-10-31-copilot-overlay-offline-guards.md

## Global constraints

- Isolated worktree; preserve other worktrees and profiles.
- Zero native/model calls or destination writes.
- Planning outputs never authorize native execution or classify isolation.
- No new dependency or production Rust/schema change.

## Review focus

- CLI input cannot accidentally select fixture preparation or invoke a client.
- Duplicate/nonfinite JSON, booleans-as-integers and missing caps reject.
- Wrong current bytes or file mode cannot produce a proposed edit.
- Repeated/reordered phases and budget exhaustion cannot plan an extra process.
- Planning prefixes cannot be represented as native evidence or a PASS.

### Task 1: Guarded byte edit and real CLI

Files: scripts/qualify-plugin-rules.py;
scripts/tests/test_copilot_overlay_planning.py. Interfaces:
plan_copilot_overlay_edit(baseline, current, plugin_key, action, mode) returns
bytes. replay_copilot_overlay_plan(packet) returns a JSON plan.

- [x] Write a CLI acceptance test with literal baseline and expected masked
      bytes.
- [x] Run it RED: the new planning option is missing.
- [x] Implement strict input validation and pure guarded edits; wire the CLI
      mode.
- [x] Add focused malformed/current-drift/mode/ownership tests, watch each fail,
      and implement the corresponding guards.
- [x] Run the focused tests and complete scripts/tests suite.

### Task 2: Phase admission planning and documentation

Interface: plan_copilot_overlay_phase(prefix, attempts, budget) returns planning
metadata only; replay_copilot_overlay_plan dispatches phase packets.

- [x] Write RED CLI/unit examples for phase order, client caps and completion.
- [x] Implement minimal prefix/attempt/cap guards and non-authorizing output.
- [x] Run focused and complete offline tests; perform real mutation checks.
- [x] Update the durable profile with APIs, examples and unimplemented
      boundaries.
- [ ] Run just fmt and just verify; garden only this feature's raw spec/plan.
- [ ] Open a stacked PR on #447 and complete the required independent review.

## Execution ledger

Execution is inline under the user's approval to continue offline preparation.
Native profile/model budget execution is not authorized by this plan.

## Retained verification

The
[offline validation record](../../technotes/evidence/2026-10-31-copilot-overlay-offline-planning/validation.json)
links the historical RED-log hashes/summaries, current source/test hashes, GREEN
suite counts and mutation definitions/results. RED implementation snapshots were
not retained; checked steps above are author-recorded execution history, not
independent native evidence. Native authority remains false.
