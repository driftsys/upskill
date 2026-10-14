# Compose complete owned-artifact plans before lifecycle effects

AD-0027

## Context

One installation can own both aggregate instruction contributions and dedicated
files. The existing pure planners each replace one form and cannot independently
establish a complete mixed installation change, cross-form binding stability or
physical safety across their combined locations.

## Options

The accepted design considered a pure coordinator over both reviewed planners,
independent caller-side invocation and concatenation, and replacing both
planners with one implementation. Independent concatenation leaves omitted
forms, cross-form bindings and collisions to every caller. Replacing the
planners rewrites reviewed editing and evidence behavior before lifecycle
integration requires that cost.

## Decision

Provide one pure complete-operation coordinator that validates global ownership,
bindings, physical form membership, affected topology and observations, then
partitions complete desired sets and invokes the existing planners. Reconstruct
and validate a single deterministic ownership vector and physical action vector.
Reuse existing file action DTOs, preserve aggregate mode evidence and retain
prior interleaved ownership order. Reuse the unchanged stable-binding validator
through narrow crate-private visibility rather than duplicate policy.

## Consequences

Callers can review one complete mixed operation without effects or successful
partial plans. Omitted forms are removed explicitly, and disjoint-form changes
retain stable binding checks. Cross-form same-location conversions remain
conflicts because contribution ownership grants no authority over manual bytes.
The coordinator adds crate-internal coupling to the file planner's validator.
Its supplied values establish no source, filesystem, permission or execution
authority; compiler integration and lifecycle activation remain separate work.

Satisfies:
[Specification §4.6](../specification/specification.md#46-mixed-owned-artifact-operation-plans).
Related: [mixed planning design](../design/mixed-owned-artifact-planning.md),
[AD-0016](0016-pure-instruction-planning.md),
[AD-0021](0021-pure-file-planning.md),
[AD-0024](0024-pure-captured-item-compilation.md).
