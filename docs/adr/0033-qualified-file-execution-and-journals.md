# Qualified file execution with durable forward recovery

AD-0033

## Context

The pure operation and recovery planners do not authorize filesystem effects.
Owned lifecycle work needs a qualified executor that preserves manual edits,
recognizes an interrupted final state write, and retains evidence when recovery
cannot safely proceed. Lifecycle state and native adapter activation remain
separate gates.

## Options

The approved design selects forward recovery from exact before/after evidence. A
multi-file atomicity promise is unavailable with independent file replacements.
Retaining old user files for rollback would introduce extra sensitive copies;
the ordinary executor retains desired outputs and final state only. The separate
opt-in backup prerequisite is recorded in
[AD-0037](0037-require-create-only-backup-journals.md). Other alternatives were
not documented.

## Decision

Expose opt-in `QualifiedScope::apply_operation` and `recover_operation` methods,
requiring the matching live scope writer guard. Accept a complete ordered file
plan and one distinct, present final State action. The caller supplies ownership
authority, final state bytes and all relevant qualified source snapshots; the
executor does not select installations or define lifecycle lockfile schema 2.

Verify supplied sources and all destination preconditions before publishing
intent. Store a strict execution journal and bound desired payloads in the
private `<scope-root>/.upskill/state/` directory. Persist each generation by
syncing the staged file, atomically renaming it, and syncing its directory. An
effectful action follows `Pending` publication, destination recheck, atomic
replacement or individual unlink, destination-directory sync, post-state
observation, and `Done` publication. Final State uses the same protocol.

Use the existing pure recovery decisions over freshly qualified observations. A
Pending post-state is synced again, including its parent directory, before
acknowledgement; a Queued post-state requires reconciliation. Unexpected bytes
or ordinary modes remain untouched. Report recorded and verified prefixes
separately. Persist a completed-cleanup marker before deleting validated
payloads, then remove the completed receipt last. Preserve unknown files and
retain conflicting receipts for reconciliation.

## Consequences and limits

The backend uses descriptor-relative operations on qualified Linux/macOS local
filesystems and returns explicit unsupported errors elsewhere. It adds no
dependencies. Existing ordinary modes are preserved; new files use selected
private modes. ACLs, special bits and unsupported extended metadata reject.
Darwin permits only `com.apple.provenance`, and replacement requires the same
value on the prior and staged files. Replacement also verifies unchanged UID and
GID; it does not attempt ownership or ACL repair.

Advisory locks coordinate cooperating upskill writers. Pinned handles and
rechecks reject observed namespace changes, but cannot exclude every race with
external editors. There is no multi-file atomicity guarantee. Sync ordering and
real process termination tests do not establish hardware power-loss durability.
Gate-4 hosted delivery evidence is retained in the
[validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate4-validation.md).
Combined lifecycle integration and M1 activation remain separate work. Current
CLI commands and schema-1 lockfiles do not consume this API.

Satisfies:
[specification §4.12](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery).
Related: [AD-0028](0028-lock-qualified-scope-directories.md),
[AD-0031](0031-select-explicit-file-permissions.md),
[AD-0032](0032-model-recovery-checkpoints-before-effects.md),
[qualified execution design](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#architecture-qualified-file-execution).
