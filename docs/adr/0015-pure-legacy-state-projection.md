# Pure projection before legacy-state migration

AD-0015

**Status**: Accepted (2026-10-12)

## Context

Schema-1 records must remain inspectable before a schema-2 lifecycle writer can
replace them. Existing effective clients and source metadata cannot establish
requested native surfaces, immutable revisions or generated-byte ownership.
Migration execution and lifecycle preservation are not integrated yet.

## Options

Immediately rewrite schema-1 lockfiles as schema 2. Rejected because command
preservation, exact backup, writer locking and recovery are not integrated.

Add optional future-state fields to schema-1 records. Rejected because old
writers could silently lose new data and historical facts would mix with new
authority.

Introduce a pure projection retaining the typed snapshot, interpreted facts,
diagnostics and candidate destinations. Selected because future migration
planning can consume independently testable descriptions without enabling effects.

## Decision

Provide a pure library projection of typed schema-1 state before adding migration
effects. Retain indexed records and tagged native integrations, diagnose malformed
facts without dropping rows, and leave requested selection unresolved. Source
parsing does not verify revisions or ownership. Layout candidates remain
unobserved relative paths.

Freeze the original Claude, Copilot and OpenCode vocabulary and nine layouts
independently of future adapters. Reuse the pure native layout helpers in the
current pipeline while validating portable names only in the projector. Preserve
current commands, persistence and pipeline behavior.

## Consequences

The library gains an intermediate model that later lifecycle work must consume.
It preserves every typed field, rather than original JSON bytes or unknown JSON
fields. Unsupported schemas remain inspectable without interpretation. No
projection result authorizes mutation, adoption or native reconciliation; planners
must determine which unresolved facts matter for each later operation.

Source verification, reproducible historical rendering, resource enumeration,
requested-selection reconciliation, schema-2 preservation, exact backup and
transactional execution remain pending. This decision does not complete M1-A2.

Satisfies: [Specification §4.1](../specification/specification.md#41-pure-legacy-state-projection).
Related: [ADR-0003](0003-generation-pipeline.md),
[ADR-0011](0011-generic-git-url-source.md),
[design](../design/generation-pipeline.md#pure-legacy-state-projection).
Rationale: [archived feature spec](../archive/specs/2026-10-12-legacy-state-projection-design.md).
