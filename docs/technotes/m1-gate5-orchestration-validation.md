# Source-backed lifecycle orchestration validation

`lifecycle::orchestration::with_prepared` is an opt-in library boundary for
explicitly selected rule item roots. It opens `LifecycleSession` under the
caller-retained matching writer guard before source work. A recovery report
stops the new request. Complete literal/all selection preflight blocks selected
legacy or unsupported owned nodes before any resolver fetch. No discovery,
production command activation or native Codex compilation is provided.

Add accepts explicit source names and aliases. Update reconstructs requested
source bindings from selected owned installations, retaining their IDs and
reference graph. Actual resolved Git commits and exact manifest digests are
recorded separately from requested source labels. Remove uses recorded direct
root references through the captured command adapter without resolving sources
or interpreting current preferences as history.

The scoped callback receives an opaque `SourcePrepared`. Resolvers, temporary
checkouts, captured source bytes, compiler bindings and the matching guarded
prepared operation remain alive throughout that callback. Its consuming `apply`
revalidates every full `CapturedRuleSource`, including Git revision, immediately
before the underlying prepared operation applies. Callers cannot substitute a
capture or compiled output after preparation. `Ready(T)` describes the callback
result; a callback that discards preparation has applied nothing.

Stable IDs are full SHA-256 values over a versioned JSON tuple containing scope,
raw root bytes, normalized requested source label, rule kind, source name and
installed alias. Git commit evidence is excluded. Existing matching bindings
reuse recorded IDs; different bindings with the computed ID reject. Repeated add
selects existing owners and adds only absent direct references.
Different-binding alias reuse rejects before resolver fetch/capture. Local
preflight canonicalizes ancestors while preserving the selected final entry for
qualification.

Target precedence is invocation, nonempty project configuration, nonempty global
configuration, then recorded update targets or caller-supplied add defaults.
Empty lists do not override history. Add without any effective selection rejects
rather than inventing a target default. Caller-supplied compiler bindings and
contract profiles are mandatory; compilation remains limited to existing Claude,
Copilot and OpenCode families. Requested targets, audience exclusions and actual
artifact consumers remain separate. The captured adapter chooses permissions and
keeps owner contracts consistent with resulting modes.

Acceptance examples cover local add/update/remove, exact revision/manifest
recording, alias and stable ID preservation, repeated add, Git branch
advancement through isolated native Git URL rewrites, target precedence and
configured update, source staleness after preparation, manual output conflict
with no effects, disjoint migrated legacy preservation and selection blocking,
audience-filtered empty output history, executable resources and manual
neighbors. Private identity examples pin raw-root framing, collision rejection
and prior-ID reuse independent of commit evidence.

Manual/shared instruction contributions and caller-supplied complete graphs
remain supported by the captured command API; this module does not introduce an
instruction compiler or bundle registry discovery. Full process interruption,
writer contention and checkpoint recovery acceptance belongs to the completion
lane. These examples alone do not complete Gate 5. Native qualification and
production activation remain Gate 6.

Validation completed on macOS with a dedicated target directory at
`/private/tmp/upskill-orchestration-target`. `just fmt`, escalated `just verify`
and explicit Rust 1.85 `just msrv-check` passed. Final focused acceptance passed
all eleven integration examples and the private identity example passed. The
full verification run includes the existing executor interruption/recovery
tests; this does not replace the completion lane's whole-lifecycle checkpoint
acceptance. The final fixture-isolation regression was added during full
verification and passed in a separate final focused run. Its deliberately unsafe
RED attempt was rejected by automatic approval review before execution; only
green evidence is claimed for that regression. Logs are under
`/private/tmp/upskill-orchestration-{red,unit,acceptance,fmt,verify,msrv}.log`.
Independent task review passed with no findings, as reported by the completion
lane. The completion lane also reports whole-lifecycle process acceptance across
162 kill checkpoints and two reconciliation passes, with an independent
acceptance-criteria audit finding no implementation blocker. Those results are
owned by its separate final-acceptance record; they are not additional coverage
from the eleven orchestration examples. Combined-tree full `just verify`
subsequently passed as recorded in the final acceptance note. Final delivery
review, hosted CI and orchestration merge remain pending. Native qualification
and production activation remain Gate 6.

The bounded orchestration plan is preserved unchanged in
[the plan archive](../archive/plans/2026-10-18-lifecycle-orchestration.md). The
[shared Gate 5 specification](../archive/specs/2026-10-15-m1-gate5-lifecycle-integration.md)
is archived after local combined acceptance; final hosted delivery remains
pending as recorded in the [acceptance note](m1-gate5-final-acceptance.md).
Durable requirements and design are in
[source-backed orchestration](../topics/0005-lifecycle-planning.md#source-backed-lifecycle-orchestration),
[requested source bindings](../topics/0003-sources.md#requested-bindings-and-scoped-application)
and
[origin-bound application](../topics/0008-execution-and-recovery.md#origin-bound-lifecycle-application).
