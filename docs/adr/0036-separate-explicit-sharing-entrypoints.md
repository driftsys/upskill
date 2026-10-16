# Separate explicit sharing entrypoints

AD-0036

## Context

Existing desired-set and upsert callers must not acquire sharing authority
merely because contract production and reconciliation become available.
Historical surface defaults cannot supply missing versioned evidence.

## Options

The implementation discussion chose separate contracted production and shared
upsert entrypoints while retaining the original restrictive APIs. Other
alternatives were not documented.

## Decision

Expose `compile_rule_desired_set_with_contracts` with explicitly supplied
profiles and `plan_shared_upsert` with explicit owner contracts. Reuse captured
compilation and common lifecycle orchestration. Keep original producer/batch
behavior, `plan_upsert` and public mixed-planner sharing guards intact. Never
synthesize contract revisions from surface defaults.

## Consequences

Callers explicitly pass versioned profiles and consume the contracted result.
Two pure upsert entrypoints remain until later integration. Exact source and
profile agreement follows AD-0034; no semantic compatibility map is invented.
Production schema 1, command dispatch, native qualification and guarded
application remain outside this slice.

Satisfies:
[specification §4.20](../specification/specification.md#420-explicit-shared-output-reconciliation).
Related:
[captured desired sets](../design/captured-rule-desired-sets.md#explicit-contract-production),
[shared upsert](../design/lifecycle-upsert.md#explicit-shared-upsert),
[AD-0034](0034-record-explicit-owner-contracts.md).
