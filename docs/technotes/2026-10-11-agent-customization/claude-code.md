# Claude Code agent customization

Research date: **2026-10-11**. Evidence: opened official documentation,
published schema files, and a read-only comparison with upskill source. No
Claude Code runtime verification, installation, or configuration changes were
performed. Versions below are documented minimums, not an assertion about the
installed CLI. Documentation and published schemas can change independently.

## Coverage

| Category                   | Native support                                                 | Main representation                            | Schema status                                                    |
| -------------------------- | -------------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------------- |
| Instructions and rules     | Supported; AGENTS.md fallback is version-dependent             | Markdown; optional rule YAML frontmatter       | Documentation-only in sources reviewed                           |
| Skills                     | Supported; Claude Code extensions exceed the portable standard | Directory containing SKILL.md                  | Frontmatter documented; standalone vendor JSON Schema not found  |
| Subagents                  | Supported                                                      | Markdown with YAML frontmatter; CLI JSON       | Frontmatter documented; standalone vendor JSON Schema not found  |
| Plugins and extensions     | Supported; component semantics differ from standalone files    | Plugin directory and optional JSON manifest    | Published SchemaStore JSON Schema found                          |
| MCP                        | Supported; transport and authentication differences matter     | JSON server configuration                      | Standalone Claude configuration schema not found                 |
| Hooks                      | Supported; agent hook type is experimental                     | Settings JSON, plugin JSON, or frontmatter     | Settings schema covers configuration; event contracts documented |
| Commands and saved prompts | Supported; legacy command files retained                       | Markdown commands, skills, dynamic MCP prompts | No separate command schema found                                 |
| Execution policies         | Supported and enforced by the client                           | Settings JSON and session flags                | Published settings JSON Schema found                             |

## Instructions and rules

Project instructions use `CLAUDE.md` or `.claude/CLAUDE.md`; user instructions
use `~/.claude/CLAUDE.md`; private project instructions use `CLAUDE.local.md`.
Ancestor files load at startup and nested files on demand. Recursive
`.claude/rules/**/*.md` files accept `paths` as a YAML list or comma-separated
string. It is the **only interpreted rule field**; `name` and `description` are
ignored. Unscoped rules load at startup; matching file access activates scoped
rules. User rules precede project rules in context, without deterministic
conflict resolution between contradictory prose. `/context` shows loaded files.

Direct `AGENTS.md` fallback requires **2.1.277**. An ancestor/project
`CLAUDE.md`, `.claude/CLAUDE.md`, or `CLAUDE.local.md` suppresses that default
fallback. `AGENTS.override.md`, `AGENTS.local.md`, and `.agents/` are not
discovered. An existing `CLAUDE.md` with `@AGENTS.md` remains supported.
Instructions guide model behavior; they do not establish permission enforcement.
[Official instructions and rules reference](https://code.claude.com/docs/en/memory).

## Skills

Locations include `~/.claude/skills/<name>/SKILL.md`, project
`.claude/skills/<name>/SKILL.md`, nested project directories, and plugin
`skills/<name>/SKILL.md`. Enterprise wins over personal, then project; plugin
skills are namespaced. Nested skills can activate on file access. Discovery uses
`/`; `/skills` lists skills.

Fields include `name`, `description`, `when_to_use`, `argument-hint`,
`arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`,
`disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`,
`hooks`, `paths`, `shell`, `metadata`, `license`, and `compatibility`. Unknown
fields are ignored. `allowed-tools` grants permission for the invocation turn;
it is not an exclusive tool list. Forking, shell injection, model overrides, and
invocation controls require Claude runtime semantics.

Claude.ai/API skill packaging permits only `name`, `description`, `license`,
`compatibility`, `metadata`, and `allowed-tools`. Fork `background` control
requires **2.1.218**; worktree fallback to main-checkout skills requires
**2.1.277**. Local personal skills do not automatically reach cloud sessions.
[Official skills reference](https://code.claude.com/docs/en/skills).

## Subagents

Standalone files use `.claude/agents/**/*.md` or `~/.claude/agents/**/*.md`.
Precedence is managed, CLI `--agents`, project, user, then plugin. Nested
project definitions closest to the working directory win; identity comes from
`name`. Required fields are `name` and `description`. Optional fields include
`tools`, `disallowedTools`, `model`, `permissionMode`, `maxTurns`, `skills`,
`mcpServers`, `hooks`, `memory`, `background`, `omitClaudeMd`, `effort`,
`isolation`, `color`, `initialPrompt`, and `experimental.cacheTtl`.

`tools` is an allowlist; omission inherits available tools. Runtime filters may
still remove declared tools. `skills` preloads full content for up to 32
distinct names; unavailable or manual-only skills are skipped. `memory` scopes
are `user`, `project`, and `local`. `isolation: worktree` currently branches
from the default branch by default, not the parent's HEAD. `omitClaudeMd`
requires **2.1.271**. Model precedence changed in **2.1.251**.

`claude --agent <name>` selects an agent for the main session. Since
**2.1.233**, `claude plugin validate .claude/agents` can detect malformed
frontmatter, but not every missing required field. Serialized tools and models
therefore do not prove equivalent execution across clients.
[Official subagent reference](https://code.claude.com/docs/en/sub-agents).

## Plugins and extensions

The optional manifest is `.claude-plugin/plugin.json`; when present, `name` is
required. Other fields include `version`, `description`, `author`,
`dependencies`, `defaultEnabled`, `userConfig`, `settings`, and component
declarations. Standard locations include `skills/`, `commands/`, `agents/`,
`hooks/hooks.json`, `.mcp.json`, `.lsp.json`, `output-styles/`, and
`workflows/`. Components remain outside `.claude-plugin/`. Explicit `agents`
paths name files, not directories. `skills` adds to default discovery; `agents`
and `commands` replace their defaults. Use
`claude plugin validate <directory> --strict`; MCP checks expanded in
**2.1.281**. Component containment and native validation exceed basic schema
validation.
[Official manifest reference](https://code.claude.com/docs/en/plugins-reference).

Register a marketplace with `claude plugin marketplace add`, then install
`<plugin>@<marketplace>` using
`claude plugin install --scope user|project|local`. Enablement lives in the
matching settings file; local overrides project, then user. A committed project
entry does not download the plugin for collaborators. `--plugin-dir` loads a
local package for one session. `claude plugin list` and `/reload-plugins`
support inspection and lifecycle management. Cloud sessions do not automatically
load machine-local or project-enabled plugins.
[Official installation reference](https://code.claude.com/docs/en/discover-plugins).

Plugin agents ignore `permissionMode`, `hooks`, `mcpServers`, and
`initialPrompt`; declare hooks and MCP servers at plugin level instead.
Plugin-root `CLAUDE.md` does not load as project context. Namespacing and nested
agent paths also change identity. Thus moving standalone files into a plugin can
change behavior even when their bodies remain identical.
[Official component reference](https://code.claude.com/docs/en/plugins/components).

## MCP

Project servers use `.mcp.json` with a `mcpServers` map. User servers and
private project-local servers use `~/.claude.json`, with local entries under the
project's path. This differs from local settings storage. Key server fields
include `type`, `command`, `args`, `env`, `url`, and `headers`. Remote HTTP is
recommended; SSE is deprecated. WebSocket is also documented. An entry with
`url` needs an explicit transport type; `sdk` entries belong to host
applications.

Precedence is local, project, user, plugin, then account connectors; managed
servers take priority. Entries replace rather than merge. The Desktop Code tab
has a documented user/project stdio exception. Project trust and authentication
remain separate from writing configuration.

Commands include `claude mcp add`, `add-json`, `list`, `get`, and `remove`;
`/mcp` inspects connections/authentication. No standalone vendor JSON Schema for
Claude's configuration wrapper was found. MCP protocol schemas and server tool
input schemas are different artifacts and cannot validate this wrapper.
[Official MCP reference](https://code.claude.com/docs/en/mcp).

## Hooks

Settings hooks use `hooks.<Event>[]`, containing `matcher` groups and `hooks`
handler arrays. Sources include user/project/local/managed settings, plugin
`hooks/hooks.json`, and skill/subagent frontmatter. Matching handlers run in
parallel; duplicate settings handlers collapse, but plugin/skill copies remain
separate. Skill hooks persist after invocation for the session; subagent hooks
last while that agent runs.

Handler `type` values are `command`, `http`, `mcp_tool`, `prompt`, and `agent`.
Common fields include `if`, `timeout`, and `statusMessage`; type-specific fields
include `command`, `args`, `async`, `url`, `headers`, `server`, `tool`, `input`,
`prompt`, and `model`. Input/output are event-specific JSON contracts.
`PreToolUse`, `PostToolUse`, `Stop`, and `SessionStart` are examples, not the
complete event set. `/hooks` inspects configuration; debug logs explain
failures. Agent hooks are experimental. HTTP `onFailure: block` requires
**2.1.295**.

The settings schema covers configuration, while event payload contracts are
documented separately. Callback timing, output decisions, and process execution
must be translated explicitly; matching names alone are insufficient.
[Official hooks reference](https://code.claude.com/docs/en/hooks).

## Commands and saved prompts

Legacy `.claude/commands/*.md` and `~/.claude/commands/*.md` remain supported;
skills supersede them for new work. Commands accept skill frontmatter except
`name` and `paths`, with identity derived from the path. A same-name skill wins.
[Skills and legacy commands](https://code.claude.com/docs/en/skills).

Plugin `commands/` files add the plugin namespace; nested paths add segments.
The manifest can alternatively map command names to `source` or inline
`content`.
[Plugin command formats](https://code.claude.com/docs/en/plugins/components).
Connected MCP prompts appear as `/servername:promptname`; this is dynamic server
discovery rather than local Markdown.
[MCP prompt lifecycle](https://code.claude.com/docs/en/mcp). The `/` menu is the
native discovery interface; built-in commands have their own availability and
are not all saved prompts. There is no separate command schema in the reviewed
references.
[Official command reference](https://code.claude.com/docs/en/commands).

## Execution policies

Strict JSON settings live at `~/.claude/settings.json`, `.claude/settings.json`,
and `.claude/settings.local.json`. Managed settings outrank `--settings`, local,
project, and user settings. Lists generally merge; environment-variable behavior
is key-specific. `/status` identifies loaded sources; `claude doctor` reports
invalid settings. The official docs link the SchemaStore settings schema and
warn that it may lag the CLI. Project/local files cannot select `auto` or
`bypassPermissions` as `permissions.defaultMode`; the bypass restriction changed
in **2.1.257**.
[Official settings reference](https://code.claude.com/docs/en/settings).

`permissions` includes `allow`, `ask`, `deny`, `defaultMode`, and
`additionalDirectories`. Rules use tool names and specifiers such as
`Bash(npm test *)`. Evaluation is deny, ask, then allow; specificity cannot
create an exception to a deny. Modes include `default`, `acceptEdits`, `auto`,
`dontAsk`, `bypassPermissions`, and `plan`. `/permissions` manages rules. OS
sandboxing is a separate enforcement boundary. Prose, skill grants, agent tool
lists, hooks, and sandbox policies are not interchangeable portability units.
[Official permissions reference](https://code.claude.com/docs/en/permissions).

## Adjacent composition

Dynamic workflows are JavaScript `.js` files in `.claude/workflows/`,
`~/.claude/workflows/`, or plugin `workflows/`. A literal `export const meta`
declares `name` and `description`; the runtime provides orchestration functions.
Project workflows override personal ones. `/workflows` manages runs;
`/reload-skills` reloads edited definitions. Availability depends on the session
and feature configuration. This is executable orchestration, unlike a procedural
skill. [Official workflow reference](https://code.claude.com/docs/en/workflows).

An upskill bundle remains upskill's composition and distribution unit. A native
Claude plugin can be one dependency or target artifact; it is not a synonym for
a bundle and does not automatically carry every standalone customization.

## Source and schema inventory

All official documentation links above were opened. The inventory distinguishes
documentation from downloadable schemas; absence means not found in this bounded
research, not proof that no artifact exists.

| Artifact                                   | Opened source                                                                                                                            | Classification and limits                                                                                                          |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Settings JSON Schema                       | [SchemaStore settings](https://json.schemastore.org/claude-code-settings.json)                                                           | Draft-07 JSON Schema, linked by vendor docs; third-party hosting, not proof of vendor authorship; covers hooks and policies        |
| Plugin JSON Schema                         | [SchemaStore source](https://raw.githubusercontent.com/SchemaStore/schemastore/master/src/schemas/json/claude-code-plugin-manifest.json) | Draft-07 JSON Schema; generation comment dated 2026-04-23; not a source-language type definition; vendor ownership not established |
| Plugin schema identifier                   | [Declared canonical identifier](https://json.schemastore.org/claude-code-plugin-manifest.json)                                           | Browser-tool retrieval failed; a subsequent direct collection succeeded. The schema is saved in the shared snapshot inventory.     |
| Markdown customization schemas             | Instructions, skills, subagents, and commands references above                                                                           | Documentation-only; no standalone vendor schema obtained                                                                           |
| MCP configuration and hook payload schemas | MCP and hooks references above                                                                                                           | Documented contracts; do not substitute protocol/tool schemas                                                                      |

The current upskill comparison raises three concrete documentation questions:
`src/ancillary.rs` describes AGENTS.md support as absent although the
compatibility bridge remains useful; Claude rule generation emits metadata
Claude ignores; shared skill generation drops `license` and `metadata` while
forwarding other extensions. These are observations for review, not
implementation decisions.
