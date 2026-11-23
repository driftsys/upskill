# Upskill roadmap

Updated: **2026-11-23**. Status: **planned delivery scope**. The MVP is released
(0.9.0 to 0.9.3). The stages after it are listed in delivery order and are not
implemented unless a stage says so. Release numbers and dates of these stages
will be assigned separately.

## Direction

Upskill manages agent customization from one source, with a client adapter
choosing native conventions/configuration or active plugin behavior for each
feature. Decisions prioritize **reliability and performance**, then
**maintainability as clients evolve**.

The plugin MVP is completed, and the shipped features are stabilized in
parallel, before a new client is added. Claude Code and Copilot CLI are
supported; OpenCode is experimental; Codex is the next client. Each client can
receive a thin upskill plugin integration backed by shared Rust CLI logic.
Client-specific packages can share one source repository.
**`driftsys/metapowers` is the marketplace repository.**

Plugin packaging and runtime loading are separate decisions: a plugin can
distribute or update content that the client loads through native conventions.
Hooks are used where they provide required behavior, rather than for every
customization.

The organizing baseline is
[separate generated client representations](https://github.com/driftsys/upskill/blob/main/docs/adr/0041-separate-client-representations-as-design-baseline.md),
with explicitly qualified sharing exceptions. Resources follow the selected
representation, ownership follows references, and removal never silently loads a
default. This does not select exact installed paths or package defaults. Each
client receives a support level instead of passing release gates.

## Delivery order

| Stage | Content                                                                | Tracking   |
| ----- | ---------------------------------------------------------------------- | ---------- |
| MVP   | Install, update, removal and notices for Claude Code and Copilot CLI.  | #283       |
| 1     | Stabilize the shipped features: review debt and old working documents. | #283, #291 |
| 2     | Plugin MVP: hooks, commands, MCP and LSP servers, catalog, smoke test. | #291       |
| 3     | Refactor where the Codex adapter needs it.                             | #291       |
| 4     | Codex adapter.                                                         | #291       |
| 5     | Extra capabilities: tools, permissions, hooks outside a plugin, MCP.   | #291       |

Stages 1 and 2 run in parallel. Stabilizing the shipped features does not block
the design and development of the plugin MVP; both must be complete before stage
3 starts.

## MVP — Released

Tracking: [MVP epic #283](https://github.com/driftsys/upskill/issues/283).

The
[re-steer decision](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md)
of 2026-11-11 replaced the earlier gate sequence. The
[re-steer design](https://github.com/driftsys/upskill/blob/main/docs/archive/specs/2026-11-11-customization-resteer-design.md)
holds the full requirements. Its steps 0 to 8 were released in 0.9.0 on
2026-11-16:

| Step | Content                                                                            |
| ---- | ---------------------------------------------------------------------------------- |
| 0    | Park unused code (section 7). Record this re-steer as one decision record.         |
| 1    | Fix [#465](https://github.com/driftsys/upskill/issues/465). Run trial T1.          |
| 2    | Ownership record, schema 2, adoption step; `remove` by recorded files (section 1). |
| 3    | `update` with modified and unowned file protection (section 1).                    |
| 4    | Repeatable operations with pending records and interruption tests (section 2).     |
| 5    | Scope reporting, both-scope `list` and `doctor`, global output paths (section 3).  |
| 6    | Recorded commits and `update --check` (section 4).                                 |
| 7    | Smoke test and support levels for Claude Code and Copilot CLI (section 6).         |
| 8    | Notice mode; Claude Code plugin; Copilot CLI plugin (section 4).                   |

Section numbers refer to the re-steer design.

After 0.9.0:

- 0.9.1 added `upskill pack`, which builds a bundle as a Claude Code or Copilot
  CLI plugin
  ([ADR-0044](https://github.com/driftsys/upskill/blob/main/docs/adr/0044-plugin-output-of-a-bundle.md)).
  Plugin output stays experimental until the smoke test of stage 2 passes.
- 0.9.1 and 0.9.2 fixed the OpenCode plugin check and OpenCode global installs.
  OpenCode stays experimental: no smoke run has passed.
- 0.9.3 (2026-11-23) fixed `doctor`, the discovery of unreadable directories and
  the notice lock. It also records the excludes of `add` in the lockfile, which
  is now written as `schema: 3`
  ([ADR-0045](https://github.com/driftsys/upskill/blob/main/docs/adr/0045-record-add-excludes-in-lockfile.md)).

## Stage 1 — Stabilize the shipped features

Fix or close the review debt that the MVP and `pack` left open. This work runs
in parallel with stage 2. A debt item that touches the code of a stage 2 change
is fixed in that change or before it.

- Fix the K1 debt (#570, #613).
- Triage the K2 debt filed against #283: fix the items that change behavior,
  close or merge the others.
- Triage the working documents in `docs/wip/` (#536). Done on 2026-11-23: only
  the hook item kind design stays there, and the
  [archive](https://github.com/driftsys/upskill/blob/main/docs/archive/README.md#triage-of-2026-11-23)
  lists the other documents with three open points.
- Delete the unused Python gate harness (#540).

## Stage 2 — Finish plugin distribution

This stage is the plugin MVP and runs in parallel with stage 1. A plugin cannot
hold an always-loaded rule file, so a bundle needs hooks and commands to be
complete as a plugin. This stage starts with one design session and one decision
record that covers all items below. The record amends
[ADR-0044](https://github.com/driftsys/upskill/blob/main/docs/adr/0044-plugin-output-of-a-bundle.md)
where an item changes it: ADR-0044 limits the plugin content to skills,
resources and agents, and its `init` command lists the MCP servers of the bundle
without installing them.

- **Hook item kind (`HOOK.md`).** One file in an item directory holds all hook
  events of that directory, one `## <event>` section for each event. The draft
  is
  [the hook item kind design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-11-18-hook-item-kind-design.md).
  The convention is agreed; the design is not approved for implementation. The
  first target is `pack` output.
- **Command item kind (`COMMAND.md`).** A command that the user calls in the
  client, rendered for each client. The design decides how a command relates to
  a skill.
- **Init command.** The `init` command of a packed plugin copies into the git
  repository the content that a plugin cannot load, today the rules of the
  bundle, with `upskill add --kind rule --project`. `pack` already writes it
  (ADR-0044). The plugin MVP defines which content `init` copies and qualifies
  it in the smoke test. `init` never writes `AGENTS.md`: the user owns it, and
  upskill only creates a `CLAUDE.md` that holds `@AGENTS.md` when no `CLAUDE.md`
  exists.
- **MCP servers in the plugin.** `pack` writes the MCP servers that a bundle
  declares into the plugin, so the client installs and removes them with the
  plugin. Today `upskill add` configures them, but `pack` output does not hold
  them.
- **LSP servers in the plugin.** `pack` writes the LSP servers that a bundle
  declares into the plugin. Both clients load LSP servers from a plugin
  (`.lsp.json` in Claude Code, `lsp.json` in Copilot CLI).
- **Catalog generation.** Write the marketplace manifests
  (`.claude-plugin/marketplace.json`, `.github/plugin/marketplace.json`) from
  the bundles of a registry, so they match the packed plugins.
- **Hosted-marketplace smoke test** in both clients, with hooks, commands, MCP
  servers, LSP servers, the `init` command and a generated catalog. When it
  passes, plugin output is no longer experimental.

The plugin MVP holds the component types that both Claude Code and Copilot CLI
load from a plugin. Component types that only one client loads (Claude Code
workflows, output styles, themes, monitors, `bin/`) are later work for that
client.

Entry file names are singular (`RULE.md`, `HOOK.md`, `COMMAND.md`). The keys of
a bundle file are plural (`items.hooks`, `items.commands`).

The design session also decides:

- How a context hook delivers its text in each client, and in which order the
  hooks of several items of one plugin are merged.
- Which content the `init` command copies into the repository besides rules, and
  whether a session-start hook also delivers rule text.
- Where packed plugins are built: committed in `driftsys/metapowers` or built by
  its CI.
- Which Copilot CLI manifest `pack` writes. The Agent Plugins 1.0 format has a
  closed schema with fixed paths under `com.github.copilot/`. `pack` writes a
  `plugin.json` at the plugin root today.
- Whether the `com.github.copilot/rules/` directory of Agent Plugins 1.0 loads
  rules natively in Copilot CLI. If a trial confirms it, Copilot CLI rules need
  neither `init` nor a hook.
- The file format of Copilot CLI plugin commands, which the documentation does
  not describe.

The component evidence of 2026-11-23 comes partly from summarized documentation.
The session checks it again against the documentation pages and an installed
plugin before deciding.

## Stage 3 — Refactor where the Codex adapter needs it

Consolidate the implementation where adding a fourth client would otherwise
duplicate code.

- Remove dead code and obsolete adapter paths proven unused by supported
  installations.
- Replace duplicated generation, configuration and lifecycle logic with shared
  code where the client contracts actually match.
- Simplify module boundaries using the working adapters as evidence, rather than
  designing abstractions ahead of their use.
- Update tests and documentation to describe the resulting architecture.

The exit gate is unchanged behavior for the supported matrix, verified by its
regression and migration tests. Refactoring does not expand product scope.

## Stage 4 — Codex adapter

Add Codex as a client for rules, skills and agents, then hooks and commands,
with its own short design. Codex starts as experimental and becomes supported
when its smoke run passes.

## Stage 5 — Extra capabilities

Extend the client adapters with these managed capabilities:

| Capability     | Scope                                                                                                      |
| -------------- | ---------------------------------------------------------------------------------------------------------- |
| Tools          | Map tool identifiers and capabilities; expose unsupported mappings                                         |
| Permissions    | Translate native approval and execution controls without silently broadening access                        |
| Hooks          | Install hooks outside a plugin; translate payloads and results between clients                             |
| MCP management | Manage server configuration outside a plugin, transport/authentication references, discovery and lifecycle |

Record a support level for each capability on each client. Test combined
configurations as well as individual features. This stage adds first-class
management; it does not remove MCP functionality already shipped.

## Later expansion

- **Cursor and Gemini CLI:** include their requirements in the model now;
  schedule adapter implementation after the Claude Code, Copilot, Codex and
  OpenCode adapters.
- **Subagents:** retain in the customization inventory; management expansion is
  not yet assigned to a release.
- **VS Code:** qualify profiles for the actual Copilot, Claude, Codex or Local
  runtime. Reuse client integrations where supported; add editor-specific
  integration only where needed. Selecting a model does not select its vendor's
  agent runtime.
- **Remote and cloud surfaces:** establish configuration discovery, executable
  deployment and trust separately before claiming support.

## References

- [Client support commitment](https://github.com/driftsys/upskill/blob/main/docs/adr/0013-agent-customization-targets.md)
- [Six-client capability evidence and plugin examples](https://github.com/driftsys/upskill/blob/main/docs/technotes/customization-adapter-evidence.md)
- [Original customization research](https://github.com/driftsys/upskill/blob/main/docs/technotes/agent-customization/README.md)
