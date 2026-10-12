# Documentation archive

This directory retains completed or superseded working documents for provenance.
An archive entry records its source paths, delivered or superseding commit or PR,
durable successor records, and any unfinished scope retained in WIP.

No historical feature plans have been classified as complete by the documentation
refactor. The [WIP inventory](../wip/README.md) tracks their reconciliation.
Archive each feature only after its completion evidence has been checked; dates,
approval labels and checked boxes are insufficient on their own.

## Legacy-state projection

Source paths:
`docs/wip/specs/2026-10-12-legacy-state-projection-design.md` and
`docs/wip/plans/2026-10-12-legacy-state-projection.md`.
The raw [spec](specs/2026-10-12-legacy-state-projection-design.md) and
[plan](plans/2026-10-12-legacy-state-projection.md) are retained unchanged.

Implementation commit: `30cdabd7146b45b457756b38b024a55ec3a00de2`.
Final implementation and regression-test commit:
`52f72bcae142e302587f493dd0c418169e22478c`.
Completion evidence: the task's full verification and Rust 1.85 all-targets
check passed; the final test-only fix passed full verification and scoped task
review. These are local implementation commits, not a claim of a PR merge.

Durable successors:
[specification §4.1](../specification/specification.md#41-pure-legacy-state-projection),
[generation design](../design/generation-pipeline.md#pure-legacy-state-projection)
and [AD-0015](../adr/0015-pure-legacy-state-projection.md).
The [command guide](../book/commands.md) describes the unchanged
command and persistence boundary.

The [foundation](../wip/specs/2026-10-11-agent-customization-foundation-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active:
source verification, historical rendering and resources, adoption,
requested-selection reconciliation, schema-2 lifecycle preservation, exact
backup and transactional execution are unfinished. Other WIP material retains
its existing disposition; this archive entry concerns only the named pair.

## Owned instruction operation planning

Source paths: `docs/wip/specs/2026-10-12-owned-instruction-planning-design.md`
and `docs/wip/plans/2026-10-12-owned-instruction-planning.md`.
The archived [spec](specs/2026-10-12-owned-instruction-planning-design.md) and
[plan](plans/2026-10-12-owned-instruction-planning.md) retain the original material;
parent-context links follow their active WIP destination and the completed plan
explicitly qualifies its historical RED sequencing deviation.

Implementation commit: `0a0fb93dd83589cb554df8d2d302d7bfdba40105`.
Final acceptance-test and documentation commit:
`7b8b52d3a09b66645a72b0a02647c58b9fa80c4d`.
Local completion evidence: ordered formatting and full verification passed with
725 Rust tests and 11 documentation-checker tests; Rust 1.85 all-targets checking
passed. Fresh task review approved functional quality without functional blockers.
These commits and checks do not claim a PR merge. Compile RED preceded code;
valid behavioral assertion RED followed an implementation draft. Subsequent stub
assertions and semantic mutation checks validate current behavior, while the
historical test-first process exception remains explicit in the plan.

Durable successors:
[specification §4.2](../specification/specification.md#42-pure-owned-instruction-planning),
[planner design](../design/owned-instruction-planning.md),
[AD-0016](../adr/0016-pure-instruction-planning.md),
[AD-0017](../adr/0017-shared-instruction-owner-evidence.md) and
[AD-0018](../adr/0018-retain-empty-instruction-aggregates.md).

The [operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active.
Closeout status recorded before the whole-file planner:

> Source/filesystem evidence, complete reference authority, per-owner consumer
> association, whole-file/resource planning, mixed-state lifecycle integration and
> transactional execution remain pending. This entry resolves only the named pair;
> other WIP material retains its existing disposition.

Current successor: the [pure whole-file planner](../specification/specification.md#44-owned-whole-file-operation-plans)
is implemented for explicit complete file sets. Source-derived resource sets,
mixed instruction/file composition, qualified evidence, durable state and
transactional execution remain pending. The operational lifecycle design and M1
plan remain active; this entry does not archive the whole-file planning pair.

## Strict captured source manifests

Source paths: `docs/wip/specs/2026-10-12-strict-source-manifests-design.md`
and `docs/wip/plans/2026-10-12-strict-source-manifests.md`.
The archived [spec](specs/2026-10-12-strict-source-manifests-design.md) and
[plan](plans/2026-10-12-strict-source-manifests.md) retain the original material;
the spec's parent-context link follows its active WIP destination.

Implementation commit: `daee42f176d8f0a54e6c810f63f454e45083de96`.
Final task correction: `2247e818edf9a4900f4e64c7c9469cf1b21823c7`.
Local completion evidence: fresh task review approved without findings; ordered
formatting and full verification passed with 763 Rust tests and 11 documentation
tests, and Rust 1.85 all-targets checking passed. These local commits and checks
do not claim a PR merge. Compile and valid behavioral assertion RED preceded
implementation; semantic mutations exposed skipped entries/errors and digest
framing, mode, directory-path and inclusion regressions, with exact restoration.

Durable successors:
[specification §4.3](../specification/specification.md#43-strict-captured-source-manifests),
[source manifest design](../design/strict-source-manifests.md),
[AD-0019](../adr/0019-separate-strict-source-capture.md) and
[AD-0020](../adr/0020-framed-source-manifest-evidence.md).

The [operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active.
Closeout status recorded before the whole-file planner:

> Whole-file/resource planning, immutable Git evidence, source stabilization,
> qualified observation, destination qualification, state/execution and Codex
> activation remain pending. Actual non-UTF-8 filesystem-name creation was
> unsupported on the development macOS filesystem; pure conversion rejection was
> tested there, with the real fixture gated to other Unix hosts. This archive entry
> changes no other WIP disposition.

Current successor: the [pure whole-file planner](../specification/specification.md#44-owned-whole-file-operation-plans)
is implemented for explicit complete file sets. Source-derived file/resource sets,
mixed instruction/file composition, immutable Git evidence, source stabilization,
qualified observation, destination qualification, state/execution and Codex
activation remain pending. The source-manifest platform evidence above is retained
as recorded; this entry does not archive the whole-file planning pair.
