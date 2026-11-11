# Read-only lifecycle preview implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement this
> accepted Gate 5 slice inline, with acceptance-first tests.

**Goal:** Describe proposed state and output changes for captured upsert and
exact reference-removal requests without fetching or filesystem effects.

**Architecture:** Decode captured state through the existing inspection boundary
and delegate planning to the existing restrictive/shared upsert or
reference-removal planner. Preserve original state bytes, expose the complete
planned next state and file actions, and distinguish semantic state changes from
schema-1 upgrade requirements. Selection blockers retain their existing typed
diagnostics; planner errors return no partial preview.

**Tech Stack:** Existing synchronous Rust library, Rust 1.85; no new
dependencies.

**Spec:**
[Active Gate 5 umbrella](../../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md).

## Constraints

- Preview receives captured bytes, desired sets and observations only; no fetch,
  scope locking, migration, backup, journal, recovery or application.
- Production commands and `lockfile::CURRENT_SCHEMA` remain on schema 1.
- Output plans establish no filesystem, native-discovery or application
  authority.
- Applying command adapters, source resolution and complete Gate 5 acceptance
  remain pending.

## Review focus

- Semantically invalid complete schema-2 state is rejected before planning.
- Distinct file locations retain the exact planner order, before/after values
  and classification.
- Metadata-only reference edits and unchanged byte/mode output actions remain
  visible.
- Selected legacy rows block the entire request while unrelated legacy/native
  rows and unknown original JSON bytes survive.
- Edited outputs reject preview and preserve every supplied input and physical
  file.

## Task 1: Captured preview adapter

Files: create `src/lifecycle/preview.rs` and `tests/lifecycle_preview.rs`;
export in `src/lifecycle/mod.rs`.

Interfaces:
`preview(state_bytes: &[u8], request: PreviewRequest<'_>, observations: &[ObservedFile]) -> Result<PreviewOutcome>`.
Requests distinguish restrictive upserts, explicit shared upserts, and exact
reference removals. Ready output retains original state bytes, validated
proposed combined state, semantic state-change classification, and ordered file
actions with create/change/remove/unchanged classifications.

- [ ] Write failing public integration examples for schema-1 new ownership
      beside retained legacy rows; multi-file creates/updates/removals/no-ops
      including mode changes; exact reference removal; typed legacy selection
      blockers; planner conflicts; malformed and semantically invalid state; and
      equivalent state with different serialization.
- [ ] Run `cargo test --test lifecycle_preview`; confirm missing preview API is
      the only failure.
- [ ] Implement the adapter by reusing existing planners and classifying their
      returned file actions; do not reimplement lifecycle selection or ownership
      logic.
- [ ] Run acceptance tests and matching upsert/retirement regressions.
- [ ] Add physical temporary-root preservation examples, including unchanged
      source/state/output/resource bytes and no storage creation. Demonstrate no
      network/source resolution is required.

## Task 2: Durable records and delivery

- [ ] Document the exact supplied-value limits in
      `docs/topics/0005-lifecycle-planning.md`; add
      `docs/technotes/m1-lifecycle-preview.md` and update relevant status/index
      entries.
- [ ] Run `just fmt`, `just verify`, and `just msrv-check`; independently review
      the finished diff.
- [ ] Archive only this completed bounded plan through SDD gardening, preserving
      the active Gate 5 umbrella and unrelated WIP.
- [ ] Commit the verified slice. Open a PR after required review and checks; do
      not claim Gate 5 or native qualification complete.

## Decisions

- R-PREVIEW-1: Exact reference-removal requests reuse the existing retirement
  boundary; CLI name/source selection remains applying-adapter work. Cost if
  wrong: the preview request API may need another selector adapter later.
- R-PREVIEW-2: Preserve existing typed selection blockers and first planner
  error behavior rather than inventing operation-wide conflict aggregation. Cost
  if wrong: callers may need a later richer diagnostic envelope.
- R-PREVIEW-3: Semantic state classification compares validated combined
  records, while schema 1 explicitly reports an upgrade requirement. Original
  bytes stay available without suggesting whitespace-only rewrites are semantic
  changes.
