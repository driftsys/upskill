# AD-0043 — One exact preview and one question per source for `update`

Status: **accepted**. Date: 2026-11-12.
[MVP epic #283](https://github.com/driftsys/upskill/issues/283). Resolves
[#497](https://github.com/driftsys/upskill/issues/497).
[#471](https://github.com/driftsys/upskill/issues/471) stays open. The behaviour
is specified in
[the specification, §2.2](../specification/specification.md#22-update) and
described for users in
[`upskill update`](../book/commands.md#upskill-update-name).

## Context

In a terminal without `--yes`, `upskill update` ran a dry run over every source,
printed four counts, and asked `Apply? [y/N]` once for the whole run. This had
four defects.

1. The preview did not name anything. It did not say which items change, which
   files are written or deleted, or which edited files are kept.
2. The preview was not exact. The dry run compared source hashes only. It did
   not classify the files on disk, so it could not say that an item would be
   skipped because the user edited one of its files. It was also computed for
   every source before any source was applied, and the apply of one source can
   change the result for a later source.
3. A run without a source change did nothing. When no source hash changed, no
   item was interrupted and `--force` was not given, `update` printed "nothing
   to do" and stopped. It did not report an edited file, did not delete the
   files of a client that the flags no longer select, and did not adopt a
   `schema: 1` lockfile. A run with `--yes` or without a terminal did all three.
4. Every source was fetched twice, once for the dry run and once for the apply.

Separately, an item with a hand-edited file was skipped, and the run exited with
code 1, even when the source of the item did not change. Every `update` then
failed until `update --force`.

## Decision

1. **One preview per source, computed just before the source is applied.**
   `update` handles the sources one at a time, sorted by source label. It
   fetches each source once and computes its preview from the fetched content,
   the files on disk and the lockfile. A source of a cross-source dependency
   (named in `requires` of an item of another source) is fetched again when the
   reinstall of that other source resolves the dependency. The preview and the
   apply come from the same planning and classification code. The apply
   classifies the files again and does not rely on the preview.
2. **One question per source that changes something.** In a terminal without
   `--yes`, `update` prints one block per source and asks `Apply? [y/N/q]` only
   when applying the source writes or deletes a file (also a file whose bytes
   are unchanged and whose mode differs from the mode of its source resource),
   changes an item record of the lockfile (any field except `pending` and
   `required_by`), completes an interrupted item, removes an item, or creates or
   changes a hand-shake file. `y` applies the source. `n` or any other text
   continues without it. `q`, end of input or a read error stops the run;
   earlier sources stay applied. Ctrl-C while the question waits exits at once
   with code 130 and changes nothing for that source. An interrupt during the
   apply of a source stops the run before the next source is shown or applied,
   with code 130.
3. **The rule for edited files.** Without `--force`, an item whose only
   difference is a modified recorded file is not skipped. This holds when the
   source hash of the item equals the recorded hash, the item has no `pending`
   record, applying the item would write or delete no other file, the item has
   no stale recorded file, and its lockfile entry would not change in any field
   other than `files`, `pending` and `required_by`. The file is kept and
   reported as `kept modified file: <path>`. The note is not a change and does
   not affect the exit code. In every other case, and for a file that upskill
   does not manage, the item is skipped. The rule applies in every mode. A
   skipped item of an applied source, or of a source whose preview changes
   nothing, makes the run exit with code 1; a skipped item of `--dry-run`, or of
   a source that the user declined, does not.
4. **Hand-shake files are previewed; plugin and MCP steps are not.** A
   `CLAUDE.md`, `opencode.json` or `.vscode/settings.json` change is listed in
   the block and is a reason to ask. The plugin and MCP steps of a bundle source
   run when the source is applied and are not a reason to ask.
5. **`--yes` applies every source.** With `--yes`, or when stdin is not a
   terminal, every source is applied without a question, also a source whose
   preview changes nothing, so its plugin and MCP steps run. `--dry-run` prints
   the same blocks and applies nothing.
6. **A `schema: 1` lockfile is adopted once per run, without a question**, for
   the first source that passes its checks, before the preview of that source is
   computed.

## Consequences

- A terminal run and an `update --yes` run reach the same end state when every
  answer is `y`, with three exceptions, each for a source whose preview changes
  nothing: its plugin and MCP steps, a change of only `required_by` in an item
  record of the source, and a change confined to a `bundles` record of the
  lockfile (the preview does not compare `bundles` records).
- A run without a source change reports an edited file, asks to delete the files
  of a client that the flags no longer select, asks to complete an interrupted
  item, and adopts a `schema: 1` lockfile.
- End of input at a question stops the run. It does not count as `n`.
- `update --yes` and a run without a terminal change in four ways:
  1. The rule for edited files of Decision 3.
  2. A source that resolves to no items while the lockfile records items from it
     is refused before any write of that source.
  3. A hand-shake file that cannot be updated (for example an `opencode.json`
     whose `instructions` is not an array) fails the source before any write of
     that source.
  4. A removal that is refused when it is planned (a symbolic link above a
     recorded path, or a recorded path that cannot be inspected) fails the
     source before its install is written.
- Known limits:
  - A terminal run does not apply a source whose preview changes nothing, so it
    does not repair a missing plugin or MCP registration of that source.
    `doctor` reports it and `update --yes` repairs it.
  - A `--dry-run` block is exact for the current disk, but does not include the
    effect of applying an earlier source.
  - A change of only `required_by` in an item record is not shown and does not
    cause a question. The reinstall of a dependency's own source computes a
    different `required_by` than the reinstall of the source that requires it,
    so counting it would cause a question on every run. The change is saved when
    the source is applied for another reason, or with `--yes`.
  - When a new version of upskill renders an unchanged source to different bytes
    in an edited file only, the new rendering is not written and only the note
    is printed, until `update --force`. `doctor` lists the modified files.
  - With `--quiet`, a terminal run does not print the blocks, the notes, the
    result lines or the closing summary. The question names the source.
- A run can still stop after earlier sources were applied (#471).

## Alternatives considered

- **Keep one question for the whole run and make the list detailed.** Rejected:
  the list stays an estimate for every source after the first.
- **Check and plan every source, then ask once, then apply all.** This would
  also resolve #471. Rejected for now: the plan of a later source depends on the
  lockfile and disk state that the apply of an earlier source produces
  (cross-source dependencies, shared destinations), so an exact whole-run plan
  needs a simulated disk and lockfile.
- **Remove the question and always apply, as `--yes` does.** Rejected:
  `update --force` deletes user edits, and the question is the place where the
  user sees which ones.
- **A question per item.** Rejected: a source is reinstalled as a unit
  (dependency closure, bundle resolution), and a partial apply of one source
  needs rules that do not exist.
