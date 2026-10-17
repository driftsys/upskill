# OpenCode customization evidence

Research date: **2026-10-11**. This is a documentation and published-schema
inspection, not a runtime compatibility test. No OpenCode installation, plugin
installation, or generated configuration was changed.

The official site now separates OpenCode 1 and OpenCode 2, both using the
`opencode` executable. The sections below identify the V1 surface matching
upskill's existing output and the material V2 differences. A client name alone
is therefore insufficient to select a plugin API or native configuration format.
See the [V1 introduction](https://opencode.ai/docs/) and
[V2 migration guide](https://opencode.ai/v2/docs/migrate-v1/).

## Configuration and source inventory

V1 accepts JSON/JSONC. Increasing precedence is remote organizational defaults,
`~/.config/opencode/opencode.json`, `OPENCODE_CONFIG`, project `opencode.json`,
`.opencode` directories, `OPENCODE_CONFIG_CONTENT`, managed files, then macOS
managed preferences. Conflicting keys override; unrelated keys survive.
Directory names are plural, with singular compatibility forms.
`OPENCODE_CONFIG_DIR` provides another content directory. Editors can validate
against the advertised schema; `opencode debug config` inspects resolved
configuration. [V1 configuration](https://opencode.ai/docs/config/)

V2 searches ancestor configuration to the filesystem root, merging distant
direct files before nearer ones, then `.opencode` files in the same order. Every
discovered `.opencode` configuration consequently outranks direct files. Both
documentation generations advertise the same schema URL, despite the conflict
recorded below. [V2 configuration](https://opencode.ai/v2/docs/config/)

| Surface               | Machine-readable contract found                                                                                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Runtime configuration | [config.json](https://opencode.ai/config.json), JSON Schema draft 2020-12; observed root reference `#/$defs/Config` |
| Instructions          | `Config.properties.instructions`, array of strings; Markdown body is documentation-only                             |
| Skills                | `Config.properties.skills` covers V1 `paths`/`urls`; no standalone SKILL.md schema found                            |
| Agents                | `#/$defs/AgentConfig`; no standalone Markdown-frontmatter schema found                                              |
| Plugins               | `Config.properties.plugin`, strings or package/options tuples; implementation uses SDK types                        |
| MCP                   | `#/$defs/McpLocalConfig`, `McpRemoteConfig`, `McpOAuthConfig`                                                       |
| Hooks                 | Plugin SDK callback types; no separate declarative hooks schema found                                               |
| Commands              | `Config.properties.command`; entries require `template`                                                             |
| Execution policies    | `#/$defs/PermissionConfig`, `ConfigV2.Experimental.Policy`                                                          |
| V1 terminal UI        | [tui.json](https://opencode.ai/tui.json), separately fetched schema; outside runtime customization                  |

The category documentation linked below supplies discovery and lifecycle
semantics that the JSON Schema does not express. Direct retrieval of the
attempted upstream plugin source URL failed; SDK-type claims here rely on
official documentation, not a verified source snapshot.

## 1. Instructions and rules

V1 reads project `AGENTS.md` and global `~/.config/opencode/AGENTS.md`.
`CLAUDE.md` and `~/.claude/CLAUDE.md` are respective fallbacks, rather than
additional rules when the corresponding AGENTS file exists. Local discovery
walks ancestors; the documentation describes first-match precedence within each
category. `/init` creates or improves project guidance. `instructions` in global
or project configuration adds paths, globs, and URLs, combined with AGENTS
content. Remote fetching has a five-second timeout. There is no documented
frontmatter, severity, or structured rule schema for the Markdown itself.
[Rules](https://opencode.ai/docs/rules/)

Portability implication: explicit instruction-file registration can preserve
separate generated rule files, but it does not create another client's
glob-based activation semantics. V2 currently discovers AGENTS files, with
ambient traversal up to home or the project boundary outside home. Its missing
CLAUDE fallback is acknowledged as a compatibility problem. Do not silently use
CLAUDE-only guidance as a V2 contract.
[Migration](https://opencode.ai/v2/docs/migrate-v1/)

## 2. Skills

V1 discovers `<name>/SKILL.md` under project/global `.opencode/skills`,
`.claude/skills`, and `.agents/skills` equivalents. Project discovery walks to
the Git worktree. YAML requires `name` and `description`; optional fields
include `license`, `compatibility`, and string-map `metadata`. Documented
constraints are lowercase kebab-case names of 1–64 characters matching the
directory, and descriptions of 1–1024 characters.
`metadata.opencode/slash: "true"` exposes a slash command;
`opencode/autoinvoke: "false"` suppresses model advertising. The model discovers
summaries and loads bodies through `skill`.
[V1 skills](https://opencode.ai/docs/skills/)

V2 also accepts flat Markdown files. IDs derive from paths; frontmatter `name`
is a display label. Frontmatter is optional at runtime, and V1 naming/length
constraints are recommendations rather than enforced validation. Later duplicate
IDs win: compatibility sources precede native global/project sources, followed
by explicit configuration entries. HTTP catalogs use `index.json` entries with
`name`, `version`, and `files`; changing version refreshes cached content.
Discovery troubleshooting is documented, but no standalone skill validator or
skill install command was found. Portable generation should retain the stricter
V1 shape. [V2 skills](https://opencode.ai/v2/docs/skills/)

## 3. Subagents

V1 agents are JSON entries under `agent` or Markdown files in
`.opencode/agents/` and `~/.config/opencode/agents/`. The filename determines
identity; YAML supplies configuration and the body supplies the prompt. Fields
include `description`, `mode` (`primary`, `subagent`, `all`), `model`,
`temperature`, `steps`, `permission`, `hidden`, and `disable`. Modes default to
`all`; `hidden` changes menu visibility, not Task authorization.
`permission.task` controls delegation by agent-name patterns. Do not map
visibility to access control. [Agents](https://opencode.ai/docs/agents/)

`opencode agent create` generates a definition; `opencode agent list` discovers
available agents. The CLI documents noninteractive creation flags and a
permission allowlist whose omitted entries are denied.
[CLI](https://opencode.ai/docs/cli/#agent)

V2 uses `agents`, `system`, `disabled`, and ordered `permissions`; model
variants join the model after `#`, and sampling options move under
`request.body`. Supported legacy definitions are translated. Keep each nested
definition in one format, rather than mixing V1/V2 fields.
[Migration](https://opencode.ai/v2/docs/migrate-v1/)

## 4. Plugins and extensions

V1 loads JavaScript/TypeScript from `.opencode/plugins/` or
`~/.config/opencode/plugins/`; npm packages are registered in `plugin`. Startup
installs package dependencies using Bun. Global config, project config, global
directories, and project directories contribute plugins in that order; hooks
compose sequentially. Equal package name/version entries are deduplicated, while
local and npm copies can both run.
[V1 plugins](https://opencode.ai/docs/plugins/)

The live V1 CLI explicitly documents `opencode plugin <module>` and alias
`plug`, with `--global` and `--force`. This is observed documentation, not an
assumption about installed releases. [CLI](https://opencode.ai/docs/cli/#plugin)

V2 uses `plugins` entries, optionally `{package, options}`. It documents
`plugin add`, `list`, `check`, `update`, and `remove` for global package
plugins, automatic config reload, and `service restart`. Local entries resolve
relative to their config file. Exact versions/commits remain pinned.
[V2 plugins](https://opencode.ai/v2/docs/plugins/)

Custom tools are another extension surface: `.opencode/tools/` and its global
equivalent contain JS/TS definitions using `tool()` with `description`, `args`,
and `execute`; arguments use Zod. These are executable definitions, not portable
prompt documents. [Custom tools](https://opencode.ai/docs/custom-tools/)

## 5. MCP

V1 `mcp.<name>` selects `type: local` with a `command` array, optional `cwd`,
`environment`, `enabled`, and `timeout`; remote entries use `type: remote`,
`url`, `headers`, and OAuth options. Startup exposes connected tools. Project
configuration can enable organizational defaults. OAuth can be automatic or
configured; credentials are separate from portable server declarations.
[MCP servers](https://opencode.ai/docs/mcp-servers/)

Documented commands are `opencode mcp add`, `list`/`ls`, `auth`, `auth list`,
`logout`, and `debug`. Connection status is operational discovery, not proof
that every exposed tool works. [CLI](https://opencode.ai/docs/cli/#mcp)

V2 nests entries under `mcp.servers`, inverts `enabled` to `disabled`, separates
catalog/execution timeouts, and uses snake-case OAuth keys. Translation must
preserve those semantics, not merely rename a top-level map.
[Migration](https://opencode.ai/v2/docs/migrate-v1/)

## 6. Hooks

V1 hooks are native JS/TS callbacks returned by a plugin, typed through
`@opencode-ai/plugin`. Examples include `tool.execute.before`,
`tool.execute.after`, `shell.env`, and an `event` callback inspecting
`session.idle`. They inherit plugin scope and startup lifecycle. No declarative
shell-hook configuration or standalone hook validator was found.
[Plugin callbacks](https://opencode.ai/docs/plugins/)

V2 uses `@opencode/plugin` and `Plugin.define({id, setup})`. Its plugin build
documentation describes a different extension API; V1 plugin implementations
require porting. A cross-client hook abstraction therefore needs an explicit
adapter and version boundary, rather than copying another client's event names
into JSON. [V2 plugin API](https://opencode.ai/v2/docs/build/plugins)

## 7. Commands and saved prompts

V1 supports `command.<name>` JSON definitions or `.opencode/commands/<name>.md`
and global equivalents. YAML contains `description`, `agent`, `model`, and
`subtask`; the body supplies the required template. `/name` invokes it.
Templates support `$ARGUMENTS`, positional `$1` arguments, file references, and
shell-output interpolation. Custom names can override built-ins. Choosing a
subagent normally delegates; `subtask` can control that behavior. No independent
command linter or removal lifecycle was found; definitions are
configuration/files. JSON schema validation covers structural fields, not prompt
meaning or shell effects. [Commands](https://opencode.ai/docs/commands/)

V2 calls the map `commands` and delegation flag `subagent`. These execution and
interpolation semantics need explicit portability handling.
[V2 configuration](https://opencode.ai/v2/docs/config/)

## 8. Execution policies

V1 `permission` accepts `allow`, `ask`, or `deny`, including tool/input pattern
maps. Last matching rules win; agent overrides follow shared permissions. `edit`
covers file mutations, `task` delegation, and `skill` skill loading. `--auto`
approves asks but preserves denials. Legacy boolean `tools` is deprecated since
1.1.1. [Permissions](https://opencode.ai/docs/permissions/)

V1 experimental resource policies use `effect`, `action`, and `resource`, with
`provider.use` documented. Global policy takes precedence over project policy,
an exception to ordinary configuration precedence.
[Policies](https://opencode.ai/docs/policies/)

V2 permissions use ordered `{action, resource, effect}` arrays; `shell` and
`subagent` replace `bash` and `task`. Agent permissions are not constrained to a
subset of the parent's. Saved approvals persist per project but cannot override
configured denials. Global/managed policies can hard-deny permission checks. The
shell still executes with host-user authority: these controls are not an OS
sandbox. [V2 permissions](https://opencode.ai/v2/docs/permissions/)

## Workflows, bundles, and unresolved evidence

Commands, skills, and plugins compose workflows. No general native
prompt-content bundle manifest was found in the inspected documentation; npm
plugin packages and HTTP skill catalogs are distinct distribution mechanisms.

The live schema exposes V1 singular maps while V2 examples advertise plural maps
using that same schema URL. Treat V2 schema validation as unresolved until a
versioned contract or corrected published schema is available. Agent
documentation also calls `description` required, whereas the observed
`AgentConfig` schema does not require it. Neither discrepancy should be silently
normalized away. [Published schema](https://opencode.ai/config.json),
[V2 schema guidance](https://opencode.ai/v2/docs/config/#schema),
[agent documentation](https://opencode.ai/docs/agents/#description)

Design inference: preserve the existing `.agents/skills`, `.opencode/agents`,
instruction registration, and permission-map implementation as a separately
tested V1 compatibility surface. Add version-aware capability decisions before
generating native V2 policies or plugin code. This research establishes
available contracts and gaps; it does not certify current upskill output against
a binary.
