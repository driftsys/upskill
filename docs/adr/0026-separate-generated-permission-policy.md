# Separate captured resource modes from generated permission policy

AD-0026

## Context

Captured resource permission evidence describes source files. A newly rendered
entrypoint has no observed destination mode, and the pure compiler performs no
destination observation or writes.

## Options

Alternatives not documented.

## Decision

Preserve each resource's optional captured mode exactly and return `mode: None`
for every generated entrypoint. Do not treat a source entrypoint's permissions
as a generated destination permission policy. A qualified writer must establish
that policy and persist observed mode evidence separately.

## Consequences

Resource evidence remains exact and distinguishable from generated-file policy.
The cost is deferred permission qualification before lifecycle activation.
`None` grants no authority to change destination permissions and is not evidence
that a destination's mode was observed.

Satisfies:
[Specification §4.5](../specification/specification.md#45-captured-item-compilation).
Related: [compiler design](../design/captured-item-compilation.md),
[AD-0024](0024-pure-captured-item-compilation.md). Rationale:
[archived spec](../archive/specs/2026-10-12-captured-item-compilation-design.md).
