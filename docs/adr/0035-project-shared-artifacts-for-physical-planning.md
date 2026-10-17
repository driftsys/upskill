# Project shared artifacts for physical planning

AD-0035

## Context

Shared lifecycle reconciliation must preserve authoritative real ownership while
reusing existing observation, topology, permission and instruction aggregate
checks. The public mixed-artifact planner intentionally rejects shared mutation.

## Options

The implementation discussion chose private per-artifact projection to reuse the
existing physical algorithm rather than introduce another physical planner.
Other alternatives were not documented.

## Decision

Validate real immutable bindings, complete candidate state and matching evidence
for affected desired multi-owner artifacts before projecting one deterministic
private owner per logical artifact identity. Call the existing mixed planner
with that projection. Require exact result identities and normalized content
agreement, then map only physical content and instruction layout metadata back
onto real candidate owners. Validate the complete state again before returning.

## Consequences

Existing physical guards apply to shared planning without a new writer or
reference authority. Projection and remapping invariants need focused examples
and review. Private owners cannot escape to returned state, references or
contracts. Supplied observations confer no filesystem or native qualification;
guarded application remains separate work.

Satisfies:
[specification §4.20](../specification/specification.md#420-explicit-shared-output-reconciliation).
Related:
[shared upsert design](../design/lifecycle-upsert.md#private-physical-projection),
[AD-0034](0034-record-explicit-owner-contracts.md).

Pure owner release with no desired value at an identity preserves untouched
survivors and their unknown evidence without requiring agreement. Supplied
physical observations and all physical checks still apply.
