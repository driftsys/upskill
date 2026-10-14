# Pure whole-file planning before lifecycle effects

AD-0021

## Context

Whole-file operation plans are needed alongside marked instruction planning
before combined state and execution are qualified. The API must make replacement
scope, identity and deterministic output explicit.

## Options

The working design considered writes in the historical pipeline, a separate pure
planner, and combined source classification, rendering and execution.

## Decision

Provide a separate borrowed-input planner of explicitly enumerated file sets and
exact observations, returning owned records and individual file actions. Defer
source derivation and mixed-plan execution.

A Set is the complete nonempty desired file set for its installation; use Remove
for its final files. Repeated installation IDs retain source, kind, source name
and installed alias, while revision may change consistently. Key and sort
locations by scope, exact root OsStr spelling and portable relative path,
without normalization or filesystem-equivalence claims.

Retain prior row positions for untouched rows and same-location replacements,
drop removed rows and append new rows in lexical location order. Sort actions by
the same key.

## Consequences

The pure boundary can be reviewed independently without enabling writes. The
cost is another inactive prerequisite until qualified coordination and execution
exist. Callers must derive complete desired sets and use a new installation
identity for a new binding. Revision-only changes still retain observed no-op
preconditions.

Lexically distinct roots remain distinct, including dot-component spellings;
later qualification must establish actual equivalence and containment. Ordering
aligns with the instruction planner while leaving caller-supplied prior order
visible until serialization.

Satisfies:
[Specification §4.4](../specification/specification.md#44-owned-whole-file-operation-plans).
Related: [whole-file planner design](../design/owned-file-planning.md).
Rationale:
[archived spec](../archive/specs/2026-10-12-owned-file-planning-design.md).
