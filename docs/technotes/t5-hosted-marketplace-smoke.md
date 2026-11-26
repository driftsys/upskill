# T5: smoke test of plugin output with a hosted git marketplace

Status: prepared, not run. Every result below is a `TODO` placeholder. Date of
the run: TODO. Binary: released `upskill` TODO (version, `upskill --version`).
Clients: Claude Code TODO (`claude --version`) and GitHub Copilot CLI TODO
(`copilot --version`), on TODO (OS and architecture). Model of each client:
TODO.

Issue [#637](https://github.com/driftsys/upskill/issues/637); decision:
[AD-0046](../adr/0046-plugin-mvp-components.md), Decision 15; design:
[plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md), section 8.
The run follows the earlier local-marketplace test
[T3](t3-pack-marketplace-smoke.md). Each result is from one run, except where a
second run is named; model answers can vary between runs. A model's statement
about its own context is weak evidence, so each check names a second source (a
marker file, a command output or a log) where one exists.

When every check of the "Required" table passes in both clients (the "Points of
AD-0046" table may end with a documented skip or a recorded negative result),
the decision records stop calling plugin output experimental (see "After the
run").

## Fixture

The fixture registry is `tests/fixtures/plugin-smoke/registry/`. It is separate
from `tests/fixtures/pack/registry/`, whose packed output golden tests compare
and whose bundles `pack --all` would pick up. The bundle `smoke` holds:

| Component | Item or key               | Observable marker                                                            | Needs at run time                 |
| --------- | ------------------------- | ---------------------------------------------------------------------------- | --------------------------------- |
| Rule      | smoke-rule                | Answer `SMOKE_RULE_MARKER` when asked for the rule marker                    | Released `upskill` on PATH (init) |
| Skill     | smoke-skill               | Answer `SMOKE_SKILL_MARKER`                                                  | Nothing                           |
| Agent     | smoke-agent               | Answer `SMOKE_AGENT_MARKER`                                                  | Nothing                           |
| Hooks     | smoke-hook, smoke-counter | Context `SMOKE_CONTEXT_MARKER` (session-start); marker file and counter file | POSIX `sh`, `date`                |
| Command   | smoke-command             | Answer `SMOKE_COMMAND_MARKER`                                                | Nothing                           |
| MCP       | smoke-mcp                 | Tool `smoke_marker` returns `SMOKE_MCP_MARKER`                               | `python3` 3.x on PATH, no network |
| LSP       | smoke-lsp                 | Marker file `${TMPDIR:-/tmp}/upskill-smoke-lsp-marker`                       | `python3` 3.x on PATH, no network |

- The command hook of `smoke-hook` runs `marker.sh` on `prompt-submit`. It
  appends `<client> <UTC time>` to `${TMPDIR:-/tmp}/upskill-smoke-hook-marker`
  and prints nothing. The client name comes from `UPSKILL_CLIENT`.
- The command hook of `smoke-counter` runs `counter.sh` on `session-start`,
  without a matcher. It appends one line to
  `${TMPDIR:-/tmp}/upskill-smoke-session-counter`. The line count is a signal
  outside the conversation: it shows how often the client starts a session or
  runs its session-start hooks, which the model's answer cannot show after
  `/clear` or compaction. It is secondary evidence: the primary evidence is the
  client log of the generated context command. The counter is a separate hook
  item because an event key appears once in an item and the context event
  already uses `session-start`. The format allows a `matcher` on a command
  event, but a `matcher` map cannot say "no matcher" for Copilot CLI, so the
  counter has none. Claude Code therefore also runs it on `resume`, which the
  context entry (`startup|clear|compact`) does not match.
- The MCP and LSP servers are inline `python3 -c` programs, because a plugin
  copies no other files for these keys. The LSP server appends `started` to its
  marker file when the client starts it, advertises `documentSymbolProvider`,
  appends `documentSymbol` and answers `[]` to `textDocument/documentSymbol`,
  and answers every other request with a `null` result. The LSP extension is
  `.smoke`, the language identifier `smokelang`.
- Locally, before the run:
  `upskill lint --strict
  tests/fixtures/plugin-smoke/registry/items` reports 6
  files checked and 0 findings, and linting the bundle file reports 0 findings.
  Linting the registry root checks only the bundle file (tracked separately as
  issue #663). The MCP and LSP programs from the packed JSON answered scripted
  exchanges (MCP: initialize and tool call; LSP: initialize, `documentSymbol`,
  an unknown request and shutdown) with exit code 0, and both hook scripts wrote
  their lines.

## Preparation

1. Create the hosted repository `driftsys/<repo>` (private is enough if both
   clients can clone it with the git configuration of the machine; the
   marketplace add commands below use the `owner/repo` form, which needs access
   through git or the client's GitHub login).
2. Copy the registry into a working directory and pack into it. `pack` runs in
   the registry root. `--source` must name the hosted repository, because the
   generated `init` command installs rules from that source. The registry and
   the packed output share one repository, as `driftsys/metapowers` does.

   ```sh
   work=$(mktemp -d)
   cp -R tests/fixtures/plugin-smoke/registry/. "$work"
   cd "$work"
   upskill pack --all --source driftsys/<repo> --out "$work/plugins" \
     --catalog "$work" --catalog-name smoke-market --catalog-owner driftsys
   ```

   Expected output: one `Wrote` line for each of `plugins/smoke-claude` and
   `plugins/smoke-copilot`, then one for each marketplace file.

3. Publish it. Run these commands only when you intend to create the repository:

   ```sh
   git init -b main && git add -A && git commit -m "chore: smoke test fixture"
   git ls-files
   git remote add origin <git url of driftsys/<repo>> && git push -u origin main
   ```

4. Use a clean state for each client so the run does not read earlier plugins: a
   new empty project directory that is a git repository (`init` needs one), and
   for Copilot CLI a separate `COPILOT_HOME`
   (`export COPILOT_HOME=$(mktemp -d)`). For Claude Code use scope `local` in
   that project. Remove both marker files before each client:
   `rm -f "${TMPDIR:-/tmp}"/upskill-smoke-*`. Note whether the client sees the
   same `TMPDIR` as your shell (`echo $TMPDIR` in the session shell tool).
5. Record the checksums of `~/.claude/settings.json` and
   `~/.copilot/config.json` before and after the run, as T3 did: TODO.

## Required checks

Each row: the action, the expected result, the observed result and a pass/fail
box. Run the Claude Code column first, then Copilot CLI, each in its own fresh
project. Replace `<repo>` with the repository name.

### Claude Code

| #   | Action                                                                                                                                                                                                                     | Expected                                                                                                                                               | Observed | Pass                |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- | ------------------- |
| C1  | `claude plugin marketplace add driftsys/<repo>`                                                                                                                                                                            | Marketplace `smoke-market` is added from the hosted repository                                                                                         | TODO     | [ ] pass / [ ] fail |
| C2  | `claude plugin marketplace list`                                                                                                                                                                                           | Lists `smoke-market` with the GitHub source                                                                                                            | TODO     | [ ] pass / [ ] fail |
| C3  | `claude plugin install smoke@smoke-market --scope local`                                                                                                                                                                   | Plugin `smoke` installs; no error about the manifest, hooks, MCP or LSP files                                                                          | TODO     | [ ] pass / [ ] fail |
| C4  | Start `claude` in the project. Run `/plugin`, `/skills`, `/agents`                                                                                                                                                         | Plugin `smoke` is enabled; skills `smoke:smoke-skill`, `smoke:smoke-command`, `smoke:init`; agent `smoke:smoke-agent`                                  | TODO     | [ ] pass / [ ] fail |
| C5  | Ask: `Use the smoke-skill skill.`                                                                                                                                                                                          | Reply holds `SMOKE_SKILL_MARKER`                                                                                                                       | TODO     | [ ] pass / [ ] fail |
| C6  | Ask: `Use the smoke-agent subagent.`                                                                                                                                                                                       | The subagent runs and the reply holds `SMOKE_AGENT_MARKER`                                                                                             | TODO     | [ ] pass / [ ] fail |
| C7  | In a new session ask: `What is the context marker?` (do not name any file)                                                                                                                                                 | Reply holds `SMOKE_CONTEXT_MARKER`; the text came from the session-start hook                                                                          | TODO     | [ ] pass / [ ] fail |
| C8  | After the first prompt of C7, run `cat "${TMPDIR:-/tmp}"/upskill-smoke-hook-marker` in another terminal                                                                                                                    | At least one line `claude <UTC time>` in the client's TMPDIR; one more line for each later prompt                                                      | TODO     | [ ] pass / [ ] fail |
| C9  | Type `/smoke:smoke-command`                                                                                                                                                                                                | The command runs only on the user's call and the reply holds `SMOKE_COMMAND_MARKER`                                                                    | TODO     | [ ] pass / [ ] fail |
| C10 | Ask the model to call the tool `smoke_marker` without typing a command; then `claude mcp list` and `/mcp`                                                                                                                  | Server `smoke-mcp` (plugin) is listed as connected; the transcript shows a call of `smoke_marker` and the returned `SMOKE_MCP_MARKER`                  | TODO     | [ ] pass / [ ] fail |
| C11 | `touch sample.smoke`; ask Claude to list the symbols of `sample.smoke` (this calls `textDocument/documentSymbol`; reading the file alone does not start the server); then `cat "${TMPDIR:-/tmp}"/upskill-smoke-lsp-marker` | Marker file holds `started` and `documentSymbol`; `/plugin` shows no LSP error for `smoke-lsp`                                                         | TODO     | [ ] pass / [ ] fail |
| C12 | Run `/smoke:init` with a released `upskill` on PATH                                                                                                                                                                        | Runs `upskill add driftsys/<repo> smoke --kind rule --project --claude`; `.claude/rules/smoke-rule.md` exists; nothing else of the bundle is installed | TODO     | [ ] pass / [ ] fail |
| C13 | Start a new session and ask: `What is the rule marker?`                                                                                                                                                                    | Reply holds `SMOKE_RULE_MARKER`                                                                                                                        | TODO     | [ ] pass / [ ] fail |
| C14 | `claude plugin uninstall smoke@smoke-market --scope local`, then `claude mcp list`                                                                                                                                         | Server `smoke-mcp` is no longer listed; skills and agent are gone                                                                                      | TODO     | [ ] pass / [ ] fail |
| C15 | `claude plugin marketplace remove smoke-market`                                                                                                                                                                            | Marketplace removed; user settings checksums unchanged (record leftovers as in T3)                                                                     | TODO     | [ ] pass / [ ] fail |

### Copilot CLI

Use the separate `COPILOT_HOME` of the preparation step.

| #   | Action                                                                                                                             | Expected                                                                                                                                                                    | Observed | Pass                |
| --- | ---------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------------------- |
| P1  | `copilot plugin marketplace add driftsys/<repo>`                                                                                   | Marketplace `smoke-market` is added; the client reads `.github/plugin/marketplace.json`                                                                                     | TODO     | [ ] pass / [ ] fail |
| P2  | `copilot plugin marketplace list` and `copilot plugin marketplace browse smoke-market`                                             | Lists the marketplace and plugin `smoke`                                                                                                                                    | TODO     | [ ] pass / [ ] fail |
| P3  | `copilot plugin install smoke@smoke-market`                                                                                        | Plugin installs from `plugins/smoke-copilot`; the message counts the skills                                                                                                 | TODO     | [ ] pass / [ ] fail |
| P4  | Start `copilot` interactively in the project. Run `/skills list` and `/agent`; ask the model to list its agents                    | Skills `smoke-skill`, `smoke-command`, `init` and agent `smoke:smoke-agent` are listed                                                                                      | TODO     | [ ] pass / [ ] fail |
| P5  | Ask: `Use the smoke-skill skill.`                                                                                                  | Reply holds `SMOKE_SKILL_MARKER`                                                                                                                                            | TODO     | [ ] pass / [ ] fail |
| P6  | Ask: `Use the smoke-agent agent.`                                                                                                  | The agent runs and the reply holds `SMOKE_AGENT_MARKER`                                                                                                                     | TODO     | [ ] pass / [ ] fail |
| P7  | In a new session ask: `What is the context marker?` (do not name any file)                                                         | Reply holds `SMOKE_CONTEXT_MARKER`; the text came from the sessionStart hook                                                                                                | TODO     | [ ] pass / [ ] fail |
| P8  | After the first prompt of P7, `cat "${TMPDIR:-/tmp}"/upskill-smoke-hook-marker`                                                    | At least one line `copilot <UTC time>` in the client's TMPDIR; one more line for each later prompt                                                                          | TODO     | [ ] pass / [ ] fail |
| P9  | **Interactive run only** (prompt mode `-p` does not process slash commands). In the interactive client type `/smoke:smoke-command` | The menu shows `smoke:smoke-command`; the reply holds `SMOKE_COMMAND_MARKER`                                                                                                | TODO     | [ ] pass / [ ] fail |
| P10 | Ask the model to call the tool `smoke_marker`; then `copilot mcp list` (or `/mcp` in the session)                                  | Server `smoke-mcp` is listed as a plugin server; the transcript shows a call of `smoke_marker` and the returned `SMOKE_MCP_MARKER`                                          | TODO     | [ ] pass / [ ] fail |
| P11 | `touch sample.smoke`; ask the model to list the symbols of `sample.smoke`; then `cat "${TMPDIR:-/tmp}"/upskill-smoke-lsp-marker`   | Marker file holds `started` and, if the client sends the request, `documentSymbol`; no error from the client. Record whether the client starts the server at all            | TODO     | [ ] pass / [ ] fail |
| P12 | Run the `init` skill (`/smoke:init`, or the name the menu shows) with a released `upskill` on PATH                                 | Runs `upskill add driftsys/<repo> smoke --kind rule --project --copilot`; `.github/instructions/smoke-rule.instructions.md` exists; nothing else of the bundle is installed | TODO     | [ ] pass / [ ] fail |
| P13 | `copilot plugin uninstall smoke` (or `smoke@smoke-market` if the client asks for the marketplace form), then `copilot mcp list`    | Server `smoke-mcp` is no longer listed; skills and agent are gone                                                                                                           | TODO     | [ ] pass / [ ] fail |
| P14 | `copilot plugin marketplace remove smoke-market`                                                                                   | Marketplace removed; `config.json` checksum of the real home unchanged                                                                                                      | TODO     | [ ] pass / [ ] fail |

Run the C12 and P12 `init` commands exactly as generated. Copy the command from
`plugins/smoke-<client>/skills/init/SKILL.md` into the observed column. In a
non-interactive Copilot CLI run T3 needed `--allow-tool='shell(upskill:*)'` and
`--allow-all-paths`; an interactive run asks for permission instead.

## Points of AD-0046 that the run can check

| #  | Point (AD-0046, "Not verified")                                                       | Action                                                                                                                                                                                                                                                                   | Expected or recorded result                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Observed | Pass                               |
| -- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------- |
| V1 | Copilot CLI removes the MCP server of a hosted-marketplace plugin on uninstall        | P13                                                                                                                                                                                                                                                                      | `smoke-mcp` absent from `copilot mcp list`; also record whether `$COPILOT_HOME/mcp-config.json` exists (T4 saw the client not write it, so its content proves nothing)                                                                                                                                                                                                                                                                                                                                                                                                       | TODO     | [ ] pass / [ ] fail                |
| V2 | Claude Code facts that come from documentation, not from a trial                      | C3 to C14: record each of the following as it is observed. The hooks file is read at `hooks/hooks.json`; `${CLAUDE_PLUGIN_ROOT}` expands in the hook command; `.mcp.json` and `.lsp.json` at the plugin root load; `startup\|clear\|compact` matcher fires               | Each fact observed or contradicted; list the contradicted ones                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | TODO     | [ ] pass / [ ] fail                |
| V3 | Claude Code context after `/clear` and compaction (matcher `startup\|clear\|compact`) | Start the C7 session with debug logging (`claude --debug --debug-file <log>`; verify the flag with `claude --help`, or use `claude --debug` and read the log under `~/.claude/debug/`). Run `/clear`, then `/compact`, and ask `What is the context marker?` after each. | Positive control first: the log shows the context command at startup; if it does not, record the log trace as inconclusive and use the counter. Primary evidence: the log shows the hook command `cat "${CLAUDE_PLUGIN_ROOT}/hooks/context/session-start.json"` run after `/clear` and after `/compact`. Secondary evidence: the counter file gains one line per action (`wc -l` before and after; it also fires on `resume`, so do not resume) and the reply holds `SMOKE_CONTEXT_MARKER`. The reply alone proves nothing: the marker can survive in the compacted summary. | TODO     | [ ] pass / [ ] fail                |
| V4 | Copilot CLI runs `sessionStart` again after compaction                                | Start the P7 session with debug logging (`copilot --log-level debug --log-dir <dir>`; verify the flags with `copilot --help`). Run `/compact`, and separately `/clear`; ask `What is the context marker?` after each.                                                    | Positive control first: the log shows the context command at startup; if it does not, the log cannot prove absence, so record the trace as inconclusive and use the counter. Primary evidence: the log shows the hook command `cat hooks/context/session-start.json` run after each action, with its timestamp and completion status, or shows that it did not. Secondary evidence: the counter file line count. Record the result either way; a negative result is a finding, not a failure of the smoke test.                                                              | TODO     | [ ] pass / [ ] fail / [ ] recorded |
| V5 | `matcher` field of a Copilot CLI hook entry                                           | **Not checkable with this fixture**: the generated Copilot entry for `smoke-hook` has no `matcher` (the fixture sets none). Adding a matcher map to the fixture would test it; see "Optional extension"                                                                  | Skipped                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | n/a      | n/a                                |
| V6 | Size limit of the context text of a hook in each client                               | **Not checkable by this run**: the fixture text is 2 lines. Finding a limit needs a separate series of growing texts                                                                                                                                                     | Skipped                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | n/a      | n/a                                |

### Optional extension for V5

Edit a copy of `items/smoke-hook/HOOK.md`: give `prompt-submit` the key
`matcher: { claude: ".*", copilot: ".*" }`, pack again, push, update the plugin
in the client (`copilot plugin update smoke`), and check that the marker file
still gains a line. Record what the client does with the value. This also tests
an update through the marketplace, which T3 did not.

## Observations

TODO. Record anything not covered by a row: error text, differences from the
local marketplace of T3, leftover files after uninstall, the Copilot CLI hook
log lines, whether the model answered a marker question from the hook text or
from a file it read, and the real `TMPDIR` of each client.

Locally verified: the generated `init` command is
`upskill add driftsys/<repo> smoke --kind rule --project --claude` in the Claude
Code plugin and the same with `--copilot` in the Copilot CLI plugin. An
`upskill add` of the fixture into a git project wrote
`.claude/rules/smoke-rule.md` and
`.github/instructions/smoke-rule.instructions.md`. Confirm that the model
reports the command output and runs no other command.

## Not tested

TODO. Candidates: Windows and Linux, an update of an installed plugin (unless
the optional extension runs), a private repository with a credential helper
other than the one in use, the second Claude Code scope.

## After the run

When every required check passes in both clients (C1 to C15 and P1 to P14, which
cover the "Done when" observations of the issue), apply the closing items of
issue #637 (AD-0046, Decision 15, requires every check to pass in both clients;
a failed or skipped required check keeps plugin output experimental). A
supplementary V point may end as a documented skip or as a recorded negative
result: remove the word "experimental" from ADR-0044, ADR-0046, the
support-level table and the book; add this note to the
[technote index](README.md); record the result in the issue.
