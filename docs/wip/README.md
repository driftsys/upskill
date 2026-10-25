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

- [Preflight discovery and import inventory](specs/2026-10-25-preflight-discovery-inventory-design.md)
  : proposed under [#417](https://github.com/driftsys/upskill/issues/417).
  Defines read-only capture, reader classification, glob/import closure,
  duplication/drift rejection and planned acceptance examples. Design approval
  and implementation remain pending; Q11 stays with its separate owner.

- [Per-client package preservation contract](specs/2026-10-25-customization-preservation-contract.md)
  : D1-D6 preservation policies approved by the user on 2026-10-25 after
  [#416](https://github.com/driftsys/upskill/pull/416) retained the draft.
  Concrete adapter profiles, implementation and native qualification remain
  pending. This does not adopt package defaults or production routes.

- [Customization delivery architecture proposal](specs/2026-10-23-customization-delivery-proposal.md)
  : native-first always-on rules decided by the user on 2026-10-24; other
  architecture amendments remain pending
  ([rules decision](specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24)).
  [AD-0040](../adr/0040-separate-customization-delivery-from-discovery.md) lists
  proposed amendments; Topic 0009 links to them without adopting them. The
  [slice-1 spec](specs/2026-10-23-customization-delivery-implementation-spec.md)
  marks the user's step 7 decisions separately from route evidence. The
  [durable step-7 inventory](../topics/0009-customization-delivery.md#amendment-inventory)
  reconciles merged Q1-Q9 outcomes; package/release choices and implementation
  remain open, so this proposal/spec are not archived
  ([design review ledger](https://github.com/driftsys/upskill/pull/385#issuecomment-6384387650)).

- [Gate 6 activation design](specs/2026-10-18-gate6-activation-design.md) and
  [corrected customization delivery plan](plans/2026-10-19-customization-delivery-reconciliation.md)
  — architecture reconciliation requested on 2026-10-19. Independent
  Codex/OpenCode rules remain independent of unchanged team `AGENTS.md`; the
  later
  [native-first amendment](../adr/0040-separate-customization-delivery-from-discovery.md#native-first-rules-and-implementation-order-2026-10-24)
  permits explicit OpenCode paths after Q5 PASS rather than mandatory plugins.
  Contribution experiments do not approve the production route. CD-01 through
  CD-10 cover every customization's discovery, intended readers, preservation
  and evidence. Implementation/native qualification remain active; production
  schema-2 activation stays blocked. Earlier original Gate 6 drafts remain
  preserved in the prior planning worktree; this revised design is the current
  planning copy.

- [Customization adapter model and delivery roadmap](specs/2026-10-14-customization-adapter-roadmap.md)
  — four-client MVP sequencing and proposed qualification gates; retained
  planning.

- [Shared updater design](specs/2026-10-14-shared-updater-design.md) and
  [implementation plan](plans/2026-10-14-shared-updater.md) — proposed
  `upskill maintain` command; implementation waits for M1 lifecycle completion
  and design review.

- Gate 5 combined lifecycle has completed local implementation, full
  verification, Rust 1.85 checks and an independent audit of all ten acceptance
  criteria. Its
  [umbrella specification](../archive/specs/2026-10-15-m1-gate5-lifecycle-integration.md)
  and [completion lane](../archive/plans/2026-10-18-gate5-completion.md) are
  archived. The
  [durable contract](../topics/0005-lifecycle-planning.md#combined-owned-content-lifecycle-contract)
  and [acceptance evidence](../technotes/m1-gate5-final-acceptance.md) record
  the implementation and remaining delivery requirements. Preview #312, origin
  #314, source resolution #315 and captured adapters #317 are merged. Final
  orchestration/acceptance still requires full review, hosted checks and merge.
  Production schema-2 activation and native qualification remain Gate 6. Other
  WIP retains its original paths and disposition under the user's accepted
  sequencing of the broader M1 and updater work.

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

## Retained context, not active proposals

The local, uncommitted draft
`specs/2026-10-23-customization-storage-delivery-candidates.md` is registered as
retained comparison context only. It is not an active proposal, an adopted
route, or a file included in this reconciliation PR. Its source bytes remain
untouched in the original worktree. The
[design review ledger](https://github.com/driftsys/upskill/pull/385#issuecomment-6384387650)
identifies the draft and requires its disposition; the
[delivery proposal](specs/2026-10-23-customization-delivery-proposal.md#records-superseded-or-amended-if-this-proposal-is-adopted)
records this retained-context choice.

### OpenCode explicit-path observation

The draft reports that explicit OpenCode instruction paths avoid the glob's
cache/token behavior in the user's experience. This remains user-reported and
unqualified; Q5's failed positives do not compare cache behavior. The
[2026-10-24 user decision](specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24)
now selects explicit paths, subject to Q5 retry PASS, and lists the amendment
from AD-0040's plugin-only wording to delivery independent of team `AGENTS.md`.
Criterion 7 remains: compare native and hook using identical rule bytes and
tasks, cached and uncached tokens, one client at a time. No performance
advantage or production route is qualified by this retained context
([Q5 packet](https://github.com/driftsys/upskill/blob/6ec187f5422d72e9329c4c7f9cb706468e2ebb6b/docs/technotes/gate6-opencode-current-routes.md),
[AD-0040](../adr/0040-separate-customization-delivery-from-discovery.md)).

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
