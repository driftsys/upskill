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

- [Customization MVP re-steer design](specs/2026-11-11-customization-resteer-design.md)
  and
  [implementation plan, part 1](plans/2026-11-11-customization-resteer-part1.md)
  — approved by the user on 2026-11-11. Replaces the gated delivery approach
  with an ownership record in the lockfile, repeatable operations and support
  levels. Part 1 covers delivery steps 0 to 4. Step 0, which removes the
  unreachable lifecycle code, is the first change.

- [Copilot-local plugin exclusion profile](../technotes/copilot-project-plugin-overlay-profile.md)
  : proposed under [#446](https://github.com/driftsys/upskill/issues/446).
  Primary references and Copilot 1.0.103 help support investigating one owned
  local plugin-map entry. Native execution, fixture provisioning and model
  allowance require approval; no new run or gate amendment is adopted.

- [Customization delivery policy comparison](../technotes/customization-delivery-policy-comparison.md)
  : decision package under
  [#442](https://github.com/driftsys/upskill/issues/442) after the completed
  Q11b isolation FAIL. Recommends separate representations as a design baseline
  without adopting it. Shared-profile consent, concrete selectors/preservation
  and any gate amendment still require their own decision and evidence. No
  further native experiment is scheduled.

- [Preflight discovery and import inventory](specs/2026-10-25-preflight-discovery-inventory-design.md)
  : proposed under [#417](https://github.com/driftsys/upskill/issues/417).
  Defines read-only capture, reader classification, glob/import closure,
  duplication/drift rejection and planned acceptance examples. Design approval
  and implementation remain pending. The merged
  [Q11 packet](../technotes/gate6-cross-client-package-isolation.md) records
  explicit-selector activation; its proposed follow-up stays with the Q11 native
  owner
  ([control design](../archive/specs/2026-10-25-q11-registration-isolation-follow-up.md)).

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

- [Shared updater design](specs/2026-10-14-shared-updater-design.md) and
  [implementation plan](plans/2026-10-14-shared-updater.md) — proposed
  `upskill maintain` command; superseded in part by section 4 of the
  [re-steer design](specs/2026-11-11-customization-resteer-design.md).
  Implementation waits for delivery step 6 and design review.

- [Agent customization foundation design](specs/2026-10-11-agent-customization-foundation-design.md)
  — AC-00 target capabilities, ownership, installation planning, migrations, and
  permission preservation; proposed contracts with explicit evidence gates.

- [Agent customization delivery roadmap](specs/2026-10-11-agent-customization-roadmap.md)
  — Claude Code, Copilot, and Codex first; instructions, skills, subagents, MCP,
  marketplaces/plugins, hooks, and permissions. Proposed sequencing and
  acceptance gates; no implementation claim.

## Superseded on 2026-11-11

The [re-steer design](specs/2026-11-11-customization-resteer-design.md)
superseded the following work. The library code that the Gate 4 and Gate 5 work
added was removed from `main` and is kept on the branch
`archive/lifecycle-2026-11`.

- [Q11 intended-client registration and unintended discovery follow-up](../archive/specs/2026-10-25-q11-registration-isolation-follow-up.md),
  with the
  [concrete project profile](../archive/specs/2026-10-27-q11b-project-profile.md)
  and the
  [native-name revision](../archive/specs/2026-10-30-q11b-native-name-profile.md),
  is archived. It was proposed under
  [#421](https://github.com/driftsys/upskill/issues/421). The results are in
  [Q11](../technotes/gate6-cross-client-package-isolation.md),
  [the 2026-10-29 attempt](../technotes/gate6-registration-isolation.md) and
  [the completed registration cell](../technotes/gate6-project-registration-isolation.md).
- [Gate 6 activation design](../archive/specs/2026-10-18-gate6-activation-design.md)
  and
  [corrected customization delivery plan](../archive/plans/2026-10-19-customization-delivery-reconciliation.md)
  are archived. They defined the CD-01 to CD-10 delivery requirements and the
  schema-2 activation gate.
- Gate 5 combined lifecycle: its
  [umbrella specification](../archive/specs/2026-10-15-m1-gate5-lifecycle-integration.md)
  and [completion lane](../archive/plans/2026-10-18-gate5-completion.md) are
  archived. The [acceptance evidence](../technotes/m1-gate5-final-acceptance.md)
  records the library implementation as it was before removal.
- [M1 gate 4: qualified file application and recovery](../topics/0008-execution-and-recovery.md#architecture-qualified-file-execution)
  was an opt-in library implementation. Its raw scope and plan are archived.

## Retained context, not active proposals

The draft `specs/2026-10-23-customization-storage-delivery-candidates.md` is
superseded on 2026-11-11 by the
[re-steer design](specs/2026-11-11-customization-resteer-design.md). A byte copy
is kept as an
[unapproved archived comparison](../archive/specs/2026-10-23-customization-storage-delivery-candidates.md).
It was registered as retained comparison context only. It is not an active
proposal or an adopted route. The
[design review ledger](https://github.com/driftsys/upskill/pull/385#issuecomment-6384387650)
identifies the draft and requires its disposition; the
[delivery proposal](specs/2026-10-23-customization-delivery-proposal.md#records-superseded-or-amended-if-this-proposal-is-adopted)
records this retained-context choice. The
[self-contained policy comparison](../technotes/customization-delivery-policy-comparison.md)
now evaluates its A/B alternatives against retained Q11/Q11b evidence without
moving the draft or adopting a layout.

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
