# Mixed owned-artifact operation planning

This record describes the implemented pure coordinator in
[`operations::plan`](../../src/operations/plan.rs). [Specification §4.6](../specification/specification.md#46-mixed-owned-artifact-operation-plans)
owns its requirements. [AD-0027](../adr/0027-compose-complete-owned-artifact-plans.md)
records the accepted composition choice. The coordinator performs no I/O and is
not called by current installation commands.

## Public boundary

`plan_owned_artifacts` borrows prior `OwnedArtifact` records,
`InstallationChange` values and `ObservedFile` values, returning an owned
`FileOperationPlan` through `anyhow::Result`. `DesiredArtifact` distinguishes
`File(DesiredFile)` from `Instruction(DesiredInstructionContribution)`.
`InstallationChange` holds either a Set with one owner and desired artifacts,
or a Remove naming an installation. Both new enums derive `Debug`, `Clone`,
`PartialEq` and `Eq` without serialization. Existing file planner DTOs represent
both forms' physical actions with exact optional before/after bytes and modes.

A Set describes the installation's complete desired state across both forms.
The coordinator validates and partitions that operation before invoking the
reviewed [file planner](owned-file-planning.md) and
[instruction planner](owned-instruction-planning.md). It then reconstructs one
complete ownership vector and one sorted physical action vector. Failure at any
stage returns no successful combined plan; inputs remain unchanged.

## Complete installations and bindings

Complete prior ownership validation precedes indexing. Global installation
bindings are checked before forms split, so changing from file-only to
instruction-only ownership at a distinct location cannot bypass the source,
kind, source-name or installed-alias binding. Source revision may change
coherently. Duplicate changes, unknown removals and empty Sets conflict.

The original plan required existing helpers to remain unchanged. The accepted
implementation ruling instead exposes `files::plan::validate_binding` as
`pub(crate)` and imports it into the coordinator. Its body and existing planner
behavior remain unchanged. This narrow visibility expansion avoids duplicating
stable-binding policy, at the cost of crate-internal coupling to the file planner.

Prior rows partition by form while preserving their relative order. Each form
receives a nonempty Set when desired, a Remove when previously owned but now
omitted, or no change when both absent. A global Remove reaches every previously
owned form. Both planners receive complete form partitions rather than partial
mixed requests. Any multi-owner record at an affected physical location
conflicts, including another selector there; untouched shared rows survive.

## Locations and supplied evidence

Ordered keys combine ownership scope, raw root `OsString` and portable relative
path. Roots must be nonempty and absolute; lexical identity does not normalize
dot components or prove host equivalence, containment or realizability. The
coordinator indexes form membership throughout the prior/desired union and marks
old locations of changed installations plus every desired location as affected.

Every supplied observation is validated before membership checks, including root,
portable path and permission-bit range. Exactly one observation must match each
affected location, even for no-ops; missing, duplicate and extra evidence fail.
A physical location cannot have both forms anywhere in the union, including an
explicit removal and replacement. Contribution ownership does not prove authority
over surrounding manual bytes. Same-form handoffs retain each planner's rules.

Ancestor/descendant locations within the same exact root and scope conflict when
either endpoint is affected. This check covers both forms, removal-only changes
and old/new transitions. Slash-component boundaries distinguish `a/b` from `ab`.
Unrelated historical topology is preserved without a filesystem safety claim.

Observations partition using validated form membership. Dedicated values retain
bytes and optional modes intact. Instruction observations pass optional exact
bytes to the aggregate planner; physical mode evidence stays with the coordinator.
The existing planners enforce owned byte/mode evidence and verified aggregate
markers, including invalid UTF-8 rejection and exact manual-byte preservation.

## Combined result

The coordinator indexes planned ownership by exact location plus optional
contribution selector. It walks the original interleaved prior rows, replacing
retained identities at their former positions and dropping removed identities.
Remaining new identities append in key order. Final combined ownership validation
checks the union, including revision coherence.

Dedicated actions retain their exact before/after values. Aggregate actions use
the supplied observed before value and the editor's after bytes. Their after mode
is the observed optional mode, or `None` for an absent aggregate. Removing the
last contribution retains the aggregate, even when empty. Modes grant no
permission or execution authority. All affected actions, including byte-identical
no-ops, remain, sort by exact location and pass a duplicate-location guard.

## Evidence and remaining integration

[`tests/operation_plans.rs`](../../tests/operation_plans.rs) covers mixed creation,
complete replacements and removals, disjoint-form binding checks, coherent and
conflicting revisions, physical collisions and topology, exact observations,
manual bytes and aggregate modes, shared ownership, raw-root/scope identity,
interleaved row reconstruction and deterministic permutations. Literal hashes
and bytes pin changed contribution metadata and a subsequent no-op roundtrip.
The tests establish supplied-value behavior rather than filesystem execution.

The [captured compiler](captured-item-compilation.md) already derives complete
per-client resource and entrypoint outputs. Connecting those outputs to lifecycle
ownership, roots, consumers and destination observations remains pending. Complete
reference authority, per-owner consumer associations, immutable source evidence,
source/target adapters, target and filesystem qualification, writer locking,
durable state and physical apply/recovery remain separate gates. The pure
[permission-selection policy](file-permission-policy.md) is implemented; physical
permission application and qualified integration remain pending (debt #278). The API
may return the first contextual conflict; complete diagnostic aggregation remains
an activation gate. No CLI selector, native support, schema-2 persistence or
installation effects are activated. M1-A2, M1-A3 and M1 remain incomplete.
