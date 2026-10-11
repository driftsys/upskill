# Shared standards and terminology

Research date: 2026-10-11. This is evidence for a future design, not a new
upskill format specification or a declaration of implemented support.

## Terminology

The user selected **agent customization** as the umbrella term.

- **Customization item**: a unit of content or configuration that upskill
  can author, generate, install, or reference. This is working terminology,
  not a decision to make every category a new top-level SSOT kind.
- **Bundle**: upskill's composition and dependency manifest, which can install
  related instructions, skills, agents, and native integrations together.
- **Plugin / extension**: a target-native distribution package. A bundle may
  refer to one; it does not imply the package can carry every customization
  category with equivalent behavior.
- **Target profile**: a proposed way to identify a client, execution surface,
  scope, and supported version. This is a design candidate, not an existing
  upskill feature.

## Agent Skills

The [Agent Skills specification](https://agentskills.io/specification) defines
`SKILL.md` with YAML frontmatter and Markdown, plus optional supporting files.
The standard fields are required `name` and `description`, and optional
`license`, `compatibility`, `metadata`, and experimental `allowed-tools`.
The name must match its containing directory; field lengths and naming rules
are specified. Clients can add extensions, but those extensions do not become
portable merely because the source parser preserves them.

The specification links a reference validator, invoked as
`skills-ref validate <skill-directory>`. No canonical standalone JSON Schema
is published on the specification page inspected for this research. Record
this as a documentation and reference-validator contract, not as proof that
no schema exists anywhere.

## Agent Plugins

The [published specification](https://agent-plugins.org/specification) inspected
for this collection identifies itself as version 1.0.0. The portable components
are skills and MCP server configuration, with fixed `skills/` and `mcp.json`
locations and a required root `plugin.json`. Other behavior belongs to
client-owned extension namespaces. Installation and permission behavior remain
client responsibilities.

The standard provides separate manifest and MCP JSON Schemas. Schema validity
does not establish all operational requirements; the specification takes
precedence when they conflict. In particular, filesystem containment,
component discovery, and failure handling need checks beyond parsing.

The [schema index](https://agent-plugins.org/schemas) identifies canonical schema
URLs. Local copies are preserved unchanged:

- [Plugin manifest schema](schemas/agent-plugins-1.0.0-plugin.schema.json).
- [Plugin MCP configuration schema](schemas/agent-plugins-1.0.0-mcp.schema.json).
- [Retrieval URLs, timestamps, and SHA-256 hashes](schemas/manifest.json).

Some client documentation references version 1.1.0. The
[upstream repository](https://github.com/agentplugins/agent-plugins-spec)
identifies 1.0.0 as published and 1.1.0 as a working draft. Draft schemas are
collected separately from a pinned source revision. Client acceptance of the
draft identifier does not make it the published common baseline.

Agent Plugins licenses documentation under CC BY 4.0 and schemas under
Apache-2.0. Canonical source links and the
[upstream licensing notice](schemas/licenses/agent-plugins-LICENSE.txt) are
retained for attribution. Schema snapshots are upstream data, not schemas
authored by upskill.

## Model Context Protocol

The [MCP schema reference](https://modelcontextprotocol.io/specification/2026-07-28/schema)
describes wire messages and protocol types. The `latest` URL resolved to this
dated version when inspected. This is distinct from the configuration file
used by a coding client to launch or connect to a server.

For design purposes, keep three contracts separate:

1. MCP protocol behavior and negotiated capabilities.
2. Portable plugin `mcp.json`, where that package format is supported.
3. Target-native server configuration, credentials, consent, and lifecycle.

A schema for one does not validate the other two. A configured server is also
not evidence that the server is reachable or authorized.

## Evidence classification

Each target report distinguishes these evidence levels:

- **Published schema**: a machine-readable artifact at a documented URL.
- **Source definition**: a vendor implementation type or parser, which may
  change with the source revision.
- **Documentation-only**: a documented format without a retrieved standalone
  machine-readable schema.
- **Unresolved**: conflicting documentation or an artifact that could not be
  retrieved. This does not mean unsupported.

Runtime support claims in this pack come from documentation or inspected
source. No client sessions, hooks, plugin installs, or MCP servers were run.
