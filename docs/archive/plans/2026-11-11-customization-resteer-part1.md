# Customization Re-steer Part 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the unreachable lifecycle code from `main`, then make `add`,
`update` and `remove` record the files they write, protect files they do not
own, and complete when run again after an interruption.

**Architecture:** The live pipeline (`src/pipeline/`) stops writing files while
it walks a source. It first builds an in-memory plan of every file for every
item, then one function checks each destination against the lockfile record,
saves the intended files as pending, writes through a temporary file and a
rename, deletes owned files that are no longer produced, and saves the final
record. The lockfile moves to `schema: 2` and gains a per-item list of written
files with SHA-256 hashes.

**Tech Stack:** Rust edition 2024 (MSRV 1.85), `serde_json`, `sha2`, `anyhow`,
`assert_cmd` + `tempfile` for CLI tests. No new dependency.

**Spec:**
[docs/wip/specs/2026-11-11-customization-resteer-design.md](../specs/2026-11-11-customization-resteer-design.md).
This plan covers delivery steps 0 to 4 of that design. Steps 5 to 8 (scope
reporting and global paths; update check and session notice; smoke test and
support levels) each get their own plan when they start, because they depend on
the result of trial T1 and on client paths that are not confirmed yet.

## Global Constraints

- Single static binary. No async runtime, no `git2`, no new dependency.
- `main.rs` performs I/O orchestration only. Only `main.rs` writes to stdout and
  stderr. Library code returns data or `Result<T>`.
- Zero warnings: compiler, clippy (`-D warnings`), docs tooling.
- Acceptance tests come first and run the binary through
  `common::upskill_cmd(&home)`. Never use a raw `Command::cargo_bin`.
- Pre-1.0: no compatibility layer, except the one schema-1 adoption step.
- Every pull request changes behaviour that a command can reach, or tests such
  behaviour (goal G8).
- Shared files edited by `src/ancillary.rs` (`CLAUDE.md`,
  `.vscode/settings.json`, `opencode.json`) are outside the ownership record.
  Their behaviour does not change.
- Plugin and MCP behaviour does not change.
- The two scopes are named **project** and **global** in all output and docs.
- Comments, commit messages, docs and pull request text use plain, literal
  English.
- One pull request per delivery step, each from its own worktree under
  `.claude/worktrees/<branch>`; run `./bootstrap` after creating it. Before each
  pull request: `just fmt`, then `just verify`. Conventional Commits. Squash
  merge.
- The design and this plan stay in `docs/wip/` until the last delivery step of
  the design; they are gardened then, not after Part 1.
- Repository administration (pushing archive branches, the ruleset, closing
  issues, deleting branches or worktrees) needs the user's confirmation at the
  moment it is done.

## Review Focus

1. A schema-1 lockfile that records an item whose output files are missing on
   disk: adoption records no file for that item and does not fail (Task 7).
2. A file that the user added inside an installed skill directory: `remove`
   keeps that file and its directory, and deletes only recorded files (Task 9).
3. An install made with `--as <alias>`: the record holds the alias paths, and
   `remove <alias>` deletes them (Task 8).
4. A resource file with the executable bit: the bit is still set after the write
   through a temporary file and a rename (Task 11).
5. An update that narrows the client selection: the owned file of the deselected
   client is deleted, and a modified one is kept and reported (Task 10).

## File Structure

| File                            | Responsibility after Part 1                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `src/lockfile.rs`               | Schema 2 types: `OwnedFile`, `LockedItem.files`, `LockedItem.pending`; load/save.                                      |
| `src/pipeline/owned.rs` (new)   | Hashing, file state classification, schema-1 adoption. No writes except adoption.                                      |
| `src/pipeline/plan.rs` (new)    | `PlannedFile`, `PlannedItem`: the in-memory result of generation for one item.                                         |
| `src/pipeline/apply.rs` (new)   | Preflight, pending save, temporary file + rename, stale deletion, final save.                                          |
| `src/pipeline/install.rs`       | Builds `PlannedItem` values. Writes nothing.                                                                           |
| `src/pipeline/output.rs`        | Path computation only. Task 8 deletes `write_output` and `copy_item_resources`; Task 10 deletes `remove_item_outputs`. |
| `src/pipeline/lifecycle.rs`     | `remove` and `update` use the record; `doctor` reports modified and missing files.                                     |
| `src/pipeline/mod.rs`           | `install_with_lockfile` calls plan, then apply. Alias relocation code is deleted.                                      |
| `src/legacy/mod.rs`             | Reduced to `pub(crate) mod layout; pub use layout::LegacyClient;`.                                                     |
| `tests/cli_ownership.rs` (new)  | CLI acceptance tests for record, removal, protection.                                                                  |
| `tests/cli_repeatable.rs` (new) | CLI acceptance tests for interruption and repeated runs.                                                               |

---

## Pull request A — delivery step 0: park unused code

### Task 1: Create the archive (operator task, no code)

This task changes the remote repository. Ask the user to confirm before each
push and before the ruleset. Do not delete any branch or worktree in this task.

- [ ] **Step 1: Fast-forward the main checkout.** In the main checkout, compare
      the uncommitted `docs/wip/README.md` change and the untracked
      `docs/wip/specs/2026-10-23-customization-storage-delivery-candidates.md`
      with `origin/main` (`git diff origin/main -- <path>`). If both are
      identical to `origin/main`, discard the local copies and run
      `git merge --ff-only origin/main`. If either differs, stop and show the
      difference to the user.
- [ ] **Step 2: Push the archive refs** (after confirmation):

```bash
git push origin origin/main:refs/heads/archive/lifecycle-2026-11
git push origin feat/native-delivery:refs/heads/archive/native-delivery
git push origin feat/native-config-contributions:refs/heads/archive/native-config-contributions
git push origin docs/q7-codex-agent-registration:refs/heads/archive/q7-codex-agent-registration
git push origin docs/q8-codex-skill-updates:refs/heads/archive/q8-codex-skill-updates
```

- [ ] **Step 3: Verify.** `git ls-remote origin 'refs/heads/archive/*'` lists
      five refs, and each commit equals the local branch tip
      (`git rev-parse <branch>`).
- [ ] **Step 4: Protect `archive/**`** (after confirmation). Create a repository
      ruleset named `archive-read-only` with target `refs/heads/archive/**`,
      enforcement `active`, and the rules `deletion`, `non_fast_forward` and
      `update`, with no bypass actor
      (`gh api repos/driftsys/upskill/rulesets -X POST --input -`).
- [ ] **Step 5: Verify.**
      `git push origin :refs/heads/archive/lifecycle-2026-11` is rejected by the
      ruleset.

### Task 2: Remove the unreachable modules

**Files:**

- Delete: `src/lifecycle/`, `src/filesystem/`, `src/filesystem.rs`,
  `src/operations/`, `src/files/`, `src/compile/`, `src/instructions/`,
  `src/source_manifest.rs`, `src/ownership.rs`
- Delete (34 test files): `tests/codex_instruction_boundary.rs`,
  `compile_items.rs`, `execution_backup.rs`, `file_permissions.rs`,
  `file_plans.rs`, `filesystem_destinations.rs`, `filesystem_scope_lock.rs`,
  `filesystem_sources.rs`, `instruction_contributions.rs`,
  `instruction_plans.rs`, every `tests/lifecycle_*.rs`, `operation_plans.rs`,
  `ownership_records.rs`, `recovery_plans.rs`, `source_manifests.rs`
- Delete: `tests/legacy_projection.rs`
- Modify: `src/lib.rs` (remove the 8 `pub mod` lines), `src/legacy/mod.rs`,
  `Cargo.toml`, `Cargo.lock`

**Interfaces:**

- Consumes: nothing.
- Produces:
  `crate::legacy::layout::{output_path, is_dir_backed,
  resource_base_path}`
  and `crate::legacy::LegacyClient` remain, unchanged.
  `crate::legacy::project_legacy` and the `Legacy*` projection types no longer
  exist; their only callers were in the deleted modules. This removal goes one
  step further than the design's list and is the author's decision under G8.

- [ ] **Step 1: Record the baseline.** Run `cargo test 2>&1 | tail -5` and note
      the count of passing test binaries.
- [ ] **Step 2: Delete the source paths and test files listed above.**
- [ ] **Step 3: Reduce `src/legacy/mod.rs`** to its module doc comment plus
      `pub(crate) mod layout;` and `pub use layout::LegacyClient;`. In
      `src/legacy/layout.rs`, delete `LegacyClient::from_label` if the compiler
      reports it as unused.
- [ ] **Step 4: Remove the `rustix` section from `Cargo.toml`**, the whole
      `[target.'cfg(any(target_os = "linux", target_os = "macos"))'.dependencies]`
      table. Run `cargo build` to update `Cargo.lock`.
- [ ] **Step 5: Verify.**

Run: `just check` Expected: PASS with zero warnings.
`grep -rn "rustix" src tests Cargo.toml` prints nothing.
`cargo tree | grep -c rustix` may still print a count from a transitive use by
`tempfile`; that is expected.

- [ ] **Step 6: Commit.**
      `refactor!: remove lifecycle library that no command can reach`

### Task 3: Make the documentation describe the remaining code

**Files:**

- Create: `docs/adr/0042-resteer-customization-delivery.md`
- Modify: `docs/adr/README.md`, `docs/adr/0015-*.md` to `docs/adr/0039-*.md`,
  `docs/design/*.md` (the 19 pages for removed modules), `docs/topics/0003-*.md`
  to `0009-*.md`, `docs/topics/README.md`,
  `docs/specification/specification.md`, `docs/book/commands.md`,
  `docs/roadmap.md`, `docs/wip/README.md`, `AGENTS.md`

**Constraints specific to this task** (from `scripts/check-docs.py`):

- `docs/compatibility.json` requires that 21 pages keep their existing heading
  anchors, and that listed assets keep their SHA-256. Do not delete or rename a
  `docs/design/` page, a heading in `docs/specification/*.md`, or a file under
  `docs/technotes/evidence/`. Add text; do not remove headings.
- `docs/book/**` stub pages include canonical files. Do not delete an included
  file.

- [ ] **Step 1: Write AD-0042** with these sections: Context (the facts of "Why
      this re-steer exists" in the design), Decision (ownership record in the
      lockfile; repeatable operations without journal, checkpoints or writer
      lock; support levels in place of activation gates; Claude Code and Copilot
      first; unused code parked on `archive/lifecycle-2026-11`), Consequences,
      and a list "Withdrawn records": AD-0015 to AD-0039. State that AD-0040 and
      AD-0041 stay accepted, and that AD-0041 is revisited only if trial T1
      shows a visible duplicate.
- [ ] **Step 2: Mark the withdrawn records.** Directly under the title of each
      of `docs/adr/0015` to `0039` and each `docs/design/` page that describes a
      removed module, insert this exact paragraph:

```markdown
> **Withdrawn on 2026-11-11 by
> [AD-0042](../adr/0042-resteer-customization-delivery.md).** The code this
> record describes was removed from `main` and is kept on the branch
> `archive/lifecycle-2026-11`. This record is retained as history.
```

    Adjust the relative link for files inside `docs/adr/`. Apply the same
    paragraph to `docs/topics/0004` to `0008`. For `docs/topics/0003` and
    `0009`, add the paragraph only above the sections that describe removed
    code.

- [ ] **Step 3: Correct the as-built documents.** In
      `docs/specification/specification.md`, keep every heading, and replace the
      body of each section that describes removed behaviour (section 4.7 and the
      other schema-2, journal and writer-lock passages) with one sentence that
      says the behaviour was withdrawn by AD-0042. In `docs/book/commands.md`,
      remove the passages that describe the opt-in lifecycle library. In
      `AGENTS.md`, remove the `rustix` bullet and the `source_manifest.rs`,
      `files/`, `operations/`, `compile/` and `lifecycle/` lines of the module
      layout, and change the `legacy/` line to "Frozen historical output
      layout".
- [ ] **Step 4: Update the planning documents.** In `docs/roadmap.md`, replace
      the gate sequence with the delivery order table of the design. In
      `docs/wip/README.md`, list the design and this plan as the first entries
      under "Active proposals", and mark the Gate 4, Gate 5, storage candidates
      and adapter roadmap entries as superseded by the design. Move the working
      documents of the removed approach
      (`docs/wip/plans/2026-10-11-instructions-rules-m1.md`,
      `2026-10-12-owned-artifact-records.md`,
      `2026-10-19-customization-delivery-reconciliation.md`,
      `2026-10-25-q11-cross-client-isolation.md`,
      `2026-10-30-q11b-native-name.md`, and the matching `docs/wip/specs/`
      files) to `docs/archive/` with `git mv`, and add them to
      `docs/archive/README.md`.
- [ ] **Step 5: Copy the trial records that the design cites.** Run
      `git diff --name-only origin/main archive/native-delivery -- docs/technotes scripts`.
      Copy with `git checkout archive/native-delivery -- <path>` only: the
      technotes for the 2026-10-18 staging-name trial and the 2026-10-25 Copilot
      isolation trial if they are absent from `main`, and
      `scripts/qualify-plugin-rules.py` and
      `scripts/qualify-native-instructions.py` if the archive versions are
      newer. If a copied note links to a file that is not on `main`, replace
      that link with a link to the file on the archive branch.
- [ ] **Step 6: Verify.**

Run: `just docs-check && just verify` Expected: PASS.

- [ ] **Step 7: Commit.**
      `docs: record the customization re-steer and withdraw lifecycle records`
- [ ] **Step 8: Open pull request A.** After it merges, ask the user to confirm
      the closing of debt issues #461, #343, #335, #330, #328, #316, #313, #311,
      #309, #308, #294, #292 and #281 with a comment that links
      `archive/lifecycle-2026-11`, and the deletion of the four archived source
      branches and their local worktrees.

---

## Pull request B — delivery step 1

### Task 4: Accept items without a `schema` field (issue #465)

**Files:**

- Modify: `src/model/common.rs` (the schema version type), `src/model/` item
  structs (`Skill`, `Rule`, `Agent`), `src/main.rs` (error printing only if the
  chain is not printed), `docs/specification/format-spec.md`
- Test: `tests/cli_add.rs`

**Interfaces:**

- Produces: `impl Default for SchemaVersion` returning version 1, and
  `#[serde(default)]` on the `schema` field of `Skill`, `Rule` and `Agent`.
  `Bundle.schema` stays required, because bundle discovery uses the field to
  recognise a bundle file.

- [ ] **Step 1: Write the failing tests** in `tests/cli_add.rs`:

```rust
#[test]
fn add_accepts_skill_without_schema_field() {
    // source/plain/SKILL.md:
    // "---\nname: plain\ndescription: A standard Agent Skills file.\n---\nBody\n"
    // run: add <source> plain --claude --project
    // assert success, and <project>/.claude/skills/plain/SKILL.md exists
}

#[test]
fn frontmatter_error_names_the_file() {
    // source/broken/SKILL.md: "---\nname: [unclosed\n---\n"
    // run: add <source> broken --claude --project
    // assert failure, and stderr contains "broken/SKILL.md"
    // (use the platform path separator)
}
```

- [ ] **Step 2: Run them.**
      `cargo test --test cli_add add_accepts_skill_without_schema_field frontmatter_error_names_the_file`
      Expected: the first FAILS with "missing field `schema`".
- [ ] **Step 3: Implement.** Add the default. If the second test fails, make
      `main.rs` print the full `anyhow` error chain for `add`.
- [ ] **Step 4: Run the tests.** Expected: PASS. Then `just check`.
- [ ] **Step 5: Update `format-spec.md`:** an item without `schema` is read as
      `schema: 1`; a bundle requires `schema`.
- [ ] **Step 6: Commit.** `fix(add): accept items without a schema field (#465)`

### Task 5: Trial T1 — duplicate visibility in Copilot (operator task)

Needs an authenticated Copilot CLI and makes paid model calls. Ask the user
before running it.

- [ ] **Step 1:** In a new temporary git repository, run
      `upskill add <fixture> --claude --copilot --project` for a fixture with
      one skill, one rule and one subagent. Each body contains a distinct random
      marker and the instruction to repeat the marker when asked.
- [ ] **Step 2:** In a new Copilot CLI session in that repository, record: the
      skill list, the agent list, and the answer to "Repeat every marker from
      your instructions". Record the Copilot CLI version.
- [ ] **Step 3:** Write `docs/technotes/t1-copilot-duplicate-visibility.md` with
      the version, the commands, and for each kind: listed once or twice,
      applied once or twice.
- [ ] **Step 4:** If any kind is listed or applied twice, stop and report to the
      user. The design requires the user's approval before the layout changes.
      Otherwise commit `docs: record trial T1 for Copilot duplicate visibility`
      and open pull request B.

---

## Pull request C — delivery step 2: ownership record and removal

### Task 6: Lockfile schema 2

**Files:**

- Modify: `src/lockfile.rs`
- Test: unit tests in `src/lockfile.rs`

**Interfaces:**

- Produces:

```rust
pub const CURRENT_SCHEMA: u32 = 2;
/// File kept beside the lockfile by the adoption step.
pub const SCHEMA1_BACKUP_NAME: &str = ".upskill-lock.json.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnedFile {
    /// `claude` | `copilot` | `opencode`.
    pub client: String,
    /// Relative to the scope root, with `/` separators on every platform.
    pub path: String,
    /// Lowercase hex SHA-256 of the bytes written.
    pub sha256: String,
}

// New LockedItem fields, both omitted from JSON when empty:
pub files: Vec<OwnedFile>,
pub pending: Vec<OwnedFile>,
```

`Lockfile::load` accepts `schema` 1 and 2 and returns the value it read. It
rejects `schema` 0 and any value above 2 with the existing messages.
`Lockfile::new()` uses schema 2. `Lockfile::save` writes what `self.schema`
holds. `pub fn needs_adoption(&self) -> bool` is true when `schema == 1`.

- [ ] **Step 1: Write the failing unit tests:**
      `load_accepts_schema_1_and_reports_needs_adoption`,
      `load_accepts_schema_2_with_files_and_pending` (round trip of one item
      with two `files` and one `pending` entry), `load_rejects_schema_3`,
      `item_without_files_omits_the_keys` (serialized JSON contains neither
      `"files"` nor `"pending"`). Change `new_lockfile_has_current_schema` to
      expect 2.
- [ ] **Step 2: Run** `cargo test --lib lockfile`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement.** Every existing construction of `LockedItem` gains
      `files: Vec::new(), pending: Vec::new()`.
- [ ] **Step 4: Run** `cargo test --lib lockfile`. Expected: PASS. CLI tests
      that assert `"schema": 1` in a freshly written lockfile now fail; change
      those assertions to 2.
- [ ] **Step 5: Commit.**
      `feat(lockfile): add schema 2 with per-item file records`

### Task 7: File state classification and schema-1 adoption

**Files:**

- Create: `src/pipeline/owned.rs`
- Modify: `src/pipeline/mod.rs` (declare `mod owned;`)
- Test: unit tests in `src/pipeline/owned.rs`

**Interfaces:**

- Consumes: `OwnedFile`, `Lockfile::needs_adoption`, `SCHEMA1_BACKUP_NAME` (Task
  6); `super::output::{output_path, resource_base_path, is_dir_backed}`.
- Produces:

```rust
pub(super) fn sha256_hex(bytes: &[u8]) -> String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileState { Absent, Owned, Modified, Unowned }

/// `recorded` holds every hash that counts as upskill's own for this path:
/// the current record and, when present, the pending record.
pub(super) fn classify(target: &Path, rel: &str, recorded: &[&str]) -> Result<FileState>;

/// Portable form used in `OwnedFile.path`.
pub(super) fn rel_to_record(rel: &Path) -> String;

/// One-time step for a schema-1 lockfile. Copies the lockfile bytes to
/// `SCHEMA1_BACKUP_NAME`, records for every item each existing file at the
/// item's layout paths with its current hash, sets `schema` to 2 and saves.
/// Returns `None` and does nothing when `lock.needs_adoption()` is false.
/// Otherwise returns the recorded paths, sorted, for the adoption notice.
pub(super) fn adopt_schema1(target: &Path, lock: &mut Lockfile) -> Result<Option<Vec<String>>>;
```

`classify` rules: path missing → `Absent`; present and `recorded` is empty →
`Unowned`; present and its hash is in `recorded` → `Owned`; otherwise
`Modified`.

Adoption paths for one item, for each client in `item.clients` (all three when
the list is empty): the entry file `output_path(kind, client, name)`, plus every
regular file below `resource_base_path(kind, client, name)`. A path that does
not exist is not recorded. Adoption cannot tell a manual file inside an item
directory from a file an earlier version wrote, so it records both; this limit
goes into the user guide in Task 9.

- [ ] **Step 1: Write the failing unit tests:** `classify_absent`,
      `classify_unowned_when_nothing_recorded`, `classify_owned_on_equal_hash`,
      `classify_owned_on_pending_hash` (two recorded hashes, file equals the
      second), `classify_modified_on_different_hash`,
      `adoption_records_existing_files_and_keeps_backup` (schema-1 lockfile with
      one Claude skill that has a `scripts/run.sh` resource and one Claude rule;
      after the call: schema 2, three `files` entries with correct hashes,
      backup file byte-identical to the original lockfile),
      `adoption_skips_missing_outputs` (item recorded, no file on disk: `files`
      is empty, result is `Ok(Some(vec![]))`),
      `adoption_is_a_no_op_on_schema_2`.
- [ ] **Step 2: Run** `cargo test --lib pipeline::owned`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit.**
      `feat(pipeline): classify installed files and adopt schema-1 lockfiles`

### Task 8: Plan every file before writing, and record what was written

**Files:**

- Create: `src/pipeline/plan.rs`, `src/pipeline/apply.rs`,
  `tests/cli_ownership.rs`
- Modify: `src/pipeline/install.rs`, `src/pipeline/mod.rs`,
  `src/pipeline/output.rs`, `src/pipeline/report.rs`, `src/lockfile.rs`
  (`items_from_report`)

**Interfaces:**

- Consumes: Task 6 and Task 7 interfaces.
- Produces:

```rust
// src/pipeline/plan.rs
#[derive(Debug, Clone)]
pub(super) struct PlannedFile {
    pub client: Client,
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    /// Unix permission bits of a copied resource; `None` for generated files.
    pub mode: Option<u32>,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedItem {
    pub kind: ItemKind,
    /// Installed name: the alias when the user passed `--as`.
    pub name: String,
    pub group: Option<String>,
    pub source_hash: Option<String>,
    pub files: Vec<PlannedFile>,
}

// src/pipeline/apply.rs
/// Writes every planned file and updates `lock.items[..].files` for the
/// planned items. In this task it writes with `fs::write`, and before writing
/// an item it deletes the files of that item's previous record that the plan
/// no longer contains. It does not save the lockfile.
pub(super) fn apply_plan(target: &Path, lock: &mut Lockfile, planned: &[PlannedItem]) -> Result<()>;
```

`InstallReport` gains `pub(crate) planned: Vec<PlannedItem>`. `InstalledItem` is
unchanged, so `main.rs` printing is unchanged.

`install_items_of_kind` gains the parameter `aliases: &BTreeMap<String, String>`
(source name → installed name), builds one `PlannedItem` per item with the alias
already applied to the name, the output paths and the resource link rewrite,
pushes it to `report.planned`, and writes nothing. The functions `relocate` and
`relocate_aliased_output` in `mod.rs`, and `write_output`, `copy_item_resources`
in `output.rs`, are deleted together with their unit tests.
`remove_item_outputs` in `output.rs` is not deleted in this task, because
`remove` and `update` in `src/pipeline/lifecycle.rs` still call it. Task 9 stops
using it in `remove`; Task 10 removes the last use in `update` and deletes the
function with its unit tests.

`install_with_lockfile` order: fetch, resolve, detect name conflicts, plan all
items, `adopt_schema1`, `apply_plan`, upsert items, save lockfile, ancillary
files. The public `install_from_local_path` keeps its signature and calls
`apply_plan` with a new empty `Lockfile`.

- [ ] **Step 1: Write the failing CLI tests** in `tests/cli_ownership.rs`:

```rust
#[test]
fn add_records_written_files_with_hashes() {
    // add the fixture skill `create-api-endpoint` with --claude --copilot --project
    // lockfile: schema == 2; the item has a `files` entry for
    // ".claude/skills/create-api-endpoint/SKILL.md" (client "claude") and for
    // ".github/skills/create-api-endpoint/SKILL.md" (client "copilot");
    // each `sha256` equals the SHA-256 of the file on disk.
}

#[test]
fn add_records_resource_files() {
    // skill with scripts/run.sh: `files` contains
    // ".claude/skills/<name>/scripts/run.sh"
}

#[test]
fn alias_install_records_alias_paths() {
    // add ... --as create-api-endpoint=make-endpoint --claude
    // `files` paths start with ".claude/skills/make-endpoint/";
    // no directory ".claude/skills/create-api-endpoint" exists.
}

#[test]
fn first_change_adopts_a_schema_1_lockfile() {
    // write a schema-1 lockfile and matching output files by hand, then run
    // `add` of a second item: lockfile schema == 2, the first item has `files`,
    // and ".upskill-lock.json.v1" holds the original bytes.
}
```

- [ ] **Step 2: Run** `cargo test --test cli_ownership`. Expected: FAIL (no
      `files` key).
- [ ] **Step 3: Implement** the plan types, the planning change in `install.rs`,
      `apply_plan`, and the `items_from_report` change that copies `files` into
      each `LockedItem`.
- [ ] **Step 4: Run** `just check`. Expected: PASS, including every existing
      `cli_*`, `pipeline_*` and `generate_*` test without changes to their
      assertions about output content.
- [ ] **Step 5: Commit.**
      `feat(add): plan all outputs before writing and record them in the lockfile`

### Task 9: `remove` deletes recorded files only

**Files:**

- Modify: `src/pipeline/lifecycle.rs` (`remove`, `doctor`),
  `src/pipeline/report.rs`, `src/cli.rs`, `src/main.rs`,
  `docs/book/commands.md`, `docs/specification/specification.md`
- Test: `tests/cli_ownership.rs`, `tests/cli_doctor.rs`

**Interfaces:**

- Consumes: `classify`, `adopt_schema1` (Task 7).
- Produces:

```rust
pub fn remove(target: &Path, filter: RemoveFilter, force: bool) -> Result<RemoveReport>;

// RemovedItem gains:
pub kept_modified: Vec<PathBuf>,

// DoctorReport gains:
pub modified_files: Vec<ModifiedFile>,   // { kind, name, files: Vec<PathBuf> }
```

`remove` behaviour per recorded file: `Owned` → delete; `Modified` → keep and
add to `kept_modified`, or delete when `force` is true; `Absent` → ignore. After
the files of an item are handled, remove each parent directory that is now
empty, from the deepest upward, and stop at the first directory that is not
empty or that is a client root (`.claude`, `.github`, `.agents`, `.opencode`).
`remove` no longer calls `remove_item_outputs`. The lockfile record of the item
is dropped in all cases. `remove` gains the flag `--force`. `main.rs` prints one
line per kept file: `kept modified file: <path>`. `doctor` fills
`missing_outputs` from the record (state `Absent`) and `modified_files` from
state `Modified`.

- [ ] **Step 1: Write the failing CLI tests:**

```rust
#[test]
fn remove_keeps_a_manual_file_in_the_skill_directory() {
    // add skill; write ".claude/skills/<name>/NOTES.md" by hand; remove <name>
    // SKILL.md is gone; NOTES.md exists; the directory exists.
}

#[test]
fn remove_deletes_empty_item_directories_but_not_client_roots() {
    // add skill --claude; remove: ".claude/skills/<name>" is gone;
    // ".claude" still exists when it holds another file.
}

#[test]
fn remove_keeps_a_modified_file_and_reports_it() {
    // add rule --claude; append a line to ".claude/rules/<name>.md"; remove
    // file exists; stderr or stdout contains "kept modified file";
    // the lockfile no longer lists the item.
}

#[test]
fn remove_force_deletes_a_modified_file() { /* same setup, --force */ }

#[test]
fn remove_cleans_an_install_made_by_schema_1() {
    // hand-written schema-1 lockfile plus output files; remove <name>
    // output files are gone.
}
```

    In `tests/cli_doctor.rs`: `doctor_reports_modified_files` (edit an
    installed file; `doctor --json` lists it under `modified_files`).

- [ ] **Step 2: Run** `cargo test --test cli_ownership --test cli_doctor`.
      Expected: the new tests FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `just check`. Expected: PASS.
      `grep -rn "remove_dir_all" src/pipeline` shows a call only inside
      `remove_item_outputs` in `output.rs`, which Task 10 deletes.
- [ ] **Step 5: Update the docs:** `commands.md` (`remove --force`, the kept
      file message, the adoption step and its limit for manual files inside an
      item directory) and `specification.md` (lockfile schema 2, the three file
      states).
- [ ] **Step 6: Commit** `feat(remove): delete only recorded, unchanged files`
      and open pull request C.

---

### Adoption notice (applies to Tasks 8 and 9)

Adoption cannot tell a file that the user edited or added before the upgrade
from a file that an earlier upskill version wrote. It records all of them as
unchanged, so a later `update` may replace them and `remove` may delete them.
The user must be told at the moment this happens.

- `InstallReport`, `RemoveReport` and `UpdateReport` each gain
  `pub adopted: Option<Vec<String>>`, filled from `adopt_schema1`.
- When the value is `Some` and not empty, `main.rs` prints to stderr:

```text
note: adopted <N> files installed by an earlier upskill version as unchanged.
Manual edits made to these files before now are not protected:
  <path>
  ...
The previous lockfile is kept as .upskill-lock.json.v1.
```

- CLI test in `tests/cli_ownership.rs`: `adoption_prints_the_adopted_files_once`
  (the first `add` on a schema-1 lockfile prints the note and each path; a
  second `add` prints no note).
- `commands.md` states both cases: generated files edited before the upgrade,
  and files added inside an item directory.

---

## Pull request D — delivery step 3

### Task 10: `add` and `update` protect modified and unowned files

**Files:**

- Modify: `src/pipeline/apply.rs`, `src/pipeline/mod.rs` (`AddOptions`),
  `src/pipeline/lifecycle.rs` (`update`), `src/pipeline/report.rs`,
  `src/cli.rs`, `src/main.rs`, `docs/book/commands.md`,
  `docs/specification/specification.md`
- Test: `tests/cli_ownership.rs`, `tests/cli_update.rs`

**Interfaces:**

- Produces:

```rust
// AddOptions gains:
/// Replace modified and unowned files. Set by `--force` on `add` and `update`.
pub overwrite: bool,

// src/pipeline/apply.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConflict {
    pub kind: ItemKind,
    pub name: String,
    pub path: PathBuf,
    pub state: ConflictState,   // Modified | Unowned
}

#[derive(Debug, Default)]
pub(super) struct ApplyOutcome {
    /// Items that were not written because of a conflict.
    pub skipped: Vec<FileConflict>,
}

pub(super) fn apply_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
    overwrite: bool,
) -> Result<ApplyOutcome>;

// InstallReport gains:
pub conflicts: Vec<FileConflict>,

// UpdateStatus gains:
SkippedModified { files: Vec<PathBuf> },
```

`apply_plan` checks every destination of every planned item before it writes
anything. A destination whose content already equals the planned bytes is not a
conflict in any state. An item with at least one `Modified` or `Unowned`
destination is skipped as a whole when `overwrite` is false: none of its files
is written, and its lockfile record is unchanged. Other items are written. A
file of the previous record that the new plan does not contain is deleted when
`Owned`, and kept and reported when `Modified`.

`AddOptions.force` keeps its current meaning (replace an item of the same name
from another source). `update` keeps `force: true` and takes `overwrite` from a
new `--force` flag. `add --force` sets both. With conflicts, `add` exits with
code 1 after writing the items without conflict, and `main.rs` prints one line
per conflict: `skipped <kind> <name>: <path> was modified` or
`... exists and is not managed by upskill`, followed by
`Use --force to replace.`. `update` reports the item with `SkippedModified` and
exits with code 1.

The public `install_from_local_path` passes `overwrite = true` to `apply_plan`,
because it is a library entry without a lockfile and keeps its current replace
behaviour.

- [ ] **Step 1: Write the failing CLI tests:**

```rust
// tests/cli_ownership.rs
#[test]
fn add_refuses_an_unowned_file_at_the_destination() {
    // write ".claude/rules/<name>.md" by hand; add the rule --claude
    // exit code 1; the file keeps the manual bytes; lockfile has no such item.
}
#[test]
fn add_accepts_an_unowned_file_with_identical_content() { /* exit 0, recorded */ }
#[test]
fn add_force_replaces_an_unowned_file() { /* exit 0, generated bytes */ }
#[test]
fn a_conflict_on_one_item_does_not_block_another_item() {
    // source with rule A and skill B; unowned file at A's path; add both
    // B is installed and recorded; A is skipped; exit code 1.
}

// tests/cli_update.rs
#[test]
fn update_keeps_a_modified_file_and_skips_the_item() {
    // add from a local source; edit the installed file; change the source;
    // update: installed file keeps the edit; output names the file;
    // lockfile hash of the item is unchanged; exit code 1.
}
#[test]
fn update_force_replaces_a_modified_file() { /* new source content on disk */ }
#[test]
fn update_deletes_a_file_the_new_version_no_longer_contains() {
    // v1 has scripts/a.sh, v2 does not: after update the file and the empty
    // "scripts" directory are gone.
}
#[test]
fn update_with_a_narrower_selection_deletes_the_owned_copilot_file() {
    // add --claude --copilot; update --claude:
    // ".github/skills/<name>/SKILL.md" is gone; `files` has no copilot entry.
}
#[test]
fn update_with_a_narrower_selection_keeps_a_modified_copilot_file() {
    // same, with the Copilot file edited first: the file exists and is reported.
}
```

- [ ] **Step 2: Run** `cargo test --test cli_ownership --test cli_update`.
      Expected: the new tests FAIL.
- [ ] **Step 3: Implement.** Remove the per-client clean-up loop at the end of
      `install_items_of_kind`; the stale deletion in `apply_plan` replaces it.
      Remove the last use of `remove_item_outputs` in `update` in
      `src/pipeline/lifecycle.rs`, then delete `remove_item_outputs` from
      `src/pipeline/output.rs` together with its unit tests.
- [ ] **Step 4: Run** `just check`. Expected: PASS.
      `grep -rn "remove_dir_all" src/pipeline` prints nothing.
- [ ] **Step 5: Update `commands.md` and `specification.md`** with the conflict
      rules, the messages and the exit codes.
- [ ] **Step 6: Commit**
      `feat(update): keep modified files and refuse unowned destinations` and
      open pull request D.

---

## Pull request E — delivery step 4

### Task 11: Repeatable operations

**Files:**

- Modify: `src/pipeline/apply.rs`, `src/pipeline/mod.rs`,
  `src/pipeline/lifecycle.rs` (`doctor`), `docs/book/commands.md`,
  `docs/specification/specification.md`
- Create: `tests/cli_repeatable.rs`

**Interfaces:**

- Consumes: `LockedItem.pending` (Task 6), `classify` with several recorded
  hashes (Task 7).
- Produces:

```rust
/// Saves the lockfile twice: once with `pending` set for every planned item
/// before the first write, and once after the last write with `pending`
/// promoted to `files` and cleared.
pub(super) fn apply_plan(
    target: &Path,
    lock: &mut Lockfile,
    planned: &[PlannedItem],
    overwrite: bool,
) -> Result<ApplyOutcome>;
```

Order inside `apply_plan`: (1) preflight as in Task 10, where the recorded
hashes of a path are its `files` hash plus its `pending` hash; (2) for each item
that passed, set `pending` to the planned files and, for an item not yet in the
lockfile, insert it with empty `files`; save; (3) write each file to
`<destination directory>/.upskill-stage-<16 hex digits>`, set `mode` when it is
`Some`, then `fs::rename` to the destination; on a write error, delete the
temporary file and return the error; (4) delete stale owned files; (5) promote
and save. The other lockfile changes of `install_with_lockfile` (bundles,
plugins, MCP) are saved after step 5 as today.

`remove` treats `pending` paths as recorded paths. `doctor` reports an item with
a non-empty `pending` as `interrupted`, with the text
`run the same command again to complete it`.

- [ ] **Step 1: Write the failing CLI tests** in `tests/cli_repeatable.rs` (the
      tests that change permissions carry `#[cfg(unix)]`):

```rust
#[test]
fn add_completes_on_a_second_run_after_a_failed_write() {
    // create ".github/skills" and make it read-only (0o555)
    // add skill --claude --copilot  -> fails
    // lockfile: the item exists with non-empty `pending`
    // restore 0o755; run the same add again, without --force -> success
    // both files exist; `pending` is absent; `files` has two entries.
}

#[test]
fn update_completes_on_a_second_run_after_a_failed_write() {
    // add v1 for both clients; change the source to v2;
    // make ".github/skills/<name>" read-only; update -> fails
    // Claude file has v2 bytes, Copilot file has v1 bytes
    // restore; update again without --force -> success, both files v2,
    // no SkippedModified status.
}

#[test]
fn a_failed_run_leaves_no_stage_file() {
    // after the failed run above, no path below the project root has a file
    // name that starts with ".upskill-stage-".
}

#[test]
fn resource_keeps_the_executable_bit() {
    // skill with scripts/run.sh mode 0o755: installed copy has 0o755.
}

#[test]
fn doctor_reports_an_interrupted_item() {
    // after the failed add: `doctor` output contains "interrupted".
}

#[test]
fn remove_cleans_an_interrupted_install() {
    // after the failed add: remove <name> deletes the Claude file and the
    // lockfile entry.
}
```

- [ ] **Step 2: Run** `cargo test --test cli_repeatable`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `just check`, then `just verify`. Expected: PASS.
- [ ] **Step 5: Update `commands.md` and `specification.md`:** the order of an
      operation, the `pending` record, and the statement that two upskill
      processes on one scope at the same time are not supported.
- [ ] **Step 6: Commit**
      `feat: complete an interrupted install or update on the next run` and open
      pull request E.

---

## After Part 1

Write the next plan for delivery step 5 (scope reporting, both-scope `list` and
`doctor`, global output paths). Its first task confirms the documented global
paths of Claude Code and Copilot CLI.
