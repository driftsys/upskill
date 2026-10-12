# Reject affected file and descendant topology

AD-0023

## Context

Supplied absent-or-file observations do not qualify parent-directory transitions.

## Options

Alternatives not documented.

## Decision

Reject strict slash-component ancestor pairs in the union of prior and desired artifact locations within the same exact root and scope whenever either is affected. Include contributions and removal-to-directory transitions such as removing a while creating a/b.

## Consequences

Same-location replacement and component-prefix siblings remain possible. Disjoint untouched pairs do not block work. The cost is that directory topology changes require a separate qualified contract.

Satisfies: [Specification §4.4](../specification/specification.md#44-owned-whole-file-operation-plans).
Related: [whole-file planner design](../design/owned-file-planning.md).
Rationale: [archived spec](../archive/specs/2026-10-12-owned-file-planning-design.md).
