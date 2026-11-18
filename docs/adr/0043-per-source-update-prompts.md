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

1. **Prepare every source, check the whole run, then one preview per source,
   computed just before the source is applied.** `update` first fetches every
   source once, sorted by source label, and resolves, checks and plans it
   without any change. A source of a cross-source dependency (named in
   `requires` of an item of another source) is fetched again when the source
   that requires it is prepared. Then the source overlap check runs once on the
   items of all these sources together and on the item directories of every
   local-path source recorded in the lockfile that the run does not otherwise
   read (#547). The same item directories must also not contain a recorded file
   (any file of the `files` and `pending` records, also a modified file that the
   removal keeps) of an item that the run removes because its source no longer
   contains it (#550). `add` refuses a source whose item directory contains a
   recorded file of an item from another source, and `remove` refuses to delete
   the recorded files of an item when the source directory of another installed
   item, recorded from a local-path source, contains one of them, also with
   `--force`; a recorded local source of an item that stays installed that
   exists and cannot be read makes `remove` fail (#550). With a `schema: 1`
   lockfile these checks read the files that the adoption would record, computed
   in memory without a write. A source that cannot be fetched (a failed
   `git clone`, or a local-path source whose path does not exist), also through
   the source of a cross-source dependency that it requires, is skipped (#551):
   its files are not written (a `schema: 1` adoption that another source causes
   still records the `files` of its lockfile entries, see Decision 6), it is
   reported on stderr with its label and the cause, the other sources are
   handled, and the run exits with code 1, also in a dry run. One run gives one
   verdict per source: a prepared source whose plan would write an item that the
   lockfile records from a skipped source is skipped too, with its own message
   (it was skipped because it requires an item of the named skipped source, not
   because it could not be fetched), until no further source is dropped. A
   skipped source is not in the prepared set, so the overlap check treats it as
   a recorded source that the run does not read: a local path that does not
   exist protects nothing, and a git source has its item directories in no
   directory on disk. Any other error in this phase stops the run before any
   adoption and before any source is applied. The prepare phase reads the
   interrupt flag at three points: before each source, after a fetch failure,
   and before the step that compares the prepared sources with the skipped
   sources. At each point, after an interrupt, it fetches no further source,
   records no further skip, and the run fails with exit code 130; the skips
   recorded before the interrupt are still reported. A Ctrl-C in a terminal also
   reaches a running `git` child process, because `git` is started without a new
   process group; a fetch that ends with an error after an interrupt is
   therefore not a fetch failure. An interrupt during the prepare step of a
   source, also of the last source, is seen at the next of these points, so the
   phase does not complete. Only an interrupt that arrives after the check
   before the comparison step reaches the apply phase: there, the observer of a
   terminal run and of a `--yes` run returns a stop for the first source, after
   the empty-source check and the adoption step of that source and before its
   preview is shown, so no source is applied; the `--dry-run` observer does not
   stop in the preview step and prints every block. The process exits with code
   130 when the flag is set. Then `update` handles the sources one at a time, in
   the same order, and computes the preview of each source from its fetched
   content, the files on disk and the lockfile. The preview and the apply come
   from the same planning and classification code. The apply classifies the
   files again and does not rely on the preview.
2. **One question per source that changes something.** In a terminal without
   `--yes`, `update` prints one block per source and asks `Apply? [y/N/q]` only
   when applying the source writes or deletes a file (also a file whose bytes
   are unchanged and whose executable bits differ from the target executable
   bits: each executable bit of its source resource for which the file has the
   matching read bit; a difference in the other permission bits is not a
   change), changes an item record of the lockfile (any field except `pending`,
   `required_by` and `commit`), completes an interrupted item, removes an item,
   or creates or changes a hand-shake file. `y` applies the source. `n` or any
   other text continues without it. `q`, end of input or a read error stops the
   run; earlier sources stay applied. Ctrl-C while the question waits exits at
   once with code 130 and applies none of the changes shown for that source; an
   adoption of a `schema: 1` lockfile that the run already saved stays saved. An
   interrupt during the apply of a source stops the run before the next source
   is shown or applied, with code 130.
3. **The rule for edited files.** Without `--force`, an item whose only
   difference is a modified recorded file is not skipped. This holds when the
   source hash of the item equals the recorded hash, the item has no `pending`
   record, applying the item would write or delete no other file, the item has
   no stale recorded file, and its lockfile entry would not change in any field
   other than `files`, `pending`, `required_by` and `commit`. The file is kept
   and reported as `kept modified file: <path>`. The note is not a change and
   does not affect the exit code. In every other case, and for a file that
   upskill does not manage, the item is skipped. The rule applies in every mode.
   A skipped item of an applied source, or of a source whose preview changes
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
6. **A `schema: 1` lockfile is adopted once per run, without a question**, after
   every source that was not skipped (because it could not be fetched, or
   because it requires an item of a skipped source) passed the checks of the
   prepare phase, for the first such source that also passes the empty-source
   check, before the preview of that source is computed. The adoption covers the
   whole lockfile: an entry of a skipped source gets the `files` record of its
   files on disk, and no other field of that entry changes. When every source is
   skipped, nothing is adopted.

## Consequences

- A terminal run and an `update --yes` run reach the same end state when every
  answer is `y`, with two exceptions, each for a source whose preview changes
  nothing: its plugin and MCP steps, and a change confined to a `bundles` record
  of the lockfile (the preview does not compare `bundles` records).
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
    cause a question: it is provenance, not a change of the item. The change is
    saved when the source is applied, and also when the preview of the source
    changes nothing; it is not saved with `--dry-run` or for a declined source.
    Each run keeps the requirers whose `requires` it did not resolve (#504).
  - When a new version of upskill renders an unchanged source to different bytes
    in an edited file only, the new rendering is not written and only the note
    is printed, until `update --force`. `doctor` lists the modified files.
  - With `--quiet`, a terminal run does not print the blocks, the notes, the
    result lines or the closing summary. The question names the source.
- A run can still stop after earlier sources were applied (#471): after a `q`
  answer, end of input or a read error at the question, an interrupt, or an
  error in the apply phase of a later source. An error in the prepare phase,
  other than a source that cannot be fetched, applies no source; a source that
  cannot be fetched, and a source that requires an item of such a source, are
  skipped and the other sources are applied.

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
- **Stop the run when one source cannot be fetched** (the behaviour from #549
  until #551). Rejected: one host that does not answer, or one deleted
  repository, blocked the update of every other source of the lockfile. Before
  #549, `update` handled one source at a time, so the sources before the failing
  one were updated. Skipping the source keeps the other sources current, and
  exit code 1 still reports the failure to scripts. Other prepare errors (a
  parse error, the overlap check, a recorded source that exists and cannot be
  read) still stop the run, because they show a state of the lockfile or of the
  disk that the user must correct before any write.
- **A question per item.** Rejected: a source is reinstalled as a unit
  (dependency closure, bundle resolution), and a partial apply of one source
  needs rules that do not exist.
