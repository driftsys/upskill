# Gate 5 reference reconciliation implementation plan

> **For agentic workers:** Use superpowers:executing-plans to implement inline.
> Steps use checkbox syntax for tracking. The user delegated continuing story
> implementation and review on 2026-10-16 without another planning checkpoint.

**Goal:** Reconcile explicit installation/bundle graph edits before artifact
planning, retaining installations until their last authoritative reference ends.

**Architecture:** Add a pure graph boundary over validated combined state.
Validate every supplied node and edge before iterative reachability pruning.
Return graph records and removed IDs, never a state write or artifact removal.

**Tech Stack:** Rust 2024, MSRV 1.85, existing types and anyhow; no
dependencies.

**Spec:**
[accepted Gate 5 scope](../specs/2026-10-15-m1-gate5-lifecycle-integration.md).
This is task 2b's first independently testable prerequisite, following #296
(merged `b01a67d`). Shared-owner artifact reconciliation, instruction adapters
and complete lifecycle selection/planning remain separate follow-on work.

## Global constraints

- Pure opt-in library, no I/O, source fetch, migration, ownership adoption,
  backup, execution, CLI dispatch changes or schema-2 production writes.
- Validate the complete prior combined state, including ownership associations
  and tagged legacy schema, before accepting any graph changes.
- Graph results contain no artifacts or legacy snapshot. They are not a valid
  final combined state and do not authorize file deletion. Later planning must
  reconcile artifacts and retain the prior tagged legacy values unchanged.
- Only explicit direct roots and bundle edges retain nodes. Artifact count,
  consumer count, source labels and matching bytes are not references.
- Installation source/kind/name/alias and scope/raw root are immutable for an
  existing ID. Requested surfaces and captured evidence/revision may change,
  subject to existing state validation and later complete artifact planning.
- Existing bundle ID retains its source and name; its recorded revision may
  change. New IDs do not bypass duplicate identity validation.
- Reject invalid edits even when their nodes would be unreachable; never prune
  malformed selected work into apparent success.
- Existing mixed/instruction planners retain their multi-owner rejection.

## Review focus

1. Removing one root of a diamond must retain the shared transitive item and
   bundle until all direct/bundle paths have ended.
2. Empty artifact sets and absent consumer associations must not become implicit
   installation removals while an explicit reference survives.
3. Dangling edges or cycles in newly unreachable work must fail before pruning.
4. Existing IDs must not switch physical scope, raw root spelling, source, kind,
   source name or alias; distinct IDs cannot bypass identity uniqueness.
5. Conflicting/repeated edits and unreachable upserts must fail rather than
   silently discard part of a request. All input records remain unchanged.

## Files and interfaces

Create `src/lifecycle/reconcile/mod.rs` for public values and reconciliation;
`src/lifecycle/reconcile/reachability.rs` for iterative graph traversal. Expose
`pub mod reconcile` in `src/lifecycle/mod.rs`. Create
`tests/lifecycle_references.rs` for public acceptance tests. Add
`docs/design/lifecycle-reference-reconciliation.md`, specification §4.15, and a
design-index entry; update the WIP inventory. No archived raw edits.

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphChanges {
    pub installations: Vec<Installation>,
    pub bundles: Vec<BundleNode>,
    pub add_references: Vec<Reference>,
    pub remove_references: Vec<Reference>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciledGraph {
    pub installations: Vec<Installation>,
    pub bundles: Vec<BundleNode>,
    pub references: Vec<Reference>,
    pub removed_installation_ids: Vec<String>,
    pub removed_bundle_ids: Vec<String>,
}
pub fn reconcile_references(
    prior: &CombinedState,
    changes: &GraphChanges,
) -> anyhow::Result<ReconciledGraph>;
```

## Task 1: Authoritative reference edits and retention

**Consumes:** state `CombinedState`, `Installation`, `BundleNode`, `Reference`,
`validate_state`. **Produces:** all interfaces above.

- [ ] Write `diamond_retains_shared_dependencies_until_last_root_ends` with two
      directly referenced bundles pointing to one dependency and one item.
      Detach one direct bundle: remove only that root bundle and its outgoing
      edge; retain the other root, shared dependency and item. Detach the last
      root: all graph records disappear and exact removed IDs are returned.
- [ ] Run `cargo test --test lifecycle_references`; require missing interface
      RED.
- [ ] Validate prior state first. Reject duplicate installation/bundle upserts,
      repeated reference edits, adding an existing edge, removing an absent
      edge, or adding/removing the same edge in one request.
- [ ] Upsert nodes with immutable binding checks above. Preserve surviving prior
      row order; append new IDs in lexical order regardless of edit order.
      Preserve retained reference order and append added references in enum
      order.
- [ ] Validate candidate nodes, dangling references and cycles using the
      existing validator with empty artifact/association vectors and temporary
      direct roots for all candidate nodes. Add missing synthetic roots only in
      that private validation value; never return them or use them for
      reachability.
- [ ] Traverse actual direct roots with iterative queues and indexed outgoing
      edges. Remove unreachable prior nodes and their outgoing references.
      Reject any explicitly upserted node that is unreachable in the result.
      Validate the final graph again without synthetic roots.
- [ ] Write `direct_and_bundle_references_are_independent` and
      `referenced_empty_installation_is_retained`; assert literal IDs and edges,
      not reference counts derived from ownership.
- [ ] Write `invalid_unreachable_graph_edits_fail_before_pruning` for dangling
      endpoints and a new unreachable cyclic bundle pair. Include prior invalid
      state and snapshot inputs before each rejection.
- [ ] Write `immutable_binding_and_duplicate_identity_reject` with independent
      changes to source/kind/source name/alias/scope/raw root and duplicate
      identity under a new ID. Validate bundle source/name immutability too.
- [ ] Write `edits_are_complete_and_deterministic` for reordered upserts/edge
      additions, duplicate/missing/overlapping edits, and unreachable upserts.
      Verify revision/requested-surface updates explicitly and an empty no-op.
- [ ] Write `deep_dependency_chain_is_iterative` with 2,000 bundles, and
      `tagged_legacy_and_prior_artifacts_are_not_mutated` with retained source
      labels and ownership rows. Graph output grants no artifact effect
      authority.
- [ ] Run focused tests. Prove removed retention traversal, binding checks and
      candidate validation fail the intended assertions; restore exact source
      bytes and rerun focused green. Run full verification after documentation.
- [ ] Update specification/design and commit
      `feat: reconcile authoritative lifecycle references`.

## Completion and next work

- [ ] Run `just fmt`, `just verify`, and
      `cargo +1.85.0 check --all-targets --locked`, all without warnings.
- [ ] Fresh independent implementation review, then required PR reviews and
      exact-head hosted CI. Apply the user's severity/effort policy.
- [ ] Garden only this plan; preserve the active Gate 5 umbrella and other WIP.
- [ ] Continue task 2b shared-owner reconciliation and complete
      add/update/remove planning. Then guarded backup/application/recovery and
      read-only adapters. This graph slice alone neither completes task 2b nor
      Gate 5/6 or #284.
