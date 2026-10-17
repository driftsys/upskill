# Customization adapter model and delivery roadmap

Date: **2026-10-14**. Status: **captured user direction and proposed delivery
gates**. Code baseline: `00d4c2c` (rebased on main). This document specifies
future work, not released functionality. Evidence is collected in the
[capability refresh](../../technotes/customization-adapter-evidence.md).

The concise [product roadmap](../../roadmap.md) owns the release scope and
order. This working document records adapter details and proposed qualification
gates.

## Scope and sequencing

The initial implementation clients are **Claude Code, Copilot CLI, Codex and
OpenCode**. Cursor and Gemini CLI remain design targets with later
implementation. This changes the three-client-first sequencing in the
[2026-10-11 roadmap](2026-10-11-agent-customization-roadmap.md), while
preserving the six-family commitment in
[ADR-0013](../../adr/0013-agent-customization-targets.md).

Release scope, ordering and implementation dependencies are maintained only in
[the product roadmap](../../roadmap.md). This proposal records adapter choices
and unresolved qualification questions; it is not an independent delivery plan.
Existing agent and MCP behavior remains part of the regression contract.

**`driftsys/metapowers` is the intended marketplace repository.** Client-native
catalogs and installation references belong to that distribution surface. This
document does not modify or publish anything in metapowers.

## Shared model and adapter ownership

Authors express customization intent once. A client adapter selects the native
representation, maps supported fields and tools, and reports unsupported intent.
Each feature decision prioritizes **reliability and performance**, followed by
**maintainability as client contracts change**. Plugin availability does not
automatically justify running hooks for every feature.

A target must identify the client family, agent runtime, editor/CLI surface,
supported version, project/user scope, OS and execution environment. A model
name is not a runtime identifier. These dimensions refine capability decisions;
they do not require implementing every combination in the first release.

Keep two choices separate:

1. **Delivery and updates:** generated files, native package installation, or a
   plugin that maintains generated files.
2. **Runtime consumption:** native discovery/configuration or active plugin
   logic such as a hook/API registration.

A skill shipped inside a plugin can still use native skill discovery. A Cursor
plugin rule or Gemini extension context file can still use native context
loading. An upskill bundle remains a composition/dependency record, not a
synonym for a native plugin manifest.

The eight inventory categories are rules, skills, subagents, MCPs, hooks,
permissions, plugins and bundles. Tools add a separate capability/mapping
concern for the following release. They must not be confused with permission
grants or with prose references to tool names in a skill.

## Initial route mapping

**N:** native discovery/configuration. **P:** active upskill plugin logic.
**U:** upskill composition. A question mark identifies a candidate requiring
runtime validation. Packaging/update transport is independent of these labels.

| Customization                           | Claude Code         | Copilot CLI         | Codex               | OpenCode v2      | Cursor, later               | Gemini CLI, later              |
| --------------------------------------- | ------------------- | ------------------- | ------------------- | ---------------- | --------------------------- | ------------------------------ |
| Rules, independent of team instructions | N                   | N                   | P?                  | P?               | N, including packaged rules | N, including extension context |
| Skills                                  | N                   | N                   | N                   | N                | N                           | N                              |
| Subagents, unsequenced                  | N                   | N                   | N                   | N                | N                           | N, release verification needed |
| MCP, following release                  | N config            | N config            | N config            | N config         | N config                    | N config                       |
| Hooks, following release                | P via native events | P via native events | P via native events | P via plugin API | P via native events         | P via native events            |
| Permissions, following release          | N controls          | N controls          | N controls          | N controls       | N, surface-specific         | N policy engine                |
| Plugin installation                     | N lifecycle         | N lifecycle         | N lifecycle         | N lifecycle      | N lifecycle                 | N extension lifecycle          |
| Bundles                                 | U                   | U                   | U                   | U                | U                           | U                              |

Codex/OpenCode plugin rule delivery is proposed because additive discovery of
independent rule files alongside existing team `AGENTS.md` was not established
for these contracts. Do not claim either client lacks project instructions. The
fallback when injection cannot meet the required semantics is an explicit
unsupported/limited result, not silent rewriting of team guidance.

Native permissions remain authoritative for the relevant runtime. Later plugin
checks may supplement them, but a prompt cannot implement sandbox enforcement.
Tool exposure, approval decisions and filesystem/network restrictions remain
separate adapter capabilities.

## One plugin integration per client

Use a thin upskill integration for each initial client, backed by the shared
synchronous Rust CLI. Distinct manifests or entrypoints are expected; distinct
repositories are not required. Prefer shared sources and client-specific
packaging over four copies of update logic. The plugin source repository and
exact package layout remain implementation choices; metapowers is the agreed
marketplace destination.

The first plugin feature is keeping the **upskill CLI and installed skills up to
date**. Rules and other managed MVP content use the same ownership-aware update
lifecycle where applicable. A plugin is a suitable maintenance entry point when
its initialization/startup event can invoke the updater reliably. It is a
trigger and adapter, not the owner of download, reconciliation or binary
replacement logic.

| Client      | Candidate maintenance entry point | Qualification question                                                             |
| ----------- | --------------------------------- | ---------------------------------------------------------------------------------- |
| Claude Code | Packaged startup command hook     | Does the installed package invoke the updater before/after discovery as intended?  |
| Copilot CLI | Packaged session-start hook       | Does the installed hook run under the selected trust/sandbox and scope?            |
| Codex       | Packaged `SessionStart` hook      | Does explicit hook trust enable the installed definition, including after updates? |
| OpenCode v2 | Plugin setup/lifecycle API        | Can maintenance run without delaying model readiness or repeating on reload?       |

This internal maintenance hook, and any internal rule-delivery hook, are
**implementation details of the MVP plugin**. General authoring, distribution
and management of user-defined hooks remain in the following release.

Plugin automatic updates and upskill CLI/content automatic updates are separate
lifecycles. A client marketplace may refresh plugin files but does not thereby
upgrade the upskill executable or reconcile the upskill lockfile. Avoid an
updater that recursively tries to update its currently executing plugin.

Recommended update behavior, to validate before implementation:

- Use one shared CLI maintenance path, with a local freshness check, a bounded
  online check, and a lock preventing concurrent sessions from applying twice.
- Keep normal model turns free of update work. Network failures retain the last
  usable CLI/content and produce an actionable maintenance status.
- Define initial install, binary update and content update separately. The
  current `Commands` enum has content `update`, not a dedicated self-update
  command; reusing that name does not implement binary updating.
- Respect pinned source refs and locally modified owned files. Define automatic
  application policy for user and committed project scope explicitly.
- State whether new content is visible in the current session, after reload, or
  only in a new session. Do not promise that a startup write changes an already
  captured skill catalog.
- Test supported binary distribution channels and OS replacement behavior.
  Package-manager-owned executables need a defined update route.

Keep the static Rust binary constraints. Client-required scripting is permitted
only in the thin client integration; it does not introduce Node/npm dependencies
into the Rust CLI or duplicate its parsers and business logic.

## VS Code profiles

VS Code is a surface, not a single portable runtime. Reuse the selected client's
plugin where the actual harness supports it; add a separate VS Code integration
only for a demonstrated editor-specific need. A native agent plugin and a VS
Code extension are different package types.

For the MVP, make the four CLI/local runtime profiles the release gates. Record
VS Code Copilot/Claude/Codex Agent Host and extension profiles separately and
qualify them with smoke tests before claiming support. VS Code Local, cloud
execution, remote/container deployment and other integrations are not implied by
the CLI gates. Their implementation scheduling remains open; this document does
not erase their requirements from the earlier roadmap.

Tool names, MCP configuration forwarding, workspace trust, hook activation,
extension lifetime and remote executable paths can change effective behavior.
Two sessions displayed in the same editor may therefore require different
adapter profiles. Selecting a different model inside one harness does not switch
its adapter.

## Qualification questions

Use the [product roadmap](../../roadmap.md) for dependency order. Adapter
qualification must establish installed lifecycle invocation, native content
discovery, reload behavior, project/user scope, offline and interrupted updates,
and startup overhead for each supported runtime profile. Initial harmless probes
do not qualify automatic maintenance or establish owned-skill support.

Rule qualification must cover team/user overrides, scope, duplicate activation,
and behavior through resumed sessions and context compaction for each supported
runtime profile.

Client-native plugin installation, updates, disablement and removal require
separate package/catalog evidence. Existing native restrictions remain
authoritative; later permission management does not postpone respecting them.

## Decisions still required before coding

1. Supported released versions, OS matrix, and the OpenCode v1 compatibility
   boundary versus the new v2 profile.
2. Update cadence/channel, pinning, project/user application policy and binary
   installation ownership. No automatic upgrade policy is invented here.
3. Whether default/required rule semantics need a source marker or only explicit
   instruction text. There is no numeric rule-priority requirement in this MVP.
4. Rule-injection capabilities and lifecycle evidence for Codex/OpenCode, plus
   any client-specific limitation or documented fallback.
5. Exact package sources and client-native catalogs in metapowers. One
   marketplace repository does not guarantee one manifest works in every client.

The design should first resolve these with the smallest installed-plugin probes
and targeted tests. Superpowers provides useful packaging and adapter examples,
but not evidence that upskill binary auto-update is already solved.
