# Captured lifecycle selection preflight

The opt-in `lifecycle::selection::select_owned_nodes` library boundary accepts
borrowed validated `CombinedState` and `Selection::All` or `Selection::Names`.
It returns `SelectionOutcome::Ready(SelectedNodes)` or `Blocked` with
descriptive candidates and typed diagnostics. Invalid entire state or malformed
requests return errors. It performs no I/O or input mutation.

## Literal selection and graph closure

Names match installed aliases, owned/legacy bundle names, and legacy plugin/MCP
names or declaring bundle labels. Matching is literal: source names, source
labels, scoped identities, glob patterns and CLI syntax are not interpreted. A
request must contain nonempty unique names and must not be empty. Unknown names
and names matching multiple rows block instead of choosing a first match. One
native row counts once even when its own and bundle names both match.

Selecting an owned bundle includes its authoritative dependency and installation
closure. Shared dependency paths are visited once. Node results retain prior
installation/bundle order, independently of request or traversal order.
Selecting an installation alone does not select its parent bundles or other
members.

## Complete blockers and preserved history

All selection includes every owned node and typed legacy item, bundle, plugin
and MCP row. Every selected legacy row blocks, including duplicate, malformed,
unsupported or apparently identical rows. Legacy indices address the unchanged
original vectors; no source label is reparsed and no ownership is inferred.
Historical native rows declaring a selected owned bundle name also block,
including bundles reached transitively. This conservative label association is
not proof of native identity or an instruction to apply historical native
effects.

Unselected malformed legacy history does not block explicitly disjoint work.
Only selected Rule installations pass the owned-kind check; selected Skill or
Agent installations block even with no artifacts. Unsupported descendants of a
selected owned bundle block too. All blockers are collected before returning;
Blocked candidates never identify an independently ready subset.

Name diagnostics are in lexical requested-name order, followed by selected
legacy items/bundles/plugins/MCPs in their original category/index order, then
unsupported installations in prior order. Candidates include all matching owned
nodes and their bundle closure even when other selection work blocks.

## Boundary and later integration

Ready means this selection preflight passed. It establishes no source
authenticity, filesystem containment, native support, compatible sharing, or
authority to remove references or apply outputs. Later orchestration must
enforce this outcome, complete desired-set/reference planning, qualified
observation and execution. Source/scope selectors and flag/config/history
precedence remain later work; production CLI selection behavior is unchanged.
Production schema remains 1. Gate 5/M1 activation, backup/recovery and Codex
qualification remain incomplete.

## Components

- [Pure selector](../../src/lifecycle/selection.rs)
- [Public acceptance examples](../../tests/lifecycle_selection.rs)
- [Dated validation record](../technotes/2026-10-16-m1-gate5-selection-validation.md)

Satisfies:
[specification §4.17](../specification/specification.md#417-captured-lifecycle-selection-preflight).
