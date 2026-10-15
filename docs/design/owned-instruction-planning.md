# Owned instruction operation planning

This record describes the pure library planner implemented in
[`instructions::plan`](../../src/instructions/plan.rs). The
[specification §4.2](../specification/specification.md#42-pure-owned-instruction-planning)
owns its requirements. The planner has no CLI dispatch, state codec or I/O.

## Public boundary

`plan_instructions` borrows prior ownership records, installation changes and
observations, and returns an owned `InstructionOperationPlan` through
`anyhow::Result`. The public DTOs derive equality, cloning and debug support;
they have no serialization contract. An `InstructionLocation` combines an
`InstallationRoot` and portable relative path. `ObservedInstructionFile` holds
that location and optional exact bytes; absence differs from an existing empty
file. A set change carries an `ArtifactOwner` and its complete desired
contribution set, including Markdown, selector and consuming surfaces. A remove
change names an installation ID.

The result contains the complete artifact vector and `PlannedInstructionFile`
entries with exact optional before bytes and resulting bytes. File plans include
content no-ops and retain aggregates after final-region removal. Caller-supplied
observations establish preconditions for a later executor, without establishing
filesystem or source authority.
[AD-0016](../adr/0016-pure-instruction-planning.md) records this boundary.

## Validation and location grouping

The planner validates prior records with `ownership::validate_artifacts`,
indexes installation bindings, and validates changes and desired contributions
before editing. Existing bindings compare source, item kind, source name and
installed alias explicitly; revision may change. Desired data passes through the
existing editor and ownership validator, including paths, owner metadata and
consumers. Duplicate changes, empty sets and unknown removals fail with context.

Affected locations are the union of prior locations involving changed owners and
desired locations. Their key is scope, native `OsString` root spelling and
portable relative path. Ordered maps and sets retain exact lexical identity
without canonicalization, alias detection or containment checks. Observations
must match that set exactly. A prior owned region cannot be reconciled from
absent bytes; observed aggregates must decode as UTF-8.

## Verified body inspection and editing

[`aggregate::inspect_owned`](../../src/instructions/aggregate.rs) shares the
private span validator with `aggregate::reconcile`. Verified spans retain body
start offsets and end-marker positions, so the reader takes exact body slices
and returns contributions in document order. It rejects duplicate IDs,
overlapping spans and invalid marker, hash or separator state through the
existing editor validation. There is no second marker parser.

At each affected location, the planner reads all verified bodies, retains
untouched selectors and combines them with the desired complete replacements. A
desired selector cannot take over an untouched owner. It calls reconciliation
once for the complete aggregate edit and joins returned selectors to the
retained or desired owner and consumer metadata. Returned contribution metadata
includes new separator context even for retained bodies. This allows subsequent
removal to restore manual bytes exactly.

A changed multi-owner contribution conflicts because the stored consumer union
does not identify each owner's selection. Untouched shared contributions remain,
including within an affected aggregate; separately owned regions are supported.
An affected whole-file record conflicts.
[AD-0017](../adr/0017-shared-instruction-owner-evidence.md) records the
shared-region restriction. Empty aggregates remain files because prior file
origin is unknown; see
[AD-0018](../adr/0018-retain-empty-instruction-aggregates.md).

## Result assembly and downstream limits

Surviving prior selectors retain their relative artifact-row order. Removed
selectors disappear; genuinely new selectors append in location and ID order.
Unaffected locations retain exact records. File plans sort by location key. The
complete result passes ownership validation before return; errors return no
partial plan and inputs remain borrowed and unchanged.

Callers must resolve direct/bundle references and native compatibility before
planning. The separate
[pure whole-file planner](../specification/specification.md#44-owned-whole-file-operation-plans)
is implemented for explicitly supplied complete file sets. The separate
[captured item compiler](captured-item-compilation.md) derives complete
per-client entrypoint and resource outputs from validated captured bytes.
Qualified scope roots, writer locking,
[destination observation](qualified-destination-observation.md) and
[qualified source observation](qualified-source-observation.md) are implemented
as separate inactive capabilities. Final-head hosted source qualification
remains required. The [combined-state validator](combined-lifecycle-state.md)
provides reference-graph authority and explicit per-owner consumer associations.
Removal planning integrates those associations and returns complete combined
state. Integrating qualified source evidence and compiled outputs with complete
add/update planning, persistence and guarded application remain pending. The
opt-in [qualified file executor and recovery](qualified-file-execution.md) are
implemented separately; hosted final-head qualification remains required. The
implemented [mixed coordinator](mixed-owned-artifact-planning.md) composes
complete explicit sets across both forms; neither individual planner authorizes
partial application of a mixed installation. Current commands, lockfile/source
formats and native client support are unchanged.

[`tests/instruction_plans.rs`](../../tests/instruction_plans.rs) covers owner
retention, complete replacements and moves, exact preconditions, conflicts,
consumer propagation, lexical scope/root identity and row ordering.
[`tests/instruction_contributions.rs`](../../tests/instruction_contributions.rs)
covers the shared reader and editor validation. These tests establish the pure
boundary, without native discovery or filesystem execution claims.

The opt-in [combined-state validator](combined-lifecycle-state.md) now checks
reference authority and explicit per-owner consumer associations.
Reference-driven removal planning now connects this state to shared-owner
retirement. Add/update planning and guarded execution remain pending.
