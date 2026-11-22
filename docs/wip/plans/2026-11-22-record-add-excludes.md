# Record the excludes of `add` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `update` leaves out every name that the last `add` of the same source
excluded.

**Architecture:** A new top-level lockfile record per source label holds the
`--exclude` names of the last `add`. The shared install commit writes it;
`update` reads it into its install options; `remove` prunes records whose source
is gone; `doctor` reports orphan records.

**Tech Stack:** Rust 2024, serde, `assert_cmd` + `tempfile` integration tests.

**Spec:** `docs/wip/specs/2026-11-22-record-add-excludes-design.md`

## Global Constraints

- No schema bump: `CURRENT_SCHEMA` stays 2.
- A lockfile with no excludes keeps its exact bytes (the field is skipped when
  empty).
- `--exclude` keeps its matching rule: source names, across every source of the
  closure.
- `remove` records nothing new.
- Integration tests use `common::upskill_cmd(&fake_home)`, never a raw
  `cargo_bin`.
- Zero warnings: `just verify` and `just sanity` pass before the PR.

## Review Focus

1. Two `add`s of one source, the second for an unrelated item without
   `--exclude`: the record keeps the exclude (Task 2 test).
2. An excluded name that is also installed from another source: `update` of the
   first source must not touch the other source's entry (Task 2 test).
3. A schema-1 lockfile with no excludes is saved unchanged (Task 1 test).
4. `remove` of one item while other items of the source remain: the record stays
   (Task 3 test).
5. `update --dry-run` reads the record and plans no excluded item (Task 2 test).

---

### Task 1: Lockfile record

**Files:**

- Modify: `src/lockfile.rs` (struct `Lockfile` about line 56, `impl Lockfile`
  about line 257)

**Interfaces:**

- Produces:
  - `pub struct LockedExclude { pub source: String, pub names: Vec<String> }`
    (Serialize, Deserialize, Debug, Clone, PartialEq, Eq).
  - `Lockfile.excludes: Vec<LockedExclude>` with
    `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, after `mcps`.
  - `pub fn record_excludes(&mut self, source: &str, added: &[String], requested: &[String])`:
    the record of `source` becomes (recorded names ∪ `added`) minus `requested`,
    sorted and deduplicated; an empty result deletes the record. Records stay
    sorted by `source`.
  - `pub fn excludes_for(&self, source: &str) -> Vec<String>`: the recorded
    names, or an empty vector.
  - `pub fn prune_excludes(&mut self)`: deletes each record whose `source` has
    no entry in `items` and no entry in `bundles`.

- [ ] **Step 1: Write the failing unit tests** in the `tests` module of
      `src/lockfile.rs`:
  - `set_excludes_sorts_deduplicates_and_replaces`:
    `set_excludes("s", ["b",
    "a", "b"])` then
    `excludes_for("s") == ["a", "b"]`; a second `set_excludes("s", ["c"])` gives
    `["c"]`.
  - `set_excludes_with_no_names_deletes_the_record`: after
    `set_excludes("s",
    [])`, `excludes.is_empty()`.
  - `prune_excludes_keeps_a_source_with_an_item_or_a_bundle`: records for `a`
    (has an item), `b` (has a bundle), `c` (has neither); after the prune, only
    `a` and `b` remain.
  - `a_lockfile_without_excludes_keeps_its_bytes`: load and save a schema-1 and
    a schema-2 lockfile with no excludes; the bytes are equal and contain no
    `"excludes"`.
- [ ] **Step 2: Run** `cargo test -q --lib lockfile::` — expect the new tests to
      fail to compile.
- [ ] **Step 3: Implement** the interfaces above.
- [ ] **Step 4: Run** `cargo test -q --lib lockfile::` — expect PASS.
- [ ] **Step 5: Commit**
      `feat(lockfile): add a per-source excludes record
      (#570)`.

### Task 2: `add` writes the record, `update` applies it

**Files:**

- Modify: `src/pipeline/prepared.rs` (`commit`, next to the `upsert_bundle`
  calls about line 886)
- Modify: `src/pipeline/lifecycle.rs` (update options, `excludes: vec![]` about
  line 1375)
- Test: create `tests/cli_update_excludes.rs`

**Interfaces:**

- Consumes: `Lockfile::record_excludes`, `Lockfile::excludes_for` (Task 1).
- Produces: after any committed install, the lockfile holds
  `record_excludes(<entry source label>, <options.excludes>, <requested item
  names>)`,
  where the requested names are the item arguments of the command (empty for a
  whole-source or bundle-file add). `update` sets its install options'
  `excludes` to `lock.excludes_for(<source label>)`, so an `update` writes back
  the same record.

- [ ] **Step 1: Write the failing CLI tests** in `tests/cli_update_excludes.rs`
      (local registries in tempdirs; each test changes the source after `add` so
      that `update` has work to do):
  - `update_keeps_an_excluded_bundle_member_out_for_a_bundle_file_source`:
    `add
    <reg>/b.bundle.yaml --exclude m`; `update --yes`;
    `.claude/skills/m` does not exist and the lock has no `m` entry.
  - `update_keeps_an_excluded_bundle_member_out_for_a_directory_source_with_a_recorded_bundle`:
    same through `add <reg> b --exclude m`, where `b` is the bundle name (see
    `add_named_bundle` in `tests/cli_update_bundle.rs`).
  - `update_keeps_an_excluded_dependency_out_for_a_plain_directory_source`: `r`
    requires skill `d` from a second registry; `add <reg> r --exclude d`;
    `update --yes`; no `d` output and no `d` entry.
  - `update_dry_run_plans_no_excluded_item`: the plain-directory setup;
    `update
    --dry-run` stdout has no line for `d`.
  - `an_unrelated_add_from_the_same_source_keeps_the_exclude`: after the
    plain-directory `add`, run `add <reg> other`; the record still lists `d`,
    and the next `update --yes` does not install `d`.
  - `naming_an_excluded_item_undoes_the_exclude`: `r` requires skill `d`, both
    in `<reg>`; `add <reg> r --exclude d`, then `add <reg> d`; the lock has no
    `"excludes"` key and `d` is installed; the next `update --yes` keeps `d`.
  - `update_does_not_touch_an_excluded_name_installed_from_another_source`: `d`
    installed directly from source `x`; `add <reg> r --exclude d`;
    `update
    --yes`; the `d` entry keeps source `x` and its file bytes.
- [ ] **Step 2: Run** `cargo test -q --test cli_update_excludes` — expect FAIL.
- [ ] **Step 3: Implement** the record write in `commit` and the read in the
      update options.
- [ ] **Step 4: Run** `cargo test -q --test cli_update_excludes` and
      `cargo test -q` — expect PASS. A golden lockfile that changes means a
      record was written for an empty set: fix the code, not the golden file.
- [ ] **Step 5: Commit**
      `fix(update): apply the excludes of the last add of a
      source (#570)`.

### Task 3: `remove` prunes the record, `doctor` reports an orphan

**Files:**

- Modify: `src/pipeline/lifecycle.rs` (`remove`, about lines 341-425; `doctor`,
  about line 550)
- Modify: `src/pipeline/report.rs` (`DoctorReport`, about line 472)
- Modify: `src/main.rs` (doctor output)
- Test: `tests/cli_update_excludes.rs`, `tests/cli_doctor.rs`

**Interfaces:**

- Consumes: `Lockfile::prune_excludes` (Task 1).
- Produces: `DoctorReport.orphan_excludes: Vec<String>` (source labels), which
  makes `is_clean()` false; `main.rs` prints one line per label, in the style of
  the other orphan lines.

- [ ] **Step 1: Write the failing tests:**
  - `removing_the_last_item_of_a_source_deletes_its_excludes_record`
    (`cli_update_excludes.rs`).
  - `removing_one_of_two_items_keeps_the_excludes_record`
    (`cli_update_excludes.rs`).
  - `doctor_reports_an_excludes_record_with_no_installed_source`
    (`cli_doctor.rs`): write the record by hand into a lockfile with no item of
    that source; `doctor` exits non-zero and prints the label.
- [ ] **Step 2: Run** the three tests — expect FAIL.
- [ ] **Step 3: Implement:** call `lock.prune_excludes()` in `remove` before the
      lockfile is saved; add the doctor check and its output line.
- [ ] **Step 4: Run** `cargo test -q` — expect PASS.
- [ ] **Step 5: Commit**
      `feat(doctor): report an orphan excludes record
      (#570)`.

### Task 4: Docs

**Files:**

- Modify: `docs/specification/specification.md` (about line 512, and the
  lockfile section)
- Modify: `docs/book/commands.md` (about line 648)
- Modify: `docs/specification/format-spec.md` (only if it lists lockfile fields)
- Modify: `AGENTS.md` (only if a statement there becomes false)

- [ ] **Step 1:** Replace the "Current limitation (#570)" sentence in both files
      with the as-built rule: the lockfile records the `--exclude` names of the
      last `add` of each source; `update` applies them; a later `add` of the
      source replaces them, and one without `--exclude` clears them; `remove`
      deletes the record when nothing of the source remains; `doctor` reports an
      orphan record.
- [ ] **Step 2:** Describe the `excludes` field in the lockfile section of the
      specification.
- [ ] **Step 3: Run** `just fmt` then `just verify` — expect exit 0.
- [ ] **Step 4: Commit** `docs: describe the recorded excludes of add (#570)`.
