# Support levels

Each client and kind has one support level. The levels are a table that is
compiled into the binary. [`upskill doctor`](commands.md#upskill-doctor) prints
the same table, and this page states it.

## The levels

| Level        | Meaning                                                                  |
| ------------ | ------------------------------------------------------------------------ |
| Supported    | The smoke test passed on the recorded client version                     |
| Experimental | Implemented; the smoke test is missing, failing or older than the client |
| Unsupported  | Not implemented, or known not to work                                    |

For a client that is not implemented, no client flag exists: `codex` has none,
so the command line rejects `--codex` as an unknown flag.

A failed or outdated cell changes the level of that cell only. It does not block
a release and does not affect another cell.

## The table

A supported cell holds the client version of the smoke run that passed and the
date of that run. `-` marks an absent version or date.

| Client   | Kind  | Level        | Client version | Checked |
| -------- | ----- | ------------ | -------------- | ------- |
| claude   | rule  | experimental | -              | -       |
| claude   | skill | experimental | -              | -       |
| claude   | agent | experimental | -              | -       |
| copilot  | rule  | experimental | -              | -       |
| copilot  | skill | experimental | -              | -       |
| copilot  | agent | experimental | -              | -       |
| opencode | rule  | experimental | -              | -       |
| opencode | skill | experimental | -              | -       |
| opencode | agent | experimental | -              | -       |
| codex    | rule  | unsupported  | -              | -       |
| codex    | skill | unsupported  | -              | -       |
| codex    | agent | unsupported  | -              | -       |

`upskill doctor` prints this table after the report of the scopes, in the
section `Support levels:`, and `upskill doctor --json` holds it in the key
`support`. `doctor` does not detect the version of an installed client. The
recorded version is the one of the smoke run.

## Known limits of the experimental OpenCode level

OpenCode at global scope is unchanged. `.agents/skills` is a documented personal
location of OpenCode. `.agents/rules/<name>/RULE.md` with `$HOME/opencode.json`,
and `.opencode/agents/`, are not documented personal locations. OpenCode stays
experimental until its design settles these paths.

## Run the smoke test

A maintainer runs the smoke test against the real clients. It needs
authenticated clients and makes paid model calls, so hosted CI does not run it.

```bash
just smoke                  # print the cells; no model call
just smoke --run            # run every cell of claude and copilot
just smoke --run --clients claude --kinds skill --scopes project
```

`just smoke` builds the release binary and runs
`scripts/qualify-native-instructions.py --smoke --upskill target/release/upskill`.
Without `--run` the script prints the cells it would run, with the result
`not-run`, and starts no command and makes no model call. With `--run` it
installs a generated source of one rule, one skill and one agent, asks the
client one question for each kind in a new session, removes the items, and
checks that the answer holds the marker of that kind. The marker appears only in
the installed item, never in the question.

| Flag                | Meaning                                                    |
| ------------------- | ---------------------------------------------------------- |
| `--run`             | Make the model calls                                       |
| `--clients`         | `claude`, `copilot`; default both                          |
| `--kinds`           | `rule`, `skill`, `agent`; default all                      |
| `--scopes`          | `project`, `global`; default both                          |
| `--allow-real-home` | Allow the global cells to write to the real home directory |
| `--timeout`         | Seconds for each client call; default 90                   |

A project cell runs in a new temporary git repository. `HOME` is not changed, so
the client stays authenticated. A global cell installs into the real home
directory, under the item name `upskill-smoke-<12 hex digits>`. It runs only
with `--allow-real-home`; without the flag the cell is `unavailable`. The script
removes the three items with `upskill remove --global --yes <name>` after the
cell, also after a timeout or an error. The global install also creates or
updates the first-time files that `add` writes (`CLAUDE.md`, `opencode.json`,
`.vscode/settings.json`) and the global lockfile; `remove` does not delete these
files.

The result of a cell is `pass`, `fail`, `timeout` or `unavailable`. A cell is
`unavailable` when the client binary is missing, its version cannot be read, the
client reports an authentication failure, or a global cell has no
`--allow-real-home`. The output has one line for each cell:

```text
<client> <kind> <scope> <result> <client version>
```

followed by one line for each client and kind, `<client> <kind> <level>`. The
level is `supported` when both scopes passed and `experimental` otherwise. The
script writes no file in the repository and exits with code 0 whenever it ran,
whatever the results.

After a run, a maintainer sets each client and kind with the level `supported`
in `src/support.rs`, with the client version and the date of the run, and
updates the table above to match. A test fails when the two differ.
