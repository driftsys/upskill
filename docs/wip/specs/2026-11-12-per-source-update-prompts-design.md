# `update` in a terminal: one exact preview and one question per source

Date: 2026-11-12. Status: **approved by the user on 2026-11-12; not
implemented**. Code baseline: `origin/main` at `ac0225e`. This document does not
describe released behaviour.

Related records:
[MVP epic #283](https://github.com/driftsys/upskill/issues/283),
[re-steer design](2026-11-11-customization-resteer-design.md),
[#471](https://github.com/driftsys/upskill/issues/471) (not in scope, see
[Non-goals](#non-goals)), [#497](https://github.com/driftsys/upskill/issues/497)
(resolved by this work).

## Why

In a terminal without `--yes`, `upskill update` runs a dry run over every
source, prints four counts, and asks `Apply? [y/N]` once:

```text
$ upskill update
  update:  3 item(s)
  unchanged:  9 item(s)
Apply? [y/N]
```

This has four defects.

1. **The preview does not name anything.** It does not say which items change,
   which files are written or deleted, or which edited files are kept.
2. **The preview is not exact.** The dry run compares source hashes only
   (`planned_source_hashes`). It does not classify the files on disk, so it
   cannot say that an item will be skipped because the user edited one of its
   files. It is also computed for every source before any source is applied, and
   the apply of one source can change the result for a later source.
3. **A run without a source change does nothing.** When no source hash changed,
   no item is interrupted and `--force` is not given, `update` prints "nothing
   to do" and stops. It does not report an edited file, does not delete the
   files of a client that the flags no longer select, and does not adopt a
   `schema: 1` lockfile. A run with `--yes` or without a terminal does all
   three. `docs/book/commands.md` and `docs/specification/specification.md`
   state this difference as a known limit.
4. **Every source is fetched twice**, once for the dry run and once for the
   apply.

The review of #489 found defect 3. A faithful preview for the whole run was
ruled out there, because `update` applies one source after the other. The user
decided on 2026-11-12 to plan the per-source design instead of recording it as
debt.

## What the user decided

These statements are the user's, from 2026-11-12. Everything else in this
document is the author's proposal, which the user read as a list and accepted
with "lgtm"; points marked "Author's call" were settled after that and were
reported to the user as a list.

1. `update` prints one block per source. The block is computed just before that
   source is applied.
2. `update` asks a question for a source only when applying that source would
   change something on disk.
3. An edited file whose item did not change in the source is kept with a note
   and exit code 0, in every mode.
4. Hand-shake files are previewed; plugin and MCP steps are not.

## Target behaviour

```text
$ upskill update
driftsys/skills
  update   skill tech-diagramming   4 files
  update   skill markspec-ears      1 file, keeps modified .claude/skills/markspec-ears/notes.md (no longer produced)
  remove   skill old-skill          no longer in source
Apply? [y/N/q] y
  2 updated, 1 removed, 1 modified file kept

./local-rules
  up to date
  kept modified file: .claude/rules/style.md

Updated 2 of 12 item(s) from 1 of 2 source(s).
```

The exact wording, alignment and colours of the block are left to the builder.
The content rules below are binding.

### The source block

`update` handles the sources in the order it uses today (sorted source label).
For each source it does these steps in order:

1. Fetch the source once.
2. Compute the **preview**: everything that applying this source now would do,
   from the fetched content and the current state of the disk and the lockfile.
3. Print the block: the source label, then one line per item that is not up to
   date, then the notes.
4. If the preview changes something on disk, ask. Otherwise print `up to date`
   and continue with the next source without a question.
5. On `y`, apply the source from the fetch of step 1. Print one result line.

A preview **changes something on disk** when applying it would write, replace or
delete at least one file under the target, or would change an item record of the
lockfile. A write that the skip-write rule of `apply_plan` suppresses (the file
on disk already equals the planned bytes) is not a change. An interrupted item
(a non-empty `pending` record) is a change, because completing it changes the
lockfile.

"An item record" means every field of the `LockedItem` entries that the apply
would save, compared with the entries in the current lockfile. A difference in
any field counts, including `git_ref`, `required_by`, `clients` and `files`.
When an item's record changes and none of its files does, its line says `update`
with the words `record only`, so that no question is asked without a line that
explains it. The `plugins`, `mcp` and `bundles` sections of the lockfile are not
compared; see
[Ancillary files, plugins and MCP servers](#ancillary-files-plugins-and-mcp-servers).
(Author's call.)

Each item line of the preview has one of these actions:

| Action     | Meaning                                                                                                                                                                                                                           |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `update`   | The item's source hash changed, or its outputs on disk differ from the plan. Files are written or deleted.                                                                                                                        |
| `complete` | The item has an interrupted `add` or `update` and its source hash did not change.                                                                                                                                                 |
| `remove`   | The source no longer contains the item. Its recorded files are deleted with the rules of `remove`.                                                                                                                                |
| `skip`     | An update of the item is withheld because a destination is modified or unmanaged, and `--force` is not given. Nothing of it is written. See [the rule for edited files](#an-edited-file-whose-item-did-not-change-in-the-source). |

An item line names each file that matters to a decision:

- a modified file that is kept (stale and modified, or belonging to a removed
  item);
- with `--force`, each modified or unmanaged file that would be replaced or
  deleted;
- for `skip`, each file that causes the skip.

A file that is written or deleted and is unchanged from its record is counted,
not named.

An item from another source that the reinstall of this source would write (a
cross-source dependency) appears in this source's block, as it appears in this
source's report today.

A source whose preview contains only `skip` lines changes nothing on disk. It is
printed without a question, and the run ends with exit code 1, as today.

### An edited file whose item did not change in the source

The user decided this on 2026-11-12. It changes a rule that #489 merged, in
every mode, including `--yes` and a run without a terminal.

Today an item with a modified or unmanaged file at one of its destinations is
skipped and the run exits with code 1, whether or not the source of the item
changed. With a hand-edited file, every `update` fails until `--force`.

New rule, without `--force`:

- **The item's source hash equals the recorded hash, and the item has no
  `pending` record:** nothing is withheld. The item is not written. Each
  modified destination is reported as `kept modified file: <path>` in the block
  of its source. This is a note, not a change: it asks no question and does not
  affect the exit code. The item has no `skip` line.
- **The item's source hash changed, or the item has a `pending` record:** an
  update is withheld. The item has a `skip` line and the run exits with code 1,
  as today.

An unmanaged file at a destination (a file with no record) keeps today's rule in
both cases: `skip` and exit code 1. The new rule covers only a recorded file
that the user modified. (Author's call.)

Known limit: when a new version of upskill renders an unchanged source to
different bytes, an item with an edited file keeps the old rendering of its
other files too, with only the note, until `update --force`. `doctor` remains
the command that lists modified files.

With `--force`, both cases replace the edited file, and the block names it.

### Answers

| Answer                                   | Effect                                                                |
| ---------------------------------------- | --------------------------------------------------------------------- |
| `y` or `yes`, any letter case            | Apply this source, then continue with the next source.                |
| `n`, `no`, an empty line, any other text | Do not apply this source. Continue with the next source.              |
| `q` or `quit`, any letter case           | Do not apply this source. Stop the run. Earlier sources stay applied. |
| End of input, or a read error            | Same as `q`.                                                          |

End of input no longer counts as a "no" that continues. This resolves the second
half of #497.

### Exit code

- 0 when no step failed and no item has a `skip` line. Answers `n` and `q` do
  not change the exit code: declining is a choice, not an error.
- 1 when an item has a `skip` line, in an applied source or in a source that had
  only `skip` lines. An item that is only noted as `kept modified file` does not
  cause exit code 1.
- 1 when a step fails. Sources applied before the failure stay applied, as
  today.

### Closing summary

After the last source, `update` prints one line that counts the items updated,
completed and removed, the total number of items considered, and the number of
sources applied out of the sources considered. When the user declined or quit,
the summary says how many sources were not applied. The line "everything is up
to date — nothing to do" is printed only when no source had a change, no source
had a note, and nothing was applied.

### A run without a source change

There is no separate case. Each source gets its preview, and the preview is
computed from the disk, not only from source hashes. Therefore:

- an edited file that is kept is reported in the block of its source;
- the files of a client that the flags no longer select appear as an `update`
  with deletions, and the source asks;
- an interrupted item appears as `complete`, and the source asks.

The limit described in defect 3 no longer exists, and the docs that state it are
corrected.

### `schema: 1` lockfile

Adoption of a `schema: 1` lockfile happens once per run, without a question,
with the existing notice and the existing `.upskill-lock.json.v1` backup. `add`
adopts without a question today, and the preview of every source needs the
adopted record. The order of #467 is kept: a source is fetched, then its overlap
check runs, then the lockfile is adopted if it is still `schema: 1`, then the
preview is computed. Adoption therefore happens for the first source that passes
its overlap check, and a first source that fails the check leaves the lockfile
untouched. The notice is printed before the block of that source. If the run
then fails or the user quits, the adoption stays. (The user accepted "once,
before the first source"; the placement after the overlap check is the author's
call, made after the builder check.)

An item of a `schema: 1` lockfile without a hash is not reported as a source
change for that reason alone: its preview is computed from the disk like any
other. This resolves the first half of #497.

### Flags and modes

| Mode                                | Behaviour                                                                                                                                                                              |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Terminal, no `--yes`                | As described above.                                                                                                                                                                    |
| `--yes`, or stdin is not a terminal | Same steps, no question: every source with a change is applied. Output and exit codes stay as they are today, except for the rule for edited files and the earlier empty-source guard. |
| `--dry-run`                         | Prints the same blocks, asks nothing, applies nothing, adopts nothing. See the note below.                                                                                             |
| `--force`                           | The preview is computed with overwrite. Each modified or unmanaged file that would be lost is named in the block.                                                                      |
| `update <name>...`                  | Unchanged selection: the sources of the named items are handled, each as a whole.                                                                                                      |
| `--quiet`                           | Blocks and the closing summary are not printed. A question is still printed, with the source label in it.                                                                              |

`--dry-run` note: every block is exact for the current disk. Because nothing is
applied, a later block does not include the effect of applying an earlier one.
The `--dry-run` output ends with one line that says so. With a `schema: 1`
lockfile, `--dry-run` computes the previews from an adopted record held in
memory and writes nothing.

### Ancillary files, plugins and MCP servers

The user accepted this section on 2026-11-12. The reinstall of a source also
does work outside `apply_plan`: it writes the hand-shake files (`CLAUDE.md`
bridge, `opencode.json` registration, `.vscode/settings.json` registration),
runs the plugin and MCP steps of a bundle source, and saves their records in the
lockfile.

- A hand-shake file that the apply would create or change is part of the
  preview, is listed in the block, and is a reason to ask.
- The plugin and MCP steps run when the source is applied, as today. They are
  not part of the preview and are not a reason to ask. A source with no other
  change is not applied in a terminal, so a missing plugin or MCP registration
  is not repaired there; `doctor` reports it and `update --yes` repairs it.

## Goals

- The block of a source states exactly what applying that source at that moment
  does to the disk and the lockfile.
- A terminal `update` and an `update --yes` reach the same end state when every
  answer is `y`.
- Each source of the run is fetched once for its own turn: the preview and the
  apply use the same fetch. A source that is fetched again as a cross-source
  dependency of another source is outside this goal; there is no run-wide cache
  of fetched sources. (Author's call.)

## Non-goals

- A question per item or per file.
- A check of every source before the first write (#471). A run can still stop
  after earlier sources were applied.
- Any change to `add`, `remove` or `doctor`.
- A change of the output of `update --yes` or of a run without a terminal, other
  than the two deliberate changes named in this document.

## Anti-goals

- A preview computed by code that is separate from the code that applies. Two
  implementations of the same decision will disagree. See
  [Constraints](#constraints).
- A question for a source that changes nothing.
- A terminal run that silently does less than a `--yes` run, except for the
  plugin and MCP steps named above.

## Constraints

- **One decision path.** The preview and the apply must come from the same
  planning and classification code: `classify` in `src/pipeline/owned.rs` stays
  the only function that decides a file state, and the conflict, stale-file and
  skip-write rules of `apply_plan` in `src/pipeline/apply.rs` must not be
  written a second time. The expected shape is that `apply_plan` is divided into
  a step that decides and returns a description, and a step that executes that
  description; the preview is the description. The builder may choose another
  shape that keeps one decision path.
- **Removals follow the same constraint.** An item that the source no longer
  contains is planned by `plan_removal` and deleted by `execute_removal` in
  `src/pipeline/lifecycle.rs`. The `remove` lines of the preview come from
  `plan_removal`, including, with `--force`, the name of each modified file that
  would be deleted. The set of removed items must be known before the apply,
  from the fetched source, not from the report of a finished install.
- **The apply decides again from the disk.** The user can take time to answer,
  and the disk can change in that time. The apply must not trust the preview for
  safety: it classifies again, as `apply_plan` does today. The result line
  reports what the apply did, not what the preview said.
- **Only `main.rs` reads stdin and writes stdout or stderr.** The library must
  give control back between the preview and the apply of each source. The
  expected shape is a callback argument of `update`, called once per source with
  the preview, that returns apply, skip or stop. The builder may choose another
  shape that keeps I/O in `main.rs`.
- **The fetch guard lives across the question.** `fetch_ssot` returns a guard
  that deletes the clone when dropped. The guard of a source must stay alive
  from its preview to the end of its apply.
- The overlap check of #467 (`check_sources_outside_outputs`) keeps its place:
  before adoption and before any write of the source. A source that fails it is
  an error before its block is printed.
- `guard_against_empty_source` runs before the block is printed and before any
  write of the source, in every mode. Under `--yes` it runs today after the
  reinstall has written; moving it is a deliberate change, and it is the second
  exception, with the rule for edited files, to "the output and exit codes of
  `--yes` stay as they are": the error text and exit code are the same, and the
  disk is left unchanged.
- Project rules: no new dependency, zero warnings, MSRV 1.85, no back-compat
  shims before 1.0 (`UpdateMode`, `UpdateStep`, `UpdateStatus::Would*` and
  `confirm_update` may be deleted or reshaped freely).

## Tests

- Acceptance tests first, in `tests/cli_update.rs` or a new
  `tests/cli_update_prompt.rs`, using the pseudo-terminal fixture
  `tests/fixtures/terminal_stdin.py` that #489 added. They are Unix-only, as the
  existing terminal tests are.
- Required cases, each with two sources unless stated: `y` then `y`; `n` then
  `y` (first source untouched on disk and in the lockfile, second applied); `q`
  on the first (second source not fetched, exit 0); end of input at the first
  question (same as `q`); a source with no change (no question); an edited file
  with no source change (note printed, no question, exit 0, also under `--yes`);
  an edited file with a source change (`skip`, exit 1); an unmanaged file at a
  destination with no source change (`skip`, exit 1); a narrowed client
  selection (deletions listed, question asked); an interrupted item (`complete`,
  question asked); `--force` (the lost file is named in the block); a
  `schema: 1` lockfile (notice before the first block, adoption kept after `q`).
- One test that the preview is exact: for a fixed scenario, the set of paths
  that the block names or counts equals the set of paths that the apply writes
  or deletes.
- One test that a source is fetched once per run.
- `--dry-run`: same blocks, nothing written, lockfile bytes unchanged, including
  with a `schema: 1` lockfile.
- All-platform tests for `--yes` and for a run without a terminal: output and
  exit codes unchanged from today, except for the two deliberate changes (the
  rule for edited files and the earlier empty-source guard). Existing tests that
  expect exit code 1 for an edited file with an unchanged source are rewritten
  to the new rule.

## Documents to update in the same pull request

- `docs/book/commands.md`: the `update` section (the paragraph that starts "In a
  terminal without `--yes`") and the interrupted-operation section.
- `docs/specification/specification.md`: the two matching passages.
- `skills/upskill-cli/SKILL.md`: the three statements about a terminal run
  without a source change, and the table row and text that say a hand-edited
  file makes `update` exit with code 1.
- An ADR for the per-source prompt, or an edit of the existing record that
  covers the `update` confirmation, whichever the index in `docs/adr/README.md`
  shows as the right home.
- Close #497 from the pull request. Leave #471 open.

## Left to the builder

- The exact text, alignment and styling of the block, the result line and the
  closing summary, within the content rules above.
- The types that carry the preview from the library to `main.rs`.
- Whether `--dry-run` and the terminal run use the same callback with a constant
  answer.
- How the work is divided into tasks and pull requests.

## Alternatives considered

1. **Keep one question for the whole run and make the list detailed.** Smaller
   change. Rejected: the list stays an estimate for every source after the
   first, which is defect 2, and the user chose the per-source picture.
2. **Check and plan every source, then ask once, then apply all.** This would
   also resolve #471. Rejected for now: the plan of a later source depends on
   the lockfile and disk state that the apply of an earlier source produces
   (cross-source dependencies, shared destinations), so an exact whole-run plan
   needs a simulated disk and lockfile. That is a larger design.
3. **Remove the question and always apply, as `--yes` does.** Simplest.
   Rejected: `update --force` deletes user edits, and the question is the place
   where the user sees which ones.
4. **A question per item.** Rejected: a source is reinstalled as a unit today
   (dependency closure, bundle resolution), and a partial apply of one source
   needs rules that do not exist.
