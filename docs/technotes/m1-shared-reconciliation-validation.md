# Explicit shared-output reconciliation validation

Validation record date: 2026-10-17. This date identifies the original evidence
record; consolidation does not rerun or extend its qualification.

This inactive library slice adds contracted captured-rule production and
explicit shared-upsert planning. It introduces no dependencies, production
command activation, schema-1 changes, native qualification or guarded
application.

## Acceptance examples

The [contract producer suite](../../tests/lifecycle_contract_producer.rs)
contains five examples covering exact captured output/source/profile bindings,
invalid profile coverage/family/revision, empty audience, mismatched capture and
mixed client bindings. Expected outputs are compared with the original producer
and independent literal consumer expectations.

The [shared planner suite](../../tests/lifecycle_sharing.rs) contains 31
examples covering new sharing, retained-owner addition, coordinated update,
selected explicit requalification, unknown retained proof, conflicting content
and modes, owner release/final retirement, immutable real bindings, exact
observations, malformed unrelated state, legacy collisions, topology,
instruction manual bytes/modes/neighbors and empty aggregate retention. Codec
roundtrips retain real owner evidence; projection identities do not enter
returned state. Original upsert rejection and single-owner parity remain
exercised. Review fixes add update-to-retirement reuse of returned instruction
metadata, new-sharing contract failures, mixed forms, identity ordering and
distinct raw roots/scopes.

## Reported checks and controls

The dispatcher reports 1,242 passing tests in the full Rust suite, thirty-six
passing focused examples, locked Rust 1.85 all-target checking and sanity exit 0
for the implementation including review test additions. These reports are
supplied delivery evidence, not independently authenticated execution by this
record. Independent review and exact-head hosted checks are separate gates.

The implementation ledger records expected unsupported-sharing RED when the new
entrypoint initially forwarded to the old restrictive planner. Assertion
controls for agreement inversion, owner reversal, retained-proof deletion and
observed-mode deletion failed as intended. Producer expectation controls also
assertion-failed. Removal of a defensive immutable-binding check survived
because graph reconciliation independently rejected the binding change. Removing
explicit byte equality also survived because independently supplied content
contracts and candidate validation rejected the differing bytes/modes. Surviving
redundant controls are not counted as detected faults; exact byte equality
remains a conservative guard beyond digest comparison. Additional controls for
discarded layout metadata, skipped new-sharing agreement checks and collapsed
raw roots/scopes assertion-failed. Source edits were restored.

Execution logs, assertion controls and restored source hashes are retained in
the
[evidence manifest](evidence/2026-10-17-m1-shared-reconciliation/manifest.json).
The gzip artifacts can be inspected with `gzip -dc <file>`. The manifest hashes
identify captured bytes; they do not authenticate an independent execution.

Whole-branch review regressions cover proof-free release from three owners to
two for both artifact forms, and real-owner diagnostics for malformed
instruction bodies and edited files. Both cases failed before the production
fixes.

Pass 1 adds exact producer contract cardinality, known survivor evidence/order,
varied output consumer maps, Git revision replacement, prior-only diagnostic
attribution and eleven-identity coverage. Typed instruction location contexts
and exact complete-context translation preserve literal destination text.

Pass 2 regression coverage verifies aggregate-wide neighbor failures report the
physical file ownership grouped by selector, and verifies missing/mode file and
absent/invalid-UTF-8 instruction diagnostics. Retained extraction checks exact
scope, raw root, selector and cause across equal paths in distinct roots/scopes.

Retained diagnostics exclude owners and selectors from unrelated files in the
same scope and root. Removing relative-path isolation assertion-fails; original
source bytes are restored before the final gates.

## Limits

These examples use supplied capture and destination values. They establish no
source authenticity, physical containment, qualified observations, native client
behavior or execution authority. Missing unselected proof remains unknown;
complete selected evidence does not authenticate its source. Instruction
compiler adapters, qualified lifecycle integration, backup, execution/recovery
and native qualification remain unfinished Gate 5/Gate 6 work.
