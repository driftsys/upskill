# GitHub Copilot agent customization

Research date: **2026-10-11**. Evidence: opened official documentation,
published schemas, and upskill source inspection. No Copilot runtime behavior
was tested. These are rolling documentation contracts, not guarantees for older
installed versions. “Not found” means no downloadable schema was identified in
the sources inspected; it does not establish that none exists.

Copilot CLI, VS Code Local, VS Code Copilot Agent Host, and GitHub cloud agent
must remain separate targets. In particular, a VS Code configuration file may be
forwarded to Agent Host without being readable by the standalone CLI.

## 1. Instructions and rules

CLI project files include `.github/copilot-instructions.md`,
`.github/instructions/**/*.instructions.md`, and `AGENTS.md`, `CLAUDE.md`, or
`GEMINI.md`. Personal files use `~/.copilot/copilot-instructions.md` and
`~/.copilot/instructions/**/*.instructions.md`; `COPILOT_HOME` relocates them.
Modular frontmatter uses `applyTo` with comma-separated globs. Applicable
instructions combine; no general conflict precedence is promised.
`/instructions` lists and toggles sources. Restart/resume or `/new` applies file
changes. [CLI instructions][instructions-cli]

VS Code modular instructions have optional `name`, `description`, and `applyTo`.
Descriptions support task-based discovery. Local user instructions may reside in
profile storage; Copilot Agent Host uses `~/.copilot/instructions`.
`chat.instructionsFilesLocations` is deprecated and Local-only.
[VS Code instructions][instructions-vscode]

Cloud agent consumes repository-wide, path-specific, and agent instruction
files. GitHub.com Chat does not have the same path-specific support. IDE
code-review support varies; Eclipse Chat lists repository-wide instructions
only. [Support matrix][instructions-support]

Schema: documentation-only frontmatter contract; downloadable JSON Schema not
found. `applyTo: "**"` is file matching, not evidence of identical always-on
behavior across all consumers.

## 2. Skills

Project locations are `.github/skills/<name>/SKILL.md`, `.agents/skills`, and
`.claude/skills`; personal CLI locations are `~/.copilot/skills` and
`~/.agents/skills`. VS Code additionally documents `~/.claude/skills`. `name`
and `description` are required; scripts/resources live beside `SKILL.md`. The
official support list includes CLI, cloud agent, code review, Copilot app, VS
Code agent mode, and JetBrains agent mode. [Skill overview][skills-about]

VS Code requires lowercase letters/digits/hyphens, directory-matching names up
to 64 characters, and descriptions up to 1024. Optional fields include
`argument-hint`, `user-invocable`, `disable-model-invocation`, and experimental
`context: fork`. Its custom skill-location setting is deprecated and Local-only.
[VS Code skills][skills-vscode]

CLI project skills precede personal skills. `copilot skill list --json`, `add`,
`remove`, `enable`, and `disable` manage them. Its reference allows a broader
name alphabet and `allowed-tools`; portable authoring should retain the stricter
VS Code naming subset. [CLI reference][cli-reference]

Schema: Agent Skills prose specification and client extensions; no official
Copilot-specific downloadable frontmatter schema found.

## 3. Custom agents and subagents

Project `.github/agents/<name>.agent.md` profiles contain Markdown plus YAML.
Shared fields include required `description`, optional `name`, `tools`, `model`,
`target`, `user-invocable`, `disable-model-invocation`, `mcp-servers`, and
`metadata`. Omitted `tools` permits all tools; `[]` permits none. Documented
aliases include `read`, `edit`/`Write`, `search`/`Grep`/`Glob`, and
`execute`/`shell`. Cloud ignores IDE `handoffs` and `argument-hint`; IDE agents
do not use profile `mcp-servers` or `metadata`. Cloud repository agents override
organization and enterprise agents. [Agent reference][agents-shared]

CLI personal agents in `~/.copilot/agents` precede project agents by ID;
`.claude/agents` is another project source. CLI offers `models`, `modelPolicy`,
`reasoningEffort`, and `include-custom-instructions`; custom subagents do not
receive repository instructions unless the last field is true. The CLI still
documents `infer`; the shared reference calls it retired.
[CLI reference][cli-reference]

VS Code also lists `~/.claude/agents`, uses `agents` to restrict subagent
choices, and accepts a fallback array in `model`, rather than CLI's `models`.
Its agent picker and Agent Customizations editor expose discovery; `handoffs`
provides transitions. Local agent-scoped hooks are preview functionality.
[VS Code agents][agents-vscode]

Schema: documentation-only; downloadable profile schema not found. Do not equate
agent tool selection with execution authorization.

## 4. Plugins and executable extensions

CLI supports legacy plugin manifests and Agent Plugins 1.0/1.1. Standard root
`plugin.json` requires canonical `$schema` and `name`; optional metadata
includes `version`, `description`, `author`, `license`, and `extensions`. Fixed
portable components are `skills/` and `mcp.json`; Copilot agents, commands,
rules, and hooks use `com.github.copilot/`. Legacy configurable component paths
do not become standard fields. Install with `copilot plugin install SOURCE`;
sources include `plugin@marketplace`, Git repositories, subdirectories, and
local paths. `marketplace add`, `list`, `update`, `enable`, `disable`, and
`uninstall` manage lifecycle. Unsupported standard versions are rejected.
[CLI plugin reference][plugins-cli]

VS Code supports standard and legacy Copilot/Claude/OpenPlugin formats. Agent
Host `/plugin` and the Agent Customizations editor currently have separate
inventories. Repository activation is not equivalent to globally installing
every component. [VS Code plugins][plugins-vscode]

Executable CLI extensions are experimental JavaScript processes:
`.github/extensions/NAME/extension.{mjs,cjs,js}` or the corresponding
`~/.copilot/extensions/NAME/` directory, also distributable in plugins. Enable
experimental features; `/extensions mode` selects loading/management behavior.
No collision precedence or downloadable extension schema was found. These are
code modules, not Markdown plugins. [CLI extensions][extensions]

Published standard schemas are listed below; legacy plugin schema URL not found.

## 5. MCP

CLI JSON uses `mcpServers`; local entries include `type`, `command`, `args`,
`env`, and `tools`; remote entries use `url` and `headers`. User configuration
is `~/.copilot/mcp-config.json`. Trusted project `.mcp.json` and
`.github/mcp.json` files override user definitions; nearer directories win, and
`.mcp.json` wins within one directory. CLI does not read `.vscode/mcp.json`. Use
`copilot mcp add`, `list`, and `/mcp`; registry search is experimental.
[CLI MCP guide][mcp-cli]

VS Code Local uses `.vscode/mcp.json` or profile configuration with `servers`,
optional `inputs`, and `sandbox`. Agent Host reads portable configuration and
receives forwarded VS Code servers except those requiring interactive inputs.
`MCP: List Servers` exposes start/stop/restart/output; IntelliSense validates
the editor format. A standalone schema URL was not found.
[VS Code MCP][mcp-vscode]

Cloud repository settings accept `mcpServers` JSON and validate on save.
Cloud/code review support tools, not MCP resources/prompts, and do not support
remote OAuth servers. Cloud requires `type` and `tools`; local servers require
`command`/`args`, remote servers `url`. Repository settings override
custom-agent MCP definitions. Personal local files are not cloud configuration.
[Cloud MCP][mcp-cloud], [agent precedence][agents-shared]

The standard plugin MCP schema below is a packaging schema, not a universal
schema for these three configuration envelopes.

## 6. Hooks

CLI combines policy, user, project, and plugin hook sources. Files include
`~/.copilot/hooks/*.json` and `.github/hooks/*.json`; settings can contain
inline hooks. JSON has `version: 1` and `hooks`, with event arrays such as
`preToolUse`, `postToolUse`, and `sessionStart`. Command fields include `type`,
`bash`/`powershell`/`command`, `cwd`, `env`, and `timeoutSec`; CLI additionally
accepts `exec`/`args`. Directory-file validation can reject individual malformed
items; structural errors reject the file. Cloud runs in ephemeral Linux:
repository hooks exist by default, `ask` becomes denial, and interactive events
differ. [Copilot hooks][hooks-copilot]

Local hooks are preview. Workspace `.github/hooks/*.json`, personal
`~/.copilot/hooks/*.json`, plugins, and agent frontmatter contribute hooks;
`/hooks` and `Chat: Configure Hooks` inspect them. Native Local entries use
PascalCase events, `command`, platform overrides, and `timeout`.
Claude-compatible matcher values are ignored by Local. Copilot Agent Host
follows the provider reference, not Local payload semantics.
[Hook discovery][hooks-vscode], [Local event contract][hooks-local]

Schema: documented JSON/event contracts; standalone downloadable hook schema not
found. Combining files does not establish portable execution order or identical
decision behavior.

## 7. Commands and saved prompts

Local `.github/prompts/*.prompt.md` and user-profile prompts use optional
`name`, `description`, `argument-hint`, `agent`, `model`, and `tools`. Invoke
through `/NAME` or `Chat: Run Prompt`; `/prompts` manages discovery. Prompt
tools override referenced-agent tools, then defaults. **Agent Host does not load
prompt files**; these are deprecated there, and Local removal is announced
without a date. Migrate to skills. No standalone schema found.
[VS Code prompts][prompts]

CLI also recognizes `.claude/commands/*.md`, deriving names from filenames;
skills win same-name collisions. Plugins and executable extensions can supply
commands. This is not evidence that cloud agent accepts IDE prompt files.
[CLI reference][cli-reference]

## 8. Execution policies

CLI separates available tools from permission grants. `--available-tools` /
`--excluded-tools` control exposure; `--allow-tool` / `--deny-tool` control
execution, with deny prevailing. Saved approvals use personal
`permissions-config.json`, scoped by absolute repository location; these are not
a shared repository policy. [CLI permissions][permissions-cli]

User `~/.copilot/settings.json`, committed `.github/copilot/settings.json`, and
uncommitted `settings.local.json` have different permitted keys. Local overrides
repository settings; managed restrictions constrain lower layers. Keys include
`sandbox`, `enabledPlugins`, `extraKnownMarketplaces`, `disabledSkills`, and
`disabledMcpServers`; merge behavior is per-key, not one universal ordering.
`copilot config --repo/--local/--global --list --json` inspects configuration.
[CLI settings][settings-cli]

VS Code uses permission levels and settings such as
`chat.tools.terminal.autoApprove`; a `false` entry asks rather than blocks.
Sandboxing is a separate boundary, with platform-dependent preview status.
`Chat: Reset Tool Confirmations` clears approvals. Cloud has no interactive
approval UI. [VS Code permissions][permissions-vscode],
[cloud environment][hooks-copilot]

Schema: documented settings/permission contracts; downloadable complete schemas
not found. Agent frontmatter restrictions must not be translated into approval
grants.

## Workflows, composition, and upskill implications

Dynamic workflows are public preview CLI/app extensions, enabled experimentally.
`copilot workflow run NAME --args JSON` runs an existing definition;
`/workflows` monitors runs. Definitions can be personal, project, or
plugin-distributed; no portable workflow serialization schema was identified.
[Workflow guide][workflows]

An upskill bundle can distribute several native components, but its dependency
graph is not an Agent Plugins manifest. Keep portable skills/MCP separate from
client extension namespaces and policy. Source inspection found concrete drift:
Copilot tool mapping drops file capabilities and can omit `tools` entirely;
global output uses `~/.github/*` instead of native `~/.copilot/*`; ancillary
documentation incorrectly claims CLI lacks project MCP configuration. New
optional fields alone are not defects. Runtime smoke tests remain outstanding.

## Source and schema inventory

The linked documents above are the source inventory. Machine-readable artifacts:

| Artifact                                                                                          | Status on research date                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Agent Plugins 1.0 plugin schema](https://agent-plugins.org/schemas/1.0.0/plugin.schema.json)     | Opened successfully; root manifest                                                                                                                                         |
| [Agent Plugins 1.0 MCP schema](https://agent-plugins.org/schemas/1.0.0/mcp.schema.json)           | Opened successfully; plugin MCP envelope                                                                                                                                   |
| [Agent Plugins 1.1 plugin identifier](https://agent-plugins.org/schemas/1.1.0/plugin.schema.json) | Named in CLI reference; canonical retrieval failed. The upstream repository labels 1.1 a working draft; source snapshots are collected separately in the shared inventory. |
| Other categories                                                                                  | Documentation-only contracts identified; no verified standalone schema URL                                                                                                 |

[instructions-cli]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions
[instructions-vscode]: https://code.visualstudio.com/docs/agent-customization/custom-instructions
[instructions-support]: https://docs.github.com/en/copilot/reference/custom-instructions-support
[skills-about]: https://docs.github.com/en/copilot/concepts/agents/about-agent-skills
[skills-vscode]: https://code.visualstudio.com/docs/agent-customization/agent-skills
[cli-reference]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference
[agents-shared]: https://docs.github.com/en/copilot/reference/custom-agents-configuration
[agents-vscode]: https://code.visualstudio.com/docs/agent-customization/custom-agents
[plugins-cli]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference
[plugins-vscode]: https://code.visualstudio.com/docs/agent-customization/agent-plugins
[extensions]: https://docs.github.com/en/enterprise-cloud@latest/copilot/concepts/agents/copilot-cli/about-cli-extensions
[mcp-cli]: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers
[mcp-vscode]: https://code.visualstudio.com/docs/agents/reference/mcp-configuration
[mcp-cloud]: https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/configure-mcp-servers
[hooks-copilot]: https://docs.github.com/en/copilot/reference/hooks-reference
[hooks-vscode]: https://code.visualstudio.com/docs/agent-customization/hooks
[hooks-local]: https://code.visualstudio.com/docs/agents/reference/hooks-reference
[prompts]: https://code.visualstudio.com/docs/agent-customization/prompt-files
[permissions-cli]: https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/allowing-tools
[settings-cli]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference
[permissions-vscode]: https://code.visualstudio.com/docs/agents/run/approvals
[workflows]: https://docs.github.com/en/copilot/how-tos/use-copilot-agents/use-dynamic-workflows
