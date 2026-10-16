# Complete lifecycle upsert planning implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans.
> Execute inline under the existing user delegation.

**Goal:** Join complete desired installation sets, explicit selection, reference
reconciliation and mixed-artifact planning without effects.

**Architecture:** A pure coordinator consumes prior CombinedState, an
UpsertRequest and exact ObservedFile values. Graph upserts derive only from
complete desired sets. Selection controls existing changes; new installs need no
prior selection. Empty complete sets retire outputs without removing referenced
installation records. Existing mixed-artifact multi-owner rejection remains
authoritative; shared contract establishment is a subsequent bounded task.

**Tech Stack:** Rust edition 2024, Rust 1.85, existing dependencies only.

**Spec:** ../specs/2026-10-15-m1-gate5-lifecycle-integration.md;
../specs/2026-10-12-m1-owned-lifecycle-design.md;
../../design/captured-rule-desired-sets.md; ../../design/lifecycle-selection.md.

## Global constraints

- No production CLI changes, schema-2 writes, new dependencies, I/O or native
  qualification claims.
- Exact scope/raw root identity; later qualification establishes physical
  equivalence.
- Preserve tagged legacy snapshots, untouched installations, artifacts and owner
  associations.
- Reject affected shared artifacts and new sharing; byte equality establishes no
  adapter contract.
- Keep source authenticity, complete diagnostic aggregation, source/scoped
  selectors, CLI precedence and backup/recovery outside this bounded API.

## Review focus

- Empty output must retain requested surfaces/source evidence and references.
- An incomplete replacement list must not modify an unselected owner or graph
  root.
- Root/consumer/exclusion mismatches in caller-built desired sets must fail.
- Legacy/native name collisions must block new and selected work without
  adoption.
- No-op output updates still require exact destination observations.

### Task 1: Complete single-owner upsert coordinator

**Files:** Create src/lifecycle/upsert.rs and tests/lifecycle_upsert.rs; export
from src/lifecycle/mod.rs; add docs/design/lifecycle-upsert.md and specification
section 4.18; update design/WIP indexes.

**Interfaces:**

- Consumes DesiredInstallationSet, Selection, GraphChanges, CombinedState,
  ObservedFile and plan_owned_artifacts.
- Produces UpsertRequest<'a> { selection: Option<&'a Selection>, sets: &'a
  [DesiredInstallationSet], bundles: &'a [BundleNode], add_references: &'a
  [Reference], remove_references: &'a [Reference] }, UpsertPlan { state:
  CombinedState, files: Vec<PlannedFile> }, UpsertOutcome::Ready(UpsertPlan) or
  Blocked(Vec<SelectionDiagnostic>), plan_upsert(prior: &CombinedState, request:
  UpsertRequest<'_>, observations: &[ObservedFile]) ->
  anyhow::Result<UpsertOutcome>.

- [ ] Write public acceptance examples before source: new direct install; update
      retaining manual aggregate text; empty desired output retains graph
      history; graph retirement; untouched legacy/history preservation; complete
      selection blockers; request authorization; exact consumer accounting; root
      mismatch; affected shared rejection; no-op observations;
      duplicate/incomplete sets; bundle selection and new nodes.

Example assertion for empty desired output:

```rust
assert_eq!(plan.state.installations, vec![set.installation.clone()]);
assert_eq!(plan.state.references, vec![direct("a")]);
assert!(plan.state.artifacts.is_empty());
assert!(plan.state.owner_consumers.is_empty());
assert_eq!(plan.files[0].after, None);
```

- [ ] Run `cargo test --test lifecycle_upsert` and preserve expected
      missing-module RED.
- [ ] Implement selection enforcement; reconcile graph; require exact selected
      surviving installation coverage and selected retirement; validate desired
      root and complete consumer accounting; translate empty sets into
      artifact-only Remove when old artifacts exist; plan complete mixed outputs
      once; rebuild exact per-owner associations from complete sets and preserve
      untouched associations; validate final state.
- [ ] Run `cargo test --test lifecycle_upsert`; Expected: all acceptance
      examples pass.
- [ ] Add assertion-failing controls for retained empty history, selection
      authorization, excluded-consumer accounting, unselected association
      preservation and no-op evidence; restore exact source bytes.
- [ ] Update design/specification/validation evidence, `just fmt`,
      `just sanity`, `just verify`; Expected: exit 0. Full logs live in this
      plan's scratch workspace.
- [ ] Commit `feat: plan complete lifecycle installation upserts`; final
      independent branch review, mandated hook reviews, gardening, PR and two
      numbered reviews before requesting named merge approval.
