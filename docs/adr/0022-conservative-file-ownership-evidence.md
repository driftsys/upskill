# Require conservative file ownership and before-state evidence

AD-0022

## Context

Equal bytes do not establish ownership. Absent mode evidence cannot verify
recorded permission identity. Consumer unions do not establish per-owner
selection authority, and the file planner does not compose marked instruction
changes.

## Options

Alternatives not documented. The source records select conservative rejection at
this boundary and defer qualification and composition.

## Decision

Require present observed bytes matching the recorded SHA-256 and exact equality
of recorded and observed optional modes for owned updates, removal and no-ops.
None and Some(0) differ. Require absence at unowned desired locations even when
present bytes and modes equal the desired output. Same-location handoff requires
selection of both old and new owners and verified before-state.

Reject affected multi-owner whole-file records and Markdown contributions, even
when all shared owners are selected. Preserve unrelated rows exactly.

## Consequences

No implicit adoption or sharing follows from equal content. Cross-platform mode
discrepancies need later qualification and reconciliation; planning grants no
permission-application authority. Shared and mixed installations await
authoritative per-owner consumer association and composition. Untouched shared
files and contributions remain representable.

Satisfies:
[Specification §4.4](../specification/specification.md#44-owned-whole-file-operation-plans).
Related: [whole-file planner design](../design/owned-file-planning.md).
Rationale:
[archived spec](../archive/specs/2026-10-12-owned-file-planning-design.md).
