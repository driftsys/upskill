# Upskill roadmap

Updated: **2026-10-19**. Status: **planned delivery scope**. This roadmap
describes upcoming work, not implemented support. Release numbers and dates will
be assigned separately.

## Direction

Upskill manages agent customization from one source, with a client adapter
choosing native conventions/configuration or active plugin behavior for each
feature. Decisions prioritize **reliability and performance**, then
**maintainability as clients evolve**.

The first four clients are **Claude Code, Copilot CLI, Codex and OpenCode**.
Each receives a thin upskill plugin integration backed by shared Rust CLI logic.
Client-specific packages can share one source repository.
**`driftsys/metapowers` is the marketplace repository.**

Plugin packaging and runtime loading are separate decisions: a plugin can
distribute or update content that the client loads through native conventions.
Hooks are used where they provide required behavior, rather than for every
customization.

## MVP — Next release

Tracking: [MVP epic #283](https://github.com/driftsys/upskill/issues/283).

Pending proposal: the
[customization delivery proposal](wip/specs/2026-10-23-customization-delivery-proposal.md)
(2026-10-23) proposes adding subagents to this MVP and moving bundles and plugin
authoring to the following release. It does not address the updater deliverable,
which keeps its place along with the three implementation dependencies below. It
is not adopted; this table is re-sequenced only when that proposal is approved.

Deliver **automatic updates, rules, skills, bundles and plugins** across the
four initial clients.

| Deliverable             | Expected outcome                                                                                              |
| ----------------------- | ------------------------------------------------------------------------------------------------------------- |
| Upskill updater plugins | Each client integration triggers shared maintenance for the upskill CLI and installed skills                  |
| Rules                   | Generic defaults yield to explicit user and team instructions; each adapter selects a qualified loading route |
| Skills                  | Native discovery and invocation, supporting resources, and reliable install/update/remove                     |
| Bundles                 | Coherent collections of content, dependencies and plugin declarations                                         |
| Plugins and marketplace | Client-native installation/update/remove and distribution through metapowers                                  |

### Implementation dependencies

Complete these prerequisites in order before implementing automatic maintenance:

1. Finish M1 and qualify its ownership-aware rule/instruction lifecycle through
   Gate 6 activation.
2. Extend the activated ownership-aware lifecycle to skills, including
   resources, local-edit preservation, update, removal and interruption
   recovery.
3. Implement shared automatic maintenance and qualify its client integrations.

M1 alone does not provide owned-skill updates. The broader four-client MVP scope
remains the release goal; individual foundation slices do not complete it.
Harmless lifecycle probes may establish candidate entrypoints before these
implementation prerequisites are complete.

### Updater qualification

Start with a small installed-plugin probe for each client. Verify that its
startup/lifecycle entry point can invoke the CLI, then implement the shared
maintenance path. Test local packages before marketplace publication.

The Rust CLI owns downloads, binary updates and installed-content
reconciliation. Plugins supply client lifecycle integration. Native marketplace
plugin updates and upskill CLI/content updates remain distinct operations.

Define update cadence, channels, pinned versions, binary installation ownership,
and project/user scope behavior before implementation. Updates must preserve
local changes, avoid duplicate work from concurrent sessions, and retain usable
installations when offline or interrupted. Document when a reload or new session
is required. Measure startup overhead against an agreed baseline and budget.

### Rules and skills

Use qualified native rule loading for Claude and Copilot. Independent
Codex/OpenCode rules require qualified plugin delivery alongside unchanged team
`AGENTS.md`. Managed contributions to that file remain experimental library
work, not the production route or an automatic fallback. Use explicit
default/override semantics in the rule text; numeric rule priorities are not an
MVP requirement.

Keep native skill loading where it meets the requirements. Adapt client metadata
and resources without duplicating content discovery in runtime hooks. Avoid
duplicate activation when content is installed both directly and through a
plugin. All customizations follow the
[delivery/discovery/activation contract](topics/0009-customization-delivery.md):
explicit reader/consumption intent, compatibility-directory/import/glob
inventories, unintended-loading and duplicate-activation controls, and
ownership/configuration preservation. Existing agents and MCP integrations
remain regression obligations; planned general hooks/tools/permissions retain
later scope.

### MVP release gate

- Compare intentional `.agents` sharing, intentional `.claude` sharing and
  selective compatibility exclusions. Establish native feasibility before
  selecting adapters and integrated coexistence before adopting a root. Keep new
  owned-skill lifecycle after Gate 6; preserve existing kinds during Gate 6 and
  qualify the new layout before its adoption and combined MVP release under the
  [shared-root evaluation](technotes/shared-content-root-evaluation.md).

- Select and record supported released-client versions, scopes and operating
  systems; distinguish OpenCode v1 compatibility from v2 support.
- Verify installed updater execution and real CLI/skill updates on all four
  clients, including offline, concurrent and interrupted runs.
- Require independent installed-plugin rule delivery for Codex/OpenCode, with
  unchanged team guidance and disabled/removed-plugin negative controls.
- Apply CD-01 through CD-11 per customization/client/scope: qualify intended
  discovery/activation and reject unintended readers or duplicate activation.
- Verify rules, skills, bundle dependencies and plugin lifecycle through native
  discovery and representative behavior, not generated-file checks alone.
- Preserve existing configurations, restrictions and supported agent/MCP
  behavior.
- Validate metapowers package/catalog references and publish installation,
  migration, update and reload guidance.

Internal maintenance and rule-delivery hooks are allowed in the MVP. General
management of user-authored hooks belongs to the following release.

## Post-MVP — Cleanup and refactoring

Tracking: [Job 1 epic #291](https://github.com/driftsys/upskill/issues/291).

Consolidate the implementation after the MVP passes its release gates and before
adding the extra customization capabilities.

- Remove dead code and obsolete adapter paths proven unused by supported
  installations.
- Replace duplicated generation, configuration and lifecycle logic with shared
  code where the client contracts actually match.
- Reconcile legacy configuration handling and migration paths. Retain required
  compatibility; explicitly deprecate supported behavior before removing it.
- Simplify module boundaries using the four working adapters as evidence, rather
  than designing abstractions ahead of their use.
- Update tests and documentation to describe the resulting architecture.

The exit gate is unchanged behavior for the supported MVP matrix, verified by
its regression and migration tests. Cleanup does not expand product scope.

## Extra capabilities — Following release

Tracking: [Job 1 epic #291](https://github.com/driftsys/upskill/issues/291).

Extend the same four adapters with these managed capabilities:

| Capability     | Scope                                                                                     |
| -------------- | ----------------------------------------------------------------------------------------- |
| Tools          | Map tool identifiers and capabilities; expose unsupported mappings                        |
| Permissions    | Translate native approval and execution controls without silently broadening access       |
| Hooks          | Manage client events, payloads, results, trust and installation lifecycle                 |
| MCP management | Manage server configuration, transport/authentication references, discovery and lifecycle |

Qualify each capability across the four clients before expanding the release
matrix. Test combined configurations as well as individual features. This stage
adds first-class management; it does not remove MCP functionality already
shipped or postpone respecting native permissions in the MVP.

## Later expansion

- **Cursor and Gemini CLI:** include their requirements in the model now;
  schedule adapter implementation after the initial four-client releases.
- **Subagents:** retain in the customization inventory; management expansion is
  not yet assigned to a release.
- **VS Code:** qualify profiles for the actual Copilot, Claude, Codex or Local
  runtime. Reuse client integrations where supported; add editor-specific
  integration only where needed. Selecting a model does not select its vendor's
  agent runtime.
- **Remote and cloud surfaces:** establish configuration discovery, executable
  deployment and trust separately before claiming support.

## References

- [Client support commitment](adr/0013-agent-customization-targets.md)
- [Six-client capability evidence and plugin examples](technotes/customization-adapter-evidence.md)
- [Original customization research](technotes/agent-customization/README.md)
