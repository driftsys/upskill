# Execution and recovery

This concern owns the requirements and implemented architecture below. Moving
these records does not change their implementation or approval status.
Library-only capabilities remain separate from production CLI activation; each
section records its own limits. Accepted decisions remain in
[ADRs](https://github.com/driftsys/upskill/blob/main/docs/adr/README.md).

## Requirements

<a id="411-ordered-recovery-checkpoint-model"></a>

### Ordered recovery checkpoint model

`operations::recovery::plan_recovery(&[RecoveryAction])` is an inactive pure
library API. Supplied File and State actions contain Queued/Pending/Done
checkpoints and Before/After/Both/Neither relations to exact expected values.
Exactly one State must be final. Empty inputs reject; state-only operations are
valid. The complete checkpoint order must match `Done* Pending? Queued*` before
observations are classified. Structural errors identify the required State
condition or invalid position.

Done requires After or Both; Pending accepts Before, After or Both; Queued
requires Before or Both. Every observation is checked, including the suffix. The
first conflict returns Reconcile with its index and both prefixes.
`recorded_prefix` preserves the count of leading Done checkpoints even when
contradicted. `verified_post_prefix` counts consecutive leading Done post-state
observations, extended through one Pending After or Both, and stops at the first
mismatch. Queued never extends it.

When all actions are Done, Complete includes the final State. Otherwise Ready
returns one step at the first unfinished index and both prefix counts. Pending
Before and Queued Before propose Apply; Pending After or Both proposes
Acknowledge; Queued Both proposes CheckpointOnly. Apply carries no physical
write authority, and the function changes no checkpoint. Public model types
derive Debug, Clone, Copy, PartialEq and Eq.

Supplied classifications are not evidence capabilities. The API performs no I/O,
serialization, journal reading, qualification or effects. The opt-in executor in
[qualified file execution and recovery](0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery)
supplies qualified file application, receipts and recovery using this model. No
lifecycle caller or schema-2 writes are activated; M1-A2, M1-A3 and M1 remain
incomplete. See
[AD-0032](https://github.com/driftsys/upskill/blob/main/docs/adr/0032-model-recovery-checkpoints-before-effects.md)
and the [recovery model](#architecture-file-recovery-model) for a public API
example.

<a id="412-qualified-file-execution-and-recovery"></a>

### Qualified file execution and recovery

`filesystem::ExecutionRequest` supplies a complete ordered sequence of
`PlannedFile` values and one distinct final State file that remains present. Its
effect-free validator rejects invalid or cross-scope paths, duplicate or
ancestor destinations, reserved journal paths, unknown/special permission bits,
changed existing ordinary modes, and new-file modes other than selected 0o600 or
0o700. The caller remains responsible for ownership and complete planning.

`QualifiedScope::apply_operation` requires its matching live writer guard and
verifies all supplied source snapshots and exact destination preconditions
before the first output effect. `recover_operation` requires the same authority
and uses validated persisted payloads without fetching sources. Both are opt-in
library methods; current CLI commands and schema-1 lockfiles do not use them.
Unsupported operating systems reject explicitly. Supported Linux/macOS paths
retain the existing qualification requirements.

`QualifiedScope::apply_operation_with_backup` prepends the exact original
`state.before` bytes as a private mode-0o600 backup action. It requires the
backup to be absent or byte- and mode-identical, validates the full request
before effects, and atomically publishes new backups without replacement.
Matching existing backups are synced with their parent and retain identity.
Backup operations use execution-journal schema 2: exactly one create-only first
action is bound to the final State's original byte digest and length. Ordinary
journal schema 1 remains supported. Recovery verifies the backup before
subsequent effects; edits require reconciliation, and a matching Pending
publication is acknowledged without rewriting. Report prefixes include the
backup at index zero. The caller validates the lifecycle transition; this
primitive does not parse original JSON or enable schema-2 production writes.

Effects replace files through exclusive sibling staging, file sync, atomic
rename and parent-directory sync, or unlink individual planned files and sync
their parents. Rechecks precede each effect. Unchanged files retain identity;
directories, extra files and empty shared aggregates are retained. Existing
ordinary modes are preserved. Replacement verifies unchanged UID/GID. ACLs,
special bits, Darwin file flags and unsupported extended attributes reject;
Darwin provenance is allowed only with an equal value on prior and staged files.

The private `<scope-root>/.upskill/state/` store binds root and directory
identities. Its directory is 0o700 and private files are 0o600. The root and
store parent must not allow group/other writes. Both execution-journal schemas
record the complete ordered actions, before/after digests, lengths and modes,
bound desired payloads, final State, generations and Queued/Pending/Done
receipts. Invalid paths, unsupported records, payload changes and mismatched
storage bindings reject without output effects. Neither journal schema activates
a new lifecycle lockfile schema.

Desired payloads are synced before initial journal publication. Each generation
uses staged-file sync, atomic publication and directory sync. Persist Pending
before each effect, sync and reobserve its post-state, then persist Done. A
Pending post-state is resynced, including its parent directory, before recovery
acknowledges it; it is not rewritten. A Queued post-state or unexpected value
requires reconciliation. No-op actions use receipt-only checkpoints, except
matching backup prerequisites which are synced before Done. Final State uses the
same protocol. Reports distinguish the recorded Done prefix from the currently
verified post-state prefix.

Completed cleanup is durably marked before removing validated payloads. The
completed journal is removed last; conflicting or unknown files are preserved.
Read-only observation creates no execution artifacts. Recovery with no published
operation performs no output work. Interrupted initial setup may retain private
orphan files rather than deleting content without established authority.

Advisory locking coordinates cooperating writers, not all external edits;
rechecks cannot exclude every namespace race. Operations are not atomic across
files. Process interruption coverage does not establish hardware power-loss
durability. Gate-4 hosted evidence is retained in its
[validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate4-validation.md).
Combined lifecycle integration and M1 activation remain separate work. See
[AD-0033](https://github.com/driftsys/upskill/blob/main/docs/adr/0033-qualified-file-execution-and-journals.md)
and the [qualified execution design](#architecture-qualified-file-execution).

## Architecture

<a id="architecture-file-recovery-model"></a>

<a id="architecture-file-recovery-model-file-recovery-checkpoint-model"></a>

### File recovery checkpoint model

This design describes the implemented inactive pure library API.

`operations::recovery::plan_recovery` consumes a borrowed slice of supplied
`RecoveryAction` values and returns `anyhow::Result<RecoveryDecision>`. Each
action has an `ActionKind` (`File` or `State`), `Checkpoint` (`Queued`,
`Pending`, `Done`) and `ObservedRelation` (`Before`, `After`, `Both`,
`Neither`). All public model types derive `Debug`, `Clone`, `Copy`, `PartialEq`
and `Eq`.

<a id="architecture-file-recovery-model-supplied-classifications"></a>

#### Supplied classifications

File represents one complete planned file edit, including an instruction
aggregate. State represents the final ownership-state checkpoint without
choosing its schema or path. Exactly one State action must be final; state-only
operations are valid. Checkpoints must follow `Done* Pending? Queued*`. The
complete sequence is structurally validated before any observation decision;
invalid structure returns a contextual error with the position or required State
condition. Diagnostic ordering is unspecified when multiple structural
conditions are invalid.

Observations compare supplied current values to exact expected before and after
values. Before and After match only their respective values; Both matches both
and represents an unchanged action; Neither matches neither. These
classifications are not qualified filesystem evidence. Native operations are not
representable.

Every Done action must match After or Both. Pending may match Before, After or
Both. Queued must match Before or Both: a coincidental unstarted After
observation does not establish application. All observations are checked,
including the suffix after the next proposed step. The first conflict in
operation order returns
`Reconcile { index, recorded_prefix, verified_post_prefix }`.

<a id="architecture-file-recovery-model-prefixes-and-next-decisions"></a>

#### Prefixes and next decisions

`recorded_prefix` counts leading Done checkpoints even if current observations
contradict them. `verified_post_prefix` counts only the consecutive leading Done
post-state observations, extended by one Pending action at After or Both. It
stops at the first mismatch; Queued never extends it, including Both.

After full validation, `Complete { completed }` reports the number of actions
when all are Done, including State. Otherwise `Ready` returns both prefixes and
one `RecoveryStep` at the first unfinished index:

- Pending Before or Queued Before proposes `Apply { index }`.
- Pending After or Both proposes `Acknowledge { index }`, avoiding repetition.
- Queued Both proposes `CheckpointOnly { index }`, with no physical write.

For a state-only operation supplied as Pending and After, the decision is Ready
with recorded prefix 0, verified post-state prefix 1 and Acknowledge at index 0.
The pending final state write is recognized without proposing it again. See the
[public model and planner](https://github.com/driftsys/upskill/blob/main/src/operations/recovery.rs)
and the
[literal acceptance cases](https://github.com/driftsys/upskill/blob/main/tests/recovery_plans.rs).

Apply is a proposed logical step, not permission to write. The function changes
no checkpoint and skips no action. The opt-in
[qualified executor](#architecture-qualified-file-execution) obtains fresh
qualified facts and advances durable checkpoints before another application
decision.

<a id="architecture-file-recovery-model-evidence-and-remaining-contracts"></a>

#### Evidence and remaining contracts

[`tests/recovery_plans.rs`](https://github.com/driftsys/upskill/blob/main/tests/recovery_plans.rs)
pins all twelve checkpoint/relation combinations independently for both roles,
whole-suffix validation, first-conflict selection, distinct prefixes and final
State handling. Independent compiled semantic controls establish that these
assertions detect omitted guards and altered decisions.

This pure API performs no I/O, serialization, journal reading, byte comparison,
locking or effects. The opt-in
[qualified executor](#architecture-qualified-file-execution) supplies physical
observations, private desired payloads, durable receipts and file application in
the restricted metadata envelope recorded by
[AD-0033](https://github.com/driftsys/upskill/blob/main/docs/adr/0033-qualified-file-execution-and-journals.md).
The
[permission-selection policy](0007-filesystem-access.md#architecture-file-permission-policy)
remains separate. Hosted executor qualification, lifecycle state integration and
activation are pending; M1-A2, M1-A3 and M1 remain incomplete. See
[AD-0032](https://github.com/driftsys/upskill/blob/main/docs/adr/0032-model-recovery-checkpoints-before-effects.md).

<a id="architecture-qualified-file-execution"></a>

<a id="architecture-qualified-file-execution-qualified-file-execution-and-recovery"></a>

### Qualified file execution and recovery

This design describes the opt-in library executor in
[`src/filesystem/execution/`](https://github.com/driftsys/upskill/blob/main/src/filesystem/execution/mod.rs).
Current commands do not call it, and it does not enable lifecycle-schema-2
writes or establish installation ownership. Gate-4 delivery evidence is retained
in its
[validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate4-validation.md);
combined lifecycle integration and M1 activation remain separate work.

<a id="architecture-qualified-file-execution-request-and-authority-boundary"></a>

#### Request and authority boundary

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

<a id="architecture-qualified-file-execution-exact-original-state-backup-prerequisite"></a>

#### Exact original-state backup prerequisite

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
[AD-0037](https://github.com/driftsys/upskill/blob/main/docs/adr/0037-require-create-only-backup-journals.md).

<a id="architecture-qualified-file-execution-components-and-file-effects"></a>

#### Components and file effects

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

<a id="architecture-qualified-file-execution-private-storage-and-strict-journal"></a>

#### Private storage and strict journal

The store is `<scope-root>/.upskill/state/`. The root and `.upskill` parent must
not be writable by group or others. The state directory is 0o700, is on the
qualified mount, and has ownership and physical bindings checked. Private files
are 0o600, single-link regular files with qualified metadata and bindings.
`owner.json` binds the root and directory identities. A generated `.gitignore`
contains `*` and `!.gitignore`. Existing state without matching owner and ignore
records rejects; arbitrary existing content is not adopted.

`operation.json` uses execution-journal schema 1 for ordinary operations and
schema 2 for backup operations, independently of the CLI lockfile schema. Strict
records reject unknown fields and contain a random operation ID, generation,
root identity, ordered action paths, a final State marker, before/after SHA-256
digests, lengths and ordinary modes, payload names and physical identities,
Queued/Pending/Done receipts, and a cleanup marker. Payloads hold desired
file/state bytes and, for backup operations, exact original State bytes as the
desired backup. Source captures and old output copies for rollback are not
retained. Original State can contain sensitive user data; the explicit backup
preserves those bytes in private storage. Payload names, identities, sizes and
hashes, complete path topology and checkpoint order are revalidated before
recovery may act. Persisted paths never become unchecked execution authority.

<a id="architecture-qualified-file-execution-checkpoint-ordering-and-forward-recovery"></a>

#### Checkpoint ordering and forward recovery

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
Both uses a receipt-only checkpoint for ordinary actions; an unchanged backup
prerequisite is synced with its parent before Done. Queued After and unexpected
values require reconciliation. Recorded completion and currently verified
completion are kept separate. Final State follows the identical protocol.

After every action is Done, cleanup first publishes `cleanup_started` in a new
durable generation. This permits later recovery to recognize completion after
some payloads have already been deleted. Remaining payloads are removed only
when their names, identities and bytes match the saved evidence. The journal is
removed and its directory synced last. Unknown files and the private store's
owner/ignore records remain. Cleanup conflicts preserve the completed receipt.
An interruption before initial publication can leave private setup or payloads;
recovery with no published operation performs no output work and does not
collect arbitrary orphan files.

<a id="architecture-qualified-file-execution-guarantees-and-qualification-evidence"></a>

#### Guarantees and qualification evidence

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

See
[AD-0033](https://github.com/driftsys/upskill/blob/main/docs/adr/0033-qualified-file-execution-and-journals.md)
and
[qualified file execution and recovery](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery).

Validation results and tested source identities are retained in the
[gate-4 evidence record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate4-validation.md).

### Guarded lifecycle application

The opt-in `lifecycle::application` boundary binds a complete owned-artifact
plan to the combined state written by the same qualified execution request. It
requires schema-1 state evidence for the first schema-2 transition and requires
the resulting state's artifact records to equal the planner output. The request
is applied under the matching `QualifiedScope` writer guard. The existing
executor creates the private exact-byte backup before output actions and uses
its durable journal for state-last recovery.

`lifecycle::application::recover` delegates to the same qualified executor
recovery API under its matching writer guard. It does not perform inspection or
recovery without an explicit caller invocation.

See the
[validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-guarded-lifecycle-application.md)
for backup and recovery examples and remaining Gate 5 work.

## Origin-bound lifecycle application

`lifecycle::application::apply_bound_schema2_write` accepts captured optional
state and backup values, a complete file plan, proposed combined state and
retained source snapshots. It requires canonical lockfile/backup paths,
validated state and complete artifact agreement. A matching scope writer guard
and executor checks remain necessary; the records alone grant no authority.

For an absent lockfile, proposed origin must be `fresh`, and the backup must be
absent. The executor receives a backup-absence no-op and a new private mode-0600
state action. For observed schema 1, the proposed migrated origin must bind its
exact bytes and retain its typed snapshot; existing create-only backup execution
runs before output effects. For observed schema 2, origin and legacy history
cannot change. A migrated backup must match the original digest/length and mode
0600, while fresh state continues to require backup absence. Unknown provenance
requires reconciliation rather than a guessed migration.

This entrypoint reuses existing source/destination rechecks and checkpointed
recovery. It does not recover before preparation, resolve sources or activate
production commands. Those applying adapter responsibilities and complete Gate 5
acceptance remain pending. See
[AD-0038](https://github.com/driftsys/upskill/blob/main/docs/adr/0038-bind-lifecycle-origin-to-original-backup.md).
