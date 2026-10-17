# Working documents

This directory contains active or unreconciled specifications and implementation
plans. It replaces `docs/superpowers/`. Historical status labels are preserved;
none of the 25 migrated documents was classified as completed merely by moving
it. Their bodies can contain historical commands, paths and assumptions.

New task designs belong in `specs/`, and their implementation plans in `plans/`.
For existing work, check its issue, implementation, tests and ADR before
selecting it for execution or archival. Pair relationships and missing design
references must be reconciled as part of that feature's review.

## Active proposals

- [Customization adapter model and delivery roadmap](specs/2026-10-14-customization-adapter-roadmap.md)
  — four-client MVP sequencing and proposed qualification gates; retained
  planning.

- [Shared updater design](specs/2026-10-14-shared-updater-design.md) and
  [implementation plan](plans/2026-10-14-shared-updater.md) — proposed
  `upskill maintain` command; implementation waits for M1 lifecycle completion
  and design review.

- [Gate 5 combined lifecycle](specs/2026-10-15-m1-gate5-lifecycle-integration.md)
  — active umbrella. The pure
  [combined-state slice](../topics/0004-state-and-ownership.md#architecture-combined-lifecycle-state)
  is implemented; its
  [plan is archived](../archive/plans/2026-10-15-m1-gate5-combined-state.md)
  with [validation evidence](../technotes/m1-gate5-state-validation.md). The
  combined-state slice merged in #295 with hosted checks passing. The pure
  [captured-rule desired-set slice](../topics/0002-generation.md#architecture-captured-rule-desired-sets)
  is also implemented; its completed task 2a
  [plan is archived](../archive/plans/2026-10-15-m1-gate5-rule-desired-sets.md)
  with [local validation evidence](../technotes/m1-gate5-desired-validation.md).
  The pure
  [reference reconciliation slice](../topics/0005-lifecycle-planning.md#architecture-lifecycle-reference-reconciliation)
  is implemented as task 2b's first bounded delivery, following #296
  (`b01a67d`); its
  [plan is archived](../archive/plans/2026-10-16-m1-gate5-reference-reconciliation.md)
  with
  [local validation evidence](../technotes/m1-gate5-reference-validation.md).
  The removal-only
  [shared-owner retirement slice](../topics/0005-lifecycle-planning.md#architecture-lifecycle-reference-removal)
  is implemented; its
  [plan is archived](../archive/plans/2026-10-16-m1-gate5-retirement.md) with
  [local validation evidence](../technotes/m1-gate5-retirement-validation.md).
  [Captured selection preflight](../topics/0005-lifecycle-planning.md#architecture-lifecycle-selection)
  is implemented for all/literal-name requests; source/scoped selection and CLI
  precedence remain pending. The pure
  [single-owner upsert coordinator](../topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert)
  connects complete desired sets to graph and mixed-artifact planning; its
  [bounded plan is archived](../archive/plans/2026-10-16-m1-gate5-upsert.md)
  with [local validation evidence](../technotes/m1-gate5-upsert-validation.md).
  The separate
  [explicit shared upsert](../topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert-explicit-shared-upsert)
  and
  [contracted producer](../topics/0002-generation.md#architecture-captured-rule-desired-sets-explicit-contract-production)
  are implemented as inactive supplied-value boundaries. Their bounded
  [spec](../archive/specs/2026-10-17-m1-shared-reconciliation.md) and
  [plan](../archive/plans/2026-10-17-m1-shared-reconciliation.md) are archived
  with [validation limits](../technotes/m1-shared-reconciliation-validation.md).
  [Read-only lifecycle inspection](../topics/0005-lifecycle-planning.md#read-only-lifecycle-inspection)
  now provides opt-in captured list/doctor and qualified scope reports.
  Source-resolution orchestration, preview, applying command adapters and
  complete lifecycle acceptance remain pending. Gate 5 remains incomplete;
  production activation remains Gate 6.

- [M1 gate 4: qualified file application and recovery](../topics/0008-execution-and-recovery.md#architecture-qualified-file-execution)
  — opt-in library implementation and durable records; hosted Linux/macOS
  qualification pending. Gate 4 and M1 remain incomplete. The approved raw scope
  and plan are archived with links to durable successors.

- [Agent customization foundation design](specs/2026-10-11-agent-customization-foundation-design.md)
  — AC-00 target capabilities, ownership, installation planning, migrations, and
  permission preservation; proposed contracts with explicit evidence gates.

- [Agent customization delivery roadmap](specs/2026-10-11-agent-customization-roadmap.md)
  — Claude Code, Copilot, and Codex first; instructions, skills, subagents, MCP,
  marketplaces/plugins, hooks, and permissions. Proposed sequencing and
  acceptance gates; no implementation claim.

## Reconciliation inventory

The maintainer of each affected feature owns its reconciliation. Until evidence
is recorded, the document remains here. Known starting points are generic Git
URLs ([ADR-0011](../adr/0011-generic-git-url-source.md)), coupling
([ADR-0009](../adr/0009-coupling-tiers-and-dependencies.md), still Proposed),
and MCP ([ADR-0010](../adr/0010-mcp-config-write.md)). The plugin descriptor
plan's referenced design is missing; recover provenance before interpreting it
as a complete pair. Windows support also has no matching design in this
inventory.

| Document                                                                                             | Disposition            |
| ---------------------------------------------------------------------------------------------------- | ---------------------- |
| [2026-05-16-install-script.md](plans/2026-05-16-install-script.md)                                   | Reconciliation pending |
| [2026-05-22-bundle-by-name-discovery.md](plans/2026-05-22-bundle-by-name-discovery.md)               | Reconciliation pending |
| [2026-05-22-windows-support.md](plans/2026-05-22-windows-support.md)                                 | Reconciliation pending |
| [2026-05-23-plugin-descriptor-modes.md](plans/2026-05-23-plugin-descriptor-modes.md)                 | Reconciliation pending |
| [2026-05-24-item-conflict-resolution.md](plans/2026-05-24-item-conflict-resolution.md)               | Reconciliation pending |
| [2026-05-24-multi-registry-search.md](plans/2026-05-24-multi-registry-search.md)                     | Reconciliation pending |
| [2026-05-24-upgrade-ux.md](plans/2026-05-24-upgrade-ux.md)                                           | Reconciliation pending |
| [2026-06-02-supporting-resources.md](plans/2026-06-02-supporting-resources.md)                       | Reconciliation pending |
| [2026-06-03-coupling-redesign-slice1.md](plans/2026-06-03-coupling-redesign-slice1.md)               | Reconciliation pending |
| [2026-06-03-coupling-redesign-slice2.md](plans/2026-06-03-coupling-redesign-slice2.md)               | Reconciliation pending |
| [2026-06-03-mcp-support-v1.md](plans/2026-06-03-mcp-support-v1.md)                                   | Reconciliation pending |
| [2026-06-04-fmt-format-body.md](plans/2026-06-04-fmt-format-body.md)                                 | Reconciliation pending |
| [2026-06-10-generic-git-url-source.md](plans/2026-06-10-generic-git-url-source.md)                   | Reconciliation pending |
| [2026-05-16-install-script-design.md](specs/2026-05-16-install-script-design.md)                     | Reconciliation pending |
| [2026-05-22-bundle-by-name-discovery-design.md](specs/2026-05-22-bundle-by-name-discovery-design.md) | Reconciliation pending |
| [2026-05-23-comment-preserving-fmt-design.md](specs/2026-05-23-comment-preserving-fmt-design.md)     | Reconciliation pending |
| [2026-05-24-item-conflict-resolution-design.md](specs/2026-05-24-item-conflict-resolution-design.md) | Reconciliation pending |
| [2026-05-24-multi-registry-search-design.md](specs/2026-05-24-multi-registry-search-design.md)       | Reconciliation pending |
| [2026-05-24-upgrade-ux-design.md](specs/2026-05-24-upgrade-ux-design.md)                             | Reconciliation pending |
| [2026-06-02-supporting-resources-design.md](specs/2026-06-02-supporting-resources-design.md)         | Reconciliation pending |
| [2026-06-03-coupling-redesign-design.md](specs/2026-06-03-coupling-redesign-design.md)               | Reconciliation pending |
| [2026-06-03-mcp-support-design.md](specs/2026-06-03-mcp-support-design.md)                           | Reconciliation pending |
| [2026-06-03-mcp-v2-convention-design.md](specs/2026-06-03-mcp-v2-convention-design.md)               | Reconciliation pending |
| [2026-06-04-fmt-format-body-design.md](specs/2026-06-04-fmt-format-body-design.md)                   | Reconciliation pending |
| [2026-06-10-generic-git-url-source-design.md](specs/2026-06-10-generic-git-url-source-design.md)     | Reconciliation pending |

## Promotion and archival

After validating a completed feature, update its specifications, accepted ADRs,
as-built design and user guide as applicable. Record remaining work explicitly,
then move only the completed feature's raw working documents into
[archive](../archive/README.md), with links to the delivery evidence and durable
successors. Unrelated or uncertain work stays here.

The [agent customization research](../technotes/agent-customization/README.md)
is maintained as an informative technote at the user's request. Its open design
questions do not change the status of accepted ADR-0013.
