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

| Client   | Kind  | Level        | Client version | Checked    |
| -------- | ----- | ------------ | -------------- | ---------- |
| claude   | rule  | supported    | 2.1.308        | 2026-11-14 |
| claude   | skill | supported    | 2.1.308        | 2026-11-14 |
| claude   | agent | supported    | 2.1.308        | 2026-11-14 |
| copilot  | rule  | experimental | -              | -          |
| copilot  | skill | experimental | -              | -          |
| copilot  | agent | experimental | -              | -          |
| opencode | rule  | experimental | -              | -          |
| opencode | skill | experimental | -              | -          |
| opencode | agent | experimental | -              | -          |
| codex    | rule  | unsupported  | -              | -          |
| codex    | skill | unsupported  | -              | -          |
| codex    | agent | unsupported  | -              | -          |

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

One smoke run is recorded, on 2026-11-14. On Claude Code 2.1.308 all six cells
passed (rule, skill and agent, each at project and global scope), so the three
`claude` cells are `supported`. One run on GitHub Copilot CLI 1.0.110 could not
be completed because the account's monthly quota was used up (HTTP 402). The
script of that run, before it handled the quota, printed `fail` for every
Copilot cell. The script now prints `unavailable` for that case, with a note
that names the quota. No Copilot cell has a result, so the Copilot cells stay
`experimental`. The tool names that the smoke mode allows for the skill and
agent cells (`SMOKE_TOOLS` in the script) are confirmed for Claude Code 2.1.308
only. They stay unconfirmed for Copilot CLI 1.0.110.

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
client one question for each kind in a new session, and checks that the answer
holds the marker of that kind. A project cell runs in a temporary repository
that the script deletes at the end. A global cell removes the items with
`upskill remove`, as described below. The marker appears only in the installed
item, never in the question.

| Flag                | Meaning                                                                                                                    |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `--run`             | Make the model calls                                                                                                       |
| `--clients`         | `claude`, `copilot`; default both                                                                                          |
| `--kinds`           | `rule`, `skill`, `agent`; default all                                                                                      |
| `--scopes`          | `project`, `global`; default both                                                                                          |
| `--allow-real-home` | Allow the global cells to write to the real home directory                                                                 |
| `--timeout`         | Seconds for each client, `upskill add` and `upskill remove` call; default 90; the `<client> --version` call always uses 15 |

The flags `--kinds`, `--scopes`, `--allow-real-home` and `--upskill` are valid
only with `--smoke`; without it the script exits with code 2.

A project cell runs in a new temporary git repository. `HOME` is not changed, so
the client stays authenticated. The path of `--upskill` may be relative; the
script resolves it, and refuses a path that is not a file when `--run` is set.

A global cell installs into the real home directory, under the item name
`upskill-smoke-<12 hex digits>`. It runs only with `--allow-real-home`; without
the flag the cell is `unavailable`. The script removes the three items with
`upskill remove --global --yes <name>` after the cell, also after a timeout or
an error. It runs `remove` after every attempt of `add`, also a failed or
timed-out one, because `add` is not atomic and a failed `add` can leave files
and the lockfile entry. When `remove` reports that the name is not in the
lockfile, nothing was left and the script prints no warning. What a global cell
leaves in the home directory after that removal, as checked with a temporary
home that held a `CLAUDE.md`, `opencode.json` and `.vscode/settings.json` before
the run:

- An existing `CLAUDE.md` is not changed. When the file is absent, `add` creates
  it with the content `@AGENTS.md`.
- `opencode.json`: the glob is `.agents/rules/**/RULE.md`. When the file is
  absent, `add` creates it with the key `instructions` and an array that holds
  the glob. When the file exists without the key `instructions`, `add` adds the
  key with an array that holds the glob. When the key holds an array, `add`
  appends the glob and keeps the other entries, and it leaves the file unchanged
  if the glob is already in the array. `add` fails when the key is not an array.
  `remove` does not revert the change.
- `.vscode/settings.json`: `add` creates the file with
  `{"chat.instructionsFilesLocations": {".github/instructions": true}}` when it
  is absent. When the file exists without that key, `add` adds the key with that
  value. When the key exists without the entry `.github/instructions`, `add`
  adds the entry with the value `true`. When the entry exists, with either
  value, `add` leaves the file unchanged. `remove` does not revert the change.
- The item files are deleted. The empty directories
  `.claude/{agents,rules,skills}` or `.copilot/{agents,instructions,skills}`
  stay.

A global cell restores the global lockfile to its bytes from before the cell.
Before `add`, the script reads `.upskill-lock.json` and notes whether
`.upskill-lock.json.v1` exists. After the removal it writes the saved bytes
back, or deletes the lockfile when it did not exist, and deletes
`.upskill-lock.json.v1` when it did not exist. The reason: `add` adopts a
`schema: 1` lockfile to `schema: 2` and writes the copy `.upskill-lock.json.v1`
unless that copy exists, and an installed upskill that predates `schema: 2`
cannot read the adopted file. The script does not restore in two cases. When the
removal fails, the lockfile then stays as the run changed it, and the warning
says so. When the script could not read the lockfile before the cell, it prints
`warning: the global lockfile was not restored: the global lockfile could not be read: <reason>`.
When both cases occur, the script prints only the removal warning. When the
restore itself fails, the script prints a `warning:` line and goes on.

If the script is killed hard during a global cell, the items stay in the home
directory. `upskill list --global` shows the name, which is `upskill-smoke-`
followed by 12 hexadecimal digits. Remove the items with
`upskill remove --global --yes <name>`.

The result of a cell is `pass`, `fail`, `timeout`, `unavailable` or
`install-failed`. A cell is `unavailable` when the client binary is missing, its
version cannot be read, the client reports an authentication failure, GitHub
Copilot CLI reports that the monthly quota is exceeded, or a global cell has no
`--allow-real-home`. A cell is `install-failed` when `upskill add` fails; the
client is not asked, and the level stays `experimental`. The output has one line
for each cell:

```text
<client> <kind> <scope> <result> <client version>
```

Each cell that has a reason adds a line
`note: <client> <kind> <scope> <reason>`. When the removal of a global item
fails, the script prints
`warning: global items <name> were not removed: <reason>; the global lockfile was left as the run changed it`.
When the restore of the lockfile fails, it prints
`warning: the global lockfile was not restored: <reason>`. With `--run`, one
line for each client and kind follows, `<client> <kind> <level>`; without
`--run` no level line appears. The level is `supported` when both scopes passed
and `experimental` otherwise. The script writes no file in the repository and
exits with code 0 whenever it ran, whatever the results.

After a run, a maintainer sets each client and kind with the level `supported`
in `src/support.rs`, with the client version and the date of the run, and
updates the table above to match. A test fails when the two differ.
