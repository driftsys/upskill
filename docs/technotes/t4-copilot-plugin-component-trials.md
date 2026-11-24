# T4: trials of Copilot CLI plugin components

Date: 2026-11-24. Client: GitHub Copilot CLI 1.0.112 on macOS, model
`claude-haiku-4.5`, 14 model calls. Each result is from one run, except where a
second run is named; model answers can vary between runs. The trials answer the
open points of the
[plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md)
([#623](https://github.com/driftsys/upskill/issues/623)).

## Setup

- Every run used a separate `COPILOT_HOME` directory. The user's `~/.copilot`
  was not written; the checksums of its `config.json` and `mcp-config.json` were
  equal before and after.
- Fixture plugins used the legacy layout (`plugin.json` at the plugin root
  without `$schema`), except where another manifest is named. Each component
  held a marker string that no other text held.
- Plugins were installed from a local marketplace or by a direct install.
  Interactive runs used a pseudo-terminal driver.
- The fixtures and session logs were temporary files and are not kept.

## Results

### 1. A plugin skill as a slash command

- In the interactive client, `/trialcmd` was changed to `/trialfix:trialcmd`,
  the session log recorded `skill.invoked`, and the reply was the marker. The
  menu lists `trialfix:<name>`.
- In prompt mode (`-p`) the client does not process slash commands: the text
  `/trialcmd` reaches the model as a normal message.
- `disable-model-invocation: true`: the skill is absent from the skill list of
  the model, and the user can still call it with the slash command.
- `user-invocable: false`: the skill stays in the skill list of the model, it is
  absent from the menu, and the slash command gives "Unknown command".
- A file `commands/trialc2.md` is listed as a skill. The call
  `/trialfix:trialc2 alpha beta` replaced `$ARGUMENTS` with `alpha beta`. The
  short form `/trialc2` gave "Unknown command"; the short form works for a file
  below `skills/`.

### 2. LSP file name

- A plugin loaded with `--plugin-dir` shows no LSP server. Only an installed
  plugin does.
- `lsp.json` at the plugin root: `copilot lsp list` shows the server with the
  plugin name, and the server process starts at every session start.
- `.github/lsp.json` also loads. With both files in one plugin, only the root
  file loads.
- `lsp-config/servers.json` did not load.
- The file must hold the key `lspServers`: `{"lspServers": {"<name>": {...}}}`.
  Without it the client reports "lspServers must be an object".
- The fixture gave `fileExtensions` as a map from a file extension to a language
  identifier: `{".ext": "language"}`.
- Not conclusive, because tested with `--plugin-dir` only: `.lsp.json`,
  `lsp-config.json`, `lsp-servers.json`, and an `lspServers` key in
  `plugin.json`.

### 3. Rules inside a plugin

- A rule file at `com.github.copilot/rules/trialrule.instructions.md` with
  `name`, `description` and `applyTo: "**"` did not reach the model, with the
  legacy manifest and with an Agent Plugins 1.0 manifest (`$schema` set).
- `copilot instruction list` printed "No instruction sources found." for both,
  and for the plugin directories `instructions/`, `.github/instructions/`,
  `rules/` and `com.github.copilot/instructions/`.
- Control: the same file in `.github/instructions/` of the project is listed.

This agrees with the trial of 2026-10-24 on 1.0.103
([gate 6 Copilot plugin components](gate6-copilot-plugin-components.md)).

### 4. MCP servers of a plugin

- With a `.mcp.json` at the plugin root
  (`{"mcpServers": {"<name>": {"command":
  ..., "args": [...]}}}`),
  `copilot mcp list` showed the server under "Plugin servers", and the model
  called its tool.
- After the plugin was removed, the server was no longer listed.
- The client did not write `mcp-config.json`.
- For a plugin from a local marketplace, `plugin uninstall` printed that the
  plugin was disabled and that nothing was removed; removing the marketplace
  removed it. A hosted marketplace was not tested.

### 5. Hook events that add context text

Each hook printed `{"additionalContext": "<marker>"}`.

| Event                 | Result                                                      |
| --------------------- | ----------------------------------------------------------- |
| `sessionStart`        | The text reached the model.                                 |
| `userPromptSubmitted` | The text reached the model, inside the user message.        |
| `postToolUse`         | The text reached the model, inside the tool result.         |
| `subagentStart`       | The text reached the subagent only, not the parent session. |
| `preToolUse`          | Not confirmed: the log shows no hook output.                |

- The texts of two `sessionStart` hooks arrived joined by a blank line, in the
  configured order (seen in 2 sessions).
- The working directory of a plugin hook is the installed plugin directory.
- The client sets these variables for a hook: `COPILOT_PLUGIN_ROOT`,
  `COPILOT_PLUGIN_DATA`, `COPILOT_PROJECT_DIR`, `COPILOT_HOOK_DIR`,
  `COPILOT_HOOK_DEPTH`, `COPILOT_HOME`, `COPILOT_CLI`, `COPILOT_CLI_PATH`,
  `COPILOT_CLI_BINARY_VERSION`, `COPILOT_CLI_RESOLVED_DIST_DIR`,
  `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA` and `CLAUDE_PROJECT_DIR`.
- The standard input of a `sessionStart` hook is JSON with `sessionId`,
  `timestamp`, `cwd` (the project directory), `source` and `initialPrompt`.

## Not tested

- Claude Code. Its facts in the design come from its documentation and from
  installed plugins.
- A plugin from a hosted marketplace.
- Whether `sessionStart` runs again after compaction.
