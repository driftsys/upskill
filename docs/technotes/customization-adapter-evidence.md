# Customization adapters: capability refresh

Evidence date: **2026-10-14**. Status: informative documentation and source
research. No Gemini, Cursor, or VS Code runtime conformance tests were
performed. This note supplements the
[2026-10-11 research pack](agent-customization/README.md) without rewriting its
historical evidence. The related proposal is the
[adapter model and delivery roadmap](../wip/specs/2026-10-14-customization-adapter-roadmap.md).
Rolling documentation establishes candidate mechanisms, not a tested release
range.

## Evidence limits

The mechanisms below are dated candidates, not verified current support. Refresh
the relevant official documentation and runtime evidence when implementing each
adapter. Historical research remains informative and does not override the
active lifecycle plan or its activation gates.

## Four initial clients

| Client      | Rules independent of team instructions                                                  | Skills and subagents                                    | MCP and permissions                                                    | Plugin lifecycle candidate                         |
| ----------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------------- |
| Claude Code | Native `.claude/rules/`; path conditions available                                      | Native skill directories and Markdown agents            | Native server configuration, tool controls and sandbox settings        | Plugin command hooks, including `SessionStart`     |
| Copilot CLI | Native `.github/copilot-instructions.md` and modular instructions                       | Native skills and custom-agent profiles                 | Native MCP configuration and CLI permission controls                   | Plugin lifecycle hooks, including `sessionStart`   |
| Codex       | `AGENTS.md` exists; additional independent rule-directory discovery was not established | Native skills and TOML custom agents                    | Native configuration, approvals, execution policy and sandbox controls | Plugin `SessionStart` hooks, subject to hook trust |
| OpenCode v2 | `AGENTS.md` exists; configured `instructions` files/globs/URLs currently do not load    | Native skill directories and Markdown/configured agents | Native `mcp.servers` and ordered permission rules                      | Plugin `setup` and session hooks                   |

Claude's [extension overview](https://code.claude.com/docs/en/features-overview)
and [plugin reference](https://code.claude.com/docs/en/plugins-reference)
separate native features from their packaging. Copilot's
[customization comparison](https://docs.github.com/en/copilot/concepts/agents/copilot-cli/comparing-cli-features),
[plugin guide](https://docs.github.com/en/copilot/concepts/agents/about-plugins),
[hook reference](https://docs.github.com/en/copilot/reference/hooks-reference),
and
[tool permission guide](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/allowing-tools)
describe the CLI contracts.

Codex
[instruction discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md)
uses at most one instruction file per directory. Alternative filenames are
fallbacks, not additional imports alongside an existing `AGENTS.md`.
[Custom agents](https://learn.chatgpt.com/docs/agent-configuration/subagents)
and [configuration](https://learn.chatgpt.com/docs/config-file/config-reference)
provide native mechanisms. The
[plugin guide](https://developers.openai.com/plugins/build/plugins) documents
hook discovery but says non-managed plugin hooks are skipped until trusted.
Plugin enablement alone therefore does not prove automatic maintenance will run.

OpenCode v2's
[instructions guide](https://dev.opencode.ai/v2/docs/instructions/) explicitly
says its accepted `instructions` array does not currently resolve files, globs,
or URLs. This differs from v1. Its
[skills](https://dev.opencode.ai/v2/docs/skills/),
[agents](https://dev.opencode.ai/v2/docs/agents/),
[MCP](https://dev.opencode.ai/v2/docs/mcp-servers/),
[permissions](https://dev.opencode.ai/v2/docs/permissions/), and
[plugin API](https://dev.opencode.ai/v2/docs/build/plugins/) are distinct
contracts. The plugin API exposes setup/cleanup and context hooks; it is not the
Claude/Copilot command-hook protocol. Existing upskill v1 behavior must not be
silently relabeled as v2 support.

## Gemini CLI

Gemini CLI supports `GEMINI.md` context, `SKILL.md` skills, local Markdown
subagents, MCP server configuration, hooks, and TOML execution policies. Its
native package is called an **extension**. An extension can expose a context
file through `contextFileName`, or a default `GEMINI.md`, so packaged rules need
not use hook injection. It can also package skills, agents, MCP definitions and
hooks. Extension management includes updates and an auto-update installation
option; changes require a session restart.
[Extension reference](https://geminicli.com/docs/extensions/reference/)

Use the native [skill](https://geminicli.com/docs/cli/skills/) and
[subagent](https://geminicli.com/docs/core/subagents/) contracts where
sufficient. Subagent field names and release support still require verification
because the earlier research found documentation/source disagreements. The
[policy engine](https://geminicli.com/docs/reference/policy-engine/) provides
allow/deny/confirmation decisions and policy tiers. It currently describes
workspace policy loading as disabled. Do not infer project policy support from
the existence of project settings.

The [IDE integration guide](https://geminicli.com/docs/ide-integration/)
describes the Gemini CLI Companion for VS Code-compatible editors and ACP
integrations. The companion supplies editor context and diff review to Gemini
CLI. This is not evidence that Gemini Code Assist, or a Gemini model selected
inside Copilot, loads Gemini CLI extensions.

## Cursor

Cursor has native `.cursor/rules/**/*.mdc` rules, skills, custom subagents, MCP
configuration, hooks, and plugin installation. Native rules support always-on,
path/relevance and manual activation; these are guidance rather than permission
enforcement. Rules do not apply to every product surface, including Cursor Tab.
[Rules](https://cursor.com/docs/rules)

Use native [skills](https://cursor.com/docs/skills) and
[subagents](https://cursor.com/docs/subagents) instead of recreating their
discovery in hooks. [Cursor Plugins](https://cursor.com/docs/plugins) can
package rules, agents, commands and hooks as well as skills and MCP servers.
Portable Agent Plugins package a smaller common surface. Plugin packaging
therefore does not imply active rule injection.

The [hook guide](https://cursor.com/docs/hooks) and
[CLI permissions](https://cursor.com/docs/cli/reference/permissions) establish
different contracts. CLI permission files do not prove editor/cloud policy
parity. Local user customization does not establish cloud availability. Cursor
is a separate client family despite its VS Code-derived editor.

## VS Code changes the target identity

The selected harness owns the lifecycle, tools and hook payloads. VS Code
documents Copilot, Claude and Codex runtimes on Agent Host, plus a separate
Local runtime in the extension host. Its hook UI is preview; provider hook
support can have a different status. A hook visible in the editor is not proof
it is compatible with the selected harness.
[VS Code hook routing](https://code.visualstudio.com/docs/agent-customization/hooks)

| VS Code session                        | Adapter implication                                                                                         |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Copilot on Agent Host                  | Start from the shared Copilot runtime contract; verify discovery and events in the selected VS Code version |
| Claude on Agent Host                   | Use Claude runtime contracts; verify package loading and execution environment                              |
| Codex on Agent Host or Codex extension | Use Codex runtime contracts; verify installed hook trust, paths and supported operations                    |
| Local agent                            | Separate VS Code discovery, tool names, hook formats and approval behavior                                  |
| Cloud session                          | Provider-side environment; local packages and executables are not automatically deployed                    |
| Gemini CLI Companion                   | Gemini CLI remains the runtime; editor context is an additional integration                                 |

The
[migration guide](https://code.visualstudio.com/docs/agent-customization/migrate-customizations)
confirms Local profile/custom-location settings differ from Copilot Agent Host
discovery. It migrates workspace MCP configuration from `.vscode/mcp.json` to
root `.mcp.json`, and user servers to Copilot configuration. Remote/container
user paths belong to the machine running Agent Host. Extension-contributed tools
also depend on the extension running in editor chat. These are concrete reasons
to test both runtime and environment.

Selecting a Claude, OpenAI or Gemini **model** does not itself select that
vendor's agent harness. Model identity and customization ownership are separate.
The
[customization editor](https://code.visualstudio.com/docs/agent-customization/overview)
scopes its view to the selected harness. Native agent plugins and VS Code
extension packages must remain distinct package types.

## Superpowers as an implementation reference

Inspected upstream commit:
[`bb92a77741419a4ab5f06e711a283343f1ada0c3`](https://github.com/obra/superpowers/tree/bb92a77741419a4ab5f06e711a283343f1ada0c3).
Source was read, not installed or executed.

- Separate
  [Claude manifest](https://github.com/obra/superpowers/blob/bb92a77741419a4ab5f06e711a283343f1ada0c3/.claude-plugin/plugin.json)
  and
  [Codex manifest](https://github.com/obra/superpowers/blob/bb92a77741419a4ab5f06e711a283343f1ada0c3/.codex-plugin/plugin.json)
  coexist with shared skills in one repository. Per-client packages do not
  require per-client repositories.
- Its
  [startup script](https://github.com/obra/superpowers/blob/bb92a77741419a4ab5f06e711a283343f1ada0c3/hooks/session-start)
  selects client-specific output envelopes and avoids duplicate context. This is
  a useful adapter pattern; it is not a CLI self-update implementation.
- Its
  [OpenCode adapter](https://github.com/obra/superpowers/blob/bb92a77741419a4ab5f06e711a283343f1ada0c3/.opencode/plugins/superpowers.js)
  separates v1/v2 tool mappings and APIs, caches bootstrap content, and accounts
  for child sessions and compaction. These are useful test cases, not code that
  upskill needs to copy wholesale.
- Its Codex manifest explicitly sets `hooks: {}` at this revision. Superpowers
  therefore cannot serve as evidence that Codex update hooks work.
- Its
  [marketplace descriptor](https://github.com/obra/superpowers/blob/bb92a77741419a4ab5f06e711a283343f1ada0c3/.agents/plugins/marketplace.json)
  illustrates separating discovery metadata from skill implementation.

For upskill, combine these packaging lessons with the official contracts above.
Do not copy bootstrap authority wording, a JavaScript YAML parser, or universal
hook output assumptions into the shared Rust model. No examined reference
establishes a ready-made updater for upskill's binary and owned installations.

## Verification boundary

The earlier session screening observed identical startup rule compliance for
native and plugin delivery with explicit default wording in Claude and Copilot.
That bounded result does not settle update latency, lifecycle coverage or
cross-client instruction authority. Codex/OpenCode rule injection and all six
clients' automatic maintenance remain runtime qualification work.

Every proposed route needs a pinned released-client test. Record documentation
support, emitted-file validation, native discovery and observed behavior as
different evidence levels. The proposed roadmap uses these gates without
claiming that the refreshed capability inventory is already implemented.

## Gate 6 pinned inventory follow-up — 2026-10-19

The [pinned profile and reader inventory](gate6-plugin-profile-inventory.md)
records the corrected independent-plugin rule candidates, all customization
reader/root conditions and campaign pins after architecture PR #334. It
supplements this dated research without rewriting earlier observations. Released
Codex parser/hook source and the integrity-verified OpenCode v2 SDK declarations
were inspected; no native plugin activation is qualified by those inspections.
