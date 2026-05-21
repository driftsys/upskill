# Portable Format for AI-Assistance Content

**Version**: 0.2.0-draft
**Status**: Draft
**Date**: 2026-05-13

---

## Abstract

This specification defines a portable, tool-agnostic format for authoring rules, skills, and agents
targeting AI coding assistants. It also defines a bundle manifest format for distributing curated sets
of these items. The format is inspired by and compatible with the Agent Skills open standard
(agentskills.io) and extends it to cover always-on rules and agent definitions, which the open standard
does not address.

The specification intentionally leaves implementation details (installation mechanics, caching, registry
protocols, CLI surface) to the implementing tool. The scope is the on-disk content contract: file layout,
frontmatter schemas, body conventions, conditional directives, and per-client override mechanics.

## Audience

Authors of AI-assistance content (rules, skills, agents) targeting multiple AI coding clients from a
single source. Implementers of tools that consume this format to generate client-specific outputs.
Maintainers of CI pipelines that validate content against this specification.

## Conformance keywords

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** in this document
are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) when,
and only when, they appear in all uppercase as shown.

## Client-name conventions

This document uses the full client names — **Claude Code**, **GitHub Copilot**, and **opencode**
— in prose. The short forms `claude`, `copilot`, and `opencode` are reserved as the lowercase
identifiers used in passthrough block keys (§3.5), `audience` lists (§3.1), and conditional
content directives (§6).

---

## 1. Terminology

**Item**: a rule, skill, or agent. The three kinds share a base schema and differ in semantic role and
a small number of kind-specific fields.

**Rule**: always-on behavioral guidance, loaded into every session of compatible clients. Rules constrain
behavior ("never do X", "always check Y").

**Skill**: on-demand procedural content, activated by name or by a client's automatic discovery mechanism.
Skills teach a procedure ("to accomplish X, follow these steps").

**Agent**: a persona definition with its own system prompt and tool scope, invocable as a subagent or
primary agent depending on the target client. Agents establish identity and role ("you are a security
reviewer; when invoked, do X").

**Bundle**: a manifest file grouping items (and optionally other bundles) by reference for distribution.
Bundles contain no content themselves.

**Client**: a target AI coding assistant. This specification recognizes three initial clients: Claude Code,
GitHub Copilot (VS Code + CLI + cloud agent), and opencode. Implementations MAY extend this list.

**SSOT**: single source of truth. The canonical form of an item, authored per this specification, from
which client-specific outputs are derived.

**Client-specific output**: a file or configuration entry written for a particular client's expected
location, layout, and frontmatter conventions. Generated from the SSOT by an implementing tool.

**Source registry**: a git repository (or local path used as one) that holds SSOT items in their
canonical form. Source registries are the only place SSOT items exist. Authors edit items in source
registries; consumers fetch from them.

**Consumer project**: a developer's working repository where they run `upskill add` (or equivalent)
to install items. Consumer projects only ever contain generated, client-specific outputs — never
SSOT items. The same applies to a consumer's global install location (`~/.agents/` and per-client
directories under `~/.config/`, `~/.claude/`, etc.): generated content only.

---

## 2. File layout

### 2.1 Item directory structure

Each item is a directory containing at minimum one entrypoint file. Kind is determined by the
entrypoint filename:

```text
<item-root>/
├── <name-a>/
│   └── RULE.md
├── <name-b>/
│   └── SKILL.md
├── <name-c>/
│   └── AGENT.md
└── <name-d>/                # optional co-location: multiple kinds in one item directory
    ├── SKILL.md             # name: <name-d>
    └── AGENT.md             # name: <name-d>
```

Constraints:

- An item directory MUST contain at least one entrypoint file (`RULE.md`, `SKILL.md`, or
  `AGENT.md`).
- An item directory MAY contain more than one entrypoint when the entrypoints share a name and
  represent a tightly-coupled set of kinds for one capability (for example, a skill paired
  with its enforcing rule, or an agent paired with a skill it preloads). When multiple
  entrypoints are present, every entrypoint's `name:` field MUST equal the directory name.
  Co-location is optional; solo-kind item directories are the common case.
- `<name>` MUST match the `name` field in every entrypoint within the directory.
- `<name>` MUST consist of lowercase letters (`a-z`), digits (`0-9`), and hyphens (`-`).
- `<name>` MUST NOT start or end with a hyphen.
- `<name>` MUST be at most 64 characters.
- Kind is determined by the entrypoint filename: `RULE.md` for a rule, `SKILL.md` for a skill,
  `AGENT.md` for an agent. Implementations MUST NOT infer kind from any other source (parent
  directory name, frontmatter field, etc.).
- `<item-root>` is the SSOT path within a **source registry** repository. SSOT items live only in
  source registries; consumer projects (where developers run `upskill add`) only ever hold
  generated, client-specific outputs and never SSOT items.
- Within a source registry, `<item-root>` MAY be any path that fits the team's organisation
  (`content/`, `skills-src/`, `skills/`, etc.). The upskill project recommends `skills/`; see
  [Conventions](./conventions.md) for the full recommended layout.
  Source registries SHOULD avoid `.agents/` as their `<item-root>` to prevent confusion with
  the consumer-side opencode canonical-store path (§7.3).

**Agent Skills standard compatibility.** A skill item directory `<item-root>/<name>/SKILL.md`
is a valid Agent Skills directory per agentskills.io: the standard requires `<dir>/SKILL.md`
with `name` matching the parent directory and permits "any additional files or directories"
in the skill directory. A sibling `RULE.md` or `AGENT.md` is therefore standard-compliant
additional content and is ignored by agentskills.io-only tooling.

### 2.2 Bundle files

Bundles are flat manifest files, not directories:

```text
<bundle-root>/
├── platform-baseline.bundle.md
├── android.bundle.md
└── rust-embedded.bundle.md
```

- The filename stem (before `.bundle.md`) MUST match the `name` field in the bundle's frontmatter.
- Bundle files MAY live anywhere within a source registry — alongside item directories under
  `<item-root>` (§2.1), in a sibling directory, or in a dedicated `bundles/` directory.
  Implementations discover bundles by scanning for the `.bundle.md` suffix and MUST NOT depend
  on a specific bundle-root path. The upskill project recommends placing bundles alongside item
  directories under `skills/`; see [Conventions](./conventions.md).

### 2.3 Per-client override files

Any entrypoint MAY have optional per-client body overrides alongside it in the same item
directory:

```text
<name>/
├── RULE.md                    # canonical, always required
├── RULE.claude.md             # optional — Claude Code body override
├── RULE.copilot.md            # optional — GitHub Copilot body override
└── RULE.opencode.md           # optional — opencode body override
```

The same pattern applies to skills (`SKILL.claude.md`, etc.) and agents (`AGENT.claude.md`,
etc.). When an item directory holds multiple kinds (§2.1), each kind has its own set of
override files keyed by entrypoint name (`RULE.claude.md`, `SKILL.claude.md`,
`AGENT.claude.md`); they coexist in the same directory without ambiguity.

Override file rules:

- Override files contain body content only; they MUST NOT include YAML frontmatter.
- Frontmatter is always taken from the canonical entrypoint.
- The `<client>` suffix MUST be one of the known client identifiers (see §6.1).

**Body-resolution order for a given client (canonical statement).** Implementations resolve the
body for client `C` as follows:

1. If `<KIND>.<C>.md` exists, use its body verbatim. Directives in the canonical body are NOT
   processed for client `C`.
2. Otherwise, use the canonical body with `<!-- @client: -->` directives (§6) processed for
   client `C`.

§6.4 cross-references this rule; this section is the authoritative statement.

### 2.4 Supporting resources

Items MAY contain supporting files in their directory:

```text
<name>/
├── SKILL.md
├── templates/
│   └── handler.ts.tmpl
├── scripts/
│   └── validate.sh
└── references/
    └── patterns.md
```

- Supporting files are referenced from the entrypoint body using standard markdown relative links.
- Implementations MUST preserve supporting files and their directory structure during generation
  (copy them alongside the generated entrypoint into the client-expected location).
- When an item directory holds multiple kinds (§2.1), supporting resources are shared: any
  entrypoint in the directory MAY reference them via relative links, and implementations MUST
  copy them alongside each generated entrypoint that references them.

---

## 3. Frontmatter schema

All entrypoint files (RULE.md, SKILL.md, AGENT.md, *.bundle.md) begin with YAML frontmatter delimited
by `---` markers. Frontmatter is followed by a blank line and then markdown body content.

### 3.1 Common fields (all kinds and bundles)

| Field         | Type     | Required | Description                                                                                                                                                          |
| ------------- | -------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `schema`      | integer  | YES      | Protocol version of this specification. Current: `1`.                                                                                                                |
| `name`        | string   | YES      | Stable identifier. See §2.1 for format constraints.                                                                                                                  |
| `description` | string   | YES      | What this item does and when to use it. Max 1024 characters. Skill-specific guidance: see §3.3.                                                                      |
| `license`     | string   | no       | SPDX identifier or custom (`proprietary`, `LicenseRef-*`).                                                                                                           |
| `audience`    | string[] | no       | Target clients (`claude`, `copilot`, `opencode`). When present, implementations MUST generate output only for listed clients. When absent, all clients are targeted. |
| `metadata`    | map      | no       | Free-form governance block. See §3.6.                                                                                                                                |

Example:

```yaml
---
schema: 1
name: license-awareness
description: Use when reviewing third-party code, AI suggestions, or external snippets to check licensing compatibility and avoid incorporating incompatible licensed code
license: proprietary
metadata:
  version: "1.0.0"
  author: platform-security
---
```

Field semantics:

- `schema`: implementations MUST reject files with `schema` greater than the highest version they
  support and SHOULD report a clear upgrade message. See §8 for evolution rules.
- `name`: used as the `/slash-command` name in clients that support it, as the directory name, and
  as the identifier in bundle manifests.
- `description`: for skills, this is the activation hint — the model reads it during discovery and
  decides whether to load the full body. For rules and agents, it serves as documentation. Front-load
  the key use case. Convention: start with "Use when…" for skills and agents.
- `license`: if the item might be shared outside the authoring organization, SHOULD be populated.
- `audience`: the canonical mechanism for restricting an item to a subset of clients. Conditional
  body directives (§6) limit body content per client; passthrough blocks (§3.5) emit
  client-specific frontmatter; `audience` is the only mechanism that filters whether the item is
  generated at all. The three are complementary: use `audience` to scope the item, directives
  to scope body content within an emitted item, and passthrough blocks to add client-specific
  frontmatter to an emitted item.
- `metadata`: see §3.6. Implementations MUST preserve unknown keys within `metadata`.

### 3.2 Rule-specific fields

Rules MAY include path-scoping:

```yaml
scope:
  paths:
    - "src/api/**/*.ts"
    - "src/handlers/**/*.ts"
```

| Field         | Type     | Required | Description                                           |
| ------------- | -------- | -------- | ----------------------------------------------------- |
| `scope`       | map      | no       | Scoping constraints.                                  |
| `scope.paths` | string[] | no       | Glob patterns. If absent or empty, rule is always-on. |

When `scope.paths` is present, implementations:

- SHOULD emit the rule with path-scoping for clients that support it (Claude Code's `paths:` array,
  GitHub Copilot's `applyTo:` comma-joined string).
- SHOULD emit the rule unconditionally for clients that lack path-scoping (opencode), accepting the
  token-cost trade-off.

### 3.3 Skill-specific fields

No additional required fields beyond §3.1.

**Skill description length.** For skills, `description` is the activation hint: discovering
clients read it on every selection decision and the text contributes to runtime context cost.
The §3.1 1024-character cap MUST be respected, but skill authors SHOULD aim for ~200 characters
or fewer. This is tighter than rules and agents, where `description` serves only as
documentation.

The Agent Skills open standard defines optional fields (`allowed-tools`, `when_to_use`,
`disable-model-invocation`, `context`, `agent`, `argument-hint`, `paths`, etc.) which implementations
MAY pass through to client outputs when generating for clients that support them. This specification
does not redefine those fields; see agentskills.io/specification for their semantics.

Implementations SHOULD allow unknown top-level fields in skill frontmatter to enable pass-through of
client-specific or standard-defined fields.

### 3.4 Agent-specific fields

```yaml
mode: subagent
model: sonnet
tools:
  - read
  - grep
  - glob
  - bash
preload-skills:
  - security-baseline
```

| Field            | Type     | Required | Default             | Description                                                                             |
| ---------------- | -------- | -------- | ------------------- | --------------------------------------------------------------------------------------- |
| `mode`           | string   | no       | `subagent`          | `primary`, `subagent`, or `all`. Primary consumer is opencode; see notes below.         |
| `model`          | string   | no       | `sonnet`            | Short alias. Spec-guaranteed aliases listed below.                                      |
| `tools`          | string[] | no       | client-supported §4 | Capability-level tool names. See §4. Default semantics defined below.                   |
| `preload-skills` | string[] | no       | (none)              | Skill names to load at agent startup. Primary consumer is Claude Code; see notes below. |

**`mode` is opencode-primary.** opencode's agent definition has a real top-level `mode:` field
with `subagent` / `primary` / `all` semantics that change agent selection behavior. Claude Code
derives mode from file location (`.claude/agents/` is always subagent); GitHub Copilot has no
equivalent concept. Implementations MUST emit `mode:` for opencode; they MAY omit it for other
clients. Authors who need opencode-specific mode behavior with no analog elsewhere SHOULD prefer
`opencode.mode` in the passthrough block (§3.5) over a top-level `mode:` field, to make the
intent explicit.

**`preload-skills` is Claude-Code-primary.** Claude Code emits this list as `skills:` in agent
frontmatter so the named skills are loaded at agent startup. GitHub Copilot and opencode have no
equivalent and ignore the field. Implementations MUST emit `skills:` for Claude Code; they MUST
NOT emit a corresponding field for clients without an equivalent.

**`model` aliases.** This specification guarantees the following short aliases:

| Alias    | Family                           |
| -------- | -------------------------------- |
| `sonnet` | Anthropic Claude Sonnet (latest) |
| `opus`   | Anthropic Claude Opus (latest)   |
| `haiku`  | Anthropic Claude Haiku (latest)  |

Implementations MUST resolve these aliases to a current client-appropriate model identifier.
Implementations MAY accept additional aliases or pass through fully-qualified model identifiers
(`anthropic/claude-sonnet-4-6`, `openai/gpt-5`, etc.); these are not guaranteed portable across
implementations.

**`tools` default.** When `tools:` is absent, implementations MUST emit, for each target client,
the set of §4 capabilities documented as supported by that client (the non-`—` cells in the §4
table). When `tools:` is present, only the listed capabilities are emitted, subject to the §4
generation rules (capability dropped with a warning when the target client has no mapping).

Implementations map these fields to each client's agent definition format:

- Claude Code: `.claude/agents/<name>.md` with `tools:` as capitalized names, `model:` as literal,
  `skills:` from `preload-skills`.
- GitHub Copilot: `.github/agents/<name>.agent.md` with `tools:` filtered per §4 mapping; `mode`
  and `preload-skills` omitted.
- opencode: `.opencode/agents/<name>.md` with `mode:`, `permission:` map (per §7.3), and
  passthrough fields. `preload-skills` omitted.

### 3.5 Client-specific passthrough blocks

Items MAY include top-level blocks named after specific clients for fields that have no neutral
equivalent:

```yaml
copilot:
  excludeAgent: code-review
opencode:
  temperature: 0.2
  color: "#8b5cf6"
```

Rules:

- Implementations MUST merge these blocks into the client-specific output only for the matching client.
- Implementations MUST NOT emit passthrough blocks for non-matching clients.
- Implementations MUST NOT validate the contents of passthrough blocks (they are opaque to the spec).
- Known passthrough block names: `claude`, `copilot`, `opencode`. Additional names follow the same
  pattern for new clients.

### 3.6 Metadata block

The `metadata` field is a free-form map for governance, versioning, and implementation-specific
extensions.

Recommended conventions (not required by this specification):

| Key       | Type   | Convention                                                                     |
| --------- | ------ | ------------------------------------------------------------------------------ |
| `version` | string | Semver, quoted (e.g., `"1.0.0"`). MUST be quoted to avoid YAML float coercion. |
| `author`  | string | Team or individual identifier.                                                 |
| `updated` | string | ISO 8601 date of last content change.                                          |

Audience targeting is a top-level field with spec-defined semantics; see `audience` in §3.1.

Implementations:

- MUST preserve all keys within `metadata` during processing.
- MUST NOT require any specific key within `metadata` beyond what the implementation itself needs.
- SHOULD validate `version` as semver when present.

### 3.7 Bundle schema

Bundles are **source-registry artifacts**. They reference items by `name`; the items themselves
live in source registries (potentially the same repo, potentially others). Consumer projects do
not contain bundle files — an implementation records which bundles a consumer project has
installed in its own state file (e.g. `.upskill-lock.json` for the upskill CLI).

```yaml
---
schema: 1
name: platform-baseline
description: Baseline rules, skills, and agents for all repositories
license: proprietary

items:
  rules:
    - license-awareness
    - security-baseline
    - commit-style
  skills:
    - pr-summary
  agents:
    - security-reviewer

requires: []

metadata:
  version: "1.2.0"
  author: platform-dx
---

# Platform baseline

What this bundle provides, who it targets, changelog...
```

| Field          | Type     | Required | Description                          |
| -------------- | -------- | -------- | ------------------------------------ |
| `schema`       | integer  | YES      | Same as §3.1.                        |
| `name`         | string   | YES      | Same as §3.1. Matches filename stem. |
| `description`  | string   | YES      | Same as §3.1.                        |
| `license`      | string   | no       | Same as §3.1.                        |
| `items`        | map      | YES      | What this bundle provides.           |
| `items.rules`  | string[] | no       | Rule names.                          |
| `items.skills` | string[] | no       | Skill names.                         |
| `items.agents` | string[] | no       | Agent names.                         |
| `requires`     | array    | no       | Bundle dependencies. See below.      |
| `metadata`     | map      | no       | Same as §3.6.                        |

`requires` entries are maps. Each entry references another bundle by `name`, optionally pinned
with a semver `version` constraint:

```yaml
requires:
  - { name: "platform-baseline", version: "^1.0.0" }
  - { name: "rust-baseline" } # any version
```

The single-form (map-only) shape avoids the polymorphic-string-or-map alternative until that
flexibility has a documented author need.

Implementations:

- MUST resolve `items.*` entries against configured item sources by matching the `name` field.
- MUST report unresolved item names as errors.
- MUST resolve `requires` transitively: installing a bundle implies installing every bundle in
  its transitive `requires` closure, then unioning the items contributed by all reached
  bundles. Item-level conflicts (the same item name resolving to different sources or versions)
  MUST be reported as errors.
- MUST reject circular `requires` chains as errors.

---

## 4. Capability-level tool vocabulary

Agent frontmatter and item bodies reference tools by neutral capability names rather than
client-specific identifiers. This decouples content from any single client's naming conventions.

| Capability   | Meaning                   | Claude Code | GitHub Copilot | opencode |
| ------------ | ------------------------- | ----------- | -------------- | -------- |
| `read`       | Read file contents        | `Read`      | —              | `read`   |
| `write`      | Create or overwrite files | `Write`     | —              | `write`  |
| `edit`       | Modify existing files     | `Edit`      | —              | `edit`   |
| `bash`       | Execute shell commands    | `Bash`      | `shell`        | `bash`   |
| `grep`       | Search file contents      | `Grep`      | —              | `grep`   |
| `glob`       | Match file paths          | `Glob`      | —              | `glob`   |
| `web-fetch`  | Retrieve URL content      | `WebFetch`  | `fetch`        | —        |
| `web-search` | Search the web            | `WebSearch` | `web_search`   | —        |

Implementations maintain the mapping from capability names to client-specific identifiers. The mapping
table above is informational; clients may change their tool names independently of this specification.

For MCP tools, which are server-specific and not covered by this vocabulary, body content SHOULD use
the `ServerName:tool_name` form (colon-separated, preserving case). This form is readable across
clients; client-specific internal forms (e.g., `mcp__server__tool`) SHOULD NOT appear in SSOT content.

This vocabulary is intentionally small. It covers the tools common to all three initial clients.
Future specification versions may extend it. For tools not covered, item bodies SHOULD describe the
desired capability in natural language rather than naming a client-specific tool.

**Generation behavior for the agent `tools:` field.** When generating a client-specific agent
output, implementations process each entry in the SSOT `tools:` list per the table above:

- Capabilities **with** a documented client mapping are emitted using the client's name.
- Capabilities **without** a documented mapping for a client (`—` cells above) are dropped from
  that client's output, and implementations SHOULD log a warning at generation time identifying
  the dropped capability and the affected client. They are not passed through under the SSOT
  name — emitting an unrecognized identifier would produce a field the client does not consume.

Authors who need client-specific tool names beyond the documented mappings use that client's
passthrough block (`claude:`, `copilot:`, `opencode:` per §3.5). The passthrough mechanism is
the supported way to surface non-spec tooling in generated output.

**opencode caveat.** opencode does not consume a `tools:` array; it consumes a `permission:`
map, with capabilities mapped per a permission model where `write` rolls into `edit`. The §4
table shows the opencode capability identifier for reference, but the actual emission into
opencode agent frontmatter follows the permission-map shape documented in §7.3 and Appendix B.

---

## 5. Body content conventions

Item bodies are markdown content following the frontmatter. These conventions ensure generated output
is portable and passes standard markdown linting (markdownlint default ruleset).

### 5.1 Heading structure

- Body content MUST NOT contain an H1 (`#`) heading. When a client requires an H1, implementations
  MUST derive it from the item's `name` field (verbatim, no transformation). `description` is not
  used for H1 derivation; using it would make the formatting guarantee in §7.4 unenforceable.
- Body headings MUST start at H2 (`##`) and MUST NOT skip levels (no H2 → H4 without an intervening H3).

Rationale: generated output prepends an H1 derived from the item metadata. An H1 in the body would
produce duplicate H1s, violating MD025 (single-title/single-h1).

### 5.2 Fenced code blocks

All fenced code blocks MUST specify a language hint. Use `text` when no specific language applies.

Rationale: MD040 (fenced-code-language) requires language hints. Generating output that violates
this rule forces downstream consumers to either suppress the lint rule or fix the output.

### 5.3 Tool references in prose

Body content SHOULD describe tools by capability rather than by client-specific name:

- YES: "Search the codebase for occurrences of the deprecated function"
- YES: "Use `GitHub:create_issue` to file a tracking issue" (MCP tool in portable form)
- NO: "Use the `Read` tool to open the file" (Claude-Code-specific)
- NO: "Use `#tool:read`" (GitHub-Copilot-specific)

### 5.4 File references

Body content SHOULD reference supporting files using standard markdown links with relative paths
from the item directory:

```markdown
See [approved licenses](./allowed-licenses.txt) for the canonical list.
```

Client-specific import syntax (`@path` for Claude Code, `#file:path` for GitHub Copilot) MUST NOT appear
in canonical bodies.

### 5.5 Prohibited constructs in canonical bodies

The following constructs MUST NOT appear in **always-rendered** body content — that is, the
portion of the canonical entrypoint body that reaches every client. They are explicitly
**permitted** inside:

- `<!-- @client:X -->` ... `<!-- @endclient -->` directive blocks (§6), when `X` matches the
  construct's client. The directive processor strips the entire block from outputs targeting
  any other client, so the construct never reaches a client that would not understand it.
- Per-client override files (§2.3) — `RULE.claude.md`, `SKILL.copilot.md`, `AGENT.opencode.md`,
  etc. — when the filename suffix matches the construct's client. Override files are loaded
  only by the matching client's generator.

| Construct                       | Why prohibited (in always-rendered content) | Client         |
| ------------------------------- | ------------------------------------------- | -------------- |
| `$ARGUMENTS`, `$0`, `$1`        | Substitution variable                       | Claude Code    |
| `${workspaceFolder}`, `${file}` | Substitution variable                       | GitHub Copilot |
| `` !`command` ``                | Shell pre-execution                         | Claude Code    |
| `@path/to/file`                 | File import                                 | Claude Code    |
| `#tool:name`                    | Tool reference                              | GitHub Copilot |
| `#file:path`                    | File reference                              | GitHub Copilot |
| `ultrathink`                    | Extended thinking trigger                   | Claude Code    |

If an item needs one of these constructs in content that would otherwise reach every client,
wrap it in a matching-client directive block (§6) or move it to a matching-client override
file (§2.3). Items that genuinely target only one client end-to-end MAY instead be marked
single-client via `audience` (§3.1).

---

## 6. Conditional content directives

Canonical bodies MAY include client-conditional content via HTML comment directives. These allow
minor per-client variations without requiring separate override files.

### 6.1 Syntax

```markdown
<!-- @client:claude -->

Content rendered only for Claude Code output.

<!-- @endclient -->
```

- Opening delimiter: `<!-- @client:<client-list> -->` on its own line.
- Closing delimiter: `<!-- @endclient -->` on its own line.
- `<client-list>` is a comma-separated list of client identifiers, optionally prefixed with `!`
  for negation.

Known client identifiers: `claude`, `copilot`, `opencode`.

### 6.2 Forms

Positive (include for listed clients):

```markdown
<!-- @client:claude,copilot -->

This content appears in Claude Code and GitHub Copilot output, not in opencode.

<!-- @endclient -->
```

Negative (include for everyone except listed clients):

```markdown
<!-- @client:!opencode -->

This content appears everywhere except opencode output.

<!-- @endclient -->
```

### 6.3 Processing rules

Implementations:

- MUST strip non-matching blocks entirely, including the delimiter lines and any surrounding
  blank lines that would otherwise produce consecutive blank lines (violating MD012).
- MUST preserve the content of matching blocks, removing only the delimiter lines.
- MUST validate that all opened directives are closed (balanced blocks).
- MUST validate that client identifiers in directives are from the known set.
- MUST reject nested directives. If nesting is detected, report an error.
- SHOULD process directives before any other body transformation (formatting, rendering).

### 6.4 Interaction with override files

Override-file precedence over directives is defined by §2.3 (per-client override file rules);
this subsection exists only as a cross-reference. When generating output for a given client,
implementations resolve the body per the rules in §2.3.

---

## 7. Generation: client-specific output

This section describes the expected output per client: which files an implementation writes for
each item kind, and how SSOT frontmatter maps onto each client's frontmatter shape. It is
informational; client conventions may change independently of this specification. Implementations
SHOULD verify output against actual client behavior.

This section is deliberately scoped to "what file and frontmatter each client reads". Adjacent
behavior — bridging files, IDE configuration mutation, third-party config-file management — is
installer behavior owned by the implementing tool and intentionally out of this specification.

### 7.1 Claude Code

| Item kind | Output path                      | Frontmatter mapping                                                              |
| --------- | -------------------------------- | -------------------------------------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md`        | `paths:` array from `scope.paths` (if non-empty).                                |
| Skill     | `.claude/skills/<name>/SKILL.md` | `name:`, `description:` pass through. Agent Skills extended fields pass through. |
| Agent     | `.claude/agents/<name>.md`       | `name:`, `description:`, `model:`, `tools:` (capitalized).                       |

### 7.2 GitHub Copilot

| Item kind | Output path                                   | Frontmatter mapping                                                                                                                                                                                                                         |
| --------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rule      | `.github/instructions/<name>.instructions.md` | `applyTo:` (comma-joined from `scope.paths`, default `"**"`), `name:`, `description:`. `copilot.excludeAgent` if present.                                                                                                                   |
| Skill     | `.github/skills/<name>/SKILL.md`              | `name:`, `description:` pass through. Follows Agent Skills open standard.                                                                                                                                                                   |
| Agent     | `.github/agents/<name>.agent.md`              | `name:`, `description:`, `model:`, `tools:` (only documented §4 mappings — `bash → shell`, `web-fetch → fetch`, `web-search → web_search` — survive; unmapped capabilities dropped). GitHub Copilot-specific fields from passthrough block. |

### 7.3 opencode

| Item kind | Output path                      | Frontmatter mapping                                                                                                                                           |
| --------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rule      | `.agents/rules/<name>/RULE.md`   | `name:`, `description:`. opencode-specific fields from passthrough block. `scope.paths` dropped (no per-rule scoping per §3.2).                               |
| Skill     | `.agents/skills/<name>/SKILL.md` | `name:`, `description:`. Agent Skills extended fields and opencode-specific fields from passthrough block. opencode walks this directory.                     |
| Agent     | `.opencode/agents/<name>.md`     | `name:`, `description:`, `mode:`, `model:`, `permission:` map (allow per capability; write rolls into edit). opencode-specific fields from passthrough block. |

### 7.4 Formatting guarantee

Generated output MUST be formatted such that:

- It is idempotent under the implementation's chosen markdown formatter (e.g., dprint).
- It passes `markdownlint-cli2` with the default ruleset (or a documented subset pinned by the
  implementation).

Implementations SHOULD format generated markdown as the final step before writing, using an embedded
or invoked formatter, so that developers running the same formatter locally see no diffs on generated
files.

---

## 8. Schema versioning

The top-level `schema` field declares the specification version the file was authored against.

Current version: **1**.

### 8.1 Rules for implementations

- MUST reject files with `schema` greater than the highest version the implementation supports.
- MUST produce a clear error message identifying the required specification version and suggesting
  an upgrade of the implementing tool.
- MUST accept files with `schema` equal to or less than the highest supported version.
- When processing older schema versions, MUST NOT silently apply newer defaults that would change
  the file's meaning.

### 8.2 Rules for specification evolution

- MINOR additions (new optional fields, new optional top-level blocks) MAY occur within the same
  schema version. Implementations MUST ignore unknown fields they don't support.
- BREAKING changes (removed fields, renamed fields, changed semantics of existing fields, new
  required fields) MUST increment the schema version.
- Each published schema version MUST have a stable reference document with a permanent URL.

---

## 9. Conformance

An implementation is conformant with this specification if:

1. It correctly parses all frontmatter fields defined in §3, accepting unknown fields within
   `metadata` and client passthrough blocks without error.
2. It validates `name` against §2.1 constraints and rejects non-conforming names with a clear error.
3. It rejects files with unsupported `schema` versions per §8.
4. It correctly processes `@client:` directives per §6.3, including balanced-block validation.
5. It resolves bundle `items` references against available item sources and reports unresolved
   names as errors.
6. It preserves supporting files from item directories in generated output per §2.4.
7. Generated output satisfies the formatting guarantee per §7.4.

A reference test corpus SHOULD accompany mature versions of this specification. For v0.1-draft,
conformance is assessed against authored examples in the implementation's own test suite.

---

## 10. Non-goals

This specification does not define:

- Client file layouts or frontmatter conventions beyond what §7 documents as informational
  context. These are owned by each client's maintainer and may change independently.
- Installation, caching, synchronization, or registry protocols. These are implementation concerns.
- CLI command surfaces, configuration files, or user-facing workflows.
- Validation of semantic content quality. This is reserved for AI-review or other semantic
  analysis mechanisms outside the scope of format validation.
- Formatting conventions beyond what §5 requires for portability and §7.4 requires for
  generated output.
- Signing, provenance, supply-chain verification, or trust models. These are important but
  separate concerns for a future specification.
- Access control, classification enforcement, or governance workflows. Implementations MAY
  use `metadata` fields to integrate with external governance systems, but the mechanisms
  are not specified here.

---

## 11. Open questions for future versions

The following topics are explicitly deferred and tracked for future specification work:

1. **Scope patterns**: whether `scope.paths` should support negation (`!test/**`), boolean
   combinations, or named scopes.
2. **Tool vocabulary**: whether the §4 table should be expanded, whether MCP tool references
   need a more structured form, and how to handle tools that exist in only one client.
3. **Agent mode semantics**: whether `primary` / `subagent` / `all` is sufficient given
   divergent agent models across clients, or whether mode should be client-passthroughed entirely.
4. **Bundle constraints**: whether `requires` should support semver ranges, platform predicates
   (`requires-client: [claude >= 2.1]`), or conditional items.
5. **Per-client partial overrides**: whether override files should support section-level overrides
   (replacing only a specific heading's content) rather than full-body replacement.
6. **Skill activation control**: whether `disable-model-invocation` and `user-invocable` from
   the Agent Skills standard should be surfaced as neutral fields or remain client passthroughs.
7. **Multi-repo item sources**: whether the specification should define a source-resolution
   protocol or leave it entirely to implementations.
8. **Content hashing**: whether items should carry a content hash for integrity verification
   during distribution, independent of `metadata.version`.

---

## Appendix A: Complete frontmatter examples

### A.1 Rule — always-on, all clients

```yaml
---
schema: 1
name: license-awareness
description: Use when reviewing third-party code, AI suggestions, or external snippets to check licensing compatibility and avoid incorporating incompatible licensed code
license: proprietary
metadata:
  version: "1.0.0"
  author: platform-security
---

## Licensing rules

- Never paste code from external sources without verifying the license.
- When AI suggests code matching a recognizable upstream project, flag it before incorporating.
- See [approved licenses](./allowed-licenses.txt) for the canonical SPDX list.
- AGPL, SSPL, and BUSL code requires explicit approval from legal.
```

### A.2 Rule — path-scoped, with GitHub Copilot passthrough

```yaml
---
schema: 1
name: api-conventions
description: Use when writing or modifying API handler files to ensure consistent request validation, error formatting, and response shape
license: proprietary
scope:
  paths:
    - "src/api/**/*.ts"
    - "src/handlers/**/*.ts"
copilot:
  excludeAgent: code-review
metadata:
  version: "2.1.0"
  author: platform-api
---

## API handler conventions

- All endpoints include input validation using the project's schema library.
- Use the standard error response format defined in `src/api/errors.ts`.
- Never return raw database errors to the client.
```

### A.3 Skill — on-demand, with directive

```yaml
---
schema: 1
name: create-api-endpoint
description: Use when scaffolding new REST API endpoints with Zod validation, following project conventions for handler structure, error handling, and test coverage
license: proprietary
metadata:
  version: "1.0.0"
  author: platform-api
---

## Create a new API endpoint

Follow these steps to scaffold a complete endpoint:

1. Create the handler file at `src/api/handlers/{resource}.ts`.
2. Define the request schema using Zod in `src/api/schemas/{resource}.ts`.
3. Register the route in `src/api/routes.ts`.
4. Create the test file at `src/api/__tests__/{resource}.test.ts`.

<!-- @client:claude -->
When creating files, use the Write tool for new files rather than Edit.
<!-- @endclient -->

## Validation pattern

All inputs MUST be validated against the Zod schema before processing.
See [validation patterns](./references/validation-patterns.md) for examples.
```

### A.4 Agent — subagent with tools and preloaded skill

```yaml
---
schema: 1
name: security-reviewer
description: Use when reviewing code for injection flaws, authentication issues, secret leaks, and insecure data handling
license: proprietary
audience:
  - claude
  - copilot
  - opencode
mode: subagent
model: sonnet
tools:
  - read
  - grep
  - glob
  - bash
preload-skills:
  - security-baseline
opencode:
  temperature: 0.2
metadata:
  version: "1.0.0"
  author: platform-security
---

## Security reviewer

You are a security-focused code reviewer. When invoked:

1. Identify injection vectors (SQL, command, path traversal, XSS).
2. Verify authentication and authorization checks on every sensitive endpoint.
3. Scan for hardcoded secrets, tokens, API keys, or credentials.
4. Flag insecure data handling (unencrypted PII, weak cryptographic choices, improper logging of sensitive data).

For each finding, report severity (critical/high/medium/low), file and line location, and a concrete remediation.
```

### A.5 Bundle — with dependency

```yaml
---
schema: 1
name: android
description: Rules, skills, and agents for Android application development teams
license: proprietary
items:
  rules:
    - android-lifecycle
    - kotlin-coroutines
    - compose-conventions
  skills:
    - create-viewmodel
    - migrate-to-compose
  agents:
    - android-reviewer
requires:
  - platform-baseline
metadata:
  version: "0.8.0"
  author: android-platform
---

## Android bundle

Layered on top of `platform-baseline`. Provides Android-specific development
conventions, Jetpack Compose patterns, and a specialized code reviewer.
```

### A.6 Agent — with per-client override file

Directory structure:

```text
agents/security-reviewer/
├── AGENT.md                   # canonical
├── AGENT.claude.md            # Claude Code body override
└── AGENT.copilot.md           # GitHub Copilot body override
```

`AGENT.md` frontmatter is used for all clients. `AGENT.claude.md` body is used when generating
for Claude Code. `AGENT.copilot.md` body is used when generating for GitHub Copilot. For opencode,
the canonical `AGENT.md` body is used (with directives processed).

---

## Appendix B: Client frontmatter mapping reference

This table summarizes how SSOT fields map to each client's frontmatter when generating output.
This is informational and subject to change as clients evolve.

| SSOT field       | Claude Code output          | GitHub Copilot output                       | opencode output                                                 |
| ---------------- | --------------------------- | ------------------------------------------- | --------------------------------------------------------------- |
| `name`           | `name:`                     | `name:`                                     | `name:`                                                         |
| `description`    | `description:`              | `description:`                              | `description:`                                                  |
| `scope.paths`    | `paths:` (array)            | `applyTo:` (comma-joined string)            | (not supported; always-on)                                      |
| `mode`           | (implicit in file location) | (agent definition)                          | `mode:`                                                         |
| `model`          | `model:` (alias)            | `model:` (full ID)                          | `model:` (provider/ID)                                          |
| `tools` (agent)  | `tools:` capitalized names  | `tools:` mapped only (§4); unmapped dropped | `permission:` map (allow per capability; write rolls into edit) |
| `preload-skills` | `skills:` in agent          | —                                           | —                                                               |
| `copilot.*`      | (stripped)                  | Merged into frontmatter                     | (stripped)                                                      |
| `opencode.*`     | (stripped)                  | (stripped)                                  | Merged into frontmatter                                         |
| `claude.*`       | Merged into frontmatter     | (stripped)                                  | (stripped)                                                      |
| `metadata.*`     | (stripped from output)      | (stripped from output)                      | (stripped from output)                                          |

---

## Appendix C: Directive quick reference

```markdown
<!-- @client:claude -->           Include only in Claude Code output
<!-- @client:copilot -->          Include only in Copilot output
<!-- @client:opencode -->         Include only in opencode output
<!-- @client:claude,copilot -->   Include in Claude Code and GitHub Copilot, not opencode
<!-- @client:!opencode -->        Include everywhere except opencode
<!-- @endclient -->               Close any directive block (required)
```

---

## Appendix D: Recommended source-registry layout

This specification is intentionally tool-agnostic and lets registries pick
any `<item-root>` (§2.1) and any `<bundle-root>` (§2.2). The upskill
project records its specific recommendation — a flat `skills/` directory
holding both items and bundles, with a nested `skills/bundles/` variant
for large registries — in the user guide:

→ [Conventions](./conventions.md)

That recommendation is non-normative; a conforming registry MAY deviate
and conforming tooling MUST NOT reject deviations.

Rules: one directive per line, no nesting, balanced open/close, known client identifiers only.
