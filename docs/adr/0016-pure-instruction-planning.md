# Pure instruction planning before lifecycle effects

AD-0016

## Context

Installation changes need independently testable aggregate edits and ownership
results before a complete mixed-state lifecycle and executor are available.

## Options

The bounded design selects a pure planner from resolved changes and
observations. Integration directly into lifecycle effects is deferred until its
evidence and execution gates are complete. Other alternatives are not
documented.

## Decision

Provide a pure borrowed-input library boundary returning complete artifact
records and exact file preconditions. Leave reference resolution, source
verification, native compatibility and all effects to later consumers.

## Consequences

The planner can be tested without enabling writes. Callers must supply complete
owner changes and exact affected observations; the result alone grants no
filesystem authority. CLI and persistence remain unchanged.

Satisfies:
[Specification §4.2](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#42-pure-owned-instruction-planning).
Related:
[planner design](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#architecture-owned-instruction-planning).
Rationale:
[archived spec](../archive/specs/2026-10-12-owned-instruction-planning-design.md).
