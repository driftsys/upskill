# Agent customization targets — Codex, Cursor, and Gemini CLI

**Status**: Accepted (2026-10-11)

## Context

The generation architecture in
[ADR-0001](0001-multi-kind-compiler-architecture.md) and
[ADR-0003](0003-generation-pipeline.md) currently serves Claude Code, GitHub
Copilot, and OpenCode. Their customization contracts have evolved, and other
clients now provide overlapping capabilities with different discovery paths,
formats, scopes, and execution semantics.

The [agent customization research](../technotes/agent-customization/README.md)
collects current documentation and available schemas for six client families.
Designing only around the existing clients would leave Codex support and later
Cursor and Gemini CLI support dependent on assumptions that may not hold for
those clients.

## Decision

upskill adopts **agent customization** as the umbrella term for the content and
configuration it manages.

The following six client families are first-class design targets:

| Target         | Implementation commitment                                                                         |
| -------------- | ------------------------------------------------------------------------------------------------- |
| Claude Code    | Maintain and evolve existing support.                                                             |
| GitHub Copilot | Maintain and evolve existing support, accounting for differences between CLI and editor surfaces. |
| OpenCode       | Maintain and evolve existing support.                                                             |
| Codex          | Include support in the intended implementation scope of the next design.                          |
| Cursor         | Include requirements in the design now; defer implementation.                                     |
| Gemini CLI     | Include requirements in the design now; defer implementation.                                     |

First-class design consideration means assessing each target's native formats,
discovery, project and user scopes, configuration ownership, and supported
capabilities. It does not imply identical capabilities across clients or support
for every product surface within a client family.

The design must consider instructions and rules, skills, subagents, plugins and
extensions, MCP configuration, hooks, commands and saved prompts, and execution
policies. This is a requirements inventory; it does not establish a new SSOT
item kind or promise implementation for every category. Capability differences
and unsupported combinations must remain explicit in the resulting design.

Bundles remain useful for composing related customization items and
dependencies, including instructions with skills. A bundle and a client's native
plugin package are distinct concepts, even when a bundle installs or refers to a
plugin.

## Scope and consequences

- Codex, Cursor, and Gemini CLI requirements inform the next design before
  adapter and model changes are selected. Cursor and Gemini CLI do not need
  implementations to complete that design.
- Detailed choices about schemas, target profiles, translation, installation,
  migration, and capability validation require subsequent design decisions.
- The six families are product targets, not a decision to collapse the
  generation and MCP target spaces described in
  [ADR-0012](0012-consumer-client-filtering.md). VS Code remains a relevant
  Copilot surface and existing MCP target.
- This ADR records a support commitment. It does not introduce working adapters,
  CLI flags, or new default installation targets. Existing released behavior
  remains documented by the current specifications.
- Research snapshots are dated evidence. Implementation must verify the relevant
  upstream contracts again rather than treating those snapshots as permanent
  vendor guarantees.

## Alternatives considered

**Design for Codex now and examine Cursor and Gemini CLI later.** Rejected
because known requirements can expose portability constraints before the
architecture is changed, without requiring immediate implementation.

**Implement all six targets together.** Deferred because the user has explicitly
separated Cursor and Gemini CLI design requirements from their implementation. A
shared design can support staged delivery.

## References

- [Research index and sources](../technotes/agent-customization/README.md)
- [Design inputs and open questions](../technotes/agent-customization/design-inputs.md)
- [ADR-0008 — Plugin installation](0008-plugin-install-shellout.md)
- [ADR-0010 — MCP server configuration](0010-mcp-config-write.md)
