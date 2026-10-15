# Lifecycle reference reconciliation

`lifecycle::reconcile::reconcile_references` is an inactive pure graph boundary.
It consumes a validated `CombinedState` and explicit `GraphChanges`, and returns
`ReconciledGraph`. It performs no I/O, source fetch, ownership adoption,
migration, backup, application or schema-2 production write.

## Interfaces and components

The public [reconciliation module](../../src/lifecycle/reconcile/mod.rs) exposes
`GraphChanges`, which carries node upserts and exact edge edits, and
`ReconciledGraph`, which carries surviving graph records and removed IDs. The
private [reachability traversal](../../src/lifecycle/reconcile/reachability.rs)
indexes bundle dependencies and memberships and follows direct roots using an
iterative queue. It keeps installation and bundle reachability separate even
when IDs have the same spelling.

The boundary reuses the combined-state validator and whole-file binding checks.
Validation-only graph values omit artifacts, consumer associations and legacy
state after the complete prior state has been validated. Those temporary values
do not replace the complete state or grant authority to apply effects.

## Authority and retention

Only `DirectInstallation`, `DirectBundle`, `BundleInstallation` and
`BundleDependency` edges establish retention. An installation survives while any
direct or transitive bundle path reaches it. Removing one root of a diamond
retains shared dependencies reached through another root. Artifact count,
audience-filtered empty output, consumers and matching source labels are not
retention references. Bundle and installation IDs remain separate namespaces.

The result contains installations, bundles, references and removed IDs. It
contains no artifacts or tagged legacy snapshot and is not a final combined
state. A removed installation ID does not authorize deleting its files. Later
lifecycle planning must reconcile complete ownership, retain unrelated owners
and legacy data, and validate the complete final state before effects.

## Explicit edits

`GraphChanges` supplies installation/bundle upserts and exact reference
additions and removals. Duplicate upserts/edits, adding an existing reference,
removing an absent reference or adding/removing the same reference in one
request conflict. An empty changes value is a no-op; this exact edit API does
not infer intent from an incomplete replacement reference list.

Existing installation IDs retain source, kind, source name, installed alias,
scope and raw root spelling. Even a spelling-only root change conflicts.
Requested surfaces and captured evidence/revision may change subject to state
validation; later artifact planning must establish matching complete outputs.
Existing bundle IDs retain source and name, while revision evidence may change.
New IDs cannot bypass installation identity uniqueness.

Every supplied node and edge is validated before reachability pruning. Private
validation-only direct roots allow the existing state validator to check all
candidate nodes, dangling edges and cycles without treating intentional detaches
as unreachable-state errors. Those roots never escape or participate in actual
retention. Explicitly upserted nodes and added references must survive in the
result; conflicting update-and-final-detach requests fail rather than silently
discard updates.

Iterative traversal uses indexed dependency and membership edges. Unreachable
prior nodes and their outgoing edges are removed. Surviving prior nodes and
references retain order; new nodes append in lexical ID order and new references
in enum order. Removed ID lists follow prior record order. Final graph
validation runs again without validation-only roots. Inputs remain unchanged on
success and failure; there is no successful partial result after an error.

## Remaining integration

The complete prior state is validated, including artifact ownership/consumer
associations and tagged legacy schema. Graph reconciliation never mutates those
prior values and does not reconstruct them in its graph-only result.

The separate [removal planner](lifecycle-reference-removal.md) reconciles
shared-owner retirement. Shared add/update, instruction adapters, selected
legacy conflict aggregation and complete add/update plans remain task 2b work.
Existing mixed and instruction planners keep their affected multi-owner
rejection. Guarded lifecycle backup/application/recovery, observational adapters
and native activation remain separate gates. This boundary alone does not
complete task 2b, Gate 5, M1 or the updater prerequisite.

## Trace

Requirements are recorded in
[specification §4.15](../specification/specification.md#415-authoritative-lifecycle-reference-reconciliation).
The [public acceptance suite](../../tests/lifecycle_references.rs) establishes
retention, edit rejection, binding immutability, deterministic output and input
preservation.
[Validation evidence](../technotes/2026-10-16-m1-gate5-reference-validation.md)
records local checks and their limits.
