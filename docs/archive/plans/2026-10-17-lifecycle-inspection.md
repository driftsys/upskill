# Read-only lifecycle inspection implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this
> accepted Gate 5 slice inline.

**Goal:** Provide opt-in library list and doctor inspection over qualified
captured state and outputs, without migration, locking, fetching or recovery.

**Architecture:** Decode schema 1 through the existing descriptive legacy
projection and schema 2 through the strict combined-state codec. Reuse qualified
destination observations and the executor's read-only journal loader. Keep
malformed state and unsafe filesystem evidence as errors, retaining valid legacy
rows and their diagnostics in reports.

**Tech Stack:** Existing Rust library; no new dependencies. Rust 1.85.

**Spec:** ../specs/2026-10-15-m1-gate5-lifecycle-integration.md

## Constraints

Production commands and schema 1 stay unchanged. Reports establish no ownership,
native support or source freshness. Inspection cannot create execution storage,
acquire writer locks or recover pending operations. Captured state bytes remain
available unchanged.

## Review focus

- Schema-1 unknown fields and duplicate rows survive observational inspection.
- Missing output evidence differs from an observed missing file.
- Shared Markdown checks ignore manual bytes and inspect only recorded regions.
- Pending or corrupt journals never trigger recovery or cleanup.
- Unsupported schemas and platforms return explicit errors.

## Task 1: Read-only journal status

- [ ] Add failing tests for absent storage and interrupted execution; assert
      exact journal/state/output bytes and directory entries remain unchanged.
- [ ] Expose `QualifiedScope::has_pending_operation() -> Result<bool>` through
      strict `Journal::load`; return an explicit unsupported-platform error
      elsewhere.
- [ ] Run focused executor tests.

## Task 2: Captured lifecycle list and doctor

- [ ] Add integration acceptance tests in `tests/lifecycle_inspection.rs`:
      schema-1 projection, mixed schema-2 state, legacy diagnostics, complete
      output observations, whole-file drift, contribution edits, manual-byte
      preservation, and exact filesystem preservation.
- [ ] Add `src/lifecycle/inspection.rs`:
      `inspect_state(&[u8]) -> Result<StateInspection>`,
      `doctor(&StateInspection, &[ObservedFile]) -> Result<Vec<ArtifactDiagnostic>>`,
      and
      `inspect_scope(&QualifiedScope, &InstallationRoot) -> Result<ScopeInspection>`.
- [ ] Return owned installation/bundle/reference records through the validated
      combined state and indexed legacy/native rows through `LegacyProjection`.
      Retain exact original bytes. Doctor reports missing, edited, mode mismatch
      and missing evidence separately; duplicate observations are errors.
- [ ] The qualified adapter observes state and owned destinations, checks
      pending recovery without writing, revalidates snapshots, and returns typed
      diagnostics. Reject artifacts belonging to another scope.
- [ ] Update `docs/topics/0005-lifecycle-planning.md` and a validation note,
      preserving historical anchors and remaining Gate 5 work.
- [ ] Run `just fmt`, `just verify`, and `just msrv-check`; independently review
      the complete diff.

Preview, applying command adapters and source-resolution orchestration remain
subsequent Gate 5 deliveries. This slice does not complete Gate 5.
