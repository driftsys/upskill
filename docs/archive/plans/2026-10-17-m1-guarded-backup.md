# Guarded migration backup implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this
> plan task by task.

**Goal:** Establish the create-only, exact first-write backup prerequisite for
Gate 5 using the qualified executor's existing journal and recovery protocol.

**Architecture:** A separate opt-in execution method accepts the observed
original state and a backup location. It prepends a restricted backup action to
the existing complete request, with explicit create-only publication persisted
in the journal. Recovery verifies that prerequisite alongside every output and
the final state. Existing executor callers and production schema 1 remain
unchanged.

**Tech stack:** Rust 2024, MSRV 1.85, existing rustix descriptor-relative
filesystem operations; no new dependencies.

**Spec:** `docs/wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md`,
acceptance criteria 2, 7 and 8. This bounded slice supplies the backup executor
primitive; combined lifecycle preparation and observational adapters follow.

## Constraints and rulings

- Keep existing `ExecutionRequest` source compatibility and existing journal
  schema-1 decoding. Backup operations use journal schema 2 with a required
  first create-only action.
- Bind backup bytes to the final state's exact observed before bytes, not a
  typed JSON reserialization.
- New backup mode is 0o600. Existing backups must match bytes and mode, remain
  singly linked, and preserve identity.
- Validate the entire request and all sources/destinations before journal or
  backup creation.
- Publish backup without overwriting an existing destination, including a
  destination appearing immediately before publication.
- Persist create-only semantics so interrupted recovery cannot use ordinary
  replacement for the backup.
- A backup mismatch during recovery prevents further outputs or final state
  changes.
- No CLI activation, ownership inference, source fetch or recursive removal.

## Review focus

1. An unexpected backup appearing at publication must survive unchanged.
2. Corrupt persisted create-only flags must reject before effects.
3. Completed backup edits must block recovery before subsequent output changes.
4. An interrupted publication must recover without rewriting a matching backup.
5. Existing exact backups must retain inode, bytes and mode.

## Task 1: Qualified create-only prerequisite

**Files:** `src/filesystem/execution/{mod,request,backend,journal,run}.rs`;
focused executor tests and `tests/execution_backup.rs`.

**Interfaces:** Add
`QualifiedScope::apply_operation_with_backup(guard, sources, request, backup: FileLocation) -> Result<ExecutionReport>`.
Consumes the exact `request.state.before` bytes; returns the existing report
with the backup included at action index zero. Existing `apply_operation` stays
unchanged. Journal actions gain optional default-false `create_only` metadata,
validated as a first non-state action with private mode and absent or identical
before/after evidence.

- [ ] Write acceptance examples for exact original bytes, existing matching
      identity preservation, mismatch rejection before effects, stale later
      output rejection and invalid backup topology.
- [ ] Observe failure before implementation, then implement request preparation
      and persisted create-only execution using exclusive staged publication.
- [ ] Add checkpoint tests for publication races, interruption/recovery and
      edited completed backups; exercise the real filesystem.
- [ ] Run focused tests, full suite, formatting, verification and locked Rust
      1.85 checks. Retain results in the slice ledger.
- [ ] Update executor design/specification and validation evidence; commit code,
      tests and docs together.

## Task 2: Independent review and delivery

- [ ] Review the entire branch with a fresh reviewer, fix confirmed findings
      with regression coverage, and record rulings.
- [ ] Open the slice PR against main and inspect exact-head hosted CI. Gate 5
      remains incomplete until lifecycle integration and observational adapters
      pass together.

## Preflight

The first backup action shares request topology, journal ordering and recovery
validation with existing files. Its create-only flag must flow unchanged from
request preparation through persisted journal validation to backend publication.
Final state remains last. Existing journals omit the flag and decode to ordinary
execution. Atomic NOREPLACE publication moves the staged file in one operation;
recovery acknowledges a matching publication without replacing it.
