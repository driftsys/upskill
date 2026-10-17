# Guarded original-state backup validation

This bounded Gate 5 slice adds `QualifiedScope::apply_operation_with_backup`. It
preserves exact caller-observed original State bytes in a private,
non-overwritten backup through the existing checkpointed executor. It neither
validates lifecycle JSON nor activates production schema-2 writes. Lifecycle
preparation, application adapters and observational inspection remain separate.

## Acceptance evidence

Five real-filesystem integration examples in
[execution_backup.rs](../../tests/execution_backup.rs) cover literal original
bytes including unknown JSON fields and whitespace, mode 0o600, retained
matching-backup identity, ordinary original State mode 0o644 with private backup
mode 0o600, mismatched bytes/modes, full-request preflight and invalid backup
topology or missing original state.

Four focused internal examples cover a file appearing immediately before
publication, edits to a completed backup blocking recovery, eight journal policy
or evidence corruptions, and syncing an unchanged matching backup before output.
The sync fixture checks exact backup-file and parent descriptor identities, call
order, failure of each sync, retained Queued receipt/generation and unchanged
outputs before recovering. Original digest/length corruptions change final State
before evidence while preserving valid backup payload evidence, isolating the
backup-to-original binding from generic payload validation. The publication race
fixture observes both the first failure and the subsequent reconciliation
outcome, checking retained manual bytes and original State.

A process test terminates real children at 37 effect/receipt boundaries across
the backup, four output positions and final State. The unchanged output has only
its two receipt boundaries. Recovery verifies literal backup bytes, private
mode, one link, unchanged backup identity after an observed publication, exact
output bytes/modes and complete recorded/verified prefixes. These checks do not
establish hardware power-loss durability.

## Regression controls and review

Before implementation, forwarding to the old executor assertion-failed on the
absent backup/action prefix, ignored backup mismatch and accepted aliases. The
new acceptance examples passed after implementation.

Three compiled fault controls each produced a targeted assertion failure:

- Replacing `NOREPLACE` with ordinary rename flags allowed a late manual file to
  be overwritten.
- Removing required create-only journal ordering allowed omitted policy
  metadata.
- Skipping unchanged-backup sync bypassed the required durable prerequisite.

The original source bytes were restored after each control and their hashes
compared. All four focused internal examples passed on restored source. Logs and
source hashes are retained in the
[evidence manifest](evidence/2026-10-17-m1-guarded-backup/manifest.json).

An independent whole-change reviewer reported no Critical, Important or Minor
findings. The reviewer did not run duplicate broad tests. Lifecycle validation,
journal authentication against coordinated malicious rewriting, complete
external-race prevention and hardware durability were explicitly excluded from
this bounded primitive. Those limits remain documented; structural validation
does not authenticate the journal against a writer with scope access.

The commit's advisory quick docs/tests pass found three Minor documentation
qualifiers and two Important assertion gaps. All were corrected, with no
additional review over the fixes. Four additional controls independently removed
original digest binding, original length binding, the backup sync call while
retaining its checkpoint, and fixed private backup mode. Each compiled and
assertion-failed; exact source was restored and all nine focused examples
passed. Seven controls are retained in total. The archived Markdown spec is a
formatted reading copy; its exact original bytes are retained as compressed
evidence because staged stdin formatting rewraps Markdown independently of
file-ignore rules.

## Verification status

The first full `just verify` run passed 1,252 Rust tests with one ignored
subprocess entrypoint, then failed the book-link check on newly added relative
links to an evidence page outside the book. The links were changed to repository
URLs. All-target Clippy and locked Rust 1.85 checking passed. Final verification
after documentation reconciliation passed: `just fmt`, `just verify` (1,252 Rust
tests, one ignored subprocess entrypoint), documentation checks and build all
completed successfully. The earlier failed runs remain retained as evidence.
Exact-head hosted checks are a separate delivery gate.

Hosted qualification must execute on the delivered head. A CI configuration or
local macOS result does not establish Linux/macOS/Windows execution evidence.
Ordinary execution journals retain schema 1; backup operations require journal
schema 2. Older binaries reject the latter. Neither journal version activates a
new lifecycle lockfile schema or completes Gate 5/M1.
