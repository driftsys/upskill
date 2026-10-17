# Record explicit per-owner contract evidence

AD-0034

## Context

Combined lifecycle state records authoritative ownership and consumers, but
historical rows contain no adapter revisions. Equal rendered bytes cannot prove
shared source semantics or target compatibility. The separate shared-upsert
boundary needs explicit evidence for every affected desired multi-owner
artifact.

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
including no-op output updates. The separate shared-upsert boundary consumes
matching agreement after validating real candidate ownership. Complete selected
requalification can replace unknown historical evidence; unselected missing
proof blocks affected desired multi-owner results. Agreement alone does not
authorize shared mutation, qualify native behavior or establish source
authenticity and physical identity.

Satisfies:
[specification §4.19](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#419-per-owner-recorded-contract-evidence).
Related:
[owner-contract design](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#architecture-lifecycle-owner-contracts),
[combined lifecycle state](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#architecture-combined-lifecycle-state),
[lifecycle upsert](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert).

Pure owner release with no desired value at an identity preserves untouched
survivors and their unknown evidence without requiring agreement. Supplied
physical observations and all physical checks still apply.
