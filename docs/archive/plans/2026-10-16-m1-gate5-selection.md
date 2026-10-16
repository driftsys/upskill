# Gate 5 named selection preflight implementation plan

> Use superpowers:executing-plans for inline execution. The user delegated
> continuing implementation without another planning approval checkpoint.

**Goal:** Resolve all or literal-name selection against complete captured state,
collecting every selected legacy, unsupported-kind, ambiguous or missing-name
blocker before any lifecycle effects.

**Architecture:** Pure selection boundary over validated CombinedState. Return
Ready owned node identifiers or Blocked diagnostics with descriptive candidates.
Never infer legacy ownership, expand filesystem paths, fetch or create file
actions.

**Spec:** ../specs/2026-10-15-m1-gate5-lifecycle-integration.md
acceptance4/9/10, and ../specs/2026-10-12-m1-owned-lifecycle-design.md
mixed-state operation contract.

## Constraints

- Production schema1 and existing CLI dispatch/selection semantics unchanged.
- Literal installed names and bundle names only; no source selectors or scoped
  identity selector in this bounded API. Names spanning multiple owned nodes or
  owned/legacy rows block; never select an arbitrary match.
- All selects every owned node and legacy item/bundle/plugin/MCP row. Named
  native records match their own name or declaring bundle label, once per row.
- Named owned bundles expand authoritative bundle dependency/installation edges;
  selected descendants retain prior order and unsupported kinds block.
  Historical plugin/MCP rows declaring any selected owned bundle name also block
  once.
- Every selected legacy row blocks even if malformed or apparently identical to
  an owned record. Unselected malformed history survives without blocking.
- Selected unsupported owned kinds block even with no artifacts; Rule supported.
- Validate entire prior state first. No argument mutation, I/O, reachability
  edits, desired-set planning, graph reference removal or permission to apply
  effects.
- Stable candidates follow prior installation/bundle order; legacy row
  diagnostics follow items/bundles/plugins/MCPs and preserve separate duplicate
  indices.
- Collect unknown/ambiguous requested names in lexical order, then selected
  legacy rows and unsupported installation diagnostics. Reject empty/duplicate
  name lists and empty names as invalid requests rather than silently widening
  selection.

## Review focus

1. Bare selection includes owned plus legacy/native blockers atomically.
2. Named disjoint owned selection ignores unrelated malformed legacy rows.
3. Same aliases across scopes/sources and across owned/legacy never choose
   first.
4. Plugin/MCP bundle labels cannot hide selected historical native effects.
5. Invalid entire state blocks even when the invalid row is not named.

## Task 1: Pure complete-state selection preflight

Files: create src/lifecycle/selection.rs, tests/lifecycle_selection.rs; expose
lifecycle::selection; add durable design/specification and validation records.

Interfaces: Selection::All | Names(Vec<String>); SelectedNodes {
installation_ids: Vec<String>, bundle_ids:Vec<String> };
LegacyRow::Item(usize)|Bundle(usize)| Plugin(usize)|Mcp(usize);
SelectionDiagnostic::UnknownName(String)|AmbiguousName{
name:String,matches:usize}|LegacyRow(LegacyRow)|UnsupportedInstallation{
installation_id:String,kind:ItemKind}; SelectionOutcome::Ready(SelectedNodes)|
Blocked { candidates:SelectedNodes, diagnostics:Vec<SelectionDiagnostic> };
select_owned_nodes(prior:&CombinedState, selection:&Selection)->anyhow::Result<
SelectionOutcome>.

- [ ] Write acceptance tests and watch unresolved-interface RED. Assert exact
      typed candidates/diagnostics for all selection, disjoint names,
      collisions, native bundle labels, aliases, repeated rows, unsupported
      kinds, invalid input, lexical name diagnostics and preserved prior
      ordering/input values.
- [ ] Validate prior state and request, index literal matches without parsing
      legacy sources, collect complete candidates and blockers; Ready only if
      none.
- [ ] Run public suite GREEN. Fault controls must assertion-fail for ignored
      legacy, hidden native bundle rows, first-match ambiguity, ignored
      unsupported kinds, missing invalid-prior validation and lost prior
      ordering; restore sources.
- [ ] Document library-only boundary; source selection, CLI orchestration,
      reference changes, add/update, native compatibility, backup/execution
      remain later work.
- [ ] Run fmt, sanity, full verify, Rust1.85 locked checks; commit and final
      independent implementation review before publication, then PR two-pass
      review and CI.
