# Portable content format for AI-assistance items and bundles

**Status**: Proposed (2026-05-01)

## Context

AI-assistance content — rules, skills, agents — targets multiple clients
(Claude Code, Copilot, opencode), each with its own on-disk conventions.
Maintaining parallel content per client does not scale.

The Agent Skills open standard ([agentskills.io](https://agentskills.io))
provides a stable base for skills, but does not cover always-on rules or
agent personas. We need one format that:

- handles all three item kinds with a shared base schema,
- supports bundle distribution without duplicating item content,
- allows minor per-client deviations without forking entire files,
- is self-describing so it can evolve without silently breaking older
  consumers.

This ADR is one of four child ADRs of [ADR-0001](./0001-v0.2-architectural-reset.md).
It owns the on-disk SSOT contract; [ADR-0003](./0003-generation-pipeline.md)
owns how that SSOT becomes per-client output.

The authoritative specification lives at [`docs/format-spec.md`](../format-spec.md).
This ADR records the design decisions; the spec records the contract.

## Decision

### Three item kinds, one base schema

Rules (`RULE.md`), skills (`SKILL.md`), and agents (`AGENT.md`) share a
common frontmatter shape: `schema`, `name`, `description`, `license`,
`metadata`. Kind-specific fields are minimal:

- **Rules** add optional `scope.paths` for path-scoping.
- **Skills** add nothing — they follow the Agent Skills standard as-is.
- **Agents** add `mode`, `model`, `tools`, `preload-skills`.

### Vocabulary

The words **rules, skills, agents** stay throughout the schema, CLI, and
docs. Client-specific words (Copilot's "instructions", Claude's "subagent")
appear only in generated output and per-client documentation, never in
upskill's own surface.

### Directory-per-item layout, symmetric across kinds

```text
.agents/
├── rules/<name>/RULE.md
├── skills/<name>/SKILL.md
└── agents/<name>/AGENT.md
```

A directory per item — for all three kinds — enables ancillary resources
(templates, scripts, reference files) without future migration. Symmetry
reduces cognitive load and simplifies tooling.

### `metadata` block for governance

A nested `metadata:` field carries versioning (`version`, quoted semver),
ownership (`author`), audience targeting (`audience`), and arbitrary
unknown keys. Implementations preserve unknown keys through round-trips so
external governance systems can layer on top without forking the schema.

### Client-specific passthrough blocks

Top-level `claude:`, `copilot:`, `opencode:` blocks carry fields that have
no neutral equivalent (Copilot's `excludeAgent`, opencode's `temperature`).
The generator merges each block only into the matching client's output;
non-matching blocks are stripped.

### Conditional content directives

`<!-- @client:X -->` / `<!-- @endclient -->` HTML comment blocks enable
minor per-client body variations without separate files. Comma-separated
client lists. Negation via `!` (`!opencode`). No nesting. Known client
identifiers: `claude`, `copilot`, `opencode`.

### Per-client override files

When directives become unwieldy, authors provide complete body overrides:
`RULE.claude.md`, `SKILL.copilot.md`, `AGENT.opencode.md`. Override files
contain body only; frontmatter always comes from the canonical entrypoint.

### Bundle as pure manifest

A bundle is a single `<name>.bundle.md` file listing items by their `name`
slug. Items live in separate item-source repos; bundles reference them,
never contain them. Bundles compose cleanly (installing multiple bundles
produces their union) and avoid content duplication.

### Schema versioning

Every entrypoint carries `schema: 1`. Implementations reject unknown
future versions with a clear upgrade message. Minor additions (new
optional fields) do not bump the schema; breaking changes (removed
fields, renamed fields, changed semantics, new required fields) do.

### Body content conventions

For portability across clients:

- **No H1 in body.** The generator produces an H1 from `name` for clients
  that require one.
- **Describe capability, not specific tools.** "Search the codebase" — not
  "use the `Read` tool."
- **MCP tools in `ServerName:tool_name` form.** Portable across clients;
  client-specific internal forms (`mcp__server__tool`) stay out of SSOT.
- **No client-specific constructs.** No `$ARGUMENTS`, no `` !`cmd` ``,
  no `@path` imports, no `#tool:` references.
- **File references via standard markdown links** with relative paths.
- **Fenced code blocks require language hints.** Use `text` when no
  specific language applies.

## Consequences

**Positive.** Authors maintain content once; tooling does the per-client
mapping. Open-standard interop: agentskills.io-compatible tools consume
our skill output unchanged. Schema versioning means we can evolve without
silently breaking older toolchains.

**Negative.** Schema evolution requires version-aware handling. Adding a
required field is a breaking change forcing a schema bump. The spec is a
living document; keeping it and the implementation in sync requires
discipline.

## Alternatives considered

**(a) Custom proprietary schema unrelated to Agent Skills.** Rejected:
fragments the ecosystem; the open standard already covers skills well and
the `metadata` block provides sufficient extension surface for org-specific
concerns.

**(b) File-per-client with no SSOT.** Rejected: drift between client files
is inevitable at scale.

**(c) Inline content in bundles.** Rejected: forces bundles to grow with
every item change. Pure-manifest bundles compose cleanly and decouple item
versioning from bundle versioning.

## References

- Authoritative spec: [`docs/format-spec.md`](../format-spec.md)
- Parent ADR: [ADR-0001](./0001-v0.2-architectural-reset.md)
- Sibling ADRs: [ADR-0003](./0003-generation-pipeline.md),
  [ADR-0004](./0004-cli-surface.md),
  [ADR-0005](./0005-vercel-skills-sh-interop.md)
- Agent Skills open standard: <https://agentskills.io>
