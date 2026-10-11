# Codex agent customization reference

Evidence date: **2026-10-11**. Official documentation and repository files were
read; client behavior was **not runtime-tested**. This reference concerns public
Codex clients. The tools and injected instructions available in this research
session are not evidence of stock CLI, IDE, desktop, or hosted support.

## Configuration baseline

Codex uses TOML. In descending precedence: command-line overrides; trusted project
`.codex/config.toml` layers, nearest directory first; selected
`~/.codex/<profile>.config.toml`; user `~/.codex/config.toml`; cloud-managed
defaults; system `/etc/codex/config.toml`; built-ins. Enforced requirements are
constraints, separate from defaults. Untrusted projects do not load their
`.codex/` configuration, hooks, or execution rules. CLI and IDE share these
layers. [Configuration basics](https://learn.chatgpt.com/docs/config-file/config-basic)

## 1. Instructions and prose rules

**Format/discovery:** plain Markdown. Global discovery uses
`$CODEX_HOME/AGENTS.override.md`, otherwise `AGENTS.md`; the default home is
`~/.codex`. Project discovery walks root to working directory, selecting at most
one nonempty file per directory: `AGENTS.override.md`, then `AGENTS.md`, then
`project_doc_fallback_filenames`. Later guidance overrides earlier guidance.
`project_doc_max_bytes` defaults to 32 KiB. Discovery occurs at session startup.

**Schema/lifecycle:** documentation-only content format; no published standalone
JSON Schema or dedicated instruction validator found. Restart a session to
rebuild its instruction chain.

**Portability:** directory-based discovery is not equivalent to arbitrary
file-glob activation. Aggregating independent rules also requires ownership and
size handling. Do not translate prose rules into Codex command-permission
`.rules` files. [AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)

## 2. Skills

**Format:** `SKILL.md`, YAML frontmatter requiring `name` and `description`, then
Markdown instructions; optional scripts, references, and assets. Sidecar
`agents/openai.yaml` provides `interface` metadata,
`policy.allow_implicit_invocation`, and `dependencies.tools` entries with
`type`, `value`, `description`, `transport`, and `url`.

**Discovery:** `.agents/skills` at each directory from working directory to
repository root, `$HOME/.agents/skills`, `/etc/codex/skills`, and bundled system
skills. Symlinks are supported. Duplicate names need not merge or override:
both may appear. Metadata loads first; full instructions load on selection.

**Lifecycle/support:** `/skills`, `$skill-name`, implicit matching, and
`$skill-installer` are documented. Changes are detected automatically, with
restart as a fallback. `[[skills.config]]` can disable individual skills.
Standalone skills support desktop, CLI, and IDE; plugin skills have broader
ChatGPT reach. No dedicated Codex skill or sidecar JSON Schema was found.
The format follows Agent Skills; dependencies and invocation metadata remain
host-specific. [Build skills](https://learn.chatgpt.com/docs/build-skills)

## 3. Subagents

**Format/discovery:** one TOML file per custom agent under `.codex/agents/` or
`~/.codex/agents/`. Required: `name`, `description`, `developer_instructions`.
Identity comes from `name`; a custom name can replace a built-in role. Optional
session settings include `model`, `model_reasoning_effort`, `sandbox_mode`,
`mcp_servers`, and `skills.config`.

**Inheritance:** omitted settings inherit. Explicit custom-file model/effort
overrides the resolved spawn/default/parent selection. Live parent permission
overrides are reapplied. Global `[agents]` includes `enabled`,
`max_concurrent_threads_per_session`, `default_subagent_model`, and
`default_subagent_reasoning_effort`.

**Lifecycle/limits:** `/agent` inspects CLI agent threads; creation is file-based.
No standalone agent schema or dedicated validation command was found. The docs
warn that the configuration-layer format may evolve. There is no established
lossless mapping from a generic tool list or `preload-skills` to this format.
[Subagents](https://learn.chatgpt.com/docs/agent-configuration/subagents)

The configuration reference additionally documents `[agents.<name>]` with
`description` and `config_file`. Preserve this distinction from standalone agent
discovery; the general config schema is not proof that it validates the entire
standalone agent envelope.
[Configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)

## 4. Plugins and extensions

**Packaging:** the recommended portable format is root `plugin.json` with the
Agent Plugins schema, identity metadata, fixed `skills/`, and optional
`mcp.json`. OpenAI-specific settings belong in `extensions.com.openai`.
Compatibility format `.codex-plugin/plugin.json` remains supported, declaring
`skills` and `mcpServers`, often pointing to `.mcp.json`. An inline OpenAI
extension replaces, rather than merges with, the compatibility overlay.

**Discovery:** repo/personal `.agents/plugins/marketplace.json` contains
`name`, `interface`, and `plugins` entries with source and install/auth policies.
Installed copies use `~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/`.
No separate Codex marketplace schema was found. Arbitrary instruction files and
custom-agent TOML are not established plugin components in these docs.
[Package plugins](https://developers.openai.com/plugins/build/plugins)

**CLI lifecycle:** `codex plugin add <plugin[@marketplace]>`, `list`, and `remove`
are documented as stable; JSON output is supported. Marketplace commands are
`add`, `list`, `upgrade`, and `remove`. These are public commands, not inferred
from this session's tools.
[Developer commands](https://learn.chatgpt.com/docs/developer-commands)

**Publishing:** public-directory upload/review/publication is a dashboard flow,
distinct from local installation. Hook-bearing packages and `.app.json` app
references cannot currently be submitted. Manual desktop distribution is
documented for plugins requiring hooks.
[Submission](https://developers.openai.com/plugins/deploy/submission)

## 5. MCP servers

**Format/scope:** `[mcp_servers.<name>]` in shared user or trusted project
`config.toml`. STDIO uses `command`, `args`, environment settings; HTTP uses
`url`, optional token/header/OAuth configuration. Policy includes
`enabled_tools`, `disabled_tools`, `enabled`, timeouts, and approval settings.
Deny lists apply after allowlists. Plugin-provided servers retain plugin-owned
transport declarations; user policy uses `plugins.<plugin>.mcp_servers.<server>`.

**Support:** local desktop, CLI, and IDE share configuration on the same host.
ChatGPT web uses plugin-backed connections and does not read local configuration.
Transport and authentication availability must not be inferred across surfaces.
Portable plugin MCP uses a different JSON format from local TOML or Codex
compatibility `.mcp.json`.
[MCP](https://learn.chatgpt.com/docs/extend/mcp)

**Lifecycle/schema:** `codex mcp add/get/list/remove/login/logout`; HTTP OAuth
only where supported. Local settings are covered by the config schema; portable
plugin MCP has its own schema. Neither schema validates a live server's behavior.
[Developer commands](https://learn.chatgpt.com/docs/developer-commands)

## 6. Hooks

**Format/discovery:** `hooks.json` beside active config layers or inline `[hooks]`;
usual locations are `~/.codex/` and project `.codex/`. Event keys contain matcher
groups with handler arrays. Command handlers use `type`, `command`, `timeout`,
`statusMessage`; MCP handlers use `type = "mcp_tool"`, `server`, `tool`, `input`.

**Events:** `SessionStart`, `SessionEnd`, `SubagentStart`, `SubagentStop`,
`UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`,
`PostCompact`, `Stop`, `Interrupt`.

**Composition/lifecycle:** matching sources accumulate; higher config precedence
does not replace lower hooks. Matching command hooks run concurrently. `/hooks`
reviews and trusts exact definitions; changed non-managed hooks need renewed
trust. Plugin installation alone does not trust hooks. Enabled by default;
`features.hooks = false` disables them. Published event input/output schemas may
lead the release; use the documentation for supported behavior.

**Limits:** MCP hooks require existing connections and exclude `SessionEnd`.
Cloud orchestration has narrower enterprise MCP-hook support and does not load
local/plugin command hooks. Hook failure is not necessarily action denial.
[Hooks](https://learn.chatgpt.com/docs/hooks)

Hooks and multi-agent support are marked stable in the configuration feature
table. This does not establish parity across every host or older installed
version. [Configuration basics](https://learn.chatgpt.com/docs/config-file/config-basic)

## 7. Commands and saved prompts

**Status:** custom prompts are deprecated in favor of skills. Existing CLI/IDE
prompts are top-level Markdown files under `$CODEX_HOME/prompts/`, commonly
`~/.codex/prompts/`; they are personal, not repository-distributed.

**Fields/lifecycle:** optional YAML `description` and `argument-hint`; body
expansion supports `$1`–`$9`, `$ARGUMENTS`, uppercase named variables supplied as
`KEY=value`, and `$$`. Invoke `/prompts:<name>` explicitly. Edit/delete files and
restart or open a new chat; nested folders are not scanned. No dedicated schema
or validation command found. Conversion to skills changes invocation and
argument-expansion semantics, so it is not a simple filename migration.
[Custom prompts](https://learn.chatgpt.com/docs/custom-prompts)

## 8. Execution policies

**Command policy:** experimental `.rules` files under `rules/` beside active
config layers, including trusted project `.codex/rules/` and
`~/.codex/rules/`. `prefix_rule` uses `pattern`, `decision`, optional
`justification`, `match`, and `not_match`. Most restrictive matches win:
`forbidden`, then `prompt`, then `allow`. These control execution outside the
sandbox; they are not prose instructions. No JSON Schema found for this rule
language. [Rules](https://learn.chatgpt.com/docs/agent-configuration/rules)

**Validation:** `codex execpolicy check --rules <file> -- <command>` reports the
decision and matches; repeat `--rules` to combine files.
[Developer commands](https://learn.chatgpt.com/docs/developer-commands)

**Filesystem/network policy:** named `[permissions.<name>]` profiles use
`default_permissions`, `extends`, filesystem access, workspace roots, and network
rules. They do not compose with legacy `sandbox_mode` settings: legacy selection
takes precedence when present. Config schema applies. This enforcement layer
cannot be faithfully replaced by skill prose or a generic agent tool allowlist.
[Permissions](https://learn.chatgpt.com/docs/permissions)

## Schema inventory and evidence limits

Published machine-readable references:

| Artifact                | Published URL                                                                                                                                               | Qualification                                                                                                  |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Codex configuration     | [config-schema.json](https://developers.openai.com/codex/config-schema.json)                                                                                | Opened; redirects to [ChatGPT Learn](https://learn.chatgpt.com/docs/config-schema.json); JSON Schema draft-07. |
| Portable plugin         | [plugin.schema.json](https://agent-plugins.org/schemas/1.0.0/plugin.schema.json)                                                                            | Versioned URL declared in official packaging docs.                                                             |
| Portable plugin MCP     | [mcp.schema.json](https://agent-plugins.org/schemas/1.0.0/mcp.schema.json)                                                                                  | Versioned URL declared in official packaging docs.                                                             |
| Hook event input/output | [Generated schema directory](https://github.com/openai/codex/tree/main/codex-rs/hooks/schema/generated)                                                     | Opened; mutable `main`, not a release pin.                                                                     |
| Example hook input      | [PreToolUse input](https://raw.githubusercontent.com/openai/codex/refs/heads/main/codex-rs/hooks/schema/generated/pre-tool-use.command.input.schema.json)   | Opened JSON Schema source.                                                                                     |
| Example hook output     | [PreToolUse output](https://raw.githubusercontent.com/openai/codex/refs/heads/main/codex-rs/hooks/schema/generated/pre-tool-use.command.output.schema.json) | Opened JSON Schema source.                                                                                     |

The config schema includes `HooksToml` and handler definitions. No separate
standalone `hooks.json`, Codex compatibility-plugin, or marketplace schema was
established. Event wire schemas are distinct: the opened PreToolUse input
requires session, working-directory, model, permission, tool, transcript, and
turn fields; its output supports `hookSpecificOutput`, including
`permissionDecision` and `updatedInput`. These contracts should not be inferred
from another client's similarly named hook events.

Documentation disagreements requiring version-specific verification:

- Packaging recommends desktop installation/testing; the command reference also
  documents stable plugin CLI installation/removal. Do not report UI-only support.
- Skill disablement examples use a `SKILL.md` path, while the configuration
  reference describes a containing-folder path. Test the intended client version.
- Subagent documentation emphasizes standalone TOML; config reference retains
  role declarations. Neither establishes removal of the other mechanism.
- Generated schemas track moving code. Record downloaded hashes and release
  compatibility separately; schema availability does not prove runtime support.

## Adjacent workflows and bundles

Skills encode reusable workflows; hooks add lifecycle execution; plugins
distribute selected capabilities. No independent Codex workflow-file or
upskill-style dependency-bundle schema was established by these sources.

In this checkout, `src/generate/mod.rs` supports Claude, Copilot, and OpenCode,
but no Codex client. `src/pipeline/output.rs` places OpenCode skill output in
`.agents/skills/<name>/SKILL.md`; client-specific rendering occurs before
installation. **Inference:** Codex can discover that already-specialized output.
A future design must address shared-path ownership and semantic compatibility,
not merely add another client enum variant.
