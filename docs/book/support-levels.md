# Support levels

Each client and kind has one support level. The levels are a table that is
compiled into the binary. [`upskill doctor`](commands.md#upskill-doctor) prints
the same table, and this page states it.

## The levels

| Level        | Meaning                                                                  |
| ------------ | ------------------------------------------------------------------------ |
| Supported    | The smoke test passed on the recorded client version                     |
| Experimental | Implemented; the smoke test is missing, failing or older than the client |
| Unsupported  | Not implemented, or known not to work; the command refuses with a reason |

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

The smoke test installs a fixture with a random marker through the real
`upskill` binary, starts a new client session, and checks that the client
returns the marker. It needs authenticated clients and makes paid model calls,
so hosted CI does not run it. A maintainer runs it with `just smoke --run`.
Without `--run` it prints the cells it would run and makes no model call. A
global cell writes to the real home directory and runs only with
`--allow-real-home`.

After a run, a maintainer sets each cell that passed in both scopes to
`Supported` in `src/support.rs`, with the client version and the date of the
run, and updates the table above to match. A test fails when the two differ.
