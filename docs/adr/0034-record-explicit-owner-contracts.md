# Record explicit per-owner contract evidence

AD-0034

## Context

Combined lifecycle state records authoritative ownership and consumers, but
historical rows contain no adapter revisions. Equal rendered bytes cannot prove
shared source semantics or target compatibility. Shared mutation remains blocked
until a later reconciliation boundary consumes sufficient evidence.

## Options

The brainstorming considered optional per-owner rows inside `CombinedState`
rather than changing `OwnerConsumers`. Consumer associations alone do not carry
contract proof. No reviewed cross-surface semantic compatibility mapping exists;
other alternatives were not documented.

## Decision

Record optional explicit owner contracts separately inside authoritative
combined state. Absence is unknown and never acquires a default revision.
Require exact source-manifest and complete surface/revision-map equality for
supplied agreement, with normalized content bound to the artifact. Changed
installations receive only the explicit evidence supplied by upsert; omission
clears their prior proof. Preserve untouched evidence and discard evidence
belonging to retired owners.

## Consequences

Historical state remains readable without invented proof. Matching bytes with
missing evidence, differing source captures or differing consumer profiles block
agreement. Semantically equivalent captures and distinct consumer subsets may
conservatively block. Callers must resubmit evidence for changed installations,
including no-op output updates. Agreement alone does not authorize shared
mutation, qualify native behavior or establish source authenticity and physical
identity.

Satisfies:
[specification §4.19](../specification/specification.md#419-per-owner-recorded-contract-evidence).
Related: [owner-contract design](../design/lifecycle-owner-contracts.md),
[combined lifecycle state](../design/combined-lifecycle-state.md),
[lifecycle upsert](../design/lifecycle-upsert.md).
