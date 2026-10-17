# Gemini CLI customization evidence

Research date: **2026-10-11**. Status: official documentation and vendor-source
inspection; **no Gemini CLI runtime testing**. Gemini CLI is a required future
design target for upskill; implementation remains deferred. This note concerns
the `google-gemini/gemini-cli` product only. It establishes no compatibility
with Gemini Code Assist or Antigravity.

The live documentation and `main` source are moving references, not a tested
release contract. A future adapter must pin a supported release and validate its
artifacts against that release. In particular, extension documentation still
labels subagents as preview, although the subagent guide says they are enabled
by default. Neither statement establishes a minimum stable version.

## Shared configuration and schema inventory

The official [configuration reference][configuration] describes JSON settings at
`~/.gemini/settings.json` and `<project>/.gemini/settings.json`, plus system
defaults and administrator settings. Its order, from weaker to stronger, is
application defaults, system defaults, user, project, system settings,
environment, CLI arguments. This is configuration precedence, not a universal
rule for all content discovery.

| Surface                           | Published evidence and schema status                                                                                                                                   |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Settings, MCP, hook configuration | [Official JSON Schema][settings-schema], draft 2020-12; includes `MCPServerConfig` and `HookDefinitionArray` definitions.                                              |
| Extension manifest                | [Vendor `ExtensionConfig` TypeScript interface][extension-type]; standalone JSON Schema **not found**. The inspected [schemas directory][schemas] lists settings only. |
| Instructions                      | [Markdown documentation][memory]; no machine schema found.                                                                                                             |
| Skills                            | [Documented frontmatter and authoring scripts][creating-skills]; no Gemini-specific standalone JSON Schema found.                                                      |
| Subagents                         | [Documented field table][agents] and [Zod validators in `agentLoader.ts`][agent-loader]; source validators, not published JSON Schema.                                 |
| Custom commands                   | [TOML documentation][commands] and [Zod `TomlCommandDefSchema`][command-loader].                                                                                       |
| Policy files                      | [TOML reference][policy] and [Zod `PolicyFileSchema`/`PolicyRuleSchema`][policy-loader].                                                                               |
| Hook input/output                 | [Event contract documentation][hook-reference]; TypeScript-like examples are not JSON Schema documents.                                                                |

The settings schema enables editor validation and completion. It does not
validate Markdown bodies, scripts, complete extension trees, or runtime
permissions. No general-purpose `gemini validate` command was established by
these sources.

## 1. Instructions and rules

`GEMINI.md` is plain Markdown context. Global instructions live at
`~/.gemini/GEMINI.md`; workspace and ancestor files provide project context.
Just-in-time discovery examines directories accessed by tools and their
ancestors up to a trusted root. The files are concatenated into instructional
context, rather than interpreted as typed permission rules.

`@file.md` imports can be relative or absolute. `context.fileName` can select a
different filename or list, including `AGENTS.md`. `/memory show` exposes the
loaded context and `/memory reload` rescans it. These are useful discovery
checks, not semantic validation. A generated rule adapter should preserve
imports deliberately and avoid promising that narrower text deterministically
overrides broader instructions. Other clients' glob-scoped rule metadata has no
documented direct equivalent here. [Context documentation][memory]

## 2. Skills

A skill directory contains `SKILL.md` with YAML `name` and `description`,
followed by instructions. Optional `scripts/`, `references/`, and `assets/`
travel with the skill. Discovery precedence increases through built-in,
extension, user, and workspace skills. User/workspace roots support both
`.gemini/skills/` and `.agents/skills/`. The alias supports shared Agent Skills
layout; it does not prove identical client activation behavior.
[Authoring guide][creating-skills]

`gemini skills install`, `link`, and `uninstall` manage content; installation
defaults to user scope and supports `--scope workspace`. `/skills list`,
`reload`/`refresh`, `enable`, and `disable` manage session discovery and use.
The management guide documents consent for remote installation and skill
activation. [Management guide][using-skills]

The authoring guide identifies native package scripts `init_skill.cjs`,
`validate_skill.cjs`, and `package_skill.cjs`; packaging produces a `.skill`
archive. These require Gemini's authoring environment and were not executed.
upskill can preserve portable skill content while treating activation consent
and discovery aliases as target behavior. [Authoring guide][creating-skills]

## 3. Subagents

Local definitions use `.gemini/agents/*.md` or `~/.gemini/agents/*.md`. YAML
frontmatter precedes a system-prompt body. Documented fields include required
`name`, `description`, and optional `kind`, `tools`, `model`, `temperature`,
`max_turns`, `timeout_mins`, and inline MCP configuration. Omitting `tools`
inherits the parent's tools; wildcards are supported, but recursive subagent
delegation is prohibited. `/agents` manages agents; `agents.overrides` provides
persistent settings overrides. A leading `@agent-name` nudges delegation.
[Subagent guide][agents]

The [command reference][cli-commands] documents `/agents list`, `reload`,
`enable`, `disable`, and `config` for discovery and configuration.

Source inspection finds strict Zod validation and snake-case `mcp_servers`,
`http_url`, and `include_tools`, which conflict with the guide's `mcpServers`
examples. Remote definitions use `kind: remote`, an agent-card URL or JSON, and
authentication fields; they represent A2A endpoints rather than a portable local
persona. [Loader source][agent-loader]

The inspected registry loads built-ins, project, user, then extensions and
rejects duplicate names after the first registration. Project loading is
trust-gated. This source-derived precedence requires release verification; even
the nearby source comment describes overwriting instead.
[Registry][agent-registry]

## 4. Plugins and extensions

Gemini calls its native packages extensions. An extension root has
`gemini-extension.json`; installations live under `~/.gemini/extensions`. The
manifest interface requires `name` and `version`; optional members include
`mcpServers`, `contextFileName`, `excludeTools`, `settings`, `themes`, `plan`,
and `migratedTo`. `contextFileName` accepts a string or array in the inspected
type. This interface is useful implementation evidence but is not a
machine-loadable JSON Schema. [Manifest type][extension-type]

`gemini extensions` provides `install`, `uninstall`, `update`, `new`, `link`,
`enable`, and `disable`; enablement supports user/workspace scope. Installation
copies content, while linking is for development. The reference says management
changes require restarting the session. `/extensions list` discovers installed
packages. [Extension reference][extensions]

Manifest settings expose `name`, `description`, `envVar`, and `sensitive`, with
sensitive values stored in the keychain. This lifecycle differs from merely
copying an upskill bundle. [Extension reference][extensions]

## 5. MCP

`mcpServers` is a map in settings, with user/project scope and the shared
configuration hierarchy. Servers use `command` for stdio, `url` for SSE, or
`httpUrl` for streaming HTTP. Common options include `args`, `env`, `cwd`,
`headers`, `timeout`, `includeTools`, `excludeTools`, `trust`, and
authentication configuration. The settings JSON Schema supplies structural
validation.

`gemini mcp add`, `list`, and `remove` manage configuration; add/remove default
to project scope. `enable`/`disable` control availability. `/mcp list` and
`/mcp auth` expose status/authentication; discovery connects to servers and
validates their advertised tools. Stdio connectivity checks depend on folder
trust. [MCP documentation][mcp]

Settings-defined server names take precedence over extension definitions.
Extension MCP declarations cannot set `trust`. Therefore a generic MCP record
needs target-specific transport, authentication, scope and trust translation;
the protocol alone does not standardize installation configuration.
[Extension reference][extensions]

## 6. Hooks

Settings hold `hooks.<Event>[]`, each containing a matcher, optional
`sequential`, and `hooks[]` command definitions. Command entries specify
`type: command`, `command`, and optional `name`, `timeout`, `description`.
Events cover tools, agent turns, models, sessions, notifications and
compression. Scripts receive JSON on stdin and return JSON on stdout. Exit 0
processes the response; exit 2 blocks according to event semantics; other
failures warn. Output fields such as `decision`, `continue`, `reason`, and
`hookSpecificOutput` have event-dependent effects.
[Hook reference][hook-reference]

User/project settings and extensions supply hooks. `/hooks panel`,
`enable-all`/`disable-all`, and named enable/disable operations manage them.
Project hook changes are fingerprinted and can require renewed trust.
[Hook overview][hook-overview]

The settings schema covers configuration, not executable behavior. A portable
hook must translate event names and JSON contracts, ship its scripts, and
preserve consent. Precedence is unresolved below; it must not be inferred from
settings precedence alone.

## 7. Commands and saved prompts

Custom commands are TOML v1 files under `~/.gemini/commands/` or project
`.gemini/commands/`; project names override user names. Subdirectories form
colon-separated names. Required `prompt` and optional `description` are checked
by the native Zod loader. [Command documentation][commands],
[loader source][command-loader]

`{{args}}` substitutes arguments, `!{...}` executes shell context collection,
and `@{...}` injects file content. Shell execution is confirmation-sensitive.
`/commands list` and `/commands reload` support discovery and refresh.
[Command documentation][commands]

There is no standalone command JSON Schema in the inspected inventory. Markdown
prompt files from another client need a TOML wrapper and deliberate
interpolation conversion. Arbitrary textual substitution could change execution
behavior; treating these files as passive prose loses their semantics.

## 8. Execution policies

The policy engine reads TOML `[[rule]]` records. Source validators require
`toolName`, `decision`, and integer `priority` from 0 through 999; optional
selectors include `subagent`, `mcpName`, `argsPattern`, `commandPrefix`,
`commandRegex`, `modes`, and `interactive`. Decisions are `allow`, `deny`, or
`ask_user`. Zod validation occurs during loading; no independent policy-lint
command was established. [Policy loader][policy-loader]

The reference documents `~/.gemini/policies/*.toml`, extension and administrator
tiers, and explicitly warns that automatic workspace policy discovery is
nonfunctional. The table ranks administrator above user, workspace, extension,
and defaults, but examples conflict. [Policy reference][policy]

Sandboxing is a separate execution control selected through `--sandbox`,
`GEMINI_SANDBOX`, or `tools.sandbox`, in that precedence. Available mechanisms
depend on the OS and external runtimes. This cannot be represented by prose
rules or assumed equivalent to another client's permission model.
[Sandbox documentation][sandbox]

## Workflow composition and unresolved evidence

A future bundle can compose persistent context, an on-demand skill, a saved
command, a specialist agent, and MCP access. An extension is one native delivery
vehicle, with `commands/`, `skills/`, `agents/`, `hooks/hooks.json`, and
`policies/` alongside its manifest. Hook declarations belong in their own file,
not the manifest. Extension policies cannot grant `allow` or YOLO behavior.
[Extension reference][extensions], [extension authoring][writing-extensions]

Generation should preserve these distinctions and record unsupported semantics.
The following findings remain release-validation gates:

- Hook documentation ranks project above user above system, while general
  settings documentation ranks system above project. Merging and override
  behavior need a pinned-runtime test. [Hooks][hook-overview],
  [configuration][configuration]
- Subagent documentation spells inline MCP differently from its strict loader.
  It also shows `[[rules]]`/`action`, whereas the policy loader accepts
  `[[rule]]`/`decision`. [Agents][agents], [agent loader][agent-loader],
  [policy loader][policy-loader]
- Policy tier prose/examples disagree with the five-tier table; the extension
  guide also describes sharing a tier with workspace policies. No generated
  policy should rely on those contradictory examples. [Policy][policy],
  [extensions][extensions]
- The extension reference requires restarts, but `/commands reload` explicitly
  includes extensions in the command reference. Verify which operations require
  restart instead of assuming one lifecycle rule. [Extensions][extensions],
  [commands reference][cli-commands]

[settings-schema]: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json
[schemas]: https://github.com/google-gemini/gemini-cli/tree/main/schemas
[configuration]: https://geminicli.com/docs/reference/configuration/
[memory]: https://geminicli.com/docs/cli/gemini-md/
[creating-skills]: https://geminicli.com/docs/cli/creating-skills/
[using-skills]: https://geminicli.com/docs/cli/using-agent-skills/
[agents]: https://geminicli.com/docs/core/subagents/
[agent-loader]: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/agents/agentLoader.ts
[agent-registry]: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/agents/registry.ts
[extensions]: https://geminicli.com/docs/extensions/reference/
[extension-type]: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/cli/src/config/extension.ts
[writing-extensions]: https://geminicli.com/docs/extensions/writing-extensions/
[mcp]: https://geminicli.com/docs/tools/mcp-server/
[hook-reference]: https://geminicli.com/docs/hooks/reference/
[hook-overview]: https://geminicli.com/docs/hooks/
[commands]: https://geminicli.com/docs/cli/custom-commands/
[cli-commands]: https://geminicli.com/docs/reference/commands/
[command-loader]: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/cli/src/services/FileCommandLoader.ts
[policy]: https://geminicli.com/docs/reference/policy-engine/
[policy-loader]: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/core/src/policy/toml-loader.ts
[sandbox]: https://geminicli.com/docs/cli/sandbox/
