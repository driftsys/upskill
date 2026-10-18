# Captured lifecycle command adapters

`lifecycle::commands` connects the opt-in combined-state planners to guarded
application. It remains a library path; production commands still write
schema 1.

Callers acquire a `QualifiedScope` and its writer guard, then call
`LifecycleSession::open`. The session recovers the executor journal before
reading state. `SessionOutcome::Recovery` stops the new request even when
recovery completed: inspect the report and retry with a new session.
Reconciliation never falls through into new work. The ready session retains
opaque observations of the canonical lockfile and exact schema-1 backup. Absent
state receives fresh origin; schema 1 receives exact-byte migration origin;
unbound schema 2 is rejected.

`prepare_upsert` first resolves the complete supplied selection. Its callback
runs only after selection succeeds, and returns complete desired sets, explicit
contracts, bundles and reference edits. Every changed installation must name a
manifest represented by a supplied `SourceSnapshot`. All old and new output
locations are enumerated once into retained qualified evidence. Pure planners
receive only affected observations. Permission selection changes both desired
file values and their corresponding contracts. New generated files are private,
ordinary existing modes survive, and resource owner execution is explicitly
selected with `ResourceExecutionPolicy` (normally `Disabled`). Instruction
contributions select generated permissions from exact before-file evidence.

`prepare_remove` derives only direct references whose literal root names were
selected. Selecting a bundle does not remove independent direct roots on its
members or dependencies. Selecting a member which has only bundle references
returns `NoDirectReference`; callers should direct the user to the declaring
bundle. Complete selection preflight rejects legacy work before any effect.
Unselected graph records, manual instruction bytes, shared survivors and
individually unowned extra resources remain preserved by the existing planners.
State, backup and executor journal destinations cannot be content outputs.

A `PreparedOperation` privately owns the next state and file plan, borrows its
session and exact source snapshots, and retains qualified destination evidence.
Its consuming `apply` method cannot receive substitute state, snapshots or
guard. It revalidates every captured input before invoking
`apply_bound_schema2_write` under the original guard. The existing executor owns
all output/state checkpoints and recovery. No additional journal or writer
exists.

## Authority and remaining limits

Ruling **R-G5-CAPTURED-1**: complete desired sets, including typed instruction
contributions, are caller-authorized supplied values. Snapshot matching proves
captured manifest association and freshness; it does not authenticate arbitrary
supplied output as compiler-produced content. The subsequent source
orchestration slice must bind its rule outputs to the exact compiler result.
This boundary does not fetch sources, qualify native discovery, enable
production schema 2, or claim Gate 5 completion. Callers must open the session
before doing source work.

Focused acceptance covers fresh and migrated add/update/remove, exact backup,
manual instruction and extra-resource preservation, shared owner release, direct
root retention, complete legacy selection rejection, stale state/source/output,
permission contracts, reserved destinations, guard mismatch, writer contention
and recovery completion/reconciliation stopping new requests. Whole-lane
interruption acceptance and hosted independent review remain completion-lane
work.

## Delivery evidence

Local validation on 2026-10-18 passed 15 focused integration cases and the
recovery-stop unit example, full `just verify` (no failures, one existing
ignored test), all-target Clippy with denied warnings, and explicit Rust 1.85
`just msrv-check`. The independent task review reported PASS with no findings.
Full verification used the authorized environment for existing FIFO and process
recovery tests. Local verification and MSRV logs are retained at
`/private/tmp/upskill-lifecycle-commands-verify.log` and
`/private/tmp/upskill-lifecycle-commands-msrv-final.log` respectively; these are
local evidence paths, not published artifacts or hosted CI evidence.

After stacking on source commit `b0e1593`, final `just verify` and Rust 1.85
checking passed again. Logs are
`/private/tmp/upskill-lifecycle-commands-stacked-verify.log` and
`/private/tmp/upskill-lifecycle-commands-stacked-msrv.log`.

The pre-PR test wave also pins nonempty unrelated source rejection, matching
captures for every installation, mixed owned/blocked complete selection, stale
metadata and sources before builder invocation, bad backup rejection, and
instruction modes after update and removal. Negative controls replacing digest
matching with source-list nonemptiness and ignoring blockers with owned
candidates both fail their focused acceptance cases. Production source was
restored exactly. Wave evidence uses
`/private/tmp/upskill-lifecycle-commands-wave-*` logs.

The raw bounded plan is retained in
[the archive](../archive/plans/2026-10-18-lifecycle-commands.md). Its durable
architecture is in
[lifecycle planning](../topics/0005-lifecycle-planning.md#captured-lifecycle-command-adapters),
with execution in
[origin-bound application](../topics/0008-execution-and-recovery.md#origin-bound-lifecycle-application)
and the existing
[AD-0038](../adr/0038-bind-lifecycle-origin-to-original-backup.md). The active
Gate 5 umbrella and completion lane retain source orchestration, whole-lane
acceptance and hosted delivery. This bounded result does not claim that the
feature branch has merged or Gate 5 has completed.
