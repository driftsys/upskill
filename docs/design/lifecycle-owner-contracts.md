# Per-owner recorded contract evidence

The inactive `lifecycle::contracts` boundary records supplied contract evidence
inside authoritative `CombinedState`. It supports rule installations and exposes
pure validation and agreement checking. Production commands and schema-1
lockfiles do not consume it. It adds no dependencies or native effects.

## Representation and validation

`CombinedState.owner_contracts` is an optional vector, defaulting to empty and
omitted from serialization when empty. Historical absence stays unknown;
schema-1 projection creates no contracts. `OwnerConsumers` continues to describe
consumer associations without implying adapter revisions or contract evidence.

An `OwnerContract` binds exact scope, raw root spelling, path, optional
instruction contribution selector and installation ID to a captured
source-manifest digest, normalized content and explicit `ConsumerContract`
surface/revision pairs. Families follow stable target surfaces. File content
includes its recorded hash and optional mode. Instruction content includes the
body hash and excludes aggregate whitespace bookkeeping, allowing preserved
manual bytes or removal of neighboring contributions without invalidating an
untouched body's evidence.

Whole-state validation checks every present row against an authoritative
owner-consumer association, installation source evidence and owned artifact. It
rejects duplicate identities, orphan evidence, changed roots/selectors, inexact
source/content bindings, empty or duplicate profiles, inexact consumer coverage
and zero revisions. Rows need not cover every owner. Strict unknown-field
rejection remains. Serialization clones the state, sorts rows by scope/raw
root/path/selector/installation identity and profiles by surface, and leaves the
input unchanged.

## Agreement boundary

`check_contract_agreement` validates the complete state, then locates an
artifact by exact supplied root, path and contribution selector. An unknown
identity is an error. It walks artifact owners in their recorded order, reports
every owner missing evidence, and compares evidenced owners with the first
evidenced owner. Source-manifest and target-profile conflicts are separate
diagnostics and can both occur for an owner. Profile vector order is immaterial.

`Matching` requires each owner to have evidence, identical full
captured-manifest digests and identical complete surface/revision maps.
Normalized content already matches the artifact through validation. Different
consumer subsets block even when output bytes match. This supplied agreement
establishes no native behavior, source authenticity, physical identity, new
ownership or execution authority.

## Lifecycle propagation

`UpsertRequest.contracts` supplies the complete explicit evidence subset for
changed installations. Omitted changed rows become unknown. Untouched rows
retain their values and positions; replaced identities retain prior positions;
new rows append in identity order. Supplied evidence for unchanged installations
and duplicate rows reject. The complete resulting state is validated before a
plan is returned, so stale or invalid proof exposes no partial plan.

Empty desired sets remove output evidence while retaining referenced
installation history. Graph retirement removes retired installations' contracts.
Reference removal preserves surviving shared owners' contracts and removes
retired owners' contracts, including instruction contributions and final empty
aggregates.

Affected multi-owner upsert and mixed-planner rejection remain intact. Shared
reconciliation must consume this evidence together with qualified source and
destination observations before those rejections can be relaxed. This bounded
slice does not complete Gate 5 or qualify native clients.

## Components

- [Contract model and agreement](../../src/lifecycle/contracts.rs)
- [Combined-state validation and codec](../../src/lifecycle/state/)
- [Upsert propagation](../../src/lifecycle/upsert.rs)
- [Reference retirement](../../src/lifecycle/retire/mod.rs)
- [Acceptance examples](../../tests/lifecycle_contracts.rs)
- [Evidence policy](../adr/0034-record-explicit-owner-contracts.md)
- [Validation limits](../technotes/2026-10-16-m1-owner-contracts-validation.md)
