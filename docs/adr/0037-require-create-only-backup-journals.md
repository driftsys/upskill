# Require create-only backup journals

AD-0037

## Context

Guarded lifecycle application needs to preserve exact observed original State
bytes before changing any output. Ordinary file replacement must not overwrite
an existing backup, including one that appears after the last observation.
Recovery must retain the same publication restriction after process termination.
This is a bounded executor prerequisite; lifecycle preparation and production
command activation remain separate work.

## Options

The accepted slice uses a separate opt-in executor method and persists the
create-only prerequisite in a versioned execution journal. Alternatives were not
documented.

## Decision

Expose `QualifiedScope::apply_operation_with_backup` without changing
`ExecutionRequest`. Require a matching writer guard and observed `state.before`
bytes. Prepend those exact bytes as action zero, with mode 0o600, and validate
the complete request, sources, destinations and metadata before creating intent
or backup files. The caller supplies lifecycle authority; the executor does not
parse or reserialize original State bytes.

Use execution-journal schema 2 for backup operations. Require exactly one
create-only action at index zero, distinct from the final State, with an absent
or identical before value. Bind its desired digest and length to the final
State's original evidence. Reject missing, moved or downgraded policy markers,
invalid private modes and mismatched evidence before recovery effects. Ordinary
execution-journal schema 1 remains supported and cannot carry a create-only
action. This journal version is independent of lifecycle lockfile schemas.

Publish a new backup with qualified sibling staging and atomic
`renameat_with(NOREPLACE)` using existing rustix support. Reject unsupported
host or filesystem behavior. Preserve an existing matching backup's identity;
sync its file and parent directory before its Done receipt. Reuse the existing
Pending/Done, payload, final State and cleanup protocol for the remaining
operation.

Recovery verifies all actions, including the backup, before progressing. Sync
and acknowledge an interrupted matching Pending publication without replacement.
An edited completed backup requires reconciliation before later effects. Include
the backup at index zero in report prefixes and retain it after successful
journal cleanup. Remove only enumerated journal files, never directories.

## Consequences and limits

Older binaries reject execution-journal schema 2; recovery requires a supporting
binary. There are no new dependencies or CLI commands. Qualified observation,
advisory locking and external namespace-race limits continue to apply. Process
termination checks do not establish hardware power-loss durability. This
primitive does not infer ownership, fetch sources, prepare lifecycle transitions
or activate lifecycle-schema-2 production writes. Gate 5 and native
qualification remain separate work.

The implementation is in
[`run.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/run.rs),
[`journal.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/journal.rs)
and
[`backend.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/backend.rs).
Public acceptance examples are in
[`execution_backup.rs`](https://github.com/driftsys/upskill/blob/main/tests/execution_backup.rs);
private publication, policy and sync checks are in
[`backup_tests.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/backup_tests.rs).

Satisfies:
[specification §4.12](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery).
Related: [AD-0033](0033-qualified-file-execution-and-journals.md),
[qualified execution design](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#architecture-qualified-file-execution).
