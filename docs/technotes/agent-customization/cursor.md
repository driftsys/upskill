# Cursor agent customization

Research date: **2026-10-11**. Evidence comes from live official documentation
and linked standards, not runtime tests. Cursor is a requirement for the future
agent customization design; implementation remains deferred. The pages are
rolling references, so this note does not establish a minimum supported Cursor
release unless one is explicitly identified below.

## Source and schema inventory

| Surface            | Primary contract                                                                                                                                  | Machine-readable schema status                                                                                                                                                                   |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Instructions/rules | [Rules](https://cursor.com/docs/rules)                                                                                                            | Cursor prose/frontmatter contract; standalone schema not found                                                                                                                                   |
| Skills             | [Skills](https://cursor.com/docs/skills)                                                                                                          | Cursor prose plus Agent Skills conventions; Cursor-specific schema not found                                                                                                                     |
| Subagents          | [Subagents](https://cursor.com/docs/subagents)                                                                                                    | Cursor prose/frontmatter contract; standalone schema not found                                                                                                                                   |
| Plugins            | [Plugin reference](https://cursor.com/docs/reference/plugins), [installation](https://cursor.com/docs/plugins)                                    | Cursor-native manifest documented in tables; separate downloadable schema not found                                                                                                              |
| Portable plugins   | [Agent Plugins manifest](https://agent-plugins.org/plugin-authors/manifest)                                                                       | Published [plugin schema 1.0.0](https://agent-plugins.org/schemas/1.0.0/plugin.schema.json) and [MCP schema 1.0.0](https://agent-plugins.org/schemas/1.0.0/mcp.schema.json), both opened as JSON |
| MCP                | [MCP](https://cursor.com/docs/mcp), [CLI MCP](https://cursor.com/docs/cli/mcp)                                                                    | Native `mcp.json` prose contract; portable plugin MCP schema is a distinct contract                                                                                                              |
| Hooks              | [Hooks](https://cursor.com/docs/hooks)                                                                                                            | Configuration and event payload tables/examples; standalone schema not found                                                                                                                     |
| Commands           | [Skills migration help](https://cursor.com/help/customization/skills), [CLI slash commands](https://cursor.com/docs/cli/reference/slash-commands) | Markdown/prose contracts; no separate machine schema found                                                                                                                                       |
| Execution policy   | [Permissions](https://cursor.com/docs/cli/reference/permissions), [CLI configuration](https://cursor.com/docs/cli/reference/configuration)        | Typed documentation tables, despite the heading “Schema”; downloadable JSON Schema not found                                                                                                     |

The Agent Plugins schemas are standards-publisher artifacts explicitly linked by
Cursor, not Cursor-owned schemas and not community reconstructions of Cursor's
private configuration. Community marketplaces and third-party schemas were not
used as normative evidence. The official
[extension API](https://cursor.com/docs/extension-api) publishes TypeScript
declaration examples, which describe a programming interface rather than a
file-validation schema. “Not found” records this investigation's result, not
proof that an artifact cannot exist.

## 1. Instructions and rules

Project rules are `.cursor/rules/**/*.mdc` files with Markdown bodies and YAML
`description`, `globs`, and `alwaysApply`. A plain `.md` there is ignored.
`alwaysApply: true` wins over the other trigger fields; otherwise globs attach
rules to matching context, descriptions support relevance selection, and neither
means manual `@` invocation. User rules live in Customize settings. Team rules
come from the dashboard. Conflict order is team, project, user; applicable rules
are combined. Nested `AGENTS.md` is supported, with more specific instructions
winning over parent instructions. [Rules](https://cursor.com/docs/rules).

`/create-rule` and Customize create project files and expose status. Referenced
`@files` are read when needed, not automatically expanded into the prompt,
across editor, CLI, and cloud. Rules do not govern Cursor Tab; user rules also
do not apply to Inline Edit. No standalone rule lint command is documented here.
Generation therefore needs both trigger metadata and native file-extension
handling, rather than treating every Markdown instruction file alike.
[Rules](https://cursor.com/docs/rules).

## 2. Skills

Skills use `<skill>/SKILL.md` with required `name` matching the folder and
`description`. Additional documented fields are `paths`,
`disable-model-invocation`, `icon`, `color`, and `metadata`; legacy `globs`
remains accepted. `scripts/`, `references/`, and `assets/` are optional
resources. Discovery includes project/user `.agents/skills/` and
`.cursor/skills/`, plus Claude/Codex compatibility directories. Nested skills
are discovered recursively; skills in nested project directories inherit
directory scope. No complete same-name precedence table was found.
[Skills](https://cursor.com/docs/skills).

Customize lists discovered skills; `/skill-name` attaches to one message, while
Custom Modes retain a skill for the session. Local user skills do not imply
remote availability: personal `.cursor/skills` require cloud sync, and
`.agents/skills` are not copied automatically to cloud/remote workers. No native
standalone validator is documented. Preserve resources and distinguish automatic
invocation, manual invocation, file scope, and session duration in any adapter.
[Skills](https://cursor.com/docs/skills).

## 3. Subagents

Custom agents are Markdown plus YAML in project `.cursor/agents/` or user
`~/.cursor/agents/`; `.claude/agents/` and `.codex/agents/` compatibility
locations also load. Project definitions win name conflicts, and `.cursor` wins
over the compatibility roots. Fields are optional `name` (filename default),
`description`, `model` (`inherit` default or a model ID), `readonly` (false),
and `is_background` (false). The body supplies the prompt.
[Subagents](https://cursor.com/docs/subagents).

Editor, CLI, and Cloud Agents support subagents. They receive fresh contexts and
return results to the parent; foreground execution blocks, background execution
does not. Creation is documented through Agent or manual files; directory
inspection exposes custom definitions. No standalone lint command or generic
portable tool-allowlist field is established by the field table. Treat
`readonly` as a Cursor execution restriction, not interchangeable with another
client's named tools. Compatibility directory discovery does not prove that
foreign frontmatter has identical semantics.
[Subagents](https://cursor.com/docs/subagents).

## 4. Plugins and extensions

Cursor supports native `.cursor-plugin/plugin.json` packages and portable root
`plugin.json` packages. Native manifests require `name`; optional metadata and
`rules`, `skills`, `agents`, `commands`, `hooks`, `mcpServers`, and `variables`
configure components. Defaults discover corresponding directories,
`hooks/hooks.json`, and `mcp.json`; explicit component paths replace that
component's defaults. Native plugin variables use a restricted JSON Schema
subset. [Plugin reference](https://cursor.com/docs/reference/plugins).

Portable Agent Plugins contain skills and MCP; native Cursor plugins
additionally carry rules, agents, commands, and hooks. Customize installs either
at project or user scope. Local development uses
`~/.cursor/plugins/local/<name>`, then reload and inspection. Installed
marketplace names override local copies; outside-target symlinks are skipped.
Admin policy can disable local imports. Team marketplaces also support optional,
default-installed, or required distribution.
[Plugins](https://cursor.com/docs/plugins).

Editor extensions can register/unregister MCP servers and plugin paths through
`vscode.cursor`; this registration route can discover folders without a native
manifest. It is a separate runtime integration from distributing files.
[Extension API](https://cursor.com/docs/extension-api).

Portable schemas do not erase implementation differences: Cursor substitutes
`${CURSOR_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_ROOT}` but explicitly does not
expand standard `${PLUGIN_ROOT}` or `${PLUGIN_DATA}`. This is a concrete
portability exception even for a schema-valid package.
[Plugin reference](https://cursor.com/docs/reference/plugins).

## 5. MCP servers

Project `.cursor/mcp.json` and user `~/.cursor/mcp.json` contain an `mcpServers`
map. Local entries use `command`, `args`, `env`; remote entries use `url`,
`headers`, and optional OAuth `auth` with `CLIENT_ID`, `CLIENT_SECRET`,
`scopes`. Supported transports are stdio, SSE, and Streamable HTTP.
Interpolation includes `${env:NAME}`, `${userHome}`, and `${workspaceFolder}`.
Desktop and web OAuth callbacks differ. [MCP](https://cursor.com/docs/mcp).

The CLI shares editor configuration and explicitly documents precedence as
project → global → nested, including discovery from parent directories. Commands
are `agent mcp list`, `list-tools <identifier>`, `login <identifier>`,
`enable <identifier>`, and `disable <identifier>`. These inspect connections and
manage approval/authentication; they are not an offline schema checker.
[CLI MCP](https://cursor.com/docs/cli/mcp).

Do not equate installing a server declaration with installing its executable,
authenticating it, or permitting its tools. Preserve transport, secret
references, and execution-location assumptions independently. Cloud availability
needs its own provisioning evidence; a local stdio declaration alone does not
establish it.

## 6. Hooks

Cursor hooks use JSON `version: 1` and an event-to-definition-array `hooks` map.
Definitions include `command` or prompt execution, `type`, `timeout`, `matcher`,
`loop_limit`, and `failClosed`. Events cover sessions, tools, subagents, shell,
MCP, files, prompts, compaction, completion, Tab, and `workspaceOpen`. Event
input and output contracts differ. [Hooks](https://cursor.com/docs/hooks).

Sources include project `.cursor/hooks.json`, user `~/.cursor/hooks.json`, team,
and enterprise configuration. All matching hooks execute: deny outranks ask,
which outranks allow. Other response fields may use last-response precedence;
there is no universal replacement rule. Project commands run from the project
root, user commands from `~/.cursor`. Cloud runs repository command hooks only
after a writable environment is available. `workspaceOpen` explicitly covers
desktop and CLI. No standalone validation command was found. The documentation
also describes Claude-hook compatibility, but translations need event-by-event
checks, including failure behavior and working directories.
[Hooks](https://cursor.com/docs/hooks).

## 7. Commands and saved prompts

The former commands documentation redirects to skills help.
`/migrate-to-skills`, documented for Cursor 2.4+, converts user/workspace
commands and eligible dynamic rules; always-on or glob-scoped rules are
excluded. This is a migration surface, not evidence that older commands
vanished. [Migration help](https://cursor.com/help/customization/skills).

Native plugins still discover `commands/` files with `.md`, `.mdc`, `.markdown`,
or `.txt`; optional YAML fields include `name` and `description`.
[Plugin reference](https://cursor.com/docs/reference/plugins). Historical
project commands use `.cursor/commands/<command>.md`.
[Cursor 1.6 announcement](https://cursor.com/changelog/1-6). The reviewed
current references do not settle legacy command collision precedence or full
editor/CLI/cloud parity. Prefer an explicit manual-skill capability in future
design while retaining legacy import as a separate compatibility question. The
CLI also has operational commands such as `/plugin`, `/mcp`, `/config`, and
`/sandbox`; these are client controls, not saved prompt files.
[CLI slash commands](https://cursor.com/docs/cli/reference/slash-commands).

## 8. Execution policies and adjacent workflows

CLI policy lives under `permissions.allow`/`deny` in global
`~/.cursor/cli-config.json` or project `.cursor/cli.json`. Tokens include
`Shell(...)`, `Read(...)`, `Write(...)`, `WebFetch(...)`, and
`Mcp(server:tool)`; deny wins over allow. Relative file patterns use workspace
scope. [Permissions](https://cursor.com/docs/cli/reference/permissions). Only
permissions are project-configurable. Global configuration includes
`version: 1`, `editor.vimMode`, `approvalMode`, and sandbox settings;
configuration must be JSON without comments. Missing/corrupt data may be
repaired by the CLI. The reviewed reference does not specify a complete
project/global permission merge algorithm.
[CLI configuration](https://cursor.com/docs/cli/reference/configuration).

Editor defaults and approval behavior have a separate security contract; Cursor
calls run modes best-effort guardrails. Do not generate security guarantees from
prompt rules or assume CLI configuration controls cloud execution.
[Agent security](https://cursor.com/docs/agent/security).

Adjacent workflows include cloud Automations configured with triggers, prompts,
tools, and optional repositories; dashboard activation is part of their
lifecycle. No portable workflow-file schema was found in that guide.
[Automations](https://cursor.com/docs/cloud-agent/automations). For upskill
design, a bundle should remain a composition/distribution concept: selecting a
native plugin, portable plugin, plain files, or hosted automation requires
explicit capability and lifecycle decisions, not a shared filename alone.
