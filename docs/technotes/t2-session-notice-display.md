# How Claude Code and Copilot CLI handle the output of a session-start hook

This note prepares trial T2 of the customization re-steer (plan Task 9). It
records what the documentation of each client states about a hook that runs when
a session starts. The trial itself has not been run: no hook was installed and
no session was started. This note is informative.

`upskill update --notice` prints one plain line on standard output for each
scope that has available updates (the project scope, then the global scope), for
example `upskill: 3 updates available (project). Run "upskill update".` It
prints a given set of updates once. The question of trial T2 is what each client
does with such a line.

## Method

Documentation only, read on 2026-11-14. The pages were read through a tool that
returns extracts, so the quoted phrases below are extracts and were not compared
character by character with the pages. No page shows a publication date.

## Claude Code

Sources: `https://code.claude.com/docs/en/hooks.md`,
`https://code.claude.com/docs/en/plugins/manifest-reference.md`,
`https://code.claude.com/docs/en/plugins/install.md`,
`https://code.claude.com/docs/en/plugins/marketplace-reference.md`. The hooks
page cites versions v2.1.251 and v2.1.296.

- **Event.** `SessionStart`. Its matcher takes the way the session started:
  `startup`, `resume`, `clear`, `compact`, `fork`.
- **Declaration in a plugin.** `hooks/hooks.json` at the plugin root, with the
  event map inside a top-level `"hooks"` key. `${CLAUDE_PLUGIN_ROOT}` is the
  absolute path of the installed plugin.
- **Standard output, exit code 0.** "Claude Code adds plain-text stdout as
  context that Claude can see and act on." The documentation does not state that
  the user sees this text.
- **JSON output.** `hookSpecificOutput.additionalContext` is added as a system
  reminder for the model; "it doesn't appear as a chat message in the
  interface". The common field `systemMessage` is a "Warning message shown to
  the user". The extract does not confirm that `systemMessage` applies to
  `SessionStart`.
- **Failure.** `SessionStart` has "No blocking or decision control". With exit
  code 2, standard error is shown to the user only and the session proceeds.
  Another non-zero code is reported as a non-blocking failure.
- **Timeout.** 600 seconds by default for a command hook; the `timeout` key of
  the handler sets it, in seconds.
- **Install.** `claude plugin marketplace add <owner>/<repo>`, then
  `claude plugin install <plugin>@<marketplace>`. A marketplace repository needs
  `.claude-plugin/marketplace.json`.

A hook file of the documented shape (not taken verbatim from the documentation,
and not run):

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          { "type": "command", "command": "upskill update --notice", "timeout": 30 }
        ]
      }
    ]
  }
}
```

## Copilot CLI

Sources: `https://docs.github.com/en/copilot/reference/hooks-configuration`,
`https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/plugins-creating`,
`https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/plugins-finding-installing`.

- **Event.** `sessionStart`; it fires when "A new or resumed session begins."
- **Declaration.** In a repository, `.github/hooks/*.json`. For a user,
  `~/.copilot/hooks/`. In a plugin, `com.github.copilot/hooks/hooks.json` in the
  Agent Plugins 1.0 layout, with `plugin.json` at the plugin root. The
  documentation gives no example of a plugin `hooks.json`.
- **Output.** For `sessionStart`, "Only `additionalContext` is consumed." The
  standard output is parsed as JSON on exit code 0, and empty output means no
  action. The documentation does not state whether `additionalContext` is shown
  to the user or given to the model, and it states no use of plain-text output.
- **Failure.** Exit code 2 is "Treated as a warning by default." Another
  non-zero code is "Logged as a hook failure. The run continues (fail-open)." A
  hook that passes its time limit is stopped and the run continues.
- **Timeout.** `timeoutSec`, 30 seconds by default.
- **Install.** `copilot plugin marketplace add <owner>/<repository>`, then
  `copilot plugin install <plugin>@<marketplace>`. A local directory is
  installed with `copilot plugin install ./<directory>`.
- **Claude Code files.** The page states that `.claude/settings.json` and
  `.claude/settings.local.json` in the repository are also read. A project that
  declares the hook for both clients could therefore run it twice in Copilot
  CLI. When the two runs do not overlap and the first run saved its cache file,
  the second run prints nothing, because the notice prints a set once. Two runs
  that overlap can both print the line, and so can every run when the cache
  directory cannot be written.

The documented shape of a hook file (not run):

```json
{
  "version": 1,
  "hooks": {
    "sessionStart": [
      { "type": "command", "bash": "upskill update --notice", "timeoutSec": 30 }
    ]
  }
}
```

## What follows for the notice

- Claude Code documents the plain output of the hook as context for the model,
  not as a message to the user. Copilot CLI documents that it consumes
  `additionalContext` and does not state whether the user or the model receives
  it. Whether the user sees the line, and whether the agent tells the user, are
  the main questions of the trial.
- Copilot CLI documents only a JSON object with `additionalContext`. A plain
  line may be ignored. If the trial confirms that, the hook command of the
  Copilot plugin must wrap the line in that object. The binary keeps printing a
  plain line, as the plan states.
- For Claude Code, a line that the user must see may need the `systemMessage`
  field, which the hook command would also have to build.
- Both clients document that the session continues when the hook fails. Copilot
  CLI also documents that it continues when the hook passes its time limit; the
  extract of the Claude Code page does not state that case. The notice command
  exits with code 0 after argument parsing (a usage error exits with code 2) and
  looks up the sources for 5 seconds at most, so a hook cannot block a session
  start. A hook command should still tolerate a missing `upskill` binary.

## What the trial must record

For each client, with the client version:

1. Whether the user sees the plain line.
2. Whether the agent receives it, and whether the agent mentions it or asks the
   user without being asked.
3. The same two points for the JSON form of that client.
4. For Copilot CLI, whether a plain line is ignored.
5. For Copilot CLI, whether a hook declared in `.claude/settings.json` also
   runs.
