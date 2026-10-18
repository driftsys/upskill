# Bind lifecycle origin to the original backup

AD-0038

## Context

The supplied-value application primitives can recheck a caller's known original
schema-1 bytes. A reopened command adapter cannot recover that knowledge merely
by observing today's backup: edited backup bytes could otherwise become its new
expected value. Fresh scopes also have no schema-1 lockfile to back up.

## Options

- Treat every observed backup as canonical. Reject: this loses the original
  exact-byte binding after a process restart.
- Add an independently authoritative backup receipt. Reject: Gate 5 requires one
  combined authoritative state and the existing executor recovery protocol.
- Carry initial-state provenance in the combined state and reuse executor
  preconditions. Select this option.

## Decision

Add optional strict `CombinedState.origin`. `fresh` identifies initialization
from absent state and permits no retained legacy snapshot. `migrated` identifies
schema-1 initialization and records the SHA-256 and length of its exact bytes;
it requires retained typed legacy history. Unknown fields and malformed bindings
are rejected. Typed legacy projection cannot invent original-byte provenance.
Existing internal schema-2 records without origin remain inspectable; bound
application rejects them until explicit reconciliation establishes provenance.

`apply_bound_schema2_write` validates complete state/artifact agreement and
canonical lockfile/backup locations before using the qualified executor. Initial
schema-1 application checks the origin against the exact observed bytes and
preserves every typed legacy row, then reuses the create-only backup protocol.
Later application preserves origin and legacy history and verifies observed
backup digest, length, schema and private mode against that durable origin. An
observed changed backup cannot become the new canonical value.

Fresh initialization requires absent state and backup, creates state with mode
0600 and includes backup absence as a journaled no-op precondition. Later fresh
writes retain that absence requirement. No imaginary schema-1 bytes or backup
are created. State, backup and output preconditions are rechecked by the same
executor under its matching writer guard, and final state remains the last
checkpoint. No new journal or recovery mechanism is introduced.

## Consequences and limits

This is an inactive Gate 5 applying boundary. Production command dispatch and
schema 1 are unchanged. Older internal strict schema-2 readers reject origin
records; no downgrade or native-client qualification is claimed. The existing
lower-level application/executor primitives remain caller-authorized interfaces;
command adapters must use the bound boundary rather than substitute today's
backup for unknown historical evidence.

The origin is integrity evidence relative to the authoritative captured state,
not authentication against someone editing both state and backup. Qualified
snapshot, advisory lock and external namespace-race limits still apply. The
[captured command adapter](../topics/0005-lifecycle-planning.md#captured-lifecycle-command-adapters)
now retains origin-bound session evidence and prepares consuming writes through
this boundary. Explicit source-backed rule orchestration now retains captures
through this adapter. Complete local lifecycle acceptance is recorded in the
[acceptance note](../technotes/m1-gate5-final-acceptance.md); final hosted
delivery and Gate 6 native qualification remain pending. No new dependency is
introduced.

Satisfies:
[Durable lifecycle origin](../topics/0004-state-and-ownership.md#durable-lifecycle-origin).
Related: [AD-0033](0033-qualified-file-execution-and-journals.md),
[AD-0037](0037-require-create-only-backup-journals.md).
