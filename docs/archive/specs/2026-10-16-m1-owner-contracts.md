# Per-owner recorded contract evidence

Accepted direction: extend the Gate 5 combined state before relaxing affected
shared-output rejection. This is an inactive supplied-value contract boundary.
Production commands and schema 1 remain unchanged.

## Representation

Add `owner_contracts` inside `CombinedState`, defaulting to an empty vector and
omitted when empty. Missing historical evidence remains unknown; never fabricate
an adapter revision from consumer unions or rendered bytes. Each `OwnerContract`
binds exact scope/raw root, path, optional contribution selector and
installation ID to a captured source-manifest digest, normalized artifact
content and explicit consumer/contract-revision pairs. Families are implied by
stable surfaces.

File content binds the exact recorded hash and optional mode. Instruction
content binds its body hash; aggregate whitespace bookkeeping is excluded so
preserving manual bytes or removing neighboring contributions does not
invalidate an untouched body's contract. Contracts currently cover Rule
installations only.

Validate every present contract against an existing authoritative owner-consumer
association, installation source evidence and owned artifact. Reject duplicates,
orphans, changed roots/selectors, wrong content/source, empty/duplicate/inexact
consumer profiles and zero revisions. Contract rows need not cover every owner.
Preserve strict unknown-field rejection and deterministic serialization, sorting
contract rows by identity and profiles by surface without mutating inputs.

## Agreement boundary

A pure `check_contract_agreement` looks up an existing artifact by exact
supplied root/path/selector. It validates the whole state and collects
missing-evidence, source-manifest and target-profile disagreements in artifact
owner order. Matching means all owners have the same captured source digest and
complete surface/revision map; normalization must also match the shared
artifact. This conservative first policy rejects different consumer sets even if
bytes match. It establishes supplied contract agreement, not native behavior,
source authenticity, physical identity or authority to add/change owners.

## Lifecycle propagation

`UpsertRequest.contracts` supplies the complete explicit contract subset for
changed installations. Omitted changed rows become unknown, never inherit stale
proof. Preserve untouched contracts exactly; retain replaced identities in their
prior positions and append new evidence in identity order. Reject supplied
contracts for unchanged installations and validate the resulting complete state.
Empty desired sets and graph retirement remove only associated retired evidence.
Reference removal retains surviving owners' contracts and removes retired
owners' contracts, including shared rows and empty instruction aggregates.

Existing mixed/upsert affected multi-owner rejection stays intact. A subsequent
shared reconciliation task must consume this evidence alongside qualified source
and destination observations before relaxing it. This slice does not complete
Gate 5 or qualify native clients.

## Acceptance

- Old schema-2 envelopes parse with no invented contract values; schema-1
  projection produces no contracts. Unknown/malformed fields still fail.
- Valid evidence roundtrips deterministically; all bindings and profiles are
  checked, including raw root spelling and instruction selector/form
  distinctions.
- Same bytes with missing, differing-source or differing-profile evidence block;
  all missing owners are reported. Profile vector order is immaterial to
  agreement.
- Contribution whitespace metadata can change without changing body contracts.
- Upsert explicitly replaces/clears selected evidence and preserves unselected
  rows; invalid evidence exposes no partial plan. Removal preserves retained
  shared contracts and removes retired contracts.
