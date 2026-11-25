# Plugin MVP design

Date: 2026-11-24. Status: **approved by the user on 2026-11-24; stories #632,
#633 and #634 implemented, #631 and #635 to #637 planned**. Decision record:
[AD-0046](../../adr/0046-plugin-mvp-components.md). Task:
[#623](https://github.com/driftsys/upskill/issues/623). Epic:
[#291](https://github.com/driftsys/upskill/issues/291).

This document is the design of stage 2 of the
[roadmap](../../roadmap.md#stage-2--finish-plugin-distribution). It does not
describe released behaviour. The decision record that follows it amends
[AD-0044](../../adr/0044-plugin-output-of-a-bundle.md). It replaces the
[hook item kind draft](../../archive/specs/2026-11-18-hook-item-kind-design.md).

Related records:
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md),
[AD-0041](../../adr/0041-separate-client-representations-as-design-baseline.md),
[AD-0042](../../adr/0042-resteer-customization-delivery.md),
[AD-0010](../../adr/0010-mcp-config-write.md),
[format specification](../../specification/format-spec.md).

## Intent

A bundle author publishes one bundle as a plugin for Claude Code and for GitHub
Copilot CLI. Today `upskill pack` writes skills, resources, agents, an `init`
command and the update-notice hook. A bundle that also needs text in every
session, a command that the user calls, an MCP server or an LSP server is not
complete as a plugin. The author also writes the marketplace files by hand.

After this work, `upskill pack` writes every component type that both clients
load from a plugin, and it writes the marketplace files. A smoke test on a
hosted marketplace then removes the "experimental" status of plugin output.

## Goals

1. A new item kind `hook` (`HOOK.md`) that `pack` renders for both clients.
2. A new item kind `command` (`COMMAND.md`) that `pack` renders for both
   clients.
3. `pack` writes the MCP servers of the bundle into the plugin.
4. A new bundle key `lsps:` that `pack` writes into the plugin.
5. `pack` writes the two marketplace files from the packed bundles.
6. The `init` command of a packed plugin copies only rules.
7. A recorded smoke test on a hosted git marketplace passes in both clients.

## Non-goals

- `upskill add` and `upskill update` do not install hooks, commands or LSP
  servers. A project install of a hook writes into settings files that other
  tools share (roadmap stage 5).
- Component types that only one client loads: Claude Code workflows, output
  styles, themes, monitors and `bin/`.
- A Codex or opencode plugin output.
- A portable input and output contract for command hooks. A hook script reads
  the input format of the client that runs it.

## Anti-goals

- `pack` must not change the meaning of an existing kind. A `RULE.md` is never
  rendered as a hook.
- A generated hook that only adds static text must not need the `upskill`
  binary.
- The marketplace files must not be able to differ from the packed plugins.
- `init` must not write `AGENTS.md`.

## Constraints

- `pack` reads no lockfile and writes only below its output paths (AD-0044,
  Decision 1).
- upskill does not install, register or own a plugin (AD-0042, Decision 8).
- Each client has its own output directory (AD-0041).
- Entry file names are singular (`HOOK.md`, `COMMAND.md`). Bundle keys are
  plural (`items.hooks`, `items.commands`, `lsps`).
- No back-compatibility before 1.0: a changed output layout replaces the earlier
  one.
- Only `main.rs` writes to stdout and stderr.

## Evidence

The component facts were checked on 2026-11-23 against the client documentation
pages and the installed plugins of Claude Code 2.1.308 and Copilot CLI 1.0.112.
The documentation pages were read through a tool that summarizes them, so a
quoted sentence is not verified byte for byte. The trials of section "Trials"
confirm the points that the design depends on.

Claude Code (documentation; installed plugins where named):

- Hooks: `hooks/hooks.json`. A hook adds text to the model's context with the
  JSON output `hookSpecificOutput.additionalContext`. Matching hooks run in
  parallel. The documentation states that the model receives every
  `additionalContext` value of one event; it gives no order.
- `SessionStart` runs again after `/clear`, after compaction, on resume and on
  fork. The matcher values are `startup`, `resume`, `clear`, `compact` and
  `fork`.
- Commands: `commands/*.md` is the older format. The documentation says that
  custom commands were merged into skills and that new plugins should use
  `skills/`. A skill with `disable-model-invocation: true` is called only by the
  user, as `/<plugin>:<name>`. `$ARGUMENTS` holds the arguments.
- MCP: `.mcp.json` at the plugin root. The servers are added and removed with
  the plugin.
- LSP: `.lsp.json` at the plugin root. Required fields: `command`,
  `extensionToLanguage`. The plugin does not install the server binary.
- A `CLAUDE.md` at the plugin root is not loaded. No rule component exists.
- Marketplace: `.claude-plugin/marketplace.json`. Required: `name`,
  `owner.name`, `plugins`; for each entry `name` and `source`. A relative path
  is a valid `source`. When the entry and `plugin.json` both set `version`,
  `plugin.json` is used and `claude plugin validate` warns.

Copilot CLI (documentation; repository trials where named):

- Manifest: the legacy `plugin.json` at the plugin root is accepted (trial of
  2026-11-19, 1.0.112). The Agent Plugins 1.0 format needs an exact `$schema`
  value; only its root manifest has a closed schema. Its Copilot-specific
  components are below `com.github.copilot/`.
- Hooks: `hooks.json` at the plugin root (trial of 2026-11-19). A hook adds text
  with the JSON output `{"additionalContext": "..."}`. The documentation lists
  `sessionStart`, `subagentStart`, `notification`, `postToolUse` and
  `postToolUseFailure` as events that accept it. Hooks of the same type run in
  the configured order, and the `sessionStart` texts are joined with a blank
  line. The text reaches the model and is not shown to the user.
- Commands: the documentation names the directory and gives no file format. In
  the trial of 2026-11-19, `commands/init.md` was listed as a skill named
  `init`.
- MCP: `.mcp.json` at the plugin root (legacy format). The documentation does
  not say that the servers are removed with the plugin.
- LSP: the documentation names both `lsp.json` and `lsp-config/servers.json` for
  a legacy plugin. Required field: `fileExtensions`.
- Rules: `com.github.copilot/rules/` is a documented path. The trial of
  2026-10-24 (1.0.103) found that these rules do not reach the model.
- Marketplace: `.github/plugin/marketplace.json`, with a relative path as
  `source` (trial of 2026-11-19).

## Trials

Five trials on Copilot CLI 1.0.112 answered the points that the design depends
on. Their record is
[technote T4](../../technotes/t4-copilot-plugin-component-trials.md).

| # | Question                                                            | Result                                                                                                                                                                         |
| - | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1 | Does the user call a plugin skill as a slash command?               | Yes, as `/<plugin>:<name>`, in the interactive client. `disable-model-invocation: true` stops the model from calling it. Prompt mode (`-p`) does not process slash commands.   |
| 2 | Which LSP file name does a legacy plugin load?                      | `lsp.json` at the plugin root, with the top-level key `lspServers`. `lsp-config/servers.json` does not load.                                                                   |
| 3 | Do rules below `com.github.copilot/rules/` reach the model?         | No, with the legacy manifest and with the Agent Plugins 1.0 manifest.                                                                                                          |
| 4 | Are the servers of a plugin `.mcp.json` added and removed with it?  | Yes, when the plugin is removed. With a local marketplace, `plugin uninstall` only disables the plugin; removing the marketplace removes it. A hosted marketplace is untested. |
| 5 | Which hook events other than `sessionStart` add text for the model? | `userPromptSubmitted` and `postToolUse`. `subagentStart` adds it for the subagent only. `preToolUse` is not confirmed.                                                         |

## Approaches

Each point lists the selected approach first. On 2026-11-24 the user approved
the selected approaches, which Claude proposed.

### Rule delivery

- **Selected: rules stay with `init`.** `init` copies the rules into the
  repository, where the path scope of a rule works. Text for every session comes
  only from a `HOOK.md` that the author writes.
- Render every rule as a session-start hook. Rejected: a path-scoped rule loses
  its scope, and a user who also ran `init` gets the text two times.
- A bundle key that selects one of the two. Rejected: it adds a setting and
  keeps both code paths.

### Command rendering

- **Selected: a `COMMAND.md` is rendered as a skill that only the user calls.**
  Both clients document the skill format. Claude Code states that skills replace
  commands.
- Render to `commands/<name>.md`. Rejected: Claude Code calls this format the
  older one, and Copilot CLI does not document its format. In trial 1, Copilot
  CLI listed a `commands/` file as a skill and did not accept its short name.
- No command kind; a frontmatter field on `SKILL.md`. Rejected: the roadmap
  fixes `COMMAND.md` as an item kind, and a command has no trigger description
  for the model.

### Context hook delivery and merge order

- **Selected: `pack` writes the text as a static JSON file, and merges the text
  of all hooks of one event.** The generated hook prints the file. The order is
  the order of the bundle closure, required bundles first, then the order of
  `items.hooks`.
- One hook entry for each item. Rejected: Claude Code runs the entries in
  parallel and defines no order for their texts.
- A hook that calls `upskill` when it runs. Rejected: static text would need the
  binary on the user's machine.

### Copilot manifest

- **Selected: keep the legacy `plugin.json` at the plugin root.** Every
  component of the 2026-11-19 smoke test loaded from it.
- Agent Plugins 1.0 layout. Rejected for the MVP: its one trial failed for
  rules, and it moves hooks, agents and commands below `com.github.copilot/`.
  Trial 3 confirmed that it does not load rules, so it gives no benefit.

### Catalog generation

- **Selected: `pack` writes the marketplace files in the same run as the
  plugins.** A file that exists keeps the entries that `pack` did not write.
- A separate `catalog` command. Rejected: the files can then differ from the
  packed plugins.

### Closure of a plugin

- **Selected: each plugin holds the hooks, commands and servers of the whole
  `requires` closure**, as it holds the skills and agents today. Each plugin is
  complete alone in both clients. A user who installs the plugins of a bundle
  and of a bundle that it requires gets the context text and the servers of the
  required bundle two times.
- A plugin holds only the hooks and servers of its own bundle. Rejected: the
  plugin is incomplete unless the user installs the other plugins, and no
  evidence shows a plugin dependency field in Copilot CLI.

### Build location

- **Selected: the packed plugins are committed in `driftsys/metapowers`, and its
  CI fails when a new `pack` run changes them.** A marketplace entry with a
  relative path needs the plugin directory in the repository.
- Build only in CI. Rejected: it needs a release branch that CI writes.

## Design

### 1. Hook item kind

The source convention is the one of the
[hook item kind draft](../../archive/specs/2026-11-18-hook-item-kind-design.md),
sections 1 to 4: one `HOOK.md` for each item directory, an `events` map in the
frontmatter, one `## <event>` section for each event. The points below complete
or change it.

**Portable events.** Each name maps to one event of each client:

| Portable             | Claude Code         | Copilot CLI           |
| -------------------- | ------------------- | --------------------- |
| `session-start`      | `SessionStart`      | `sessionStart`        |
| `session-end`        | `SessionEnd`        | `sessionEnd`          |
| `prompt-submit`      | `UserPromptSubmit`  | `userPromptSubmitted` |
| `pre-tool`           | `PreToolUse`        | `preToolUse`          |
| `post-tool`          | `PostToolUse`       | `postToolUse`         |
| `permission-request` | `PermissionRequest` | `permissionRequest`   |
| `stop`               | `Stop`              | `agentStop`           |
| `subagent-start`     | `SubagentStart`     | `subagentStart`       |
| `subagent-stop`      | `SubagentStop`      | `subagentStop`        |
| `pre-compact`        | `PreCompact`        | `preCompact`          |

**Context hooks.** An event without `run` is a context hook. A context hook is
valid only on an event where both clients accept context text: `session-start`,
`prompt-submit`, `post-tool` and `subagent-start` (Claude Code documentation;
trial 5 for Copilot CLI). The text of a `subagent-start` hook reaches the
subagent, not the parent session. `upskill lint` reports an error for a context
hook on another event.

**Matcher.** `matcher` is a string or a map with the keys `claude` and
`copilot`. A string is written unchanged for both clients. The Copilot CLI
documentation lists `matcher` as a field of a hook entry, next to `bash` and
`timeoutSec`; no trial has used it. With a map, a client without a key gets no
hook for that event. Tool names differ between the clients, so a tool matcher
normally needs the map form. A `session-start` context hook without `matcher`
gets the Claude Code matcher `startup|clear|compact`, because `/clear` and
compaction remove the text. It gets no matcher in Copilot CLI.

**Generated files for context hooks.** For each event and each distinct matcher,
`pack` joins the texts with one blank line and writes one file:

- Claude Code: `hooks/context/<event>.json` with
  `{"hookSpecificOutput": {"hookEventName": "<Event>", "additionalContext": "<text>"}}`.
- Copilot CLI: `hooks/context/<event>.json` with
  `{"additionalContext": "<text>"}`.

A second matcher group of the same event gets the file name `<event>-2.json`,
and so on in closure order. The hook entry prints the file: `cat` with the
plugin root variable in Claude Code (`${CLAUDE_PLUGIN_ROOT}`); in Copilot CLI a
`bash` command and a `powershell` command that read the file by a path relative
to the plugin root, which is the working directory of a plugin hook (trial 5).
The text passes through the existing `@client` directive processing before it is
written.

**Command hooks.** An event with `run` is a command hook. `run` is the name of
one file in the item directory, without arguments. `pack` copies the item
directory without `HOOK.md` to `hooks/<item>/`, with the file modes, and writes
one hook entry for each command hook, in closure order. The Copilot CLI entry
has a `bash` command only: a command hook does not run in PowerShell. The entry
sets the environment variable `UPSKILL_CLIENT` to `claude` or `copilot` and runs
the script. `timeout` is in seconds (`timeout` in Claude Code, `timeoutSec` in
Copilot CLI). The script reads the input and writes the output of the client
that runs it.

**Name.** The `name` of a hook may differ from its directory name, as for a
rule. `<item>` in an output path is the `name`.

**Hook file.** `pack` writes one hook file for each client (`hooks/hooks.json`
for Claude Code, `hooks.json` for Copilot CLI). It holds the update-notice hook
when the client output has rules (AD-0044, Decision 5), then the context hooks,
then the command hooks.

**Other commands.** `upskill new hook <name>` writes a `HOOK.md` with one
`session-start` context hook. `upskill fmt` orders the frontmatter keys as for
the other kinds. `upskill add` and `upskill update` skip the hook items of a
bundle, and `main.rs` prints one line on stderr with the number of skipped items
and the reason. `upskill add <source> <name>` with the name of a hook item is an
error (exit 1) that names `upskill pack`. `--kind` keeps its three values.

**Kind lists.** The code names the three entry files in closed lists
(`src/pipeline/discovery.rs`, `src/pipeline/mod.rs`, `src/scaffold.rs`) and so
does the content format (`docs/topics/0001-content-format.md`). The stories of
the two new kinds extend each list.

### 2. Command item kind

```markdown
---
name: release-notes
description: Write the release notes of the current branch
argument-hint: "[version]"
---

Write the release notes for version $ARGUMENTS.
```

- The entry file is `COMMAND.md`. A bundle lists commands under
  `items.commands`. Co-located resources are copied as for a skill.
- `name` equals the directory name, as for a skill. A `COMMAND.md` and a
  `SKILL.md` in one directory would have the same name, so `upskill lint`
  reports an error for that directory.
- Every co-located resource of the directory is copied into the skill directory
  of the command, as for a skill.
- Frontmatter: `name` and `description` are required. `argument-hint` is
  optional. `$ARGUMENTS` in the body stands for the text that the user gives.
- Claude Code output: `skills/<name>/SKILL.md` with the frontmatter `name`,
  `description`, `argument-hint` and `disable-model-invocation: true`.
- Copilot CLI output: `skills/<name>/SKILL.md` with the frontmatter `name`,
  `description` and `disable-model-invocation: true` (trial 1). The user calls
  it as `/<plugin>:<name>`.
- A command and a skill with the same name in one bundle closure are an error of
  `pack`.
- `pack` writes the `init` command through the same renderer. A bundle that has
  rules and a command named `init` is an error of `pack`.
- `upskill lint`, `upskill fmt` and `upskill new command <name>` support the
  kind. `upskill add` and `upskill update` skip command items in the same way as
  hook items.

### 3. Init command

- `init` exists in a client output only when the bundle closure has rules for
  that client (AD-0044, Decision 5, unchanged).
- It runs `upskill add <source> <bundle> --kind rule --project` with the client
  flag. It copies nothing else.
- It lists the third-party plugins of the client, as today. It no longer lists
  MCP servers, because the plugin holds them.
- It does not write `AGENTS.md`. `upskill add` creates a `CLAUDE.md` that holds
  `@AGENTS.md` only when no `CLAUDE.md` exists (existing behaviour).

### 4. MCP servers

- `pack` writes `.mcp.json` at the plugin root of each client output. It holds
  the `mcps` of the bundle closure under the key `mcpServers`.
- A local server is written as `command`, `args`, `env`. A remote server is
  written as `type`, `url`, `headers`. The private functions
  `mcp_servers_local_value` and `mcp_servers_remote_value` of `src/ancillary.rs`
  build these objects today; the story makes them usable from `pack`.
- Two bundles of one closure that declare the same server name with different
  content are an error of `pack`.
- An output without MCP servers has no `.mcp.json`.
- `requires-env` is not written into the plugin. The plugin description is the
  place where the author names required variables.

### 5. LSP servers

A new bundle key:

```yaml
lsps:
  rust-analyzer:
    command: rust-analyzer
    args: []
    extensions:
      .rs: rust
```

- `command` and `extensions` are required. `extensions` maps a file extension
  that starts with a dot to a language identifier. `args` and `env` are
  optional.
- Claude Code output: `.lsp.json` at the plugin root, one key for each server,
  with `command`, `args`, `env` and `extensionToLanguage`.
- Copilot CLI output: `lsp.json` at the plugin root, with the servers below the
  key `lspServers`, each with `command`, `args`, `env` and `fileExtensions` (the
  same map as `extensions`; trial 2).
- The plugin does not install the server binary.
- `upskill add` and `upskill update` do not configure LSP servers. They report
  the skipped servers in the same line as skipped hooks and commands.
- Two bundles of one closure that declare the same server name with different
  content are an error of `pack`.

### 6. Catalog generation

New command forms:

```text
upskill pack <bundle>... --source <src> --out <dir> [--catalog <root>]
upskill pack --all       --source <src> --out <dir> [--catalog <root>]
```

- `pack` accepts several bundle names. `--all` selects every bundle of the
  source registry.
- `--catalog <root>` names the root directory of the marketplace repository.
  `--out` must be inside it. `pack` writes
  `<root>/.claude-plugin/marketplace.json` for the Claude Code outputs and
  `<root>/.github/plugin/marketplace.json` for the Copilot CLI outputs. A client
  that the client flags exclude gets no file.
- An entry holds `name` (the bundle name), `source` (the relative path from the
  root to the plugin directory, with the prefix `./`) and `description`. It
  holds no `version`: the plugin manifest holds the version.
- When a marketplace file exists, `pack` keeps its top-level fields and every
  entry whose name is not a packed bundle, and replaces the entries of the
  packed bundles. With `--all`, it also removes an entry whose `source` is
  inside `--out` and names no packed plugin directory.
- `pack` deletes no plugin directory. A directory of a bundle that left the
  registry stays until the author deletes it.
- When a marketplace file does not exist, `--catalog-name` and `--catalog-owner`
  are required, and `pack` stops with a usage error (exit 2) without them. They
  give `name` and `owner.name`, which both marketplace formats require.
- Entries are sorted by name. The marketplace files are written after every
  plugin directory is in place.

### 7. Build location

- `driftsys/metapowers` commits the packed plugins and the marketplace files.
- Its CI runs `upskill pack --all --force --catalog .` with a released upskill
  and fails when `git status --porcelain` reports a changed or untracked file.

### 8. Hosted-marketplace smoke test

- A fixture bundle holds one rule, one skill, one agent, one hook with a
  `session-start` context event and a command hook, one command, one local MCP
  server and one LSP server.
- `pack --catalog` writes its plugins and marketplace files, and the result is
  pushed to a hosted git repository of the `driftsys` organization. The story of
  the smoke test selects the repository.
- In each client, the test installs the plugin from the hosted marketplace and
  observes each component: the skill and the agent are listed, the context text
  reaches the model, the command hook runs, the user calls the command, the MCP
  server is listed, the LSP server starts, `init` installs the rule with a
  released upskill, and uninstalling the plugin removes the MCP server. The
  command check in Copilot CLI needs an interactive run, because prompt mode
  does not process slash commands (trial 1).
- The record is a technote. When every check passes, the decision record removes
  the "experimental" status of plugin output (AD-0044, Consequences).

## Amendments to AD-0044

- Decision 1: `pack` also builds hooks, commands, MCP servers and LSP servers,
  accepts several bundles, and writes below `--catalog` as well as `--out`.
- Decision 4: `init` no longer lists MCP servers. `pack` renders it as a command
  item, so its output path changes from `commands/init.md` to
  `skills/init/SKILL.md`.
- Decision 5: the hook file also holds the hooks of the hook items. An output
  without rules has a hook file when the closure has hook items.
- Decision 8: `pack` resolves each named bundle, or with `--all` every bundle,
  in the current directory.
- Alternatives: "Deliver rules through a session hook" stays rejected, now with
  the reason of section "Rule delivery".
- Alternatives, "Ship an init script": Copilot CLI 1.0.112 sets
  `COPILOT_PLUGIN_ROOT` and `CLAUDE_PLUGIN_ROOT` for a plugin hook (technote
  T4). AD-0044 states that Copilot CLI did not expand the Claude plugin-root
  variable in the prototype. No trial has tested the variable in command text.

## Left to the builder

- The module layout of the two new kinds in `model/`, `parse/` and `generate/`.
- The wording of lint messages and of the stderr line for skipped items.
- The exact shell text of the generated hook commands.
- The format of the file name for a second matcher group, if the one above
  collides with an event name.

## Not verified

- A size limit of `additionalContext` in each client.
- Whether Copilot CLI runs `sessionStart` again after compaction.
- The tool names that a Copilot CLI `matcher` accepts.
- Context text from a `pre-tool` hook in Copilot CLI (trial 5 did not confirm
  it). `pre-tool` is not in the context set.
- Whether a Copilot CLI plugin from a hosted marketplace is removed with its MCP
  servers on uninstall. The smoke test checks it.

## Stories

One story for each row, under #291:

1. [#631](https://github.com/driftsys/upskill/issues/631): hook item kind in
   `lint`, `fmt`, `new` and `pack`.
2. [#632](https://github.com/driftsys/upskill/issues/632): command item kind in
   `lint`, `fmt`, `new` and `pack`, with `init` rendered through it.
3. [#633](https://github.com/driftsys/upskill/issues/633): MCP servers in `pack`
   output; `init` stops listing them.
4. [#634](https://github.com/driftsys/upskill/issues/634): bundle key `lsps` and
   LSP servers in `pack` output.
5. [#635](https://github.com/driftsys/upskill/issues/635): several bundles,
   `--all` and `--catalog` in `pack`.
6. [#636](https://github.com/driftsys/upskill/issues/636): packed plugins and
   the CI check in `driftsys/metapowers`.
7. [#637](https://github.com/driftsys/upskill/issues/637): hosted-marketplace
   smoke test and the removal of the "experimental" status.
