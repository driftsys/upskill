# AD-0046 — Plugin MVP: hooks, commands, servers and catalog in `pack` output

Status: **accepted**. Date: 2026-11-24. Resolves
[#623](https://github.com/driftsys/upskill/issues/623). Amends
[AD-0044](0044-plugin-output-of-a-bundle.md). The design is
[the plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md); the
evidence is [technote T4](../technotes/t4-copilot-plugin-component-trials.md).

This record decides planned work. No decision below is implemented when the
record is accepted.

## Context

AD-0044 limits the plugin output of `upskill pack` to skills, resources and
agents, with an `init` command for rules and the update-notice hook. A plugin
cannot hold an always-loaded rule file in Claude Code or in GitHub Copilot CLI:
Claude Code does not load a `CLAUDE.md` at the plugin root, and rule files
inside a Copilot CLI plugin do not reach the model (technote T4, trial 3). A
bundle that needs text in every session, a command that the user calls, an MCP
server or an LSP server is therefore not complete as a plugin. The author also
writes the marketplace files by hand, so they can differ from the packed
plugins.

Stage 2 of the [roadmap](../roadmap.md#stage-2--finish-plugin-distribution)
requires one record that covers all these items before any of them is
implemented.

## Decision

1. **Scope.** `pack` writes every component type that both Claude Code and
   Copilot CLI load from a plugin: skills, resources, agents, hooks, commands,
   MCP servers and LSP servers. `upskill add` and `upskill update` do not
   install hooks, commands or LSP servers. They skip these items of a bundle and
   `main.rs` reports the number of skipped items on stderr. Naming a hook or
   command item directly in `add` is an error that names `upskill pack`.
2. **Rules stay with `init`.** `pack` never renders a `RULE.md` as a hook. The
   `init` command copies the rules of the bundle into the repository with
   `upskill add <source> <bundle> --kind rule --project` and the client flag,
   and copies nothing else. It lists the third-party plugins of the client and
   no longer lists MCP servers. It does not write `AGENTS.md`. Text for every
   session comes only from a hook item that the author writes.
3. **Hook item kind.** The entry file is `HOOK.md`, one for each item directory,
   listed under `items.hooks`. The frontmatter holds an `events` map from a
   portable event name to its settings (`matcher`, `run`, `timeout`), and the
   body holds one `## <event>` section for each event. An event without `run` is
   a context hook; an event with `run` is a command hook. The portable events
   and their client names are in section 1 of the design.
4. **Context hooks.** A context hook is valid only on `session-start`,
   `prompt-submit`, `post-tool` and `subagent-start`, the events where both
   clients accept context text. `pack` writes the text as a static JSON file in
   the output format of each client, and the generated hook prints the file. The
   hook does not need the `upskill` binary.
5. **Merge order.** For each event and each distinct matcher, `pack` joins the
   texts of all hook items into one file and one hook entry. The order is the
   order of the bundle closure, required bundles first, then the order of
   `items.hooks`. A `session-start` context hook without a matcher gets the
   Claude Code matcher `startup|clear|compact`.
6. **Command hooks.** `run` names one file of the item directory. `pack` copies
   the item directory without `HOOK.md` to `hooks/<name>/` and writes one hook
   entry for each command hook, in closure order. The entry sets
   `UPSKILL_CLIENT` to `claude` or `copilot`. The script reads the input and
   writes the output of the client that runs it. The Copilot CLI entry is a
   `bash` command only.
7. **Matcher.** `matcher` is a string, written unchanged for both clients, or a
   map with the keys `claude` and `copilot`. With a map, a client without a key
   gets no hook for that event.
8. **Command item kind.** The entry file is `COMMAND.md`, listed under
   `items.commands`, with the frontmatter `name`, `description` and the optional
   `argument-hint`. `name` equals the directory name. `pack` renders a command
   in both clients as `skills/<name>/SKILL.md` with
   `disable-model-invocation: true`, so only the user calls it, as
   `/<plugin>:<name>`. `pack` renders the `init` command through the same
   renderer. A command and a skill with the same name in one bundle closure, and
   a command named `init` in a bundle with rules, are errors of `pack`.
9. **MCP servers.** `pack` writes the `mcps` of the bundle closure to
   `.mcp.json` at the plugin root of each client output, under the key
   `mcpServers`. The client adds and removes the servers with the plugin.
   `requires-env` is not written.
10. **LSP servers.** A new bundle key `lsps` maps a server name to `command`,
    the optional `args` and `env`, and `extensions`, a map from a file extension
    to a language identifier. `pack` writes `.lsp.json` for Claude Code
    (`extensionToLanguage`) and `lsp.json` with the key `lspServers` for Copilot
    CLI (`fileExtensions`). The plugin does not install the server binary.
11. **Closure.** Each plugin holds the hooks, commands, MCP servers and LSP
    servers of the whole `requires` closure, as it holds the skills and agents
    (AD-0044, Decision 1). Two bundles of one closure that declare the same
    server name with different content are an error of `pack`.
12. **Copilot CLI manifest.** `pack` keeps the legacy `plugin.json` at the
    plugin root, with `hooks.json`, `.mcp.json` and `lsp.json` at the root. It
    does not write the Agent Plugins 1.0 layout.
13. **Catalog.** `pack` accepts several bundle names, or `--all` for every
    bundle of the source registry. With `--catalog <root>`, it writes
    `<root>/.claude-plugin/marketplace.json` and
    `<root>/.github/plugin/marketplace.json` after the plugin directories are in
    place. An entry holds `name`, `source` (a relative path) and `description`,
    and no `version`. An existing file keeps its top-level fields and the
    entries that `pack` did not write. With `--all`, an entry whose `source` is
    inside `--out` and names no packed plugin directory is removed. `pack`
    deletes no plugin directory. A new file needs `--catalog-name` and
    `--catalog-owner`; without them `pack` stops with a usage error (exit 2).
14. **Build location.** `driftsys/metapowers` commits the packed plugins and the
    marketplace files. Its CI runs `upskill pack --all --force --catalog .` with
    a released upskill and fails when `git status --porcelain` reports a changed
    or untracked file.
15. **Experimental status.** Plugin output stays experimental until a smoke test
    with a hosted git marketplace passes in both clients, with one component of
    each type, the `init` command and a generated catalog. The check of a
    command in Copilot CLI needs an interactive run, because prompt mode does
    not process slash commands.

## Amendments to AD-0044

- Decision 1: `pack` also builds hooks, commands, MCP servers and LSP servers,
  accepts several bundles, and writes below `--catalog` as well as `--out`.
- Decision 4: `init` no longer lists MCP servers, and `pack` renders it as a
  command item.
- Decision 5: the hook file also holds the hooks of the hook items. A client
  output without rules has a hook file when the closure has hook items.
- Alternatives, "Deliver rules through a session hook": no longer deferred. It
  is rejected by Decision 2 of this record.
- Alternatives, "Ship an init script": Copilot CLI 1.0.112 sets
  `COPILOT_PLUGIN_ROOT` and `CLAUDE_PLUGIN_ROOT` in the environment of a plugin
  hook (technote T4). No trial has tested the variable in command text, so the
  reason given there is not confirmed for hooks.

## Consequences

- A user who installs two plugins of one marketplace, where one bundle requires
  the other, gets the context text of the required bundle two times in every
  session, and its servers declared two times. An author avoids this by not
  publishing a required bundle as a separate plugin.
- A bundle with rules still needs the `init` step and the `upskill` binary. A
  bundle without rules needs neither.
- A command hook script is not portable by itself: it must read `UPSKILL_CLIENT`
  and handle the input format of each client. A command hook does not run in
  PowerShell in Copilot CLI.
- A `COMMAND.md` and a `SKILL.md` cannot share a directory, because both would
  have the directory name.
- `upskill add` of a bundle installs less than the plugin of the same bundle: no
  hooks, no commands, no LSP servers.
- A plugin directory of a bundle that left the registry stays until the author
  deletes it; the CI check of Decision 14 does not report it.
- The output layout of `init` changes from `commands/init.md` to
  `skills/init/SKILL.md`. Plugins packed with an earlier upskill must be packed
  again.

## Not verified

- A size limit of the context text of a hook in each client.
- Whether Copilot CLI runs `sessionStart` again after compaction.
- The `matcher` field of a Copilot CLI hook entry. The documentation lists it;
  no trial has used it.
- Whether Copilot CLI removes the MCP servers of a plugin from a hosted
  marketplace on uninstall. The smoke test of Decision 15 checks it.
- The Claude Code facts come from its documentation and from installed plugins,
  read on 2026-11-23, not from a trial. The smoke test of Decision 15 checks
  them.

## Alternatives considered

- **Render every rule as a session-start hook.** Rejected: a path-scoped rule
  loses its scope, and a user who also ran `init` gets the text two times.
- **A bundle key that selects rule delivery by `init` or by hook.** Rejected: it
  adds a setting and keeps both code paths.
- **Render a command to `commands/<name>.md`.** Rejected: Claude Code documents
  this format as the older one, Copilot CLI does not document its format, and
  Copilot CLI lists such a file as a skill without its short name (technote T4,
  trial 1).
- **No command kind; a frontmatter field on `SKILL.md`.** Rejected: the roadmap
  fixes `COMMAND.md` as an item kind, and a command has no trigger description
  for the model.
- **One hook entry for each hook item.** Rejected: Claude Code runs the entries
  of one event in parallel and defines no order for their texts.
- **A context hook that calls `upskill` when it runs.** Rejected: static text
  would need the binary on the user's machine.
- **A plugin that holds only the hooks and servers of its own bundle.**
  Rejected: the plugin is incomplete unless the user also installs the plugins
  of the required bundles, and no evidence shows a plugin dependency field in
  Copilot CLI.
- **The Agent Plugins 1.0 layout for Copilot CLI.** Rejected: it does not load
  rules (technote T4, trial 3), so it gives no benefit over the legacy layout
  that the smoke test of 2026-11-19 confirmed.
- **A separate `catalog` command.** Rejected: the marketplace files could then
  differ from the packed plugins.
- **Build the packed plugins only in CI.** Rejected: a marketplace entry with a
  relative path needs the plugin directory in the repository, so CI would need a
  release branch that it writes.
- **Event in the file name, `@event` markers, several frontmatter blocks, or
  hooks inside the bundle file.** Rejected for the reasons in the
  [hook item kind draft](../archive/specs/2026-11-18-hook-item-kind-design.md).
