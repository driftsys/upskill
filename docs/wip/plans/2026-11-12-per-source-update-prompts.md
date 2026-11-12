# Per-source update prompts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a terminal, `upskill update` prints one exact preview per source
and asks one question per source that changes something on disk; an edited file
whose item did not change is kept with a note and exit code 0 in every mode.

**Architecture:** `apply_plan` is divided into a step that decides
(`decide_plan`, no write) and a step that executes (`execute_plan`).
`install_with_lockfile` is divided into `prepare_install` (fetch, resolve,
check, plan; no write) and methods on the prepared value that adopt, preview and
commit. `lifecycle::update` runs these steps per source and calls an
`UpdateObserver` between the preview and the commit; `main.rs` implements the
observer for a terminal, for `--yes` and for `--dry-run`.

**Tech Stack:** Rust edition 2024, MSRV 1.85, `anyhow`, `assert_cmd`,
`tempfile`, the pseudo-terminal fixture `tests/fixtures/terminal_stdin.py`.

**Spec:**
[docs/wip/specs/2026-11-12-per-source-update-prompts-design.md](../specs/2026-11-12-per-source-update-prompts-design.md).
Read it before any task. Where this plan and the spec differ, the spec is right.

## Global Constraints

- No new dependency. Zero warnings: `cargo clippy --all-targets -- -D warnings`.
- MSRV 1.85. No back-compat shims: delete replaced types outright.
- Only `src/main.rs` reads stdin or writes stdout or stderr.
- `classify` in `src/pipeline/owned.rs` stays the only function that decides a
  file state. The conflict, stale-file and skip-write rules exist once, in
  `decide_plan`.
- The commit of a source calls `decide_plan` again; it never executes the
  decision that the preview was built from.
- Integration tests build the command with `common::upskill_cmd(&home)`. A
  relative local source on a command line is a literal that starts with `./`.
- Code comments, commit messages and docs use plain, literal English.
  Conventional Commits. Commit trailer:
  `Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>`.
- Before each commit: `just fmt`. Before each pull request: `just verify`.
- Local runs are macOS only. Tests that need a pseudo-terminal carry
  `#[cfg(unix)]`; every other new test must pass on Windows (path separators,
  `\r\n`).

## Pull requests

- **PR 1** (Tasks 1-2): the rule for edited files. Branch
  `feat/keep-unwithheld-edits` from `origin/main`.
- **PR 2** (Tasks 3-7): the per-source prompt. Branch
  `feat/per-source-update-prompts`, stacked on PR 1 until PR 1 merges.

Each branch lives in its own worktree under `.claude/worktrees/<branch>`; run
`./bootstrap` there once.

## File structure

| File                                      | Responsibility after this plan                                                        |
| ----------------------------------------- | ------------------------------------------------------------------------------------- |
| `src/pipeline/apply.rs`                   | `decide_plan`, `execute_plan`, `apply_plan` (the two in sequence), `PlanDecision`     |
| `src/pipeline/prepared.rs` (new)          | `PreparedInstall`: the state of one source between fetch and commit; `InstallPreview` |
| `src/pipeline/mod.rs`                     | `install_with_lockfile` as prepare, adopt, commit; `AddOptions`                       |
| `src/pipeline/owned.rs`                   | `adopt_schema1` divided into an in-memory step and a save step                        |
| `src/pipeline/lifecycle.rs`               | `update` per source with an observer; removal preview                                 |
| `src/pipeline/report.rs`                  | `SourcePreview`, `PreviewItem`, `UpdateObserver`, `UpdateReport` fields               |
| `src/ancillary.rs`                        | `pending_handshake_changes`                                                           |
| `src/main.rs`                             | The three observers, block printing, the question, the closing summary                |
| `tests/cli_update_prompt.rs` (new)        | Terminal acceptance tests (`#![cfg(unix)]`)                                           |
| `tests/cli_update.rs`, `cli_ownership.rs` | All-platform tests of `--yes`, `--dry-run`, the rule for edited files                 |

## Review Focus

1. **The disk changes between the preview and the answer.** A file that the
   preview counted as unchanged is edited before `y`. Expected: the commit
   decides again, skips or keeps that item by the rules, and the result line
   reports what happened. Test in Task 5.
2. **An item kept by the rule for edited files loses its `files` record.**
   `install_with_lockfile` upserts every item that is not skipped with the files
   that `apply_plan` recorded; an item that is neither written nor skipped would
   get an empty record. Expected: its lockfile entry is byte-identical after the
   run. Test in Task 2.
3. **A cross-source dependency with an edited file.** The dependency is planned
   by the reinstall of another source, where "source hash unchanged" must be
   judged against the dependency's own entry. Expected: same rule as for an
   entry-source item. Test in Task 2.
4. **`q` or end of input after a `schema: 1` adoption.** Expected: the adoption
   and the backup stay, the notice was printed, no later source is fetched. Test
   in Task 6.
5. **`--dry-run` with a `schema: 1` lockfile.** Expected: blocks are printed,
   the lockfile bytes are unchanged and no `.upskill-lock.json.v1` exists. Test
   in Task 5.

---

### Task 1: Divide `apply_plan` into decide and execute

No behaviour change. Every existing test passes unchanged.

**Files:**

- Modify: `src/pipeline/apply.rs` (`apply_plan`, `ItemChanges`)

**Interfaces:**

- Produces, all `pub(super)` in `apply.rs`:

  ```rust
  /// What `execute_plan` will do, computed without any change on disk or
  /// in `lock`. One entry per planned item, in the order of `planned`.
  pub(super) struct PlanDecision { pub items: Vec<ItemChanges> }

  pub(super) struct ItemChanges {
      pub kind: ItemKind,
      pub name: String,
      pub files: Vec<OwnedFile>,
      pub delete: Vec<PathBuf>,
      pub kept: Vec<(PathBuf, Vec<OwnedFile>)>,
      pub conflicts: Vec<FileConflict>,
      pub unchanged: BTreeSet<PathBuf>,
  }

  pub(super) fn decide_plan(
      target: &Path,
      lock: &Lockfile,
      planned: &[PlannedItem],
      overwrite: bool,
  ) -> Result<PlanDecision>;

  pub(super) fn execute_plan(
      target: &Path,
      lock: &mut Lockfile,
      planned: &[PlannedItem],
      decision: PlanDecision,
      overwrite: bool,
      new_entries: Option<&[LockedItem]>,
  ) -> Result<ApplyOutcome>;
  ```

  `apply_plan` keeps its signature and becomes `decide_plan` followed by
  `execute_plan`.

- [ ] **Step 1: Write the failing test** in the `tests` module of `apply.rs`:
      `decide_plan_changes_nothing_on_disk_or_in_the_lockfile`. Build a target
      with one recorded item whose new plan writes one new file, rewrites one
      file and has one stale file. Call `decide_plan`. Assert: the directory
      tree (paths and bytes) and the serialized `lock` are equal before and
      after; `decision.items[0].delete` holds the stale path; the new file is
      not in `unchanged`.
- [ ] **Step 2:** Run `cargo test -p upskill decide_plan_changes_nothing` and
      confirm it fails to compile (`decide_plan` not found).
- [ ] **Step 3: Implement.** Move the code of `apply_plan` from its start to the
      "planned file cannot also be a directory" check into `decide_plan`; it
      only reads `lock`. Move the `pending` save, the write loop and the final
      save into `execute_plan`. Move the rustdoc paragraphs to the function that
      owns each rule; `apply_plan` keeps a short rustdoc that links both.
- [ ] **Step 4:** Run `cargo test` and
      `cargo clippy --all-targets -- -D warnings`. Expected: all pass.
- [ ] **Step 5: Commit**
      `refactor: divide apply_plan into a decide step and an execute step`.

### Task 2: Keep an edited file of an unchanged item, with a note

**Files:**

- Modify: `src/pipeline/apply.rs`, `src/pipeline/mod.rs` (`AddOptions`,
  `install_with_lockfile`), `src/pipeline/lifecycle.rs` (`update`),
  `src/main.rs` (only if a message text changes)
- Modify: `docs/book/commands.md`, `docs/specification/specification.md`,
  `skills/upskill-cli/SKILL.md`
- Test: `tests/cli_update.rs`, `tests/cli_ownership.rs`, `apply.rs` unit tests

**Interfaces:**

- Consumes: `decide_plan`, `execute_plan`, `ItemChanges` (Task 1).
- Produces:

  ```rust
  // apply.rs
  pub(super) enum Disposition {
      /// Written (no conflict, or `overwrite`).
      Write,
      /// Not written; an update is withheld. Reported in `ApplyOutcome::skipped`.
      Skip,
      /// Not written; nothing is withheld. Its modified destinations are
      /// reported in `ApplyOutcome::kept_edits`. Its lockfile entry is unchanged.
      KeepEdits,
  }
  // new field of ItemChanges:
  pub disposition: Disposition,
  // new field of ApplyOutcome:
  pub kept_edits: Vec<FileConflict>,
  // decide_plan gets two parameters after `overwrite`:
  //   new_entries: Option<&[LockedItem]>, keep_unwithheld_edits: bool
  // mod.rs, new field of AddOptions (default false):
  pub keep_unwithheld_edits: bool,
  ```

  `KeepEdits` is chosen only when all of these hold: `keep_unwithheld_edits`;
  not `overwrite`; `conflicts` is not empty; every conflict has
  `ConflictState::Modified` and its path is a planned file of the item (not a
  kept stale file); the item has a lockfile entry with an empty `pending`
  record; `new_entries` holds the item and its `hash` equals the entry's `hash`,
  both `Some`; `delete` and `kept` are both empty; the item's entry in
  `new_entries` equals the lockfile entry in every field except `files`,
  `pending` and `required_by`; every planned file that is not a conflict path is
  in `unchanged`. Otherwise the rule of today applies (`Write` or `Skip`).

  `update` sets `keep_unwithheld_edits: true`. `add` leaves it false.

- [ ] **Step 1: Write the failing acceptance tests** in `tests/cli_update.rs`.
      All use `update --yes`.
  - Rewrite `update_skips_an_edited_item_when_no_source_changed` as
    `update_keeps_an_edited_file_with_a_note_when_no_source_changed`: exit code
    0; stderr contains `kept modified file:` followed by the path; stderr does
    not contain `skipped`; the edited file's bytes are unchanged; the lockfile
    bytes are equal before and after (Review Focus 2).
  - `update_skips_an_edited_item_when_the_source_changed`: exit code 1, stderr
    contains `skipped skill res-skill:` (keep the existing test
    `update_keeps_a_modified_file_and_skips_the_item` if it already asserts
    this, and add nothing).
  - `update_skips_an_edited_item_when_a_deselected_client_file_would_be_deleted`:
    install for Claude and Copilot, edit the Claude file, run
    `update --yes --claude`. Exit code 1; the Copilot file still exists.
  - `update_skips_an_item_with_an_unmanaged_file_when_no_source_changed`: delete
    the item's entry from the lockfile `files` record for one path (or place a
    file at a planned path of a new client), exit code 1.
  - `update_keeps_an_edited_file_of_a_dependency_from_another_source` (Review
    Focus 3): reuse the fixture of
    `update_of_a_name_reports_a_skipped_dependency_from_another_source`, with no
    source change. Exit code 0, note printed.
  - `add_again_still_skips_an_edited_item` in `tests/cli_ownership.rs`: a second
    `add` of the same source with an edited file exits with code 1 (unchanged
    behaviour of `add`).
- [ ] **Step 2:** Run the new tests; confirm each fails for the expected reason
      (exit code 1 where 0 is expected).
- [ ] **Step 3: Write the unit tests** for `decide_plan` in `apply.rs`: one per
      condition of `KeepEdits` listed above, each asserting `Skip` when that one
      condition is false, and one asserting `KeepEdits` when all hold.
- [ ] **Step 4: Implement.** Compute `disposition` at the end of the per-item
      loop of `decide_plan`. In `execute_plan`, replace the two
      `!overwrite && !conflicts.is_empty()` tests with a match on `disposition`;
      a `KeepEdits` item gets no `pending` record, is not written, and its
      conflicts go to `kept_edits`. In `install_with_lockfile`, add the
      `(kind, name)` of each `kept_edits` conflict to the set that the final
      upsert loop skips, and append the `kept_edits` paths to
      `report.kept_modified`. In `update`, an item that is in neither `skipped`
      nor changed keeps the status it gets today (`UpToDate`).
- [ ] **Step 5:** Run `cargo test` and clippy. Rewrite any other existing test
      that expected exit code 1 for an edited file with an unchanged source and
      no other change; list each rewritten test in the report.
- [ ] **Step 6: Update the docs.** In `docs/book/commands.md` (the `update`
      section, near line 205), `docs/specification/specification.md` (near line
      160) and `skills/upskill-cli/SKILL.md` (the table row near line 62 and the
      text near lines 177 and 206): state the three-condition rule of the spec
      section "An edited file whose item did not change in the source" and its
      known limit. Do not yet change the sentences about a terminal run; Task 7
      owns them.
- [ ] **Step 7:** `just fmt`, `just verify`, commit
      `feat: keep an edited file of an unchanged item with a note on update`.
      Open PR 1 (body: the rule, the change of `--yes` behaviour that #489
      merged, the known limit).

### Task 3: Preview of the hand-shake files

**Files:**

- Modify: `src/ancillary.rs`

**Interfaces:**

- Produces:

  ```rust
  /// The hand-shake files that `ensure_claude_bridge`,
  /// `ensure_opencode_rules_registered` and
  /// `ensure_vscode_instructions_registered` would create or change, as
  /// paths relative to `target`, sorted. Writes nothing.
  pub fn pending_handshake_changes(target: &Path, has_rules: bool) -> Result<Vec<PathBuf>>;
  ```

  Each `ensure_*` function and this function must share the code that computes
  the new content, so that they cannot disagree.

- [ ] **Step 1: Write the failing unit tests** in `ancillary.rs`:
      `pending_handshake_changes_lists_each_missing_file` (empty target,
      `has_rules` true: three paths),
      `..._is_empty_after_the_ensure_functions_ran` (run the three `ensure_*`,
      then expect an empty list), and `..._writes_nothing` (tree equal before
      and after). Add one case with `has_rules` false and one with an
      `opencode.json` that already holds the entry among other keys.
- [ ] **Step 2:** Run them; confirm the compile failure.
- [ ] **Step 3: Implement** by extracting, for each `ensure_*`, a private
      function that returns `Option<new content>` (None when nothing changes).
- [ ] **Step 4:** Run `cargo test ancillary` and clippy. Commit
      `feat: report the hand-shake files that an install would change`.

### Task 4: Divide `install_with_lockfile` into prepare, adopt, preview, commit

No behaviour change for `add`. Every existing test passes unchanged.

**Files:**

- Create: `src/pipeline/prepared.rs`
- Modify: `src/pipeline/mod.rs`, `src/pipeline/owned.rs`

**Interfaces:**

- Consumes: `decide_plan`, `execute_plan`, `Disposition` (Tasks 1-2);
  `pending_handshake_changes` (Task 3).
- Produces, `pub(super)` unless marked:

  ```rust
  // owned.rs: the in-memory part of adopt_schema1. Changes `lock` only.
  pub(super) fn adopt_schema1_in_memory(target: &Path, lock: &mut Lockfile)
      -> Result<Option<Vec<String>>>;
  // adopt_schema1 keeps its signature: in-memory step, then backup and save.

  // prepared.rs
  /// One source after fetch, resolution, the conflict check, the overlap
  /// check and planning. Owns the fetch guards. Nothing was written.
  pub(super) struct PreparedInstall { /* private */ }

  pub(super) fn prepare_install(
      source: &InstallSource,
      target: &Path,
      items: &[String],
      options: &AddOptions,
  ) -> Result<PreparedInstall>;

  impl PreparedInstall {
      /// `(kind, installed name) -> source hash` of every planned item.
      pub fn planned_hashes(&self) -> BTreeMap<(ItemKind, String), Option<String>>;
      /// What `commit` would do now, against `lock`. Writes nothing.
      pub fn preview(&self, target: &Path, lock: &Lockfile) -> Result<InstallPreview>;
      /// Adopts if needed, executes, runs the plugin and MCP steps, saves the
      /// lockfile and writes the hand-shake files: the second half of today's
      /// `install_with_lockfile`.
      pub fn commit(self, target: &Path, plugin_scope: PluginScope) -> Result<InstallReport>;
  }

  pub(super) struct InstallPreview {
      pub items: Vec<PreviewItem>,       // see Task 5 for the type
      pub handshake: Vec<PathBuf>,
  }
  ```

  `install_with_lockfile` becomes `prepare_install(...)?.commit(...)`. `preview`
  builds the lockfile entries exactly as `commit` does (extract the shared code
  that computes `new_items`), calls `decide_plan`, and maps each `ItemChanges`
  to a `PreviewItem`. `PreviewItem` and its `record_only` rule are defined in
  Task 5; in this task create the type in `report.rs` with the fields given
  there.

- [ ] **Step 1: Write the failing unit test** in `prepared.rs`:
      `preview_equals_what_commit_changes`. For a fixture with one changed item,
      one unchanged item and one stale file: take `preview`, collect its written
      and deleted paths, then `commit` and compare with `InstallReport::changed`
      as sets. Add `prepare_and_preview_write_nothing` (tree and lockfile bytes
      equal).
- [ ] **Step 2:** Run; confirm the compile failure.
- [ ] **Step 3: Implement.** `prepare_install` is the code of
      `install_with_lockfile` up to and including
      `apply::check_installed_names`; it must not load-and-keep a `Lockfile` for
      later writing (the conflict check loads its own copy). `commit` loads the
      lockfile, adopts, and runs the rest. Keep the `AdoptedBeforeFailure`
      context on errors after adoption.
- [ ] **Step 4:** Run `cargo test` and clippy; expect all green with no test
      edited. Commit
      `refactor: divide install_with_lockfile into prepare, preview and commit`.

### Task 5: `update` per source with an observer

**Files:**

- Modify: `src/pipeline/report.rs`, `src/pipeline/lifecycle.rs`,
  `src/pipeline/mod.rs` (re-exports), `src/main.rs`
- Test: `tests/cli_update.rs`, `tests/cli_interrupted.rs`

**Interfaces:**

- Consumes: `prepare_install`, `PreparedInstall` (Task 4); `plan_removal`,
  `execute_removal` (existing).
- Produces, all `pub` in `report.rs`:

  ```rust
  pub enum PreviewAction { Update, Complete, Remove, Skip }

  pub struct PreviewItem {
      pub kind: ItemKind,
      pub name: String,
      pub action: PreviewAction,
      /// Files that would be written with new content.
      pub write: Vec<PathBuf>,
      /// Files that would be deleted.
      pub delete: Vec<PathBuf>,
      /// Modified files that are kept (stale, or of a removed item).
      pub kept_modified: Vec<PathBuf>,
      /// With `--force`: modified or unmanaged files that would be lost.
      pub forced: Vec<PathBuf>,
      /// For `Skip`: the files that cause the skip.
      pub blocking: Vec<PathBuf>,
      /// True when only the lockfile record of the item changes.
      pub record_only: bool,
  }

  pub struct SourcePreview {
      pub source: String,
      /// Items that are not up to date. An up-to-date item has no entry.
      pub items: Vec<PreviewItem>,
      /// `kept modified file` notes of items kept by the rule for edited files.
      pub kept_edits: Vec<PathBuf>,
      pub handshake: Vec<PathBuf>,
      /// Paths adopted from a `schema: 1` lockfile just before this preview.
      pub adopted: Option<Vec<String>>,
  }
  impl SourcePreview {
      /// True when a commit would write or delete a file or change an item
      /// record: an item whose action is not `Skip`, or a hand-shake file.
      pub fn has_changes(&self) -> bool;
  }

  pub enum SourceDecision { Apply, Skip, Stop }

  pub struct SourceResult { pub source: String, pub updated: usize,
      pub completed: usize, pub removed: usize, pub kept_modified: usize,
      pub skipped: usize }

  pub trait UpdateObserver {
      fn preview(&mut self, preview: &SourcePreview) -> SourceDecision;
      fn applied(&mut self, result: &SourceResult);
  }

  // UpdateReport: new fields
  pub sources_considered: usize,
  pub sources_applied: usize,
  /// Sources with changes that were not applied (answer no, quit, dry run).
  pub sources_not_applied: usize,
  pub stopped: bool,
  // UpdateStatus: delete WouldChange, WouldRemove, WouldCompleteInterrupted;
  // add NotApplied (an item of a source with changes that was not applied).
  ```

  ```rust
  // lifecycle.rs: replaces the old signature. UpdateMode is deleted.
  pub fn update(
      target: &Path,
      names: &[String],
      plugin_scope: PluginScope,
      selection: &ClientSelection,
      overwrite: bool,
      dry_run: bool,
      observer: &mut dyn UpdateObserver,
  ) -> Result<UpdateReport>;
  ```

  Per source, in this order: `prepare_install`; `guard_against_empty_source` on
  `planned_hashes()`; unless `dry_run`, `adopt_schema1` when the lockfile needs
  it (with `dry_run`, `adopt_schema1_in_memory` on a copy that is never saved);
  build the `SourcePreview` from `PreparedInstall::preview` and from
  `plan_removal` for each entry that `planned_hashes()` does not contain;
  `observer.preview`; on `Apply` and not `dry_run`: `commit`, then the removals
  with `plan_removal` and `execute_removal` computed again, then
  `observer.applied`; on `Stop`: set `stopped` and leave the loop. `dry_run`
  ignores the decision and never commits.

  `record_only` is true for an item with no file to write or delete whose
  planned `LockedItem` differs from the lockfile entry in a field other than
  `files` and `pending`.

- [ ] **Step 1: Write the failing tests** in `tests/cli_update.rs` (all
      platforms, no terminal):
  - `update_dry_run_writes_nothing_with_a_schema_1_lockfile` (Review Focus 5):
    lockfile bytes equal, no `.upskill-lock.json.v1`, stdout names the item.
  - `update_refuses_an_empty_source_before_any_write`: two sources, the second
    resolves to no items; with `--yes`, exit code 1, the existing error text,
    and (new) the first source applied, the second untouched, no file of the
    second source rewritten.
  - `update_yes_applies_a_source_without_a_change`: a bundle source with no
    change still records its plugin outcome (assert on the lockfile `plugins`
    section staying present and the run exiting 0).
  - In `lifecycle.rs` unit tests, with a recording observer:
    `the_commit_decides_again_after_the_preview` (Review Focus 1): the observer
    edits a file that the preview listed under `write`, returns `Apply`; the
    report marks that item `SkippedModified` and the file keeps the edit.
    `stop_does_not_fetch_later_sources`: second source is a local path that does
    not exist; returning `Stop` at the first gives `Ok` with `stopped`.
    `a_source_is_fetched_once`: count calls through a local-path source whose
    directory the observer deletes inside `preview` before returning `Apply`;
    the commit succeeds because the planned bytes are already in memory.
    `preview_lists_a_removed_item_and_its_kept_file`.
    `skip_only_source_has_no_changes`.
- [ ] **Step 2:** Run; confirm the failures.
- [ ] **Step 3: Implement** `report.rs` types, then `update`. In `main.rs`, add
      three observers and keep today's printed output for the first two:
      `ApplyAll` (`preview` returns `Apply`, prints nothing) for `--yes` and for
      a run without a terminal; `DryRun` (returns `Skip`); and a temporary
      `AskEachSource` for a terminal that prints the source label, asks
      `Apply? [y/N]` when `has_changes()` and returns `Skip` otherwise (Task 6
      replaces it). Delete `UpdateStep`, `update_step`, `confirm_update`,
      `print_update_plan` and their tests. `print_update_report` handles
      `NotApplied` (dry run: the words `would update`; `--dry-run` output for
      the existing dry-run tests must keep the strings they assert, adapt the
      match arms, not the tests, where possible).
- [ ] **Step 4:** Run `cargo test` and clippy. Existing tests that assert the
      old terminal behaviour ("nothing to do" with no source change, a single
      question) are rewritten in Task 6; in this task mark only those with
      `#[ignore = "rewritten in the next task"]` and list them in the report.
- [ ] **Step 5:** Commit
      `feat: run update source by source with a preview before each apply`.

### Task 6: The terminal block, the answers and the closing summary

**Files:**

- Modify: `src/main.rs`
- Create: `tests/cli_update_prompt.rs` (`#![cfg(unix)]`)
- Modify: `tests/cli_update.rs`, `tests/cli_repeatable.rs`, `tests/cli_quiet.rs`
  (remove the `#[ignore]` markers of Task 5 by rewriting or deleting those
  tests)

**Interfaces:**

- Consumes: `UpdateObserver`, `SourcePreview`, `SourceResult`, `UpdateReport`
  fields (Task 5).
- Produces: `struct TerminalObserver` in `main.rs`; pure functions
  `fn render_block(preview: &SourcePreview) -> Vec<String>` and
  `fn parse_answer(line: Option<&str>) -> SourceDecision` (`None` is end of
  input or a read error), both unit-tested.

Fixed texts:

- Question: `Apply? [y/N/q]` on stderr. With `--quiet`:
  `Apply changes from <source>? [y/N/q]`.
- A source without changes: the line `up to date` under the source label.
- Note line: `kept modified file: <path>`.
- Action words at the start of an item line: `update`, `complete`, `remove`,
  `skip`. A record-only item: the words `record only`. A hand-shake file: a line
  that starts with `write` and the path.
- Dry-run last line:
  `Each block is computed for the current state; a later block does not include the effect of an earlier one.`
- Closing summary: `Updated <n> of <m> item(s) from <a> of <s> source(s).`; when
  `sources_not_applied > 0`, append `<k> source(s) not applied.`; the line
  `everything is up to date — nothing to do` only when no source had a change or
  a note.

`parse_answer`: `y`/`yes` → `Apply`; `q`/`quit` → `Stop`; `None` → `Stop`;
anything else, including an empty line → `Skip`. Letter case and surrounding
white space are ignored.

- [ ] **Step 1: Write the failing acceptance tests** in
      `tests/cli_update_prompt.rs`, using a helper copied from
      `update_in_terminal` in `tests/cli_update.rs` (move the helper to
      `tests/common/mod.rs` and use it from both files). Two local sources `a`
      and `b`, each with one skill, both changed, unless stated:
  - `yes_then_yes_applies_both`: input `y\ny\n`; both updated; exit 0; summary
    `Updated 2 of 2 item(s) from 2 of 2 source(s).`
  - `no_then_yes_leaves_the_first_source_untouched`: tree of `a`'s outputs and
    `a`'s lockfile entry equal before and after; `b` updated; summary ends with
    `1 source(s) not applied.`
  - `quit_stops_before_the_second_source`: input `q\n`; output does not contain
    the label of `b`; nothing written; exit 0.
  - `end_of_input_stops_like_quit`: empty input.
  - `a_source_without_a_change_asks_nothing`: only `b` changed; output holds
    exactly one `Apply?`.
  - `an_edited_file_without_a_source_change_is_a_note`: no source changed, one
    edited file; output has `kept modified file:`, no `Apply?`, exit 0.
  - `a_skip_only_source_asks_nothing_and_exits_1`: edited file and source
    changed.
  - `a_narrowed_selection_lists_the_deletion_and_asks`: `update --claude` after
    an install for two clients.
  - `an_interrupted_item_is_listed_as_complete_and_asks`: reuse the fixture of
    `tests/cli_repeatable.rs`.
  - `force_names_the_file_that_is_lost`: the block contains the edited path
    before the question.
  - `adoption_notice_comes_before_the_first_block_and_stays_after_quit` (Review
    Focus 4): schema-1 lockfile, input `q\n`; the notice precedes the first
    source label; `.upskill-lock.json.v1` exists; the lockfile has `schema: 2`.
  - `the_block_names_exactly_what_the_apply_changes`: parse the paths that the
    block names or counts for one source, answer `y`, compare with the files
    whose bytes or existence changed.
  - `quiet_prints_only_the_question`: with `--quiet`, the output holds the
    question with the source label and no block line.
  - `a_missing_handshake_file_is_listed_and_asks`: delete `CLAUDE.md` after the
    install, no source change; the block lists it; `y` recreates it.
- [ ] **Step 2:** Run `cargo test --test cli_update_prompt`; confirm failures.
- [ ] **Step 3: Write the unit tests** for `parse_answer` (each row of the table
      above) and `render_block` (one preview per action, `record_only`, a
      hand-shake line, a count of unnamed files).
- [ ] **Step 4: Implement** `TerminalObserver`, replacing `AskEachSource`.
      `preview` prints the adoption notice when `adopted` is set, then the
      block, then asks only when `has_changes()`. `applied` prints the result
      line. The exit code: 1 when any item has status `SkippedModified`, as
      `finish_update` does today.
- [ ] **Step 5:** Run `cargo test`, clippy, and the `#[ignore]` list of Task 5
      must be empty (`grep -rn 'rewritten in the next task' tests src` prints
      nothing). Commit `feat: ask one question per source in a terminal update`.

### Task 7: Documents, decision record, and the pull request

**Files:**

- Modify: `docs/book/commands.md`, `docs/specification/specification.md`,
  `skills/upskill-cli/SKILL.md`, `docs/adr/README.md`
- Create or modify: one record under `docs/adr/`

- [ ] **Step 1:** In `docs/book/commands.md`, replace the passage of the
      `update` section from "In a terminal without `--yes`, `update` first runs
      a dry run" to the end of that paragraph, and the terminal sentences of the
      interrupted-operation section (near line 618), with the behaviour of the
      spec sections "The source block", "Answers", "Exit code", "Closing
      summary", "`schema: 1` lockfile", "Flags and modes" and "Ancillary files,
      plugins and MCP servers". Quote the fixed texts of Task 6 exactly.
- [ ] **Step 2:** Make the same change in the two matching passages of
      `docs/specification/specification.md` (near lines 162-176 and 381).
- [ ] **Step 3:** In `skills/upskill-cli/SKILL.md`, correct the three statements
      about a terminal run without a source change (near lines 62, 177 and 206).
- [ ] **Step 4:** Search `docs/adr/` for the record that describes the `update`
      confirmation (`grep -rln 'Apply?' docs/adr`). If one exists, edit it in
      place; otherwise create the next numbered record,
      `per-source-update-prompts`, with the decision, the alternatives of the
      spec and the plugin and MCP limit, and add it to `docs/adr/README.md`.
- [ ] **Step 5:** Check every sentence that this branch adds, changes or makes
      false in the docs against the code: run
      `grep -rn -i 'nothing to do\|Apply?\|dry run first' docs skills README.md`
      and read each hit.
- [ ] **Step 6:** `just fmt`, `just verify`. Commit
      `docs: describe the per-source update prompt`. Open PR 2. The body says
      `Closes #497`, states the two deliberate changes of `--yes` behaviour (the
      earlier empty-source guard; the rule for edited files came with PR 1), and
      states that #471 stays open.
