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
