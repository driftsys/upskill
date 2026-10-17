# Agent customization delivery roadmap

Date: 2026-10-11. Status: **Proposed delivery roadmap**. Baseline: `b866f5e` (PR
#250); no adapter implementation is included here.

Sequencing update (2026-10-14): the user now requests four initial clients and
an updater/content/plugin MVP before tools, permissions, hooks and MCP
management. See the [new roadmap](2026-10-14-customization-adapter-roadmap.md).
The milestones below retain the earlier proposal for context; use the newer
roadmap for release order. Existing foundation and lifecycle requirements remain
inputs to that work.

## Outcome and scope

Make upskill a reliable authoring, installation, and maintenance tool for
**agent customization**, with Claude Code, GitHub Copilot, and Codex delivered
first. Cover instructions/rules, skills, subagents, MCP servers,
marketplaces/plugins, hooks, and permissions. Keep bundles as the composition
unit for content, dependencies, and native integrations.

The user selected these three implementation priorities and added hooks and
permissions explicitly. The sequencing, surface boundaries, and work breakdown
below are recommendations for that request. They refine the implementation order
without replacing [ADR-0013](../../adr/0013-agent-customization-targets.md).
OpenCode remains supported and covered by regression checks; new OpenCode
capabilities follow the priority three. Cursor and Gemini CLI remain required
design targets with implementation deferred.

“Support” means a declared, tested contract for a client surface, version,
operating system, scope, category, and packaging form. It does not mean every
native option has an equivalent on another client. Each capability must be
classified as supported, limited, unsupported by that client, or deferred by
upskill. Missing implementations must not be described as native limitations.

## Starting point

The merged
[research pack](../../technotes/2026-10-11-agent-customization/README.md)
provides dated contracts and schema snapshots; its client reports have no native
runtime conformance evidence. Implementation must select and test released
versions rather than treating rolling documentation as a version guarantee.

| Area                  | Existing upskill baseline                                                  | Work required                                                        |
| --------------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Content               | `Client` generates rules, skills, and agents for Claude, Copilot, OpenCode | Add Codex; correct semantics, metadata, and scope discovery          |
| MCP                   | Separate Claude, Copilot, VS Code, and OpenCode targets                    | Add Codex; refresh paths/transports; reconcile lifecycle             |
| Plugins               | Per-client bundle descriptors and native CLI operations                    | Add Codex; verify lifecycle, marketplace ownership, and packaging    |
| Hooks and permissions | No first-class managed category                                            | Define native contracts, ownership, validation, and lifecycle        |
| Bundles and state     | Existing dependencies, content, plugin/MCP declarations, lockfile          | Extend without losing existing installations or manual configuration |

Code anchors: `src/generate/mod.rs`, `src/model/bundle.rs`, `src/mcp.rs`,
`src/plugin.rs`, `src/pipeline/`, and `src/lockfile.rs`. Research identifies
Copilot user-path and tool-mapping drift, shared `.agents/skills` exposure, and
stale MCP assumptions. Reproduce these before choosing fixes; no research
observation alone establishes runtime behavior.

## Target surfaces

| Surface                                                                           | Delivery requirement                                                                               |
| --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Claude Code CLI, local execution                                                  | Primary target across all seven categories                                                         |
| Copilot CLI, local execution                                                      | Primary target across all seven categories                                                         |
| Codex CLI, local execution                                                        | Primary target across all seven categories                                                         |
| Copilot in VS Code Local                                                          | Required companion target; separate discovery, MCP, hooks, and policy contracts                    |
| Codex IDE using local configuration                                               | Verify shared configuration discovery for each applicable category; do not infer UI/runtime parity |
| Copilot Agent Host, cloud agents, hosted Codex, Claude web/desktop-specific modes | Record boundaries now; separate delivery after local support                                       |
| OpenCode                                                                          | Preserve supported behavior; resolve regressions caused by shared changes                          |
| Cursor and Gemini CLI                                                             | Exercise the proposed model against their research; defer adapters                                 |

Project and user scope are required wherever the chosen native surface supports
them. Private/local and managed/enterprise scopes must be identified, but fleet
policy deployment is deferred. Unsupported scopes must be diagnosed before
writes; substituting a home directory is not scope support. M0 selects exact
versions and an OS matrix, including Linux, macOS, and Windows where the native
client is available. Platform-specific limitations remain visible.

## Design principles and early decisions

Use a portable content core plus validated native extensions. Avoid a universal
schema that flattens discovery, events, or enforcement into approximate fields.
Keep the existing Rust/static-binary constraints and separate pure generation
from installation effects. Do not introduce a runtime plugin SDK as a
prerequisite.

The foundation design must settle these contracts before adapters expand:

1. **Target identity and capabilities.** Family, surface, scope, version, OS,
   packaging form, discovery, and feature state. Preserve the existing
   distinction between content clients and integration targets in ADR-0012.
2. **Artifact and action model.** One customization may produce Markdown,
   JSON/JSONC, TOML, policy files, sidecars, scripts, and native CLI operations.
   Define validation and ownership for each, including editable shared files.
3. **State and lifecycle.** Record provenance, source/resolved revision, owned
   paths or keys, dependency references, and actual operation outcomes. Plan
   before applying; preserve user changes; make interrupted work recoverable.
4. **Schema evolution.** Choose versioned source and lockfile migrations. Old
   installs must remain updatable/removable; newer unsupported schemas need
   actionable errors. A change of default target selection needs an explicit
   compatibility decision, not an incidental new enum member.
5. **Permission preservation.** Keep prose, tool exposure, permission grants,
   approval policy, and filesystem/network enforcement separate. Unsupported
   restrictions fail validation rather than disappear. Distinguish absent,
   empty, and populated allowlists from the first content milestone.
6. **Bundles.** Extend composition to the new categories, with per-target
   selection, dependency ordering, cycle/conflict detection, and provenance for
   components supplied both directly and by plugins. A bundle is not a native
   plugin manifest.
7. **Author and consumer workflows.** Extend scaffolding, formatting, linting,
   validation, installation previews, listing, updating, removal, and
   diagnostics for each category. Errors identify the source field and affected
   target. Validate native extensions as well as the common model; unknown
   policy or hook fields cannot silently pass through as effective
   configuration.

Exact field names, command syntax, new item kinds versus bundle declarations,
and parser dependencies are M0 design outputs. This roadmap does not pre-approve
a schema or supersede existing ADRs. Record accepted changes in concern-specific
ADRs, especially updates to ADR-0003, ADR-0008, ADR-0010, and ADR-0012.

## Milestones and dependencies

Deliver by category across the priority clients, so Codex receives useful
support early and translation differences are exercised before later categories
depend on them. Complete each category's lifecycle before describing it as
supported. Milestones are ordered deliverables, not calendar estimates or
release numbers.

| Milestone | Deliverable                                  | Depends on        | Exit evidence                                                            |
| --------- | -------------------------------------------- | ----------------- | ------------------------------------------------------------------------ |
| M0        | Target contracts and installation foundation | Research/ADR-0013 | Accepted contracts, version matrix, migration and ownership tests        |
| M1        | Instructions and rules                       | M0                | Native discovery and scope tests; existing-file preservation             |
| M2        | Skills and bundle composition                | M1                | Discovery/invocation, resources, metadata, collision and lifecycle tests |
| M3        | Subagents                                    | M2                | Native agent discovery, inheritance and restriction tests                |
| M4        | MCP                                          | M3                | Config/discovery plus controlled connection/tool tests                   |
| M5        | Marketplaces and plugins                     | M4                | Native package validation and install/update/remove reconciliation       |
| M6        | Hooks                                        | M5                | Native event, result, failure, trust, and update tests                   |
| M7        | Permissions and execution policies           | M6                | Effective allow/ask/deny and sandbox checks per supported surface        |
| M8        | Release qualification and migration guide    | M1–M7             | Complete supported matrix and end-to-end bundle evidence                 |

Permission semantics are designed in M0 and tested wherever restrictions appear
in M1–M6. M7 adds full policy authoring/distribution; it is not permission to
ship earlier categories with lost restrictions. Native permission investigations
can run early. A native plugin that introduces hooks or policy changes is gated
by M6/M7 for those components; M5 alone cannot claim their managed support.

### M0 — Contracts, compatibility, and ownership

Produce the detailed design, capability inventory, supported-version fixtures,
and install-plan/state contract. Review the model against Cursor and Gemini CLI
requirements before committing it. Introduce only the common machinery needed by
M1; evolve it through later category slices.

Inventory native fields, events, handler types, transports, and policy controls
per target, not just seven category-level checkmarks. Give every inventory entry
a supported, limited, native-unsupported, or deferred disposition and a delivery
reference. This makes later extension work visible without requiring every
experimental vendor feature in the first release.

Acceptance examples:

- Selecting an unsupported surface/scope or an untranslatable restriction
  reports the item, target, reason, and remedy before mutation.
- Two selected clients that discover the same path produce one compatible owned
  artifact or a conflict; incompatible client-filtered content is never silently
  shared. Removing one consumer preserves an artifact still used by another.
- Preview lists file/key changes, native operations, and effective permission
  changes. It performs no installs, hook execution, or authentication.
- Repeating an install is idempotent. A changed user-owned key causes a precise
  conflict; unrelated text, comments, keys, and secret references survive.
- Partial native-operation failure remains visible and retryable. Do not promise
  rollback of an external installer unless the adapter can actually perform it.
- Upgrade fixtures cover prior source labels and lockfiles, including
  [#236](https://github.com/driftsys/upskill/issues/236). Source-format, state,
  and generated-path migrations have separate compatibility tests.

### M1 — Instructions and rules

Implement project/user instruction discovery, unconditional instructions,
directory/path-scoped rules where supported, aggregation, ordering, and owned
edits. Add Codex as an explicit selectable target without silently changing
existing defaults. Resolve Copilot personal paths and Claude bridge/fallback
assumptions against the pinned versions.

For Codex, define how multiple rule items contribute to `AGENTS.md` and how
existing content, override files, nested instructions, and size limits are
handled. Arbitrary file globs must not silently become directory-wide rules.

Acceptance: a fresh native session discovers the generated instruction at the
right scope; matching and nonmatching contexts distinguish conditional rules; an
unsupported condition is rejected or requires an explicit author alternative.
Install/update/remove preserves manual guidance and other bundles'
contributions.

### M2 — Skills and composition

Preserve portable Agent Skills metadata, scripts, references, and assets;
support native sidecars and invocation controls through validated extensions.
Address Codex `agents/openai.yaml`, Copilot surface naming/discovery, and shared
`.agents/skills` ownership. Apply permission checks to native tool grants too.
Keep scripts as packaged resources; installation does not execute them.

Acceptance: one bundle installs instructions plus a resource-bearing skill into
all selected targets; the native client lists and invokes it; resource links and
metadata survive; implicit/manual invocation controls behave as declared.
Duplicate names and shared directories have deterministic, documented outcomes.
Updating or removing one dependency leaves other owners and user files intact.
Commands/saved prompts receive an explicit disposition: prefer skills for new
portable workflows, retain supported legacy inputs or document migration; do not
invent a universally portable slash-command kind.

### M3 — Subagents

Support native Claude/Copilot Markdown profiles and Codex TOML agents, with
prompts, description/identity, models, tool exposure, instruction inheritance,
skill dependencies, and native execution settings. Model aliases and skill
preloading need documented mappings or native alternatives. Follow
[ADR-0014](../../adr/0014-versioned-agent-model-mappings.md): ship offline,
versioned S/A/B/C defaults, allow explicit mapping updates and persistent global
overrides, and resolve models at installation or regeneration. Copilot uses
ordered `models` preferences with one agent-wide effort; verify native model IDs
before implementing the generator. Preserve the difference between selecting a
main agent and delegating to a subagent.

Acceptance: native discovery and a bounded delegated task use the intended
profile. Tests distinguish omitted tools, no tools, and a restricted tool set;
an unmappable restriction cannot produce an unrestricted agent. Test native
inheritance and precedence, including conflicting names and parent constraints.
Prose mentioning a skill is not reported as native skill preloading. Features
such as memory, worktree isolation, and handoffs get explicit capability entries
rather than silently discarded fields.

Model acceptance: offline installs are repeatable for a pinned mapping version;
a mapping update changes no installed agent until explicit regeneration. Global
overrides survive the update and win at regeneration. For Copilot, generated
preferences preserve the documented order, use `modelPolicy: preferred`, and
apply one supported effort to the whole list. Tests cover a user without access
to any preferred model, a stale native ID, and an incompatible effort without
claiming that `auto` enforces a tier.

### M4 — MCP configuration and lifecycle

Manage stdio and remote HTTP contracts supported by each selected target; record
deprecated or unavailable transports explicitly. Cover project/user
configuration, tool filters, timeouts, environment/header references, and
authentication requirements without copying credential values into bundles,
lockfiles, or diagnostics. Keep CLI and VS Code configuration envelopes
separate.

Acceptance: install/update/remove and target narrowing reconcile only owned
server entries. Resolve [#244](https://github.com/driftsys/upskill/issues/244)
through an accepted prune-versus-explicit-removal decision and corresponding
tests. A controlled server proves connection and tool discovery; offline,
unauthenticated, configured, and connected remain distinct states. Missing
credentials produce setup guidance. Stdio process launch and HTTP authentication
tests run in disposable environments, not the developer's real configuration.

### M5 — Marketplaces, native plugins, and package output

Manage marketplace registration separately from plugin installation, enablement,
and updates. Add Codex descriptors and refresh Claude/Copilot lifecycle
commands. Record source, marketplace identity, requested/resolved version where
available, scope, and dependencies. Where a native installer cannot pin a
version, report that limitation instead of promising reproducibility.

Add generation of native plugin packages from compatible bundle subsets, with
portable Agent Plugins output where supported and validated native extensions.
Standalone-to-plugin translation must diagnose components whose meaning changes
or which the format cannot carry. Keep VS Code extensions distinct from agent
plugin packages. Public marketplace hosting/submission is outside this roadmap.

Acceptance: a package validates with the selected native tooling, installs,
appears in native inventory, updates, disables/enables where supported, and
uninstalls without removing unrelated packages or shared marketplaces. A plugin
and a direct item providing the same capability produce an explicit collision or
deduplication decision. Update can repair the managed state it advertises; reuse
relevant lifecycle evidence from
[#229](https://github.com/driftsys/upskill/issues/229), while its
OpenCode-specific CLI repair remains maintenance work. Required native
operations cannot be recorded as successful when the CLI is missing or
installation was skipped.

### M6 — Hooks

Introduce hook authoring and distribution with native event identifiers,
matchers, handler types, input/output contracts, resource paths, environment,
timeouts, working directory, and supported execution platforms. Start with
command handlers and session/tool lifecycle events; inventory and then add
remaining supported events and handler types through explicit follow-up slices.
Do not equate same-named events or translate hook JSON by renaming keys alone.

Acceptance: controlled fixtures prove event firing, matcher selection, payload
parsing, allowed result handling, denial where supported, timeout, malformed
output, and failure behavior in each client. Preserve concurrency/order
limitations. Scripts resolve after installation and handle paths containing
spaces. Updating a hook exposes changed behavior and renewed trust requirements;
upskill does not manufacture native trust approvals. Removal cleans only owned
definitions/resources. Plugin hooks and standalone hooks receive separate tests.

Hooks and permissions interact but remain separate categories: hooks may
influence actions; they are not a substitute for a sandbox. Codex requires trust
of changed non-managed definitions, independently of installation.
[Official hook contract](https://learn.chatgpt.com/docs/hooks).

### M7 — Permissions and execution policies

Manage four distinct controls: tool exposure, tool/command approval decisions,
filesystem access, and network access. Cover native allow/ask/deny semantics,
policy inheritance, scope, and managed constraints. Use native policy artifacts
where equivalent portable intent cannot be proven. Do not turn an agent tool
list into a permission grant or assume instructions enforce access control.

The policy design must account for Claude settings, Copilot CLI session grants
versus saved local approvals, VS Code approval settings, and Codex command rules
plus sandbox/permission profiles. Do not distribute personal approval caches as
team policy. If a control exists only as launch-time options, report that and
provide an explicit application recipe; adding a client launcher requires a
separate design decision.

Acceptance: an isolated runtime test attempts permitted and prohibited actions,
with actual tool execution recorded. Conflicting allow/deny rules, omitted/empty
lists, parent/managed constraints, scope overrides, updates, and removal have
tests. Preview identifies broader access and its provenance. Installation does
not change trust or managed policy; policy activation is explicit. A required
control without a faithful native implementation fails before writes.

Version-sensitive examples, checked against official documentation on
2026-10-11:

- Claude evaluates permission rules with native precedence; sandboxing is a
  separate boundary.
  [Claude permissions](https://code.claude.com/docs/en/permissions).
- Copilot CLI tool availability and execution authorization are separate;
  session flags are not persisted approval state.
  [Copilot permissions](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/allowing-tools).
- A VS Code terminal auto-approval value of `false` requests approval rather
  than denying execution.
  [VS Code approvals](https://code.visualstudio.com/docs/agents/run/approvals).
- Codex permission profiles are beta and do not generally compose with legacy
  sandbox settings. Test the selected mode and managed exceptions explicitly.
  [Codex permissions](https://learn.chatgpt.com/docs/permissions).

### M8 — Qualification and release

Ship a maintained capability matrix and migration guide. Demonstrate a realistic
bundle combining the seven categories, including plugin-provided components, on
every claimed surface/scope/OS combination. Test fresh install, repeated
install, upgrade, target narrowing, interrupted operation, repair, and removal.
Exercise the existing OpenCode paths for regressions. Publish smaller category
releases as ready; the overall support claim waits for this qualification gate.

Every stable advertised capability needs a pinned client/version fixture and
native evidence. Experimental features remain opt-in with limitations recorded.
Refresh source/schema provenance when upstream changes; do not claim native
conformance from schema validation, golden files, or mocked CLI exits alone.
Untested or deferred cells remain visible and prevent an unqualified “full
support” claim. No runtime tests have been performed as part of this roadmap.

## Work breakdown and delivery gates

These identifiers are planning references, not GitHub issue numbers. For each
milestone, turn its acceptance examples into focused story/task issues and
implementation plans before coding; avoid a single PR per entire milestone.

| Planning ID | First reviewable slices                                                                                   |
| ----------- | --------------------------------------------------------------------------------------------------------- |
| AC-00       | Capability/version inventory; architecture decisions; migration/ownership fixtures; installation planning |
| AC-01       | Existing instruction/path corrections; Codex aggregation; conditional-rule conformance                    |
| AC-02       | Portable metadata/resources; Codex sidecars; bundle/shared-path lifecycle                                 |
| AC-03       | Existing tool mapping corrections; Codex profiles; inheritance and delegation conformance                 |
| AC-04       | Existing MCP contract corrections; Codex TOML; authentication/status; narrowing reconciliation            |
| AC-05       | Marketplace ownership; native lifecycle adapters; bundle-to-plugin packaging; update/repair               |
| AC-06       | Hook contract/resource model; per-client command hooks; native event/handler coverage; plugin integration |
| AC-07       | Native policy model; per-client policy application; conflict and effective-enforcement tests              |
| AC-08       | Combined bundle qualification; migration guide; capability publication and release evidence               |

No open epic was present in the issue inventory checked on 2026-10-11.
Repository rules reserve epic creation to humans, so this roadmap does not
create orphan stories. Once a parent epic exists, create children with
`Epic: #N`, the matching `epic:<name>` label and type, and update its checklist.
Size the actual slices before applying the repository severity/effort/priority
matrix. Existing issues remain open references; this roadmap does not claim to
resolve them.

Inventory evidence:
`gh issue list --repo driftsys/upskill --state open --limit 100 --json number,title,labels`
returned four issues: #245, #244, #236, and #229; none had an epic label.
Recheck this inventory before selecting a parent or creating child issues.

For each implementation slice:

1. Record the selected native contract and testable acceptance criteria.
2. Write failing integration examples, then focused unit/generation tests.
3. Implement in an isolated worktree and prove install/update/remove behavior.
4. Run native discovery and semantic checks in disposable homes/projects. Record
   unavailable clients as unverified, not passing.
5. Update specifications, accepted ADRs, implemented design, and user guidance.
   Run `just fmt` and `just verify`, review, then PR/CI/merge.
6. Archive completed working documents with evidence; keep unfinished scope
   here.

## Recommended next step

The
[AC-00 foundation proposal](2026-10-11-agent-customization-foundation-design.md)
develops the contracts below. It remains proposed and lists the native
validation evidence needed before M0 can be completed.

Start AC-00 with a focused design covering target identity, capability
validation, artifact ownership, and migrations, plus permission preservation and
shared-path examples. Its output should make AC-01 implementable without
revisiting the seven-category inventory. Do not start by adding Codex to every
existing enum or by rewriting all adapters at once.
