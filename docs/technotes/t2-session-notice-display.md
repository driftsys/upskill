# How Claude Code and Copilot CLI handle the output of a session-start hook

This note belongs to trial T2 of the customization re-steer (delivery step 8).
It records what the documentation of each client states about a hook that runs
when a session starts, and the results of the trial: Claude Code in
non-interactive mode and in the interactive terminal, Copilot CLI in prompt
mode, and the session-notice plugins of both clients loaded from a directory and
installed through a marketplace, and a later trial of Copilot CLI in prompt mode
with a request to the model in the context text. The interactive display of
Copilot CLI has not been observed. This note is informative.

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
  `~/.copilot/hooks/`. In a plugin, the documentation names
  `com.github.copilot/hooks/hooks.json` in the Agent Plugins 1.0 layout, with
  `plugin.json` at the plugin root. The documentation gives no example of a
  plugin `hooks.json`. Observed instead: Copilot CLI 1.0.112 loaded a plugin
  whose hooks are in `hooks.json` at the plugin root (see the plugin layout
  section below).
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
  `additionalContext` and does not mention it without being asked. When the text
  asks the model to tell the user, the model did so in every run of a later
  trial in prompt mode (see the request trial below), and the Copilot plugin
  puts that request before the notice. In the interactive terminal of Claude
  Code, only a hook output with `systemMessage` is displayed to the user (see
  the interactive terminal section below). Whether the user sees the line in the
  interactive terminal of Copilot CLI is not observed.
- Copilot CLI documents only a JSON object with `additionalContext`, and the
  trial confirmed that it ignores a plain line in prompt mode (see the Copilot
  CLI trial result below). The hook command of the Copilot plugin therefore
  wraps the line in that object. The binary keeps printing a plain line, as the
  plan states.
- For Claude Code, a line that the user must see needs the `systemMessage`
  field, and a line that the model must also receive needs `additionalContext`.
  The hook command of the Claude Code plugin builds one JSON object with both
  fields (form D).
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
- The text of `systemMessage` did not reach the model: when asked for the text,
  the model answered `NONE`.

Not observed in this non-interactive trial (the interactive terminal is covered
by the next section):

- The plain text output of `claude -p` without the stream format.
- A session that starts in another way than `startup` (`resume`, `clear`,
  `compact`, `fork`).
- A run with the user settings loaded (`--setting-sources project` left them
  out).

What follows for the Claude Code plugin: a plain line reaches the model, and the
model does not report it on its own. A hook that must inform the user and the
model prints JSON with both `systemMessage` and `additionalContext`. The display
of `systemMessage` in the interactive terminal was confirmed by a person (next
section).

## Trial result: Claude Code, interactive terminal

Observed by the maintainer on 2026-11-15 with Claude Code 2.1.308, in four
temporary git repositories. In each, the hook was declared in the project
`.claude/settings.json` for the event `SessionStart`, with one of the same four
forms A to D as in the non-interactive trial. No plugin was installed.

| Form of the hook output                             | Displayed to the user                                   |
| --------------------------------------------------- | ------------------------------------------------------- |
| A: the plain line                                   | no                                                      |
| B: JSON with `hookSpecificOutput.additionalContext` | no                                                      |
| C: JSON with `systemMessage`                        | yes, above the prompt as `SessionStart:startup says: …` |
| D: JSON with both fields                            | yes, the same display as form C                         |

Observed:

- Claude Code asked to trust the folder before the first start. After that, each
  hook ran.
- Only the forms with `systemMessage` (C and D) are displayed. The display has
  the shape `SessionStart:startup says: <line>`.
- Combined with the non-interactive result (the model receives forms A, B and D,
  and not form C), form D informs both the user and the model. The Claude Code
  plugin prints form D.

Not observed:

- A session that starts in another way than `startup`.
- The display in the interactive terminal after an install through a marketplace
  (the marketplace install ran in prompt mode only).

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
- A hook that a plugin declares: not run in this trial (see the plugin sections
  below).
- Form B declared in `.claude/settings.json`.
- A hook declared for the user in `~/.copilot/hooks/`.

What follows for the Copilot plugin:

- Copilot CLI ignores a plain line. The hook command of the Copilot plugin must
  wrap the line of `upskill update --notice` in a JSON object with
  `additionalContext`, and print nothing when the binary prints nothing.
- The model receives the notice and does not report it without being asked. No
  form that shows the line to the user was found in prompt mode in this trial. A
  later trial found that the model reports the line when the text asks for it
  (see the request trial below).
- A hook declared in `.claude/settings.json` runs in Copilot CLI too, so a
  project that declares the hook for both clients would run it twice in Copilot
  CLI. This is an inference: the trial declared the hook in one place only. The
  effect of two runs is described in the Copilot CLI section above.
- A repository hook does not run in a folder that Copilot CLI does not trust. A
  plugin hook did run in such a folder (see the plugin sections below).

## Trial result: Copilot CLI, a request to the model in the context text

Run on 2026-11-18 for issue #530 with GitHub Copilot CLI 1.0.112, in prompt
mode, in three temporary git repositories, one for each variant. The hook was
declared in `.github/hooks/notice.json` as in form B above, and the script
printed a fixed JSON object, without the `upskill` binary. Each request used the
command of the trial above, with `--trust-folder`. The model that Copilot CLI
used was not identified.

The notice line was
`upskill: 2 updates available (driftsys/metapowers, acme/skills). Run 'upskill update'.`
The value of `additionalContext` was:

- V0: the notice line.
- V1: `Tell the user at the start of your first reply:` and a space, then the
  notice line.
- V2:
  `Notice for the user from the upskill session hook. At the start of your first reply, show the user this line exactly, then continue with their request:`
  and a space, then the notice line.

Prompt 1 was `Reply with the single word READY.` Prompt 2 was
`List the files in this directory.` Each cell was run 5 times. A run counts when
the first line of the output is the notice line, with no change.

| Variant | Prompt 1 | Prompt 2 |
| ------- | -------- | -------- |
| V0      | 0 of 5   | 0 of 5   |
| V1      | 5 of 5   | 5 of 5   |
| V2      | 5 of 5   | 5 of 5   |

Observed:

- With V1 and V2 the output was the notice line, then the answer to the prompt.
  With V0 no output held the text `upskill update`.
- On prompt 2 the model ran tools to list the files, although the command passed
  `--available-tools=`. Prompt 2 therefore covers a reply that includes tool
  calls.
- A check before the series, with V0 and a prompt that asked for a quote of the
  notice, returned the line. This shows that the hook ran.
- Every request exited with code 0.

Not observed:

- The interactive terminal.
- The plugin `hooks.json` with the `upskill` binary: the hook of this series was
  a repository hook that printed a fixed text. The notice line of this series
  holds single quotes, while the binary prints double quotes. The plugin series
  below covers both points.
- Another model than the default one, other prompts, other wordings.

Plugin series, run on the same day after the hook command of the Copilot plugin
received the text of V1. The plugin was loaded with `--plugin-dir` from
`plugins/upskill-copilot`, with the binary built from the same commit first on
`PATH`, in a temporary project with one available update. The plugin was not
installed from a marketplace, so the hook ran once for each session. Each of the
5 runs used prompt 1 and a new, empty cache directory:

```text
PATH=<build directory>:$PATH XDG_CACHE_HOME=<new directory> copilot --no-auto-update --disable-builtin-mcps --available-tools= --no-ask-user --trust-folder --plugin-dir <repository>/plugins/upskill-copilot -p "Reply with the single word READY."
```

- In 5 of 5 runs the first line of the output was
  `upskill: 1 update available (project). Run "upskill update".`, with no
  change, followed by an empty line and `READY`.
- After each run the cache directory held `upskill/notice.json`, which shows
  that the plugin hook ran the binary.
- Every run exited with code 0.

Not observed in the plugin series: prompt 2, a second session with the same
cache directory, and an install through a marketplace.

What follows for the Copilot plugin: its hook command puts the text of V1 before
the notice. The user sees the notice only when the model follows the request.
The counts above are 15 runs of that wording with one client version and one
model, and they are not a guarantee.

## Trial result: the session-notice plugins loaded from a directory

Run on 2026-11-15, in prompt mode, in a temporary project that had one available
update, with a separate cache directory for each pair of runs. Each plugin was
loaded with `--plugin-dir`, and two sessions followed one another in the same
project. The prompt asked whether the context holds a notice about upskill
updates, and asked for an exact quote or `NONE`.

| Client and plugin                   | Session 1       | Session 2       |
| ----------------------------------- | --------------- | --------------- |
| Claude Code 2.1.308, Claude plugin  | quoted the line | answered `NONE` |
| Copilot CLI 1.0.112, Copilot plugin | quoted the line | answered `NONE` |

The second session printed nothing because the notice prints a set of updates
once.

Limit of these runs: the Copilot CLI run was made at commit `648239a`, before
the later changes to the hook command (`cd --` before the directory change and
the replacement of control characters) and before the plugin was renamed to
`upskill`. The Claude Code run was also made before the rename and before the
change for control characters. The current plugin files were not run in these
runs. The marketplace install in the next section ran the plugin files as they
were on 2026-11-15, before the request text of issue #530 was added to the hook
command of the Copilot plugin.

## Trial result: install through a marketplace

Run on 2026-11-15. Method: the marketplace was added from a local directory that
held the content of `driftsys/metapowers` pull request 78 before its merge
(`<client> plugin marketplace add <directory>`, then
`<client> plugin install upskill@metapowers`). The runs used a temporary project
with one available update, a new cache directory for each client, prompt mode,
and the same prompt as the other plugin runs (it asks whether the context holds
a notice about upskill updates, and asks for an exact quote or `NONE`). The
plugin files were those of 2026-11-15, after the rename to `upskill` and the
change for control characters, and before the request text of issue #530. The
marketplace and the plugin were removed after the runs.

| Client                                           | Session 1       | Session 2       |
| ------------------------------------------------ | --------------- | --------------- |
| Claude Code 2.1.308, user scope                  | quoted the line | answered `NONE` |
| Copilot CLI 1.0.112, plugin from the marketplace | quoted the line | answered `NONE` |

Copilot CLI used the manifest `.github/plugin/marketplace.json` and loaded the
plugin from `plugins/upskill-copilot` in place. It copied nothing. No
`--trust-folder` flag was passed.

Not done:

- An install from the GitHub repository `driftsys/metapowers`. The same content
  was merged there afterwards.
- The interactive terminal for this install.

## Copilot CLI plugin layout and environment

Observed with Copilot CLI 1.0.112 and a probe plugin loaded with `--plugin-dir`.
The probe hook wrote its working directory, a filtered list of its environment
and its standard input to a file.

- Copilot CLI loaded a plugin whose manifest is `plugin.json` at the plugin root
  and whose hooks are in `hooks.json` at the plugin root.
- A plugin hook starts with the plugin root as its working directory, not the
  project directory.
- The environment holds `COPILOT_PROJECT_DIR` (the project directory),
  `COPILOT_PLUGIN_ROOT` and `CLAUDE_PROJECT_DIR`.
- The JSON on standard input holds `cwd`, `sessionId` and `source` (the probe
  run also showed `timestamp` and `initialPrompt`).
- This is why the hook command of the Copilot plugin enters
  `COPILOT_PROJECT_DIR` before it runs `upskill update --notice`: the notice
  finds the project scope from the working directory.
- A plugin hook ran in a folder that Copilot CLI did not trust, while a
  repository hook did not (see the Copilot CLI trial result above).

## What the trial must record

With the client version for each point. Claude Code 2.1.308:

1. Whether the user sees the plain line. Answered: no, in the interactive
   terminal. In `claude -p` the final text did not hold it, and the stream held
   it in the `system/hook_response` event.
2. Whether the agent receives it, and whether the agent mentions it or asks the
   user without being asked. Answered for `claude -p`: the model received the
   plain line and did not mention it without being asked.
3. The same two points for the JSON form of that client. Answered (forms B, C
   and D above): `systemMessage` is displayed to the user and does not reach the
   model, and `additionalContext` reaches the model and is not displayed.

Copilot CLI 1.0.112:

1. Whether the user sees the plain line, and whether the agent receives it and
   mentions it. Answered for prompt mode: the plain line was ignored and did not
   reach the model, and did not appear in the standard output or the standard
   error. Open for the interactive terminal.
2. The same points for its JSON form. Answered for prompt mode: the model
   received `additionalContext` and did not mention it without being asked. The
   line appeared in the output only as the model's answer to prompt 2. With a
   request to tell the user before the notice, the model reported the notice in
   15 of 15 runs (see the request trial above). Open for the interactive
   terminal.
3. Whether a plain line is ignored. Answered: yes, in prompt mode.
4. Whether a hook declared in `.claude/settings.json` also runs. Answered: yes,
   in a trusted folder, for the plain line (form E). Form B in that file was not
   run.
5. Whether a hook that a plugin declares runs, and where. Answered: yes, from
   `hooks.json` at the plugin root, with the plugin root as working directory,
   and in a folder that was not trusted.

An install of either plugin through a marketplace is answered: the plugin files
of 2026-11-15 passed it in prompt mode (see the marketplace section above).
Still open: the interactive display of Copilot CLI, a hook declared in
`~/.copilot/hooks/`, the interactive terminal for an install through a
marketplace, and an install from the GitHub repository `driftsys/metapowers`.
