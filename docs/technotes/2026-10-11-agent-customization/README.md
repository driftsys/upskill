# Agent customization research

Evidence date: **2026-10-11**. Upskill baseline: `ae31bb5` (0.8.1).

This pack collects current documentation, schemas, and compatibility questions
for the next upskill design. It covers existing and proposed customization
categories; research coverage is not a declaration of implemented support.
No coding-client sessions, plugin installations, hook executions, or MCP
connections were used to verify behavior.

## Agreed direction

**Agent customization** is the umbrella term. Bundles remain the composition
mechanism for installing related items, including instructions plus skills.
The design must consider Claude Code, GitHub Copilot, OpenCode, Codex, Cursor,
and Gemini CLI. Codex is an intended implementation target; Cursor and Gemini
CLI implementation is deferred.

This support commitment is recorded in accepted
[ADR-0013](../../adr/0013-agent-customization-targets.md).

Start with [design inputs](design-inputs.md) for the agreed scope and unresolved
architecture questions, and [shared standards](standards.md) for terminology and
the boundary between portable content and native runtime behavior.

## Target reports

Each report covers instructions/rules, skills, subagents, plugins/extensions,
MCP, hooks, commands/saved prompts, and execution policies. Workflows and bundle
composition are adjacent concerns, not assumed universal runtime features.

| Target                        | Report focus                                                                      | Schema evidence                                                             |
| ----------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| [Claude Code](claude-code.md) | Standalone versus plugin behavior; discovery and permission differences           | SchemaStore settings and plugin schemas, identified as community-maintained |
| [GitHub Copilot](copilot.md)  | CLI, VS Code Local, VS Code Agent Host, and cloud differences                     | Standard plugin schemas; other contracts documented by surface              |
| [OpenCode](opencode.md)       | V1/V2 configuration and plugin lifecycle boundaries                               | Published runtime schema; mismatch with V2 documentation recorded           |
| [Codex](codex.md)             | Markdown instructions/skills, TOML agents/settings, hooks, policies, and plugins  | Public config schema and vendor-generated hook event schemas                |
| [Cursor](cursor.md)           | Editor/CLI/cloud discovery, native and portable plugins, hooks                    | Standard plugin schemas; native formats documented in prose                 |
| [Gemini CLI](gemini-cli.md)   | Context, extensions, TOML commands/policies, and source/documentation differences | Settings schema and links to native source validators/types                 |

## Schema collection

[The schema inventory](schemas/README.md) describes the saved artifacts.
[manifest.json](schemas/manifest.json) records their origin, retrieval time,
SHA-256 digest, publisher, and classification. Source snapshots use commit pins.
SchemaStore artifacts are distinguished from vendor-owned schemas, and Agent
Plugins 1.1 working-draft artifacts are distinguished from published 1.0.

No schemas were reconstructed from examples or prose. An absent standalone
schema is recorded as not found in the inspected sources, not as proof that the
feature is unsupported. The collected artifacts are evidence snapshots and are
not loaded by upskill at runtime.

## Findings that affect the design

1. **Shared discovery creates cross-client exposure.** Several clients read
   `.agents/skills` and compatibility directories. Existing OpenCode-specialized
   output can be consumed by another client. Define ownership and compatibility
   before sharing generated paths. See [Codex](codex.md) and [Cursor](cursor.md).
2. **Tool restrictions cannot disappear during translation.** Copilot distinguishes
   omitted, empty, and populated tool lists. Existing upskill drops some mapped
   capabilities and can emit an unrestricted agent. See [Copilot](copilot.md).
3. **Scope changes paths and semantics.** Replacing the output root with `$HOME`
   does not establish user-scope discovery. Local state does not automatically
   reach cloud sessions. Each adapter needs explicit scope and surface support.
4. **Packaging changes behavior.** Standalone agents and plugin agents can accept
   different fields. Portable Agent Plugins currently standardizes skills and
   MCP, while other content uses client extensions. See [Claude](claude-code.md)
   and [shared standards](standards.md).
5. **Schema validity is narrower than compatibility.** Discovery, precedence,
   permissions, trust, and execution must be verified against the selected
   release. Golden output and parsing checks cannot establish those properties.
6. **Documentation conflicts need explicit disposition.** OpenCode V2 documentation
   and the advertised schema differ. Gemini examples and native parsers differ.
   Preserve the evidence and select a tested contract during design rather than
   silently normalizing it. See [OpenCode](opencode.md) and [Gemini](gemini-cli.md).

## Corrections to the preliminary conversation review

The broader collection found details that the initial bounded review missed:

- Codex documents native hooks and plugin CLI lifecycle operations; its
  plugin-authoring guide's desktop workflow is not evidence of UI-only support.
- Copilot CLI documents project MCP configuration; the older assumption that
  only personal MCP configuration exists is stale.
- Agent Plugins 1.1 is identified by its upstream repository as a working draft,
  even though some client documentation recognizes its schema identifier.
- OpenCode's live V1 CLI documents `plugin <module>`; V2 documents a different
  plugin command family. Existence and behavior must be qualified by generation.

These corrections are reflected in the individual reports. They are research
findings, not changes to installed integrations.

## Documentation lifecycle review

[The RIDL comparison and refactor proposal](documentation-refactor.md) examines
working documents, the published book, durable specifications and design,
technical notes, decisions, and archives. It proposes a lifecycle for upskill's
existing documentation and this research pack. The resulting refactor places this pack in `docs/technotes/`, as requested,
and records the disposition in the comparison. Older plans remain in WIP pending
reconciliation.

## Verification and limits

The collection is checked for complete category coverage, Markdown formatting,
local link integrity, JSON parsing, manifest checksums, and schema reference
integrity where references are internal to a downloaded document. External
schema references remain upstream dependencies. These checks concern the
reference pack, not client-runtime conformance.

Before implementing an adapter, select supported client versions and test native
discovery, scope, precedence, and behavior. The report date and a source commit
do not by themselves identify a tested released client version.
