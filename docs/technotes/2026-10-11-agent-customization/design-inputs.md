# Agent customization design inputs

Research date: 2026-10-11. Status: collected requirements and questions;
implementation design is not yet approved.

The target support commitment is accepted in
[ADR-0013](../../adr/0013-agent-customization-targets.md); the detailed
architecture questions below remain open.

## Agreed scope

- Use **agent customization** as the umbrella term.
- Include Claude Code, GitHub Copilot, OpenCode, Codex, Cursor, and Gemini CLI
  in the design.
- Codex is an intended implementation target.
- Cursor and Gemini CLI are required design targets; their implementation is
  deferred. Research coverage does not imply upskill support.
- Preserve bundles for installing coherent collections such as instructions plus
  skills, including dependencies and native integrations.

## Category coverage

The research covers eight categories discussed with the user. This inventory
does not imply eight new SSOT kinds or immediate implementation of each one.

| Category                 | Question the design must answer                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| Instructions / rules     | How are persistent and conditional instructions discovered, scoped, combined, and owned?                          |
| Skills                   | Which metadata and resources are portable, and which sidecars or runtime controls are target-specific?            |
| Subagents                | How are prompts, models, tool exposure, skills, and execution settings represented without inventing equivalence? |
| Plugins / extensions     | Which native packages can be installed and reconciled, and what behavior changes inside a package?                |
| MCP                      | How are server configuration, transport, credential references, and actual connection state distinguished?        |
| Hooks                    | Which events, payloads, return values, and execution contexts are supported by each target?                       |
| Commands / saved prompts | Is a reusable prompt a native command, a skill invocation, or an unsupported artifact for this surface?           |
| Execution policies       | Which native controls enforce tool, command, filesystem, and network restrictions?                                |

Workflows are an adjacent research concern with implementation deferred pending
concrete use cases. Supporting scripts, references, and assets belong to the
owning item or package; they do not automatically require separate item kinds.

## Questions to resolve in the design

1. Define a target as more than a vendor name: account for execution surface,
   scope, supported version, and configuration discovery.
2. Define collision and ownership behavior for directories discovered by
   multiple clients. Client-filtered output must not silently serve another
   client with incompatible instructions.
3. Distinguish omitted, explicitly empty, and restricted tool configurations.
   Unsupported restrictions must not silently broaden effective access.
4. Separate prose instructions, tool availability, approvals, and sandbox
   enforcement. Similar words in vendor schemas are not proof of equivalence.
5. Represent output as potentially several artifacts: Markdown, JSON, JSONC,
   TOML, sidecars, supporting resources, and native install operations.
6. Preserve user-authored configuration and track ownership of generated files,
   merged configuration entries, and native package installations.
7. Define scope-aware paths and migration from old paths, including stale
   generated output and shared-directory collisions.
8. Preserve the common content core while validating target extensions. Model
   aliases and skill preloading require explicit target semantics.
9. Define bundle composition across content and integrations, with duplicate
   detection and provenance when a native plugin provides overlapping items.
10. Specify lifecycle evidence: installed configuration, native discovery,
    enablement, and successful execution are different states.
11. Record which schema or documented contract each adapter supports.
    Timestamped research is evidence, not an automatic dependency on every
    upstream change.
12. Choose native discovery and semantic checks for conformance. Golden outputs
    alone cannot establish that a client loads or enforces generated content.

## Scope boundary

This research does not change the current SSOT schema, adapters, lockfile,
client configuration, or installed tools. It collects design inputs; decisions
and implementation tasks follow from a separate design review.
