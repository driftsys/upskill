# Qualified file execution and recovery

This design describes the opt-in library executor in
[`src/filesystem/execution/`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/mod.rs).
Current commands do not call it, and it does not enable lifecycle-schema-2
writes or establish installation ownership. Gate-4 delivery evidence is retained
in its
[validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/2026-10-15-m1-gate4-validation.md);
combined lifecycle integration and M1 activation remain separate work.

## Request and authority boundary

`ExecutionRequest` contains ordered `files: Vec<PlannedFile>` and a final
`state: PlannedFile`. `ExecutionRequest::validate` checks paths, scope equality,
permission values and destination topology without filesystem effects. All paths
share an absolute scope root, must be valid relative paths, and must not collide
or overlap by ancestry. `.upskill/state/` and its ancestors are reserved. State
must be distinct and present after execution. State-only requests are valid;
unchanged and removal actions retain their ordered positions.

`QualifiedScope::apply_operation` borrows its matching `ScopeWriterGuard`, the
caller-supplied source snapshots, and the complete request. It verifies those
sources, qualifies every destination and its metadata, compares every exact
before value, then revalidates source, destination and guard bindings before
creating durable intent. `recover_operation` uses the same guard and the saved
payloads; it performs no source fetch. A caller must provide the complete
ownership-aware plan and all source evidence relevant to it. Neither API selects
source refs, installations, consumers or ownership policy.

`ExecutionReport` contains `recorded_prefix`, `verified_post_prefix` and
`ExecutionOutcome::{Complete, Reconcile { index }}`. Conflicting observations
return Reconcile without modifying the conflicting file. Invalid requests,
corrupt journals and I/O failures return contextual errors; errors during
execution include recorded and verified prefixes; failed qualification reports a
conservative verified prefix of zero because current observations are
unavailable. Complete includes the final State action and successful qualified
cleanup.

## Exact original-state backup prerequisite

`QualifiedScope::apply_operation_with_backup` takes the complete request, guard
and source snapshots plus a backup `FileLocation`. It requires an observed
`state.before`, prepends its exact original bytes as action zero, and validates
the complete resulting topology. It does not parse or reserialize those bytes;
the caller remains responsible for the lifecycle transition. The backup must be
absent or already match the original bytes and private mode 0o600. A mismatch
rejects before journal creation or output effects. All other destination and
source preconditions are also checked before the backup is created.

New backups use the qualified sibling staging protocol and
`renameat_with(NOREPLACE)`, which atomically refuses an existing destination.
Unsupported filesystem behavior fails closed. A matching existing backup is
synced with its parent before its Done receipt and retains physical identity. No
backup is overwritten or removed by this method.

Backup operations use execution-journal schema 2, independently of lifecycle
lockfile schemas. It requires exactly one create-only first action, mode 0o600,
an absent or identical before value, and byte digest/length equal to the final
State's original evidence. Following actions use ordinary execution. Original
execution-journal schema 1 remains accepted and cannot contain a create-only
action. Removing, moving or downgrading the create-only marker rejects. Older
binaries reject the new journal version; recovery requires a supporting binary.

Recovery verifies the backup alongside the entire action sequence before each
step. An interrupted publication with a matching Pending post-state is synced
and acknowledged without rewriting it. Editing even a completed backup blocks
later outputs and final State. Reports include the backup at index zero in their
recorded and verified prefixes. Cleanup retains the backup. This generic
primitive does not validate schema-1 JSON, prepare lifecycle ownership, or
activate production commands.

The persisted prerequisite is recorded in
[AD-0037](../adr/0037-require-create-only-backup-journals.md).

## Components and file effects

[`request.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/request.rs)
validates the supplied plan.
[`backend.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/backend.rs)
implements qualified descriptor-relative observations, parent traversal, sibling
staging, replacement, unlink and sync.
[`journal.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/journal.rs)
owns storage, strict decoding, payload binding, generations and cleanup.
[`run.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/run.rs)
classifies fresh observations and executes only steps permitted by
`operations::recovery::plan_recovery`.

Before each effect, the backend rechecks the selected destination and parent
binding. A replacement uses an exclusive 0o600 sibling staging file, verifies
its identity, bytes and selected mode, syncs it, renames it relative to the held
parent, and syncs that directory. Newly created output parents use 0o700 and
have their bindings synced. Removal unlinks one planned unchanged file.
Unchanged files retain physical identity. Output directories, extra resources,
manual aggregate bytes supplied by planning and empty aggregate files are not
recursively removed.

Existing ordinary permission bits must remain exact; unknown modes and special
bits reject. New outputs require 0o600 or explicitly selected 0o700. Existing
destination UID/GID must match the qualified root, and replacement verifies that
the staged file retains the prior UID/GID. Linux extended attributes, Darwin
ACLs and file flags, and other unsupported metadata reject. Darwin permits only
`com.apple.provenance`; replacement proceeds only if its value matches on prior
and staged files. The implementation does not copy or repair ACLs or metadata.

[`macos_acl.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/macos_acl.rs)
uses minimal Darwin ACL inspection through the system ABI without a new
dependency. The ABI and absence interpretation follow the SDK `sys/acl.h` and
Apple Libc `posix1e/acl_file.c` and `gen/filesec.c`: `NULL` with `ENOENT` on a
live file descriptor denotes an absent ACL property. Other failures reject, as
do ACLs containing an entry.

## Private storage and strict journal

The store is `<scope-root>/.upskill/state/`. The root and `.upskill` parent must
not be writable by group or others. The state directory is 0o700, is on the
qualified mount, and has ownership and physical bindings checked. Private files
are 0o600, single-link regular files with qualified metadata and bindings.
`owner.json` binds the root and directory identities. A generated `.gitignore`
contains `*` and `!.gitignore`. Existing state without matching owner and ignore
records rejects; arbitrary existing content is not adopted.

`operation.json` uses execution-journal schema 1, independently of the CLI
lockfile schema. Strict records reject unknown fields and contain a random
operation ID, generation, root identity, ordered action paths, a final State
marker, before/after SHA-256 digests, lengths and ordinary modes, payload names
and physical identities, Queued/Pending/Done receipts, and a cleanup marker.
Payloads hold desired file/state bytes only. Sources, credentials and old user
file copies are not archived. Payload names, identities, sizes and hashes,
complete path topology and checkpoint order are revalidated before recovery may
act. Persisted paths never become unchecked execution authority.

## Checkpoint ordering and forward recovery

Desired payloads are exclusively created and synced before the initial journal
is published. Each journal generation is written and synced to a private
exclusive file, checked against the prior publication, atomically renamed, and
followed by a state-directory sync.

An effectful action follows this order:

1. Publish and sync Pending.
2. Reobserve and verify the selected destination and parent binding.
3. Replace or unlink the file, sync its parent, and observe the desired state.
4. Reobserve the complete action sequence and publish Done through the recovery
   acknowledgement path.

Each recovery iteration qualifies observations and metadata for the whole
sequence before asking the pure recovery planner for a step. This includes Done
and unchanged Queued actions, existing parents, and destination UID/GID, before
any effect, receipt advancement or cleanup. Pending Before permits application;
Pending After or Both permits acknowledgement. Acknowledgement resyncs the
observed file, when present, and its parent directory before Done, covering an
interruption or error after the namespace effect but before its sync. Queued
Both uses a receipt-only checkpoint; Queued After and unexpected values require
reconciliation. Recorded completion and currently verified completion are kept
separate. Final State follows the identical protocol.

After every action is Done, cleanup first publishes `cleanup_started` in a new
durable generation. This permits later recovery to recognize completion after
some payloads have already been deleted. Remaining payloads are removed only
when their names, identities and bytes match the saved evidence. The journal is
removed and its directory synced last. Unknown files and the private store's
owner/ignore records remain. Cleanup conflicts preserve the completed receipt.
An interruption before initial publication can leave private setup or payloads;
recovery with no published operation performs no output work and does not
collect arbitrary orphan files.

## Guarantees and qualification evidence

This backend relies on qualified local Linux/macOS filesystem operations and
explicitly rejects unsupported hosts/forms. Scope locks coordinate cooperating
writers, including recovery; process exit releases the lock. Rechecks and pinned
handles detect observed external changes, but cannot prevent every namespace
race or make a multi-file operation atomic.

The focused tests beside each component cover request validation, exact
preconditions, ordinary modes, metadata rejection, journal corruption, payload
identity, manual edits and prefix reporting.
[`process_tests.rs`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/process_tests.rs)
terminates real child processes at intent/effect/sync/receipt, initial journal
publication, generation publication and completed-cleanup boundaries. It checks
exact final bytes and modes, retained file identity after observed effects,
writer-lock release, and preservation of unexpected edits and extra files. A
compiled Pending-omission control was observed to fail these assertions. These
are process interruption checks, not hardware power-loss tests.

The local full test suite and Rust 1.85 all-target check passed for the
candidate implementation. Independent review identified parent-preflight, error
prefix and public-export findings; each has a regression fix. Final
`just verify`, all-target Clippy and the Rust 1.85 all-target check passed after
those fixes. Exact-head hosted results are retained in the gate-4 evidence
record. Configuration alone is not execution evidence. Durable lifecycle state,
schema-2 activation, legacy adoption and native Codex qualification remain
separate work.

See [AD-0033](../adr/0033-qualified-file-execution-and-journals.md) and
[specification §4.12](https://github.com/driftsys/upskill/blob/main/docs/specification/specification.md#412-qualified-file-execution-and-recovery).

Validation results and tested source identities are retained in the
[gate-4 evidence record](https://github.com/driftsys/upskill/blob/main/docs/technotes/2026-10-15-m1-gate4-validation.md).
