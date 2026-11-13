# Customization Re-steer Part 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Report available updates with `upskill update --check`, publish a
support level for each client and kind backed by a smoke test against the real
client, and report available updates once at session start through a hook plugin
for Claude Code and one for Copilot CLI.

**Architecture:** The lockfile records the commit that each git source resolved
to at install time. A new library module `src/pipeline/check.rs` compares that
commit with the result of `git ls-remote` (through a trait, so that unit tests
need no git) and compares the item hash for a local path source; it reads both
scopes and changes no file. A new module `src/support.rs` holds the support
levels as a constant table that `doctor` prints. A new module `src/notice.rs`
wraps the check with a cache file, so that a session-start hook prints one line
only when the set of available updates changes.

**Tech Stack:** Rust edition 2024 (MSRV 1.85), `serde`, `serde_json`, `anyhow`,
`assert_cmd` + `tempfile` for CLI tests; Python 3 standard library for the smoke
test. No new dependency.

**Spec:**
[docs/wip/specs/2026-11-11-customization-resteer-design.md](../specs/2026-11-11-customization-resteer-design.md),
sections 4 and 6, goals G5 to G7. This plan is delivery steps 6, 7 and 8. Each
step is one pull request and is releasable.

## Global Constraints

- Single static binary. No async runtime, no `git2`, no new dependency.
- `main.rs` performs I/O orchestration only. Only `main.rs` writes to stdout and
  stderr. Library code returns data or `Result<T>`.
- Zero warnings: compiler, clippy (`-D warnings`), docs tooling. Run
  `just sanity` before a push; `just verify` does not lint test code.
- Acceptance tests come first and run the binary through
  `common::upskill_cmd(&home)`. Never use a raw `Command::cargo_bin`.
- Pre-1.0: no compatibility layer. JSON output changes shape without a fallback.
- The two scopes are named **project** and **global** in all output and docs.
- `update --check` and the notice mode change no file under a scope root. The
  notice mode writes only its cache file.
- upskill never injects credentials. `git ls-remote` uses the bare URL, as
  `git clone` does today.
- A failed or outdated smoke cell changes the level of that cell only. It does
  not block a release and does not affect another cell.
- The smoke test needs authenticated clients and makes paid model calls. Hosted
  CI does not run it. Ask the user before each run.
- Comments, commit messages, docs and pull request text use plain, literal
  English.
- One worktree and one branch for each pull request, under
  `.claude/worktrees/<branch>`; run `./bootstrap` after creating it. Every Write
  and Edit uses the absolute path that starts with the worktree directory.
  Before each pull request: `just fmt`, then `just verify`, then `just sanity`.
  Conventional Commits. Squash merge.
- Each pull request starts from `origin/main` after the previous one is merged.
  The whole-branch review (`/review` passes 1 and 2) runs before each pull
  request.
- The design and the plans stay in `docs/wip/` until Task 13.

## Decisions made by this plan

The design leaves these points open. Each one is the author's decision and is
listed for the user to check before the implementation starts.

1. **Lockfile field**: `LockedItem.commit`, the 40-character commit that the
   source of the item resolved to. Absent for a local path source. The lockfile
   stays at `schema: 2`; the field is optional.
2. **An item without a recorded commit** (installed before step 6) is reported
   by `update --check` as `no recorded commit` with the hint to run
   `upskill update` once. It is not counted as an update.
3. **`update` records the commit of every source that it fetches and does not
   decline**: an applied source, and a source that is not applied because it has
   no changes. An entry whose item is withheld (a skipped item) keeps its
   recorded commit. A source that the user declines, and a dry run, record
   nothing. Without this, a remote commit that changes no item would be reported
   by `update --check` on every run.
4. **A changed commit lists every item of that git source.** The check does not
   fetch, so it cannot tell which items the new commit changes. `update` then
   reports the unchanged items as up to date.
5. **Time limit**: 10 seconds in total for `update --check`, 5 seconds for the
   notice mode. No flag. Sources are checked one after the other; a source that
   is not reached before the limit is reported as `not checked`.
6. **Exit code of `update --check`**: 0 when the check ran, also when updates
   exist and when a source was not reachable. 2 for a usage error.
7. **`update --check` reports both scopes**, as `list` and `doctor` do.
   `--global` or `--project` limits it to one scope. Item names limit it to
   those items. It cannot be combined with `--dry-run`, `--yes`, `--force` or a
   client flag.
8. **No `--json` for the check.** The notice mode calls the library directly.
9. **Support levels are one level for each client and kind.** A cell is
   `supported` when its smoke test passed at both scopes. The table holds the
   client version and the date of the run.
10. **Codex has no client flag today**, so no command refuses it: the level
    `unsupported` is printed by `doctor` and the user guide only.
11. **The smoke test is a new mode `--smoke` of
    `scripts/qualify-native-instructions.py`.** It writes nothing to the
    repository; it prints one result line for each cell. The maintainer edits
    the table in `src/support.rs`.
12. **The global smoke cells use the real home directory of the maintainer**,
    because an authenticated client reads its credentials and its global content
    from there. They run only with `--allow-real-home`, install items with a
    random name `upskill-smoke-<hex>`, and remove them with
    `upskill remove --global --yes` in a `finally` block.
13. **Notice flag**: `upskill update --notice`. Network period: 24 hours.
14. **Cache file**: `notice.json` in `$XDG_CACHE_HOME/upskill/`, or
    `~/.cache/upskill/` when the variable is not set. This is the parent of the
    directory that `index::cache_dir()` returns.
15. **The notice prints one line for each scope that has updates.** The line for
    the global scope ends with `Run "upskill update --global".`
16. **Release version**: decided by `git std bump` from the commit history.

## Review Focus

1. A remote commit that changes no item: after `upskill update --yes`,
   `update --check` reports no update for that source (Task 2).
2. An item that the user edited, whose source content did not change but whose
   source commit did: `update` keeps the edited file with a note and does not
   report the item as skipped (Task 2).
3. A source pinned to an annotated tag: the recorded commit and the `ls-remote`
   result are both the commit that the tag points to, not the tag object, so the
   check reports no update (Task 3).
4. `git ls-remote` against a host that asks for a password: the check does not
   wait for input and reports the source as not reachable (Task 3).
5. The notice mode with a cache file that is not valid JSON, or a cache
   directory that cannot be written: it prints nothing more than a correct
   notice and exits with code 0 (Task 10).

## File Structure

| File                                       | Responsibility after this plan                                                              |
| ------------------------------------------ | ------------------------------------------------------------------------------------------- |
| `src/lockfile.rs`                          | `LockedItem.commit`; `SourceStamp`.                                                         |
| `src/fetch.rs`                             | `head_commit`, `ls_remote`, `run_until`. The only place that starts `git`.                  |
| `src/pipeline/check.rs` (new)              | `RemoteLookup`, `check_scopes` and the check report types. Reads lockfiles; writes nothing. |
| `src/pipeline/lifecycle.rs`                | `update` records the commit of each fetched source.                                         |
| `src/support.rs` (new)                     | `SupportLevel`, `SupportCell`, `SUPPORT`. No I/O.                                           |
| `src/notice.rs` (new)                      | Cache file format, the decision whether to print, the notice lines.                         |
| `src/cli.rs`, `src/main.rs`                | `--check` and `--notice` on `update`; printing of the check, the levels and the notice.     |
| `scripts/qualify-native-instructions.py`   | `--smoke` mode.                                                                             |
| `tests/cli_update_check.rs` (new)          | Acceptance tests of step 6.                                                                 |
| `tests/cli_support_levels.rs` (new)        | Acceptance tests of step 7.                                                                 |
| `tests/cli_update_notice.rs` (new)         | Acceptance tests of step 8.                                                                 |
| `scripts/tests/test_smoke_native.py` (new) | Offline tests of the smoke mode.                                                            |

---

## Pull request A — delivery step 6: recorded commits and `update --check`

Worktree `.claude/worktrees/feat-update-check`, branch `feat/update-check`.

Test file header for `tests/cli_update_check.rs`, shared by Tasks 1 to 4. Copy
the git fixture of `tests/cli_legacy_source_labels.rs` (the struct with `new`,
`write_skill`, `git`, `commit`, `cmd_at`): it creates a local repository and
maps `https://example.invalid/team/skills.git` to it with `GIT_CONFIG_*`
variables and `url.file://<repo>.insteadOf`.

```rust
//! ATDD tests for the recorded source commit and `update --check`
//! (re-steer design, section 4).
mod common;

const REMOTE: &str = "https://example.invalid/team/skills";

/// `home`, `repo` (holds `.git`, the project scope), `outside` (not in a git
/// repository) and `upstream` (the local repository behind `REMOTE`).
struct Env { /* tmp, home, repo, outside, upstream */ }
fn env() -> Env;                       // upstream holds skills `alpha` and `beta`, one commit
fn run(e: &Env, cwd: &Path, args: &[&str]) -> std::process::Output; // with the GIT_CONFIG_* mapping
fn upstream_head(e: &Env) -> String;   // `git rev-parse HEAD` in upstream
fn advance(e: &Env, path: &str, body: &str) -> String; // write, commit, return the new HEAD
fn lock(root: &Path) -> serde_json::Value;
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>>;    // every file under root, with bytes
fn stdout(out: &std::process::Output) -> String;
fn stderr(out: &std::process::Output) -> String;
```

### Task 1: Record the resolved commit on `add`

**Files:**

- Create: `tests/cli_update_check.rs`
- Modify: `src/lockfile.rs`, `src/fetch.rs`, `src/pipeline/git.rs`,
  `src/pipeline/prepared.rs`, `src/pipeline/install.rs`, `src/pipeline/apply.rs`
- Test: `tests/cli_update_check.rs`, unit tests in `src/fetch.rs`

**Interfaces:**

- Produces, in `src/lockfile.rs`:

  ```rust
  // on LockedItem, after `git_ref`:
  /// Commit that the source resolved to when the item was installed or last
  /// updated. Absent for a local path source and for an item installed
  /// before this field existed.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub commit: Option<String>,

  /// Where an item came from: the source label, its pinned ref, and the
  /// commit that the fetch resolved.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct SourceStamp { pub label: String, pub git_ref: Option<String>, pub commit: Option<String> }
  ```

  `items_from_report` takes `source_for: impl Fn(ItemKind, &str) -> SourceStamp`
  in place of the `(String, Option<String>)` pair.

- Produces, in `src/fetch.rs`:

  ```rust
  /// `git rev-parse HEAD` in `dir` (the clone or a directory inside it).
  pub(crate) fn head_commit(dir: &Path) -> Result<String, String>;
  ```

- Produces, in `src/pipeline/git.rs`: `fetch_ssot` returns a struct in place of
  the pair, and every caller is updated:

  ```rust
  pub struct FetchedSource {
      pub root: PathBuf,
      pub guard: Option<tempfile::TempDir>,
      /// `None` for a local path source.
      pub commit: Option<String>,
  }
  pub fn fetch_ssot(source: &InstallSource) -> Result<FetchedSource>;
  ```

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn add_records_the_commit_of_a_git_source() {
      let e = env();
      let out = run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      assert!(out.status.success(), "{}", stderr(&out));
      let head = upstream_head(&e);
      for item in lock(&e.repo)["items"].as_array().unwrap() {
          assert_eq!(item["commit"], head.as_str(), "{item}");
      }
  }

  #[test]
  fn add_records_no_commit_for_a_local_path_source() {
      let e = env();
      let out = run(&e, &e.repo, &["add", e.upstream.to_str().unwrap(), "--claude"]);
      assert!(out.status.success(), "{}", stderr(&out));
      for item in lock(&e.repo)["items"].as_array().unwrap() {
          assert!(item.get("commit").is_none(), "{item}");
      }
  }
  ```

  Unit test in `src/fetch.rs`, with the existing `make_test_repo`:
  `head_commit_of_a_clone_is_the_commit_of_the_source` (clone with
  `shallow_clone`, assert `head_commit(clone)` equals `git rev-parse HEAD` of
  the source and has 40 characters) and
  `head_commit_of_an_annotated_tag_clone_is_the_tagged_commit` (create
  `git tag -a v1 -m v1`, clone with `git_ref = Some("v1")`, assert the result
  equals `git rev-parse v1^{commit}` of the source).

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --test cli_update_check`. Expected: the first test fails
  because `commit` is absent.

- [ ] **Step 3: Implement**

  `head_commit` in `src/fetch.rs`. `clone_to_tempdir` calls it on the clone
  directory and fills `FetchedSource.commit`. In `src/pipeline/prepared.rs` and
  `src/pipeline/install.rs`, every struct that holds a `git_ref` of a fetched
  source (the fields at `prepared.rs:35`, `install.rs:321` and `install.rs:581`
  on the baseline) gains `commit: Option<String>` beside it, filled from the
  same fetch; a cross-source dependency records the commit of its own source.
  The closure passed to `items_from_report` returns a `SourceStamp`.

  In `src/pipeline/apply.rs`, `same_record` also clears `commit`, and its doc
  comment names the field: a new commit of a source whose item content is equal
  is not a change of the item.

- [ ] **Step 4: Run `cargo test` — all pass. Commit**

  `feat: record the commit of a git source in the lockfile`

---

### Task 2: `update` records the commit of each fetched source

**Files:**

- Modify: `src/pipeline/lifecycle.rs` (`update`), `src/pipeline/report.rs` (doc
  comments of `UpdateStatus::UpToDate` and `SourcePreview::has_changes`)
- Test: `tests/cli_update_check.rs`

**Interfaces:**

- Consumes: `FetchedSource.commit`, `LockedItem.commit` (Task 1).
- Produces: after `update`, every lockfile entry of a source that was applied,
  or that was not applied because it has no changes, holds the fetched commit,
  except an entry whose item is withheld (a skipped item), which keeps its
  recorded commit. The lockfile is saved once more for a source without changes
  only when a commit value differs. `UpdateReport.changed` and
  `SourcePreview::has_changes` do not count this write.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn update_records_a_new_commit_that_changes_no_item() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      let head = advance(&e, "README.md", "unrelated\n");
      let before = tree(&e.repo.join(".claude"));
      let out = run(&e, &e.repo, &["update", "--yes"]);
      assert!(out.status.success(), "{}", stderr(&out));
      assert_eq!(tree(&e.repo.join(".claude")), before);
      for item in lock(&e.repo)["items"].as_array().unwrap() {
          assert_eq!(item["commit"], head.as_str());
      }
  }

  #[cfg(unix)]
  #[test]
  fn a_terminal_update_without_changes_records_the_commit_and_asks_nothing() {
      // common::update_in_terminal_with_env with input "" and the GIT_CONFIG_*
      // mapping as the environment.
      // Assert: returncode 0, stdout holds no "Apply?", every item has the new commit.
  }

  #[test]
  fn update_keeps_an_edited_file_when_only_the_commit_changed() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      let edited = e.repo.join(".claude/skills/alpha/SKILL.md");
      std::fs::write(&edited, "edited by the user\n").unwrap();
      advance(&e, "README.md", "unrelated\n");
      let out = run(&e, &e.repo, &["update", "--yes"]);
      assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
      assert_eq!(std::fs::read_to_string(&edited).unwrap(), "edited by the user\n");
      assert!(!stderr(&out).contains("skipped"), "{}", stderr(&out));
  }

  #[test]
  fn a_dry_run_records_no_commit() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      let before = std::fs::read(e.repo.join(".upskill-lock.json")).unwrap();
      advance(&e, "skills/alpha/SKILL.md", "---\nname: alpha\ndescription: changed\n---\nnew\n");
      run(&e, &e.repo, &["update", "--dry-run"]);
      assert_eq!(std::fs::read(e.repo.join(".upskill-lock.json")).unwrap(), before);
  }
  ```

  `tests/common/mod.rs` has no terminal helper that takes environment variables.
  Add
  `pub fn update_in_terminal_with_env(root: &Path, home: &Path, args: &[&str], input: &str, env: &[(&str, String)]) -> serde_json::Value`
  and give the private `run_update_in_terminal` an `env` parameter that it
  applies to the command; the existing callers pass `&[]`. Add
  `tests/common/mod.rs` to the files of this task.

- [ ] **Step 2: Run them and confirm the failures**

  Expected: the first two fail on the commit value; the third passes or fails
  depending on Task 1 and must pass at the end of this task.

- [ ] **Step 3: Implement in `update`**

  For each previewed source, after the decision: when the source is applied, the
  commit is written with the item records (Task 1 does this through
  `SourceStamp`). When the source is not applied because
  `SourcePreview::has_changes()` is false and the run is not a dry run, set
  `commit` on every lockfile entry whose `source` equals the label of the source
  and whose item is not withheld (not a `Skip` preview item), and save the
  lockfile if a value changed. Do nothing for `SourceDecision::Stop`, for a
  declined source, and for a dry run.

- [ ] **Step 4: Run `cargo test` — all pass. Commit**

  `feat(update): record the commit of each fetched source`

---

### Task 3: Remote lookup with a time limit

**Files:**

- Modify: `src/fetch.rs`
- Test: unit tests in `src/fetch.rs`

**Interfaces:**

- Produces, in `src/fetch.rs`:

  ```rust
  #[derive(Debug, PartialEq, Eq)]
  pub(crate) enum RunError { TimedOut, Failed(String) }

  /// Run `cmd` and return its stdout. Kills the child and returns `TimedOut`
  /// when it has not exited at `deadline`. `Failed` holds the first line of
  /// stderr, or the spawn error.
  pub(crate) fn run_until(cmd: std::process::Command, deadline: std::time::Instant)
      -> Result<String, RunError>;

  /// The commit that `git_ref` (or `HEAD` when `None`) names at `url`.
  /// `Ok(None)` when the remote has no such ref.
  pub(crate) fn ls_remote(url: &str, git_ref: Option<&str>, deadline: std::time::Instant)
      -> Result<Option<String>, RunError>;

  /// True for 40 lowercase or uppercase hexadecimal characters.
  pub(crate) fn is_full_commit_hash(git_ref: &str) -> bool;
  ```

- [ ] **Step 1: Write the failing unit tests**

  With `make_test_repo` and a `file://` URL:
  - `ls_remote_without_a_ref_is_the_head_commit`
  - `ls_remote_of_a_branch_is_its_commit`
  - `ls_remote_of_a_lightweight_tag_is_its_commit`
  - `ls_remote_of_an_annotated_tag_is_the_tagged_commit` — equals
    `git rev-parse v1^{commit}`, not `git rev-parse v1`
  - `ls_remote_of_a_branch_named_like_a_tag_is_the_branch` — create branch `v2`
    and tag `v2` on different commits; expect the branch commit
  - `ls_remote_of_an_unknown_ref_is_none`
  - `ls_remote_of_a_missing_repository_fails` — `Err(RunError::Failed(_))` with
    a non-empty message
  - `run_until_kills_a_command_that_passes_the_deadline` (`#[cfg(unix)]`) —
    `sleep 5` with a deadline 200 ms from now returns `Err(RunError::TimedOut)`
    in less than 2 seconds
  - `is_full_commit_hash_accepts_forty_hex_characters_only` — 40 hex is true; 39
    characters, 41 characters, `main`, and 40 characters with a `g` are false

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --lib fetch::`. Expected: compile error, functions missing.

- [ ] **Step 3: Implement**

  `run_until`: spawn with stdin null and stdout and stderr piped, then loop on
  `try_wait` with a 20 ms sleep until exit or the deadline; on the deadline
  `kill` and `wait`. Read the pipes after the exit (the output of `ls-remote`
  for the patterns below is a few lines, below the pipe buffer size).

  `ls_remote`: the command is `git ls-remote <url> HEAD` without a ref, and
  `git ls-remote <url> refs/heads/<ref> refs/tags/<ref> refs/tags/<ref>^{}` with
  one. Set `GIT_TERMINAL_PROMPT=0` so that git fails in place of asking for a
  password. Each output line is `<hash>\t<name>`. Choose in this order:
  `refs/heads/<ref>`, `refs/tags/<ref>^{}`, `refs/tags/<ref>`, `HEAD`.

- [ ] **Step 4: Run `cargo test --lib fetch::` — all pass. Commit**

  `feat: resolve a remote ref with a time limit`

---

### Task 4: The check library

**Files:**

- Create: `src/pipeline/check.rs`
- Modify: `src/pipeline/mod.rs` (declare, re-export), `src/pipeline/git.rs`
  (make the clone URL of an `InstallSource` available to the module)
- Test: unit tests in `src/pipeline/check.rs`

**Interfaces:**

- Consumes: `ls_remote`, `RunError`, `is_full_commit_hash` (Task 3);
  `LockedItem.commit` (Task 1); `ScopeRoot`; `hash_item_dir`;
  `parse_install_source_label`.
- Produces, in `src/pipeline/check.rs`, re-exported from `pipeline`:

  ```rust
  /// Resolves the commit of a git source. `GitRemote` is the implementation
  /// that runs `git ls-remote`; tests pass their own.
  pub trait RemoteLookup {
      fn resolve(&mut self, source: &InstallSource, deadline: Instant)
          -> Result<Option<String>, LookupError>;
  }
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum LookupError { TimedOut, Failed(String) }
  pub struct GitRemote;

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum SourceStatus {
      UpToDate,
      /// `from` and `to` are commits for a git source and `None` for a local
      /// path source.
      UpdateAvailable { from: Option<String>, to: Option<String> },
      /// The ref is a full commit hash. Not looked up.
      PinnedToCommit,
      /// A git source with an item that has no recorded commit.
      NoRecordedCommit,
      /// `reason` is one line.
      NotReachable { reason: String },
      /// The time limit passed before this source was looked up.
      NotChecked,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct CheckedSource {
      /// The source label, as recorded in the lockfile.
      pub source: String,
      pub status: SourceStatus,
      /// For `UpdateAvailable`: every item of a git source; the items whose
      /// hash differs for a local path source. Sorted by kind, then name.
      /// Empty for every other status.
      pub items: Vec<(ItemKind, String)>,
  }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct ScopedCheck { pub scope: Scope, pub root: PathBuf, pub sources: Vec<CheckedSource> }

  impl ScopedCheck { pub fn update_count(&self) -> usize } // items of UpdateAvailable sources

  /// Check every source of the lockfile of each root, in lockfile order.
  /// `names` limits the check to the items with those names; empty means all.
  /// A git source is looked up at most once for all roots. Changes no file.
  pub fn check_scopes(roots: &[ScopeRoot], names: &[String], deadline: Instant,
      remote: &mut dyn RemoteLookup) -> Result<Vec<ScopedCheck>>;
  ```

  Rules of `check_scopes`, for one source of one root (the entries that share a
  `source` label):

  | Condition, tested in this order                                                  | Status                                            |
  | -------------------------------------------------------------------------------- | ------------------------------------------------- |
  | Local path source                                                                | see below                                         |
  | The ref is a full commit hash                                                    | `PinnedToCommit`                                  |
  | `Instant::now() >= deadline`                                                     | `NotChecked`                                      |
  | `resolve` returns `Err(TimedOut)`                                                | `NotReachable { reason: "time limit reached" }`   |
  | `resolve` returns `Err(Failed(m))`                                               | `NotReachable { reason: m }`                      |
  | `resolve` returns `Ok(None)`                                                     | `NotReachable { reason: "ref <ref> not found" }`  |
  | An entry has no recorded commit                                                  | `NoRecordedCommit`                                |
  | Every recorded commit equals the remote commit (compared without regard to case) | `UpToDate`                                        |
  | Otherwise                                                                        | `UpdateAvailable { from: <first differing>, to }` |

  Local path source: for each entry, find the item directory under the path with
  the name `source_name`, or `name` when it is absent, using the item discovery
  of `src/pipeline/discovery.rs`, and compute `hash_item_dir`. An entry whose
  hash differs from its recorded `hash`, or whose directory is not found, is an
  item with an update. No such entry: `UpToDate`. The path does not exist:
  `NotReachable { reason: "path not found" }`.

  A source label that `parse_install_source_label` rejects is `NotReachable`
  with the parse error as the reason. A root without a lockfile gives a
  `ScopedCheck` with no sources.

- [ ] **Step 1: Write the failing unit tests**

  Each test writes a lockfile with `Lockfile::save` into a tempdir and passes a
  `FakeRemote` (a map from source label to result, with a call counter):
  - `a_source_with_the_recorded_commit_is_up_to_date`
  - `a_source_with_another_commit_lists_every_item` — two items, sorted
  - `a_full_commit_hash_ref_is_not_looked_up` — the call counter stays 0
  - `an_entry_without_a_commit_is_no_recorded_commit`
  - `a_failed_lookup_is_not_reachable_and_later_sources_are_checked`
  - `a_passed_deadline_is_not_checked` — deadline `Instant::now()`, counter 0
  - `one_source_in_two_scopes_is_looked_up_once` — counter 1
  - `names_limit_the_check` — a source with no selected item is absent
  - `a_local_item_with_another_hash_has_an_update` and
    `a_local_item_with_the_recorded_hash_is_up_to_date` — install from a local
    source with `install_with_lockfile`, then change one `SKILL.md`
  - `a_missing_local_path_is_not_reachable`
  - `the_check_changes_no_file` — the lockfile bytes are equal before and after

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --lib pipeline::check`. Expected: compile error.

- [ ] **Step 3: Implement `check.rs`; declare it in `pipeline/mod.rs`**

  `GitRemote::resolve` builds the clone URL with the functions that `fetch_ssot`
  uses (`github_clone_url`, `git_clone_url`; make them `pub(super)`) and maps
  `RunError` to `LookupError`.

- [ ] **Step 4: Run `cargo test` — all pass. Commit**

  `feat: check the sources of both scopes for updates without a fetch`

---

### Task 5: The `update --check` command and its documentation

**Files:**

- Modify: `src/cli.rs` (`Update`), `src/main.rs` (`UpdateArgs`, `run_update`,
  new `run_update_check`, `print_check`), `docs/book/commands.md`,
  `docs/book/recipes.md`, `docs/specification/specification.md`,
  `docs/roadmap.md`, `AGENTS.md`
- Test: `tests/cli_update_check.rs`

**Interfaces:**

- Consumes: `check_scopes`, `GitRemote`, `ScopedCheck` (Task 4);
  `report_scopes(global, project)` in `src/main.rs`.
- Produces: `upskill update --check [names] [--global | --project]`. Output on
  stdout, exact form:

  ```text
  project (/path/to/repo):
    update available    https://example.invalid/team/skills  1a2b3c4 -> 5d6e7f8
      skill alpha
      skill beta
    not reachable       owner/other: <reason>
  global (/home/user):
    up to date
  2 updates available. Run "upskill update".
  ```

  One line for each source that is not up to date; `up to date` when a scope has
  sources and none has such a line; `nothing installed` when a scope has no
  source. Status labels, padded to one width: `update available`,
  `no recorded commit`, `pinned to a commit`, `not reachable`, `not checked`.
  Commits are shortened to 7 characters. A local path source prints no commits.
  A `no recorded commit` line ends with `: run "upskill update" once`. The last
  line is `No updates available.` when the count is 0. When only the global
  scope has updates, the last line ends with `Run "upskill update --global".`;
  when both have, it names both commands, joined by `and`.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn check_reports_no_update_after_an_install() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      let out = run(&e, &e.repo, &["update", "--check"]);
      assert_eq!(out.status.code(), Some(0));
      assert!(stdout(&out).contains("  up to date\n"), "{}", stdout(&out));
      assert!(stdout(&out).ends_with("No updates available.\n"), "{}", stdout(&out));
  }

  #[test]
  fn check_lists_the_items_of_a_source_with_a_new_commit_and_changes_no_file() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      let old = upstream_head(&e);
      let new = advance(&e, "skills/alpha/SKILL.md", "---\nname: alpha\ndescription: changed\n---\nnew\n");
      let before = (tree(&e.repo), tree(&e.home));
      let out = run(&e, &e.repo, &["update", "--check"]);
      assert_eq!(out.status.code(), Some(0));
      let text = stdout(&out);
      assert!(text.contains(&format!("update available    {REMOTE}  {} -> {}", &old[..7], &new[..7])), "{text}");
      assert!(text.contains("    skill alpha\n") && text.contains("    skill beta\n"), "{text}");
      assert!(text.ends_with("2 updates available. Run \"upskill update\".\n"), "{text}");
      assert_eq!((tree(&e.repo), tree(&e.home)), before);
  }

  #[test]
  fn check_reports_both_scopes_and_a_flag_limits_it() {
      // add --claude in repo; add --global --claude from outside; advance.
      // Without a flag: a "project (" block and a "global (" block, 4 updates,
      // the last line names both commands. With --global: no "project (" line.
  }

  #[test]
  fn check_reports_an_unreachable_source_and_exits_with_code_0() {
      // After the install, run with the insteadOf mapping pointed at a
      // directory that does not exist.
      // Assert: exit code 0, a "not reachable" line that names REMOTE,
      // last line "No updates available."
  }

  #[test]
  fn check_does_not_look_for_newer_tags() {
      // upstream: tag v1; add REMOTE@v1; advance and tag v2.
      // Assert: "up to date". Then move v1 (`git tag -f v1`): "update available".
  }

  #[test]
  fn check_skips_a_source_pinned_to_a_commit() {
      // Write `"ref": "<40 hex>"` into the lockfile entries, advance.
      // Assert: "pinned to a commit", "No updates available."
  }

  #[test]
  fn check_of_a_local_path_source_lists_only_the_changed_item() {
      // add <upstream path>; change skills/alpha/SKILL.md without a commit.
      // Assert: "skill alpha" is listed, "skill beta" is not, 1 update.
  }

  #[test]
  fn check_after_an_update_reports_no_update() {
      // advance; update --yes; update --check: "No updates available."
  }

  #[test]
  fn check_refuses_flags_that_apply_changes() {
      for flag in ["--dry-run", "--yes", "--force", "--claude"] {
          let e = env();
          let out = run(&e, &e.repo, &["update", "--check", flag]);
          assert_eq!(out.status.code(), Some(2), "{flag}");
      }
  }
  ```

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --test cli_update_check`. Expected: exit code 2, unknown
  argument `--check`.

- [ ] **Step 3: Implement**

  In `src/cli.rs`, add to `Update`:

  ```rust
  /// Report the sources that have a newer version, for both scopes,
  /// without changing a file and without a fetch.
  #[arg(long = "check", conflicts_with_all = ["dry_run", "yes", "force", "claude", "copilot", "vscode", "opencode"])]
  check: bool,
  ```

  Replace the sentence `` `update` always fetches. `` in the doc comment of
  `Update` and add `upskill update --check` to its examples. In `src/main.rs`,
  `run_update` calls `run_update_check(names, global, project)` first when
  `check` is set: `report_scopes`, then `check_scopes` with a deadline of
  `Instant::now() + CHECK_TIME_LIMIT`
  (`const CHECK_TIME_LIMIT: Duration =
  Duration::from_secs(10)`), then
  `print_check`. It prints no scope line of its own, because each block names
  its scope.

- [ ] **Step 4: Update the documentation**

  - `docs/book/commands.md`: a section `update --check` with the output above,
    the meaning of each status, the time limit, and the exit code.
  - `docs/book/recipes.md`: a recipe "See whether a source has a newer version".
  - `docs/specification/specification.md`: the `commit` field of an item record
    and the rules of the check (the table of Task 4).
  - `docs/roadmap.md`: mark step 6 as delivered, in the form used for step 5.
  - `AGENTS.md`: name `--check` in the paragraph on scopes (`update --check`
    reports both scopes).

- [ ] **Step 5: Run `just fmt`, `just verify`, `just sanity` — all pass.
      Commit**

  `feat(update): report available updates with --check`

---

## Pull request B — delivery step 7: support levels and smoke test

Worktree `.claude/worktrees/feat-support-levels`, branch `feat/support-levels`.

### Task 6: The support level table and `doctor`

**Files:**

- Create: `src/support.rs`, `tests/cli_support_levels.rs`,
  `docs/book/support-levels.md`
- Modify: `src/lib.rs`, `src/pipeline/report.rs` (`DoctorScopes`),
  `src/pipeline/lifecycle.rs` (`doctor_scopes`), `src/main.rs`
  (`print_doctor_scopes`), `docs/book/SUMMARY.md`, `docs/book/commands.md`
- Test: `tests/cli_support_levels.rs`, unit tests in `src/support.rs`

**Interfaces:**

- Produces, in `src/support.rs`:

  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
  #[serde(rename_all = "lowercase")]
  pub enum SupportLevel { Supported, Experimental, Unsupported }
  impl std::fmt::Display for SupportLevel // "supported" | "experimental" | "unsupported"

  #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
  pub struct SupportCell {
      /// `claude` | `copilot` | `opencode` | `codex`.
      pub client: &'static str,
      pub kind: ItemKind,
      pub level: SupportLevel,
      /// Client version of the smoke run that passed. `None` unless supported.
      pub client_version: Option<&'static str>,
      /// Date of that run, `YYYY-MM-DD`. `None` unless supported.
      pub checked: Option<&'static str>,
  }

  /// Twelve cells: four clients in the order above, each with rule, skill, agent.
  pub const SUPPORT: &[SupportCell];
  ```

  Initial values: every `claude`, `copilot` and `opencode` cell is
  `Experimental`; every `codex` cell is `Unsupported`; no version, no date.

- Produces: `DoctorScopes.support: Vec<SupportCell>`, filled from `SUPPORT` by
  `doctor_scopes`. It does not affect `is_clean()`. `doctor --json` has the key
  `support`. `doctor` prints, after the scope reports:

  ```text
  Support levels:
    claude    rule   experimental
    claude    skill  experimental
    …
    codex     agent  unsupported
  ```

  A supported cell ends with `(<client> <version>, <date>)`.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn doctor_prints_one_level_for_each_client_and_kind() {
      // In an empty git repository: `doctor`.
      // Assert: stdout holds "Support levels:" and 12 lines after it; the
      // exit code is the one `doctor` gives without the section (0).
  }

  #[test]
  fn doctor_json_holds_the_support_table() {
      // `doctor --json`: `support` is an array of 12 objects with the keys
      // client, kind, level, client_version, checked.
  }

  #[test]
  fn the_user_guide_table_equals_the_compiled_table() {
      // Read docs/book/support-levels.md from CARGO_MANIFEST_DIR. For each
      // cell of upskill::support::SUPPORT, assert that the file holds the row
      // "| <client> | <kind> | <level> | <version or -> | <date or -> |"
      // after collapsing runs of spaces.
  }
  ```

  Unit tests in `src/support.rs`:
  `the_table_has_one_cell_for_each_client_and_kind` (12 cells, no duplicate
  pair) and `only_a_supported_cell_has_a_version_and_a_date`.

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --test cli_support_levels`.

- [ ] **Step 3: Implement the module, the `doctor` field and the printing**

- [ ] **Step 4: Write `docs/book/support-levels.md`**

  The meaning of the three levels (design section 6, the table), the cell table
  in the row form of the test, the statement that `doctor` prints the same table
  and does not detect client versions, the known limits of the experimental
  OpenCode level (decision 3 of the step 5 plan), and how a maintainer runs the
  smoke test (Task 7). Add the page to `docs/book/SUMMARY.md` and link it from
  the `doctor` section of `docs/book/commands.md`.

- [ ] **Step 5: Run `cargo test` — all pass. Commit**

  `feat(doctor): print the support level of each client and kind`

---

### Task 7: The smoke mode of the qualification script

**Files:**

- Create: `scripts/tests/test_smoke_native.py`
- Modify: `scripts/qualify-native-instructions.py`, `justfile`
- Test: `scripts/tests/test_smoke_native.py`

**Interfaces:**

- Consumes: `invoke`, `command`, `copilot_response`, `isolated_git_environment`
  of the script.
- Produces, in the script:

  ```python
  SMOKE_CLIENTS = ('claude', 'copilot')
  SMOKE_KINDS = ('rule', 'skill', 'agent')
  SMOKE_SCOPES = ('project', 'global')

  def smoke_source(root, name, marker):
      """Write one rule, one skill and one agent named `name` under
      root/rules, root/skills, root/agents. Each body tells the client to
      answer the smoke question for its kind with `marker[kind]`.
      Returns the source path."""

  def smoke_prompt(kind, name):
      """The question for one cell. rule: asks for the marker of the standing
      instructions. skill: asks the client to use the skill `name`.
      agent: asks the client to delegate to the agent `name`."""

  def classify_smoke(response, marker, other_markers):
      """'pass' when the response holds `marker` and none of `other_markers`;
      'fail' otherwise."""

  def run_smoke(args, runner=invoke):
      """For each selected client and scope: install with
      `<upskill> add <source> --<client> --project|--global`, ask one question
      per kind in a new client session, remove the items, and return a list of
      {'client', 'kind', 'scope', 'result', 'client_version'}. `result` is
      'pass', 'fail', 'timeout' or 'unavailable'."""
  ```

  Command line: `--smoke`, `--upskill <path>` (required with `--smoke`),
  `--clients`, `--kinds`, `--scopes`, `--allow-real-home`, with `--run` and
  `--timeout` as today. `just smoke *args` builds the release binary and runs
  `python3 scripts/qualify-native-instructions.py --smoke --upskill target/release/upskill {{args}}`.

  Rules:
  - Without `--run`, the mode prints the cells it would run and makes no model
    call.
  - A project cell runs in a new temporary git repository. `HOME` is not
    changed, so the client stays authenticated.
  - A global cell runs only with `--allow-real-home`; without it the cell is
    reported as `unavailable` with the reason. It installs with the item name
    `upskill-smoke-<12 hex>`, and removes it with
    `<upskill> remove --global --yes <name>` in a `finally` block, also after a
    timeout or an exception.
  - The three markers of one run are different random values.
  - The output is one line for each cell,
    `<client> <kind> <scope> <result> <client version>`, then one line for each
    client and kind with the level that the table may hold: `supported` when
    both scopes passed, `experimental` otherwise. The mode writes no file in the
    repository.
  - The exit code is 0 when the mode ran, whatever the results.

- [ ] **Step 1: Write the failing offline tests**

  `scripts/tests/test_smoke_native.py`, loading the script the way
  `scripts/tests/test_qualify_native_instructions.py` does, with a fake `runner`
  that records commands and returns scripted output:
  - `test_source_holds_one_item_of_each_kind_with_its_own_marker`
  - `test_without_run_no_command_is_started`
  - `test_a_response_with_the_marker_passes`
  - `test_a_response_with_the_marker_of_another_kind_fails`
  - `test_global_cells_are_unavailable_without_allow_real_home` — and no command
    holds `--global`
  - `test_global_items_are_removed_after_a_timeout` — the runner raises
    `subprocess.TimeoutExpired` for the client call; the recorded commands end
    with `remove --global --yes upskill-smoke-…`
  - `test_a_kind_is_supported_only_when_both_scopes_pass`
  - `test_exit_code_is_0_when_a_cell_fails`

- [ ] **Step 2: Run them and confirm the failures**

  Run:
  `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_smoke_native.py'`.

- [ ] **Step 3: Implement the mode and the `just smoke` recipe**

- [ ] **Step 4: Run `just lint` — passes. Commit**

  `test: add a smoke test of installed content against the real clients`

---

### Task 8: Run the smoke test and set the levels (operator task)

Needs authenticated Claude Code and Copilot CLI, makes paid model calls, and
with `--allow-real-home` writes to the real home directory. Ask the user before
running it.

- [ ] **Step 1:** Run `just smoke --run --scopes project`. Then, with the user's
      agreement, `just smoke --run --allow-real-home`.
- [ ] **Step 2:** Confirm that `upskill list --global` shows no `upskill-smoke-`
      item afterwards.
- [ ] **Step 3:** For each client and kind that the output reports as
      `supported`, set the cell in `src/support.rs` to `Supported` with the
      client version and the date of the run. Leave every other cell unchanged.
      Update the table in `docs/book/support-levels.md` to match;
      `the_user_guide_table_equals_the_compiled_table` passes.
- [ ] **Step 4:** Mark step 7 as delivered in `docs/roadmap.md`. For a cell that
      failed, open a debt issue under epic #283 with the output line and the
      client version. A failed cell does not block the pull request.
- [ ] **Step 5:** Run `just fmt`, `just verify`, `just sanity`. Commit
      `docs: record the support levels of the first smoke run`.

---

## Pull request C — delivery step 8: session notice

Worktree `.claude/worktrees/feat-update-notice`, branch `feat/update-notice`.

### Task 9: Trial T2 — how each client shows hook output (operator task)

This task runs before the code, because its result decides the form of the
notice output. Needs authenticated clients and makes paid model calls. Ask the
user before running it.

- [ ] **Step 1:** Read the current hook documentation of Claude Code plugins and
      of Copilot CLI plugins. Record: the name of the session-start event, the
      file that declares a hook in a plugin, and what the client does with the
      standard output and the exit code of the hook.
- [ ] **Step 2:** For each client, write a local plugin in a temporary directory
      whose session-start hook prints the fixed line
      `upskill: 1 update available (project). Run "upskill update".` Install it
      with the client's own command.
- [ ] **Step 3:** Start a new session of each client. Record: whether the user
      sees the line, whether the agent receives it, and whether the agent
      mentions it or asks the user without being asked. Repeat with the form
      that the documentation gives for a message shown to the user, if that form
      differs from plain standard output.
- [ ] **Step 4:** Write `docs/technotes/t2-session-notice-display.md` with the
      client versions, the plugin files, and the result for each client and
      form. Uninstall the trial plugins.
- [ ] **Step 5:** If no form of a client shows the line to the user or to the
      agent, stop and report to the user: the plugin of that client is then not
      published, and the other client continues. If a client needs a form other
      than one plain line on standard output, record the form here and apply it
      in the hook command of Task 12, not in the binary: the binary prints plain
      lines.

      Commit `docs: record trial T2 for the session notice`.

---

### Task 10: The notice decision and its cache

**Files:**

- Create: `src/notice.rs`
- Modify: `src/lib.rs`, `src/index.rs` (share the cache root)
- Test: unit tests in `src/notice.rs`

**Interfaces:**

- Consumes: `ScopedCheck`, `SourceStatus`, `CheckedSource` (Task 4).
- Produces, in `src/index.rs`: `pub fn cache_root() -> PathBuf`
  (`$XDG_CACHE_HOME/upskill` or `~/.cache/upskill`); `cache_dir()` returns
  `cache_root().join("index")`.
- Produces, in `src/notice.rs`:

  ```rust
  pub const NETWORK_PERIOD_SECS: u64 = 24 * 60 * 60;
  pub const CACHE_FILE_NAME: &str = "notice.json";

  /// State of one scope root in the cache file.
  #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
  pub struct ScopeState {
      /// Unix time in seconds of the last check that contacted the sources.
      pub checked_at: u64,
      /// The available updates found by that check, sorted.
      pub updates: Vec<String>,
      /// The set that the last printed notice reported, sorted.
      pub reported: Vec<String>,
  }

  /// The cache file: scope root path to state.
  #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
  pub struct NoticeCache { pub scopes: BTreeMap<String, ScopeState> }

  impl NoticeCache {
      /// An absent, unreadable or invalid file is an empty cache.
      pub fn load(path: &Path) -> NoticeCache;
      /// Write to a temporary file in the same directory, then rename.
      pub fn save(&self, path: &Path) -> Result<()>;
  }

  /// True when the state of `root` is older than the period, or absent.
  pub fn needs_check(cache: &NoticeCache, root: &Path, now: u64) -> bool;

  /// The identity of each available update of a scope, sorted:
  /// `<kind>:<name>@<to>` for a git source and `<kind>:<name>` for a local
  /// path source.
  pub fn update_set(check: &ScopedCheck) -> Vec<String>;

  /// Store the result of a check: `checked_at = now` and, unless the check
  /// reached no source (`reached` is false), `updates`.
  pub fn record_check(cache: &mut NoticeCache, root: &Path, updates: Vec<String>, reached: bool, now: u64);

  /// The line to print for `root`, and the cache change that goes with it.
  /// Returns `None` when `updates` is empty (and clears `reported`) or
  /// equals `reported`. Otherwise sets `reported = updates` and returns the
  /// line.
  pub fn decide(cache: &mut NoticeCache, scope: Scope, root: &Path) -> Option<String>;
  ```

  Notice lines, exact:

  ```text
  upskill: 3 updates available (project). Run "upskill update".
  upskill: 1 update available (global). Run "upskill update --global".
  ```

  `reached` is false when every source of the scope is `NotReachable` or
  `NotChecked`: the earlier `updates` stay, so that a session without network
  access neither prints a new notice nor loses the known set.

- [ ] **Step 1: Write the failing unit tests**

  - `an_absent_cache_needs_a_check`
  - `a_state_younger_than_the_period_needs_no_check` — `now - checked_at` of
    86,399 is false; 86,400 is true
  - `a_new_set_prints_once` — `decide` returns the line, a second call returns
    `None`
  - `a_changed_set_prints_again` — record a set with another `to` commit
  - `an_empty_set_prints_nothing_and_clears_the_reported_set` — then the first
    set again prints again
  - `a_check_that_reached_no_source_keeps_the_updates_and_sets_checked_at`
  - `one_update_uses_the_singular_form`
  - `the_global_line_names_the_global_flag`
  - `an_invalid_cache_file_loads_as_empty`
  - `save_then_load_returns_the_same_cache`
  - `update_set_is_sorted_and_names_the_target_commit`

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --lib notice::`.

- [ ] **Step 3: Implement `src/notice.rs` and `cache_root`**

- [ ] **Step 4: Run `cargo test` — all pass. Commit**

  `feat: decide when a session notice of available updates is printed`

---

### Task 11: The `update --notice` command and its documentation

**Files:**

- Create: `tests/cli_update_notice.rs`
- Modify: `src/cli.rs` (`Update`), `src/main.rs` (`run_update_notice`),
  `docs/book/commands.md`, `docs/book/recipes.md`,
  `docs/specification/specification.md`, `AGENTS.md`
- Test: `tests/cli_update_notice.rs`

**Interfaces:**

- Consumes: Task 10; `check_scopes`, `GitRemote` (Task 4); `report_scopes`.
- Produces: `upskill update --notice`. It accepts no item name and no other flag
  of `update` (`conflicts_with_all`, including `check`, `global` and `project`).
  Behaviour:

  1. The roots are those of `report_scopes(false, false)`. A root without a
     lockfile is left out and gets no cache entry.
  2. Load the cache. For the roots where `needs_check` is true, call
     `check_scopes` once with a deadline of 5 seconds and `record_check` each.
  3. For each root, in the order project then global, `decide`; print each
     returned line to stdout.
  4. Save the cache if it changed. A failed save is ignored.
  5. Exit with code 0 in every case after argument parsing, also when a step
     returns an error; an error prints nothing.

  The command prints no scope line and writes nothing to stderr.

Test file header: the `Env` and helpers of `tests/cli_update_check.rs`, plus
`XDG_CACHE_HOME` set to `<tmp>/cache` in `run`, and:

```rust
fn cache_path(e: &Env) -> PathBuf;                 // <tmp>/cache/upskill/notice.json
fn age_cache(e: &Env, secs: u64);                  // subtract `secs` from every checked_at
```

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn notice_prints_nothing_when_no_update_exists() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      let out = run(&e, &e.repo, &["update", "--notice"]);
      assert_eq!(out.status.code(), Some(0));
      assert_eq!(stdout(&out), "");
      assert_eq!(stderr(&out), "");
  }

  #[test]
  fn notice_prints_one_line_once_for_a_set_of_updates() {
      let e = env();
      run(&e, &e.repo, &["add", REMOTE, "--claude"]);
      advance(&e, "skills/alpha/SKILL.md", "---\nname: alpha\ndescription: changed\n---\nnew\n");
      let first = run(&e, &e.repo, &["update", "--notice"]);
      assert_eq!(stdout(&first), "upskill: 2 updates available (project). Run \"upskill update\".\n");
      let second = run(&e, &e.repo, &["update", "--notice"]);
      assert_eq!(stdout(&second), "");
  }

  #[test]
  fn notice_does_not_contact_the_source_again_within_the_period() {
      // add; notice (stores an empty set); advance; notice prints nothing;
      // age_cache(86_400); notice prints the line.
  }

  #[test]
  fn notice_prints_again_when_the_set_changes() {
      // advance; notice prints; advance again; age_cache(86_400); notice
      // prints again; a further notice prints nothing.
  }

  #[test]
  fn notice_prints_nothing_and_exits_with_code_0_when_the_source_is_unreachable() {
      // Point the insteadOf mapping at a directory that does not exist.
  }

  #[test]
  fn notice_survives_an_invalid_cache_file_and_an_unwritable_cache_directory() {
      // Write "not json" to cache_path: the line is printed, exit code 0.
      // (unix) Make <tmp>/cache read-only: exit code 0, stderr is empty.
  }

  #[test]
  fn notice_changes_no_file_outside_the_cache() {
      // tree(repo) and tree(home) are equal before and after a notice that
      // prints a line.
  }

  #[test]
  fn notice_reports_the_global_scope_with_its_own_line() {
      // add --global from `outside`; advance; notice run in `outside`:
      // "upskill: 2 updates available (global). Run \"upskill update --global\".\n"
  }

  #[test]
  fn notice_without_a_lockfile_prints_nothing_and_writes_no_cache_entry() {}

  #[test]
  fn notice_refuses_other_flags_and_names() {
      // ["--check"], ["--global"], ["--yes"], ["alpha"]: exit code 2.
  }
  ```

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --test cli_update_notice`.

- [ ] **Step 3: Implement the flag and `run_update_notice`**

  The time comes from `SystemTime::now()` in `main.rs`; the library functions
  take `now` as an argument.

- [ ] **Step 4: Update the documentation**

  - `docs/book/commands.md`: a section `update --notice` — purpose, the two line
    forms, the 24-hour period, the cache file path, exit code 0.
  - `docs/book/recipes.md`: "Report available updates when a session starts",
    with the plugin install command of each client once Task 12 has published
    them, and the hook command for a user who writes the hook by hand.
  - `docs/specification/specification.md`: the notice rules and the cache file
    format.
  - `AGENTS.md`: one sentence on `update --notice` and the cache file.

- [ ] **Step 5: Run `just fmt`, `just verify`, `just sanity` — all pass.
      Commit**

  `feat(update): print a session notice of available updates once`

---

### Task 12: The updater plugins in `driftsys/metapowers` (other repository)

This task changes another repository. Ask the user before pushing a branch
there. The pull request there is separate from pull request C and is opened
after pull request C is merged and a binary with `--notice` is installed.

- [ ] **Step 1:** Read the layout of `driftsys/metapowers` and the way it
      publishes a plugin for Claude Code and for Copilot CLI. Follow it.
- [ ] **Step 2:** Add one plugin for each client that trial T2 confirmed. Each
      plugin holds only a session-start hook, in the form recorded by Task 9,
      whose command is `upskill update --notice`. The hook must not fail the
      session start when `upskill` is not on `PATH`: the command form recorded
      by Task 9 covers that case, or the hook command is
      `command -v upskill >/dev/null 2>&1 && upskill update --notice || true`.
      The plugin holds no skill, no rule and no other hook.
- [ ] **Step 3:** Install each plugin with the client's own command from the
      branch. In a project with an available update, start a session twice: the
      notice appears in the first session and not in the second. Add the result
      to `docs/technotes/t2-session-notice-display.md` in this repository.
- [ ] **Step 4:** Open the pull request in `driftsys/metapowers`. After the
      merge, put the install command of each plugin into the recipe of Task 11.

---

### Task 13: Garden the working documents and release (operator task)

Publishing a release is outward-facing. Ask the user before `just release` and
before `just publish`.

- [ ] **Step 1:** Mark step 8 as delivered in `docs/roadmap.md`.
- [ ] **Step 2:** Run the `sdd-gardening` skill for the re-steer: the design,
      the part 1 plan, the step 5 plan and this plan move from `docs/wip/` to
      `docs/archive/`, the durable records are updated, and the entry of the
      re-steer leaves `docs/wip/README.md`. Edit AD-0042 in place where the
      delivered behaviour differs from its text.
- [ ] **Step 3:** Run `just verify` and `just sanity`. Open and merge the pull
      request of the gardening.
- [ ] **Step 4:** With the user's confirmation, run `just release`, then
      `just publish`. State the support levels of the release in its notes: the
      table of `docs/book/support-levels.md`.

## After this plan

Codex, then OpenCode, each with its own short design. Plugin and catalog
generation from a bundle, with its own design. The open review debt under epic
#283 is not part of this plan.
