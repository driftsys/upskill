# Offline Claude settings preservation regressions

Status: offline regression coverage, 2026-11-03. This records application-side
preservation behavior, not a passing native mutation profile.

## Scope and evidence

The [approved Claude probe](2026-11-01-claude-local-catalog-contract.md)
accepted the generated catalog and scoped commands but rewrote the pre-existing
settings document. Its sanitized receipts already supply registry-decoder
fixtures. The new
[preservation tests](../../tests/native_claude_preservation.rs) consume the same
settings texts, modes and SHA-256 digests without copying private profile data
or invoking a native executable.

The initial regression work was prepared on `fix/claude-settings-preservation`
at `b5ad241`. Its source files remain untouched in that worktree. The tests and
this note were integrated into `feat/native-delivery` after `9cb8ab9` and are
independently reverified against that branch. No previous full-suite result is
assumed.

## Required behavior

- The captured before/after documents have equal JSON values but different
  bytes. Both retain mode `0600`; semantic equality is insufficient.
- The existing span-based contribution planner can add and remove owned plugin
  and marketplace selectors while preserving the exact surrounding bytes and
  mode on the forward patch, then restoring the exact captured baseline on
  removal. This proves the pure planner's behavior only.
- A fake native command can report a confirmed binding while reserializing
  settings. The real durable coordinator must return `RecoveryRequired`, retain
  root markers and journal evidence, and leave the authoritative lockfile
  unpublished. A separate mode-only mutation must have the same outcome.
- `ObserveOnly`, `Forward` and `Restore` must all preserve the diverged settings
  bytes and mode. They must neither replay a native effect nor invoke its
  inverse to overwrite the changed document.

The fake serializer retains all JSON values while changing layout. It models the
lexical failure class in the receipt; it is not a second native run or a claim
that every Claude rewrite uses this serializer. The mode-only control is
separate so byte drift cannot mask a missing mode check.

## Preservation resolution and remaining native blocker

The coordinator's existing exact-file preflight already enforces this behavior;
no new production repair or serializer is required for the regression. Native
success metadata cannot override file divergence. If a command rewrites a
captured settings document outside its planned owned delta, the operation
remains incomplete and recovery retains the changed file. It cannot adopt the
rewrite as a new restoration baseline or silently overwrite it.

Under the
[approved D2/D6 contract](../wip/specs/2026-10-25-customization-preservation-contract.md),
the tested native command sequence remains unsupported for exact settings
preservation. Reordering pre-existing keys is not an empty-container allowance.
The closed production admission registry prevents this candidate from becoming
an executable production profile. A passing span-writer round trip does not make
the native command safe, authorize direct registry edits or establish worker
quiescence.

Completing a supported Claude adapter still requires a separately reviewed
native mechanism that preserves the protected document, with attributable
before/after observations and passing native preservation controls. An upstream
selective writer or a distinct configuration surface could be investigated as a
candidate; neither is established by these offline tests. A proposal to
normalize or restore native-rewritten whole files must first define and obtain
approval for its exact ownership, concurrent-edit, non-secret capture,
quiescence and inverse contract. Do not implement that as an implicit repair.

No native rerun, profile mutation, provider call, qualification amendment or
production adapter completion is part of this change.

## Verification

The integrated tests independently passed all three cases with the existing
coordinator. A temporary mutation replaced its completed-file equality
precondition with unconditional acceptance. The regression then failed because
the authoritative lockfile was published despite settings divergence. The
original production source was restored byte-for-byte before the normal suites
were rerun. This establishes that the test protects a completion boundary,
rather than only checking a failure label.

A fresh review found that final baseline restoration alone could conceal an
intermediate rewrite by the span planner. A second temporary mutation forced
forward reserialization: the original round-trip assertion still passed. The
added exact forward bytes/mode assertion then failed on that same mutation. The
planner source was restored before final verification. No production source
change remains from either mutation experiment.

Integration repeated both mutation controls against the current production
source. Each mutant compiled and failed its intended acceptance assertion; both
source files were restored byte for byte. The bounded offline receipts are
`/private/tmp/claude-preservation-integration-mutations.json` and the
corresponding `claude-preservation-integration-mutation-*.log` files. Focused
test output is retained in
`/private/tmp/claude-preservation-integration-focused.log`. Full repository
verification and independent review remain pending at this stage.
