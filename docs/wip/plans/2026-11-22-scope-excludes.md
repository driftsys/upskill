# Per-scope excludes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace PR #608's per-source excludes records with one set of excluded
names per lockfile, applied in the dependency and bundle resolvers.

**Architecture:** `Lockfile.excludes` becomes a sorted `Vec<String>`. `add` and
`update` compute their effective excludes as (recorded set) − (names this run
installs by name) and pass them to `prepare_install` as today. The two resolvers
in `src/pipeline/install.rs` (`Resolver`, `BundleResolver`) skip an excluded
dependency or bundle member before recording any `required_by` link.

**Tech Stack:** Rust 2024, serde, assert_cmd + tempfile integration tests.

**Spec:** `docs/wip/specs/2026-11-22-scope-excludes-design.md`

## Global Constraints

- Lockfile key: top-level `"excludes": [<name>, ...]`, sorted, deduplicated,
  omitted when empty (`skip_serializing_if = "Vec::is_empty"`). A lockfile
  without excludes keeps its exact bytes.
- An exclude never applies to a positional `add` name, nor to a name in an
  `update` item filter (an item installed by name).
- `add <src> d` (positional `d`) removes `d` from the set.
- `add --exclude x` where `x` matches no item of the closure: exit 2, lockfile
  unchanged.
- Skip messages go to stderr, are suppressed by `--quiet`, and are printed only
  by `main.rs` (modules return data).
- Zero warnings; run `just sanity` before a push (it lints tests).
- Pre-1.0: no migration of the old `[{source, names}]` shape. That shape was
  never released, so it is deleted outright.
- Ruling: `add S --exclude d` keeps its current behavior of removing an
  installed `d` that came from `S` (explicit intent of that command).

## Review Focus

1. Item `d` installed directly from source B, then `add A r --exclude d` where
   `r` requires `d`: B's `d` stays installed and gets no `skill:r` in
   `required_by` (Task 2 test).
2. `d` excluded on `add A`, required by an item of another source T: `update`
   does not reinstall `d`, and two updates in a row print no change (Task 2).
3. Local source given as a symlinked path (`/tmp` vs `/private/tmp`): the
   exclude still holds, because there is no label key (Task 1 test).
4. `add A --exclude typo`: exit 2, lockfile bytes unchanged (Task 3).
5. Windows: no test asserts a `/` path separator (Task 1 fixes line 233).

---

### Task 1: Per-scope lockfile shape

**Files:**

- Modify: `src/lockfile.rs` (field at ~66, `record_excludes`/`excludes_for`/
  `prune_excludes` at ~399-440, unit tests ~540-620)
- Modify: `src/pipeline/mod.rs:200-240` (`effective_add_excludes` caller)
- Modify: `src/pipeline/prepared.rs:~891-900` (record call)
- Modify: `src/pipeline/lifecycle.rs` (~422 prune call, ~797-808 doctor check,
  ~1391 update excludes)
- Modify: `src/pipeline/report.rs` (~519 `orphan_excludes`), `src/main.rs`
  (doctor output for it)
- Test: `tests/cli_update_excludes.rs`, `tests/cli_doctor.rs`

**Interfaces:**

- Produces: `Lockfile.excludes: Vec<String>`;
  `Lockfile::record_excludes(&mut self, added: &[String], requested: &[String])`
  storing (recorded ∪ added) − requested, sorted. `LockedExclude`,
  `excludes_for`, `prune_excludes`, `DoctorReport.orphan_excludes` are deleted.
- Produces:
  `fn effective_excludes(recorded: &[String], own: &[String], named: &[String]) -> Vec<String>`
  in `src/pipeline/mod.rs` (rename of `effective_add_excludes`): recorded names
  not in `named`, plus `own`. `update` calls it with its item filter as `named`
  and `own = []`.

- [ ] **Step 1: Write failing tests.** Lockfile unit tests:
      `record_excludes_accumulates_sorts_and_deduplicates` (`["b","a","b"]` →
      `["a","b"]`), `record_excludes_removes_the_requested_names`,
      `a_lockfile_without_excludes_keeps_its_bytes` (schema 1 and 2). CLI tests
      in `cli_update_excludes.rs`:
      `an_exclude_holds_across_a_symlinked_source_path` (add via a symlink path,
      update via the real path, `d` stays out),
      `the_lockfile_records_one_sorted_excludes_list`
      (`lock["excludes"] ==
  ["d"]`). Delete the record-pruning tests
      (`removing_the_last_item_of_a_source_deletes_its_excludes_record`,
      `removing_one_of_two_items_keeps_the_excludes_record`) and the doctor
      orphan test; keep `cli_doctor`'s valid-record test, asserting a clean
      report. Fix line 233 to compare the `source` value with the expected label
      built from the path, not `contains("/x")`.
- [ ] **Step 2:** `cargo test --test cli_update_excludes --lib lockfile` → new
      tests fail.
- [ ] **Step 3:** Implement the interfaces above; update every caller. `update`
      (lifecycle ~1391) passes
      `effective_excludes(&lock.excludes, &[], &item_filter)`.
- [ ] **Step 4:** `just test` → all pass.
- [ ] **Step 5:** Commit
      `refactor(lockfile): record excludes once per scope (#570)`.

### Task 2: Exclude in the resolvers

**Files:**

- Modify: `src/pipeline/install.rs` (`Resolver` ~463-575, its construction ~605;
  `BundleResolver` ~727-860, construction ~864)
- Test: `tests/cli_update_excludes.rs`

**Interfaces:**

- Consumes: effective excludes as `options.excludes` (Task 1).
- Produces: both resolvers hold `excludes: BTreeSet<String>` and
  `skipped: BTreeSet<(ItemKind, String)>`; the closure results expose `skipped`
  so Task 3 can report it.

- [ ] **Step 1: Write failing tests:**
      `an_excluded_dependency_gets_no_required_by_link_on_a_direct_install`
      (Review Focus 1: after `add B d` then `add A r --exclude d`, `d` is
      installed with `source` = B and empty `required_by`; after `update`, still
      installed), `an_excluded_dependency_required_by_another_source_stays_out`
      (Review Focus 2: `update` twice; `d` absent both times; second update
      reports no removal),
      `an_excluded_cross_source_transitive_dependency_stays_out`,
      `an_excluded_member_of_a_required_bundle_stays_out`.
- [ ] **Step 2:** Run them → fail.
- [ ] **Step 3:** In `Resolver::visit`, when `requirer.is_some()` and the name
      is in `excludes`, insert into `skipped` and return before the
      `required_by` insert (~535); do not visit its requires. In
      `BundleResolver`, skip excluded member items the same way. Top-level
      requested names are never checked here.
- [ ] **Step 4:** `just test` → pass.
- [ ] **Step 5:** Commit
      `fix(pipeline): apply excludes in dependency and bundle expansion (#570)`.

### Task 3: Typo refusal and visibility

**Files:**

- Modify: `src/pipeline/prepared.rs`, `src/pipeline/report.rs`, `src/main.rs`
  (add/update output, `list` output)
- Test: `tests/cli_update_excludes.rs`, `tests/cli_list.rs`

**Interfaces:**

- Consumes: resolver `skipped` (Task 2).
- Produces: `InstallReport.excluded: Vec<(ItemKind, String)>` (items skipped by
  an exclude this run, including top-level whole-source skips); `ListReport`
  carries `excludes: Vec<String>` per scope.

- [ ] **Step 1: Write failing tests:**
      `an_exclude_that_matches_nothing_is_refused` (exit code 2, stderr names
      `typo`, lockfile bytes unchanged), `add_prints_each_excluded_item` (stderr
      contains `skill d` and `excluded`), `quiet_hides_the_excluded_lines`, and
      in `cli_list.rs` `list_shows_the_recorded_excludes` (stdout contains
      `excluded: d`).
- [ ] **Step 2:** Run → fail.
- [ ] **Step 3:** Refuse an own `--exclude` name that is in neither the entry
      source's scanned names nor `skipped`, before any write, using the same
      usage-error path that `add` uses for its other exit-2 refusals. Fill
      `excluded`; print in `main.rs` one stderr line per item unless `--quiet`;
      `list` prints `excluded: <names joined by ", ">` when the set is
      non-empty.
- [ ] **Step 4:** `just test` → pass.
- [ ] **Step 5:** Commit
      `feat(add): refuse unknown excludes and report skipped items (#570)`.

### Task 4: Docs

**Files:**

- Modify: `docs/adr/0045-record-add-excludes-in-lockfile.md` (edit in place:
  per-scope shape, resolver application, undo, typo refusal, alternatives
  including the per-source design and why it was dropped),
  `docs/specification/specification.md` §2.2 and §4, `docs/book/commands.md`,
  `AGENTS.md` (lockfile keys if listed)

- [ ] **Step 1:** Edit each statement about excludes to match Tasks 1-3; remove
      every mention of per-source records, pruning and the doctor orphan check.
- [ ] **Step 2:** `just fmt && just verify && just sanity` → pass.
- [ ] **Step 3:** Commit `docs: describe per-scope excludes (#570)`.
