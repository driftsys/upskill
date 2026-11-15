# How Claude Code and Copilot CLI handle the output of a session-start hook

This note belongs to trial T2 of the customization re-steer (plan Task 9). It
records what the documentation of each client states about a hook that runs when
a session starts, and the result of the Claude Code part of the trial in
non-interactive mode and of the Copilot CLI part in prompt mode. The interactive
display of both clients has not been observed. This note is informative.

`upskill update --notice` prints one plain line on standard output for each
scope that has available updates (the project scope, then the global scope), for
example `upskill: 3 updates available (project). Run "upskill update".` It
prints a given set of updates once. The question of trial T2 is what each client
does with such a line.

## Method

The two documentation sections below: documentation only, read on 2026-11-14.
The pages were read through a tool that returns extracts, so the quoted phrases
below are extracts and were not compared character by character with the pages.
No page shows a publication date.

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
  it. The Copilot CLI trial in prompt mode showed that the model receives
  `additionalContext` and does not mention it without being asked. Whether the
  user sees the line in the interactive terminal is not observed for either
  client.
- Copilot CLI documents only a JSON object with `additionalContext`. A plain
  line may be ignored. If the trial confirms that, the hook command of the
  Copilot plugin must wrap the line in that object. The binary keeps printing a
  plain line, as the plan states. The trial confirmed that Copilot CLI ignores a
  plain line in prompt mode (see the Copilot CLI trial result below).
- For Claude Code, a line that the user must see may need the `systemMessage`
  field, which the hook command would also have to build.
- Both clients document that the session continues when the hook fails. Copilot
  CLI also documents that it continues when the hook passes its time limit; the
  extract of the Claude Code page does not state that case. The notice command
  exits with code 0 after argument parsing (a usage error exits with code 2) and
  looks up the sources for 5 seconds at most, so a hook cannot block a session
  start. A hook command should still tolerate a missing `upskill` binary.

## Trial result: Claude Code, non-interactive mode

Run on 2026-11-14 with Claude Code 2.1.308, in a temporary git repository. The
hook was declared in the project file `.claude/settings.json` of that
repository, for the event `SessionStart` without a matcher. No plugin was
installed and no plugin files exist for this trial: the hook was declared in the
project settings file of the temporary repository (the plan asked for a plugin).
Each hook command appended a line to a log file, to show that it ran, and then
printed its text. Eight model requests were made, two for each form:

```text
claude -p "<prompt>" --output-format stream-json --verbose --include-hook-events --setting-sources project --no-session-persistence
```

The settings file was the same for every form, with `<form>` replaced by the
name of the script of that form. The four scripts are named `a.sh`, `b.sh`,
`c.sh` and `d.sh` here; these names are only for this description:

```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"sh \"$CLAUDE_PROJECT_DIR/<form>.sh\""}]}]}}
```

Each script (`a.sh` for form A, `b.sh` for form B, `c.sh` for form C, `d.sh` for
form D) first appended the word `ran` to a log file in the repository, then
printed: form A, the line followed by a newline; form B,
`{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"<line>"}}`;
form C, `{"systemMessage":"<line>"}`; form D, one JSON object with both fields.

Prompt 1 was `Reply with the single word READY.` Prompt 2 asked whether the
context holds a notice about upskill updates and asked for an exact quote. The
printed line was
`upskill: 1 update available (project). Run "upskill update". [ref <marker>]`.

| Form of the hook output                             | Hook ran | Model quoted the line (prompt 2) | Model mentioned it without being asked (prompt 1) |
| --------------------------------------------------- | -------- | -------------------------------- | ------------------------------------------------- |
| A: the plain line                                   | yes      | yes                              | no                                                |
| B: JSON with `hookSpecificOutput.additionalContext` | yes      | yes                              | no                                                |
| C: JSON with `systemMessage`                        | yes      | no (answered `NONE`)             | no                                                |
| D: JSON with both fields                            | yes      | yes                              | no                                                |

Observed:

- The project hook ran without a trust or permission step, and every run exited
  with code 0. This was observed for `claude -p` only.
- With the plain line, the model received the text labelled as the output of the
  hook (`SessionStart:startup hook success: <line>`).
- In the output stream, the line appeared only in the `system/hook_response`
  event, as the raw standard output of the hook, for every form. That event is
  written only with `--verbose --include-hook-events`.
- With prompt 1 the final text was `READY` for every form. The final text of the
  reply did not hold the notice. The line was present in the output stream only
  in the `system/hook_response` event, which is written with
  `--verbose --include-hook-events`.
- The text of `systemMessage` did not reach the model.

Not observed:

- The interactive terminal: whether the plain line, `additionalContext` or
  `systemMessage` is displayed to the user there.
- The plain text output of `claude -p` without the stream format.
- A session that starts in another way than `startup` (`resume`, `clear`,
  `compact`, `fork`).
- A run with the user settings loaded (`--setting-sources project` left them
  out).

What follows for the Claude Code plugin: a plain line reaches the model, and the
model does not report it on its own. A hook that must inform the user and the
model would print JSON with both `systemMessage` and `additionalContext`. The
display of `systemMessage` in the interactive terminal has to be confirmed by a
person before the plugin is published.

## Trial result: Copilot CLI, non-interactive mode

Run on 2026-11-15 with GitHub Copilot CLI 1.0.112, in prompt mode, in temporary
git repositories. No plugin was installed: the hook was declared in the
repository (the plan asked for a plugin). Each hook script first appended a line
to a log file, to show that it ran, and then printed its text. Each request used
this command, with `--trust-folder` only in the second series. The standard
output and the standard error were captured to separate files:

```text
copilot --no-auto-update --disable-builtin-mcps --available-tools= --no-ask-user [--trust-folder] -p "<prompt>"
```

Prompt 1 was `Reply with the single word READY.` Prompt 2 asked whether the
context holds a notice about upskill updates and asked for an exact quote, or
`NONE`. The printed line was
`upskill: 1 update available (project). Run "upskill update". [ref <marker>]`.

Forms A and B used the file `.github/hooks/notice.json`, with `<repository>`
replaced by the path of the temporary repository:

```json
{"version":1,"hooks":{"sessionStart":[{"type":"command","bash":"sh \"<repository>/hook.sh\"","timeoutSec":30}]}}
```

- Form A: the script prints the plain line.
- Form B: the script prints `{"additionalContext":"<line>"}`.
- Form E: no `.github/hooks` file. The hook was declared only in
  `.claude/settings.json`, in the Claude Code shape below, and the script prints
  the plain line:

```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"sh \"<repository>/hook.sh\""}]}]}}
```

First series, without `--trust-folder` (5 requests: A prompt 1, A prompt 2, B
prompt 1, B prompt 2, E prompt 2):

- No hook ran: every log file stayed empty.
- The standard error held, for A and B,
  `Warning: Skipped repository hooks (.github/hooks) because the folder "<path>" is not trusted.`
  The Copilot log held, for E, the same warning with `(repository settings)` in
  place of `(.github/hooks)`.
- Every request exited with code 0.

Second series, with `--trust-folder` (the same 5 requests). The help text of the
option says that it trusts the configuration of the working directory for one
new prompt-mode session only, without saved trust.

| Form                                              | Hook ran | Model quoted the line (prompt 2) | Model mentioned it without being asked (prompt 1) | Line in standard output or standard error |
| ------------------------------------------------- | -------- | -------------------------------- | ------------------------------------------------- | ----------------------------------------- |
| A: plain line, `.github/hooks`                    | yes      | no (answered `NONE`)             | no (answered `READY`)                             | no                                        |
| B: JSON with `additionalContext`, `.github/hooks` | yes      | yes                              | no (answered `READY`)                             | only as the model's answer to prompt 2    |
| E: plain line, hook in `.claude/settings.json`    | yes      | no (answered `NONE`)             | not run                                           | no                                        |

Observed:

- A repository hook did not run in a folder that Copilot CLI did not trust.
- The hook ran with the repository root as its working directory. Each hook
  script appended `ran cwd=$(pwd)` to a log file, and the logged path was the
  path of the temporary repository.
- Every request exited with code 0.

Not observed:

- The interactive terminal: whether any form is displayed to the user there, and
  how the trust question is asked there.
- A hook that a plugin declares: the trust rule for a plugin hook is not known.
- Form B declared in `.claude/settings.json`.
- A hook declared for the user in `~/.copilot/hooks/`.

What follows for the Copilot plugin:

- Copilot CLI ignores a plain line. The hook command of the Copilot plugin must
  wrap the line of `upskill update --notice` in a JSON object with
  `additionalContext`, and print nothing when the binary prints nothing.
- The model receives the notice and does not report it without being asked. No
  form that shows the line to the user was found in prompt mode.
- A hook declared in `.claude/settings.json` runs in Copilot CLI too, so a
  project that declares the hook for both clients would run it twice in Copilot
  CLI. This is an inference: the trial declared the hook in one place only. The
  effect of two runs is described in the Copilot CLI section above.
- A repository hook does not run in a folder that Copilot CLI does not trust.

## What the trial must record

With the client version for each point. The Claude Code part above answered
these points for non-interactive mode only:

1. Whether the user sees the plain line. Answered for `claude -p`: the final
   text did not hold it; the stream held it in the `system/hook_response` event.
   Open for the interactive display of Claude Code.
2. Whether the agent receives it, and whether the agent mentions it or asks the
   user without being asked. Answered for `claude -p`: the model received the
   plain line and did not mention it without being asked.
3. The same two points for the JSON form of that client. Answered for
   `claude -p` (forms B, C and D above). Open for the interactive display.

Copilot CLI 1.0.112, answered for prompt mode only:

1. Whether the user sees the plain line, and whether the agent receives it and
   mentions it. The plain line was ignored and did not reach the model. It did
   not appear in the standard output or the standard error. Open for the
   interactive terminal.
2. The same points for its JSON form. The model received `additionalContext` and
   did not mention it without being asked. The line appeared in the output only
   as the model's answer to prompt 2. Open for the interactive terminal.
3. Whether a plain line is ignored. Yes, in prompt mode.
4. Whether a hook declared in `.claude/settings.json` also runs. Yes, in a
   trusted folder, for the plain line (form E). Form B in that file was not run.

Open for Copilot CLI: the interactive terminal, a hook that a plugin declares,
and a hook declared in `~/.copilot/hooks/`.
