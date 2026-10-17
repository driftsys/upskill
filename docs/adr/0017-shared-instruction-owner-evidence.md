# Require owner evidence before changing shared instruction regions

AD-0017

## Context

Owned artifact records store owners and a union of consumers. That union cannot
recover each owner's consuming surfaces when one owner changes.

## Options

Inferring per-owner consumers from the union risks incorrect selection and
ownership. The documented bounded choice rejects affected shared regions while
retaining untouched shared records.

## Decision

Require a single prior owner for each contribution affected by a changed owner.
Preserve untouched multi-owner contributions, including at affected aggregate
locations. Continue supporting separately owned regions in one aggregate.

## Consequences

Changing shared regions needs later authoritative per-owner association. The
restriction avoids inventing evidence and does not drop unrelated ownership.

Satisfies:
[Specification §4.2](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#42-pure-owned-instruction-planning).
Related:
[planner design](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#architecture-owned-instruction-planning).
Rationale:
[archived spec](../archive/specs/2026-10-12-owned-instruction-planning-design.md).
