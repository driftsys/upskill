# Customization Re-steer Step 5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every command state the scope it used, make `list` and `doctor`
report the project scope and the global scope together, and make a global
install write Copilot content to the directories that Copilot reads for the
whole machine.

**Architecture:** A new library type `Scope` (`project` or `global`) and a
`ScopeRoot` (scope plus root directory) replace the `global`/`project` boolean
pair after `main.rs` resolves the flags. The pipeline receives the scope and
uses it in one place: the output path computation in `src/pipeline/output.rs`.
The frozen layout in `src/legacy/layout.rs` does not change, and adoption of a
`schema: 1` lockfile keeps using it, so an existing global install is recorded
at its released paths and the next `update --global` moves it through the
ownership record (write the new path, delete the owned old path). `list` and
`doctor` run once for each scope and return one report for each scope.

**Tech Stack:** Rust edition 2024 (MSRV 1.85), `serde`, `anyhow`, `assert_cmd` +
`tempfile` for CLI tests. No new dependency.

**Spec:**
[docs/wip/specs/2026-11-11-customization-resteer-design.md](../specs/2026-11-11-customization-resteer-design.md),
section 3, and the parts of section 1 that name `doctor` and both scopes. This
plan is delivery step 5 and is one pull request.

## Global Constraints

- Single static binary. No async runtime, no `git2`, no new dependency.
- `main.rs` performs I/O orchestration only. Only `main.rs` writes to stdout and
  stderr. Library code returns data or `Result<T>`.
- Zero warnings: compiler, clippy (`-D warnings`), docs tooling. Run
  `just sanity` before a push; `just verify` does not lint test code.
- Acceptance tests come first and run the binary through
  `common::upskill_cmd(&home)`. Never use a raw `Command::cargo_bin`, except in
  `tests/cli_global_scope.rs`, which holds the documented exemption.
- Pre-1.0: no compatibility layer. The JSON output of `list` and `doctor`
  changes shape without a fallback.
- The two scopes are named **project** and **global** in all output and docs.
- Shared files edited by `src/ancillary.rs` (`CLAUDE.md`,
  `.vscode/settings.json`, `opencode.json`) keep their current behaviour in both
  scopes. Plugin and MCP behaviour does not change. `scope_to_plugin_scope` in
  `src/main.rs` does not change.
- OpenCode output paths do not change in either scope (design non-goal).
- `src/legacy/layout.rs` does not change.
- Comments, commit messages, docs and pull request text use plain, literal
  English.
- One worktree: `.claude/worktrees/feat-scope-reporting`, branch
  `feat/scope-reporting`; run `./bootstrap` after creating it. Every Write and
  Edit uses the absolute path that starts with the worktree directory. Before
  the pull request: `just fmt`, then `just verify`, then `just sanity`.
  Conventional Commits. Squash merge.
- The design and this plan stay in `docs/wip/` until the last delivery step of
  the design.

## Decisions made by this plan

The design leaves these points open. Each one is the author's decision and is
listed for the user to check before the implementation starts.

1. **Global Copilot paths**, relative to `$HOME`:
   `.copilot/skills/<name>/SKILL.md`,
   `.copilot/instructions/<name>.instructions.md`,
   `.copilot/agents/<name>.agent.md`. Claude Code paths are the same in both
   scopes (`.claude/skills`, `.claude/rules`, `.claude/agents`), because
   `~/.claude/` is the documented personal location.
2. **Environment variables that move a client directory** (`COPILOT_HOME`,
   `CLAUDE_CONFIG_DIR`) are not read. A recorded path stays relative to the
   scope root. The user guide states this limit.
3. **OpenCode at global scope is unchanged**: `.agents/skills` is a documented
   personal location; `.agents/rules/<name>/RULE.md` with `$HOME/opencode.json`
   and `.opencode/agents/` are not. This is recorded as a known limit of the
   experimental level and is left to the OpenCode design.
4. **The hand-shake files at global scope are unchanged** (`$HOME/CLAUDE.md`,
   `$HOME/.vscode/settings.json`, `$HOME/opencode.json`). The design keeps the
   behaviour of `src/ancillary.rs`. Task 6 opens a debt issue for them.
5. **Only `add` refuses to choose a scope.** Outside a git repository and
   without a flag, `update`, `remove` and `remove-mcp` keep the global scope and
   print the scope line; `list` and `doctor` show both scopes.
6. **`--global` or `--project` on `list` and `doctor` limits the report to that
   scope.** Without a flag the project scope is the current directory, in or
   outside a git repository. When the current directory is `$HOME`, the report
   holds the global scope only.
7. **JSON shape**: `list --json` prints `{"scopes": [...]}` and `doctor --json`
   prints `{"scopes": [...], "in_both_scopes": [...]}`, also when one scope is
   selected.
8. **An item installed in both scopes is a note.** `doctor` prints it and the
   exit code does not change. A recorded file at an earlier global path is a
   finding and gives exit code 1.
9. **No scan for unrecorded files at earlier paths.** `doctor` reports the
   earlier global paths that a record or a `schema: 1` entry covers. Deleting an
   unrecorded file after a confirmation (design section 1, decision 8) is not in
   this step; Task 6 opens a debt issue for it.

## Review Focus

1. A global install made by a released version, with a `schema: 2` record at
   `.github/skills/<name>/SKILL.md`: `update --global` writes the `.copilot`
   path and deletes the old file, without `--force` (Task 2).
2. The same install with a `schema: 1` lockfile: adoption records the `.github`
   paths, and the same run moves them (Task 2).
3. An old global Copilot file that the user edited: `update --global` writes the
   new path, keeps the edited file and names it (Task 2).
4. `list` and `doctor` run with the current directory equal to `$HOME`: the
   global scope appears once and no item is reported as installed in both scopes
   (Tasks 3 and 4).
5. `add` with an invalid source outside a git repository: the source error is
   reported, exit code 2, and its text is unchanged (Task 1).

## File Structure

| File                        | Responsibility after this step                                                                                 |
| --------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `src/scope.rs` (new)        | `Scope`, `ScopeRoot`. No I/O.                                                                                  |
| `src/main.rs`               | Resolves flags to `ScopeRoot` values, prints the scope line, the per-scope reports and the scope hint.         |
| `src/pipeline/output.rs`    | Output paths for a scope. Global Copilot paths live here. Overlap check for a scope.                           |
| `src/pipeline/mod.rs`       | `AddOptions.scope`. Re-exports the new report types and functions.                                             |
| `src/pipeline/install.rs`   | Passes `options.scope` to the path functions.                                                                  |
| `src/pipeline/lifecycle.rs` | `update` passes the scope on; `doctor` takes a `ScopeRoot`; `list_scopes`, `doctor_scopes`, `installed_names`. |
| `src/pipeline/report.rs`    | `ScopedList`, `ScopedDoctor`, `DoctorScopes`, `EarlierLayoutFile`, `BothScopesItem`.                           |
| `tests/cli_scope.rs` (new)  | Acceptance tests of this step.                                                                                 |

---

### Task 1: Scope type, scope line, and `add` refusal

**Files:**

- Create: `src/scope.rs`, `tests/cli_scope.rs`
- Modify: `src/lib.rs`, `src/main.rs` (`install_target`, `run_add`,
  `run_update`, `run_remove`, `run_remove_mcp`), `src/cli.rs` (help text of the
  scope flags of `add`)
- Test: `tests/cli_scope.rs`, `tests/cli_global_scope.rs`

**Interfaces:**

- Produces, in `src/scope.rs`:

  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
  #[serde(rename_all = "lowercase")]
  pub enum Scope { #[default] Project, Global }
  impl std::fmt::Display for Scope   // "project" | "global"
  impl Scope { pub fn other(self) -> Scope }

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct ScopeRoot { pub scope: Scope, pub root: std::path::PathBuf }
  ```

- Produces, in `src/main.rs`:

  ```rust
  /// What a command does outside a git repository when no flag is passed.
  enum NoFlag { Refuse, Global }
  fn resolve_scope(global: bool, project: bool, no_flag: NoFlag) -> Result<ScopeRoot, ScopeError>;
  fn scope_root(scope: Scope) -> anyhow::Result<ScopeRoot>; // cwd or home_dir()
  fn print_scope_line(at: &ScopeRoot); // stderr, not printed with --quiet
  ```

  `ScopeError` has two variants: `NeedsFlag { cwd: PathBuf }` (exit code 2) and
  `Other(anyhow::Error)` (exit code 1). `install_target` and the private `Scope`
  enum of `main.rs` are deleted; `is_global` in `run_add` and `run_update`
  becomes `at.scope == Scope::Global`.

Test file header, shared by every task (copy `stage_source` and `copy_dir_all`
from `tests/cli_list.rs`):

```rust
//! ATDD tests for scope reporting, both-scope `list` and `doctor`, and the
//! global output paths (re-steer design, section 3).
mod common;

/// `(tmp, home, repo, source)`: `repo` holds `.git`, `source` holds the
/// fixture items, `outside` is not inside a git repository.
struct Env { tmp: tempfile::TempDir, home: PathBuf, repo: PathBuf, outside: PathBuf, source: PathBuf }
fn env() -> Env;
fn run(env: &Env, cwd: &Path, args: &[&str]) -> std::process::Output;
fn stderr(out: &std::process::Output) -> String;
fn stdout(out: &std::process::Output) -> String;
```

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn add_outside_a_git_repository_without_a_flag_asks_for_one() {
      let e = env();
      let out = run(&e, &e.outside, &["add", e.source.to_str().unwrap()]);
      assert_eq!(out.status.code(), Some(2));
      let err = stderr(&out);
      assert!(err.contains("not inside a git repository"), "{err}");
      assert!(err.contains("--project") && err.contains("--global"), "{err}");
      assert!(!e.home.join(".upskill-lock.json").exists());
      assert!(!e.outside.join(".upskill-lock.json").exists());
  }

  #[test]
  fn add_with_an_invalid_source_outside_a_git_repository_reports_the_source() {
      let e = env();
      let out = run(&e, &e.outside, &["add", "not-a-valid-source"]);
      assert_eq!(out.status.code(), Some(2));
      assert_eq!(stderr(&out), "error: source must be in owner/repo format\n");
  }

  #[test]
  fn add_update_and_remove_state_the_scope_and_its_root() {
      let e = env();
      let src = e.source.to_str().unwrap();
      let project = format!("scope: project ({})", e.repo.canonicalize().unwrap().display());
      for args in [vec!["add", src], vec!["update", "--yes"], vec!["remove", "api-conventions"]] {
          let out = run(&e, &e.repo, &args);
          assert!(out.status.success(), "{args:?}: {}", stderr(&out));
          assert!(stderr(&out).contains(&project), "{args:?}: {}", stderr(&out));
      }
      let global = format!("scope: global ({})", e.home.display());
      for args in [vec!["add", "--global", src], vec!["update", "--yes"], vec!["remove", "api-conventions"]] {
          // Outside a git repository, `update` and `remove` use the global scope.
          let out = run(&e, &e.outside, &args);
          assert!(out.status.success(), "{args:?}: {}", stderr(&out));
          assert!(stderr(&out).contains(&global), "{args:?}: {}", stderr(&out));
      }
  }

  #[test]
  fn quiet_hides_the_scope_line() {
      let e = env();
      let out = run(&e, &e.repo, &["--quiet", "add", e.source.to_str().unwrap()]);
      assert!(!stderr(&out).contains("scope:"));
  }
  ```

  The project root in the scope line is the string that
  `std::env::current_dir()` returns. If that differs from `canonicalize()` on
  the test platform, compare against the value that the command prints for
  `add --project` in the same directory, not against a computed path.

- [ ] **Step 2: Run them and confirm the failures**

  Run: `cargo test --test cli_scope`. Expected: the first test fails with exit
  code 0, the third fails for `update` and `remove`.

- [ ] **Step 3: Implement `src/scope.rs` and declare it in `src/lib.rs`**

- [ ] **Step 4: Replace `install_target` in `src/main.rs`**

  `run_add` calls `parse_install_source` first, then
  `resolve_scope(global, project, NoFlag::Refuse)`. `run_update`, `run_remove`
  and `run_remove_mcp` use `NoFlag::Global`. `run_add`, `run_update` and
  `run_remove` call `print_scope_line` before the pipeline call; `run_remove`
  calls it after the confirmation question. Exact text of the refusal, printed
  through `print_error`:

  ```text
  not inside a git repository: pass --project to install into <cwd>, or --global to install for every project of this user
  ```

  The scope line keeps the current format of `add`: `scope: <scope> (<root>)`.

- [ ] **Step 5: Update the help text and the tests that used the fallback**

  In `src/cli.rs`, the doc comments of `add` and of its `--project` flag no
  longer describe a fallback to global. In `tests/cli_global_scope.rs`, replace
  `auto_fallback_when_not_in_git_repo_uses_global` by
  `update_outside_a_git_repository_uses_global` (install with `--global`, then
  `update --yes` without a flag, assert the global scope line). Run `cargo test`
  and add `--project` or a `.git` directory to any other test that relied on the
  fallback of `add`; do not change what such a test asserts.

- [ ] **Step 6: Run `cargo test` — all pass. Commit**

  `feat: state the scope in add, update and remove, and stop add from choosing global`

---

### Task 2: Global output paths for Copilot

**Files:**

- Modify: `src/pipeline/output.rs`, `src/pipeline/mod.rs` (`AddOptions`),
  `src/pipeline/install.rs`, `src/pipeline/owned.rs`,
  `src/pipeline/lifecycle.rs` (`update`, `doctor` fallback), `src/main.rs`
- Test: `tests/cli_scope.rs`, unit tests in `src/pipeline/output.rs`

**Interfaces:**

- Consumes: `Scope`, `ScopeRoot` (Task 1).
- Produces, in `src/pipeline/output.rs`:

  ```rust
  pub(super) fn output_path(scope: Scope, kind: ItemKind, client: Client, name: &str) -> PathBuf;
  pub(super) fn resource_base_path(scope: Scope, kind: ItemKind, client: Client, name: &str) -> PathBuf;
  /// The path that released versions wrote in both scopes.
  pub(super) fn released_output_path(kind: ItemKind, client: Client, name: &str) -> PathBuf;
  pub(super) fn released_resource_base_path(kind: ItemKind, client: Client, name: &str) -> PathBuf;
  pub(super) fn check_source_outside_outputs(at: &ScopeRoot, kind: ItemKind, name: &str, source_dir: &Path, source_root: &Path) -> Result<()>;
  ```

- Produces: `AddOptions.scope: Scope` (default `Scope::Project`), and
  `pipeline::update(at: &ScopeRoot, names, plugin_scope, selection, overwrite, dry_run, observer)`
  — the first parameter replaces `target: &Path`.

Path table for `output_path`. Every other cell delegates to
`released_output_path`:

| Scope  | Client  | Skill                             | Rule                                           | Agent                             |
| ------ | ------- | --------------------------------- | ---------------------------------------------- | --------------------------------- |
| Global | Copilot | `.copilot/skills/<name>/SKILL.md` | `.copilot/instructions/<name>.instructions.md` | `.copilot/agents/<name>.agent.md` |

`resource_base_path` keeps the existing rule: the entry file's directory for a
directory-backed kind, the sibling `<name>/` directory otherwise.

- [ ] **Step 1: Confirm the three global Copilot paths against the published
      documentation**

  The paths above come from a documentation review of 2026-11-13 that read page
  summaries. Read the pages directly (`curl -sL <url>`) and find each directory
  name:
  - `https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference`
    — `skills/` and `agents/` under `~/.copilot`;
  - `https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions`
    — `~/.copilot/instructions/**/*.instructions.md`;
  - `https://code.claude.com/docs/en/skills`, `/memory`, `/sub-agents` —
    `~/.claude/skills`, `~/.claude/rules`, `~/.claude/agents`.

  Put the URL of each page in the doc comment of `output_path`. If a page
  contradicts a cell, stop and report; do not choose another path.

- [ ] **Step 2: Write the failing acceptance tests**

  ```rust
  const OLD: [&str; 3] = [
      ".github/skills/create-api-endpoint/SKILL.md",
      ".github/instructions/api-conventions.instructions.md",
      ".github/agents/security-reviewer.agent.md",
  ];
  const NEW: [&str; 3] = [
      ".copilot/skills/create-api-endpoint/SKILL.md",
      ".copilot/instructions/api-conventions.instructions.md",
      ".copilot/agents/security-reviewer.agent.md",
  ];

  #[test]
  fn global_add_writes_the_personal_copilot_directories() {
      let e = env();
      let out = run(&e, &e.repo, &["add", "--global", e.source.to_str().unwrap()]);
      assert!(out.status.success(), "{}", stderr(&out));
      for rel in NEW { assert!(e.home.join(rel).is_file(), "{rel}"); }
      for rel in OLD { assert!(!e.home.join(rel).exists(), "{rel}"); }
      // Claude Code paths are the same in both scopes.
      assert!(e.home.join(".claude/skills/create-api-endpoint/SKILL.md").is_file());
      assert!(e.home.join(".claude/rules/api-conventions.md").is_file());
      assert!(e.home.join(".claude/agents/security-reviewer.md").is_file());
      let lock = fs::read_to_string(e.home.join(".upskill-lock.json")).unwrap();
      assert!(lock.contains("\".copilot/skills/create-api-endpoint/SKILL.md\""));
      assert!(!lock.contains(".github/"));
  }

  #[test]
  fn project_add_keeps_the_github_paths() {
      let e = env();
      run(&e, &e.repo, &["add", e.source.to_str().unwrap()]);
      for rel in OLD { assert!(e.repo.join(rel).is_file(), "{rel}"); }
      assert!(!e.repo.join(".copilot").exists());
  }

  /// A global install as a released version left it: files at the
  /// `.github` paths, recorded in a `schema: 2` lockfile.
  fn released_global_install(e: &Env);

  #[test]
  fn global_update_moves_a_released_install_without_force() {
      let e = env();
      released_global_install(&e);
      let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
      assert!(out.status.success(), "{}", stderr(&out));
      for rel in NEW { assert!(e.home.join(rel).is_file(), "{rel}"); }
      for rel in OLD { assert!(!e.home.join(rel).exists(), "{rel}"); }
      assert!(!e.home.join(".github").exists(), "empty directories are removed");
  }

  #[test]
  fn global_update_moves_a_schema_1_install() {
      // As above, after rewriting the lockfile to `schema: 1` without `files`.
      // Same assertions, plus `.upskill-lock.json.v1` exists.
  }

  #[test]
  fn global_update_keeps_an_edited_file_at_the_old_path() {
      let e = env();
      released_global_install(&e);
      fs::write(e.home.join(OLD[1]), "edited by the user\n").unwrap();
      let out = run(&e, &e.outside, &["update", "--global", "--yes"]);
      assert!(e.home.join(NEW[1]).is_file());
      assert_eq!(fs::read_to_string(e.home.join(OLD[1])).unwrap(), "edited by the user\n");
      assert!(stderr(&out).contains(OLD[1]) || stdout(&out).contains(OLD[1]));
  }

  #[test]
  fn global_update_previews_the_move_in_a_dry_run() {
      let e = env();
      released_global_install(&e);
      let out = run(&e, &e.outside, &["update", "--global", "--dry-run"]);
      let text = stdout(&out);
      assert!(text.contains(NEW[0]) && text.contains(OLD[0]), "{text}");
      for rel in OLD { assert!(e.home.join(rel).is_file(), "{rel}"); }
  }
  ```

  `released_global_install` runs `add --global`, then for each Copilot pair
  moves the file from `NEW[i]` to `OLD[i]`, replaces the `.copilot/…` path
  strings in `$HOME/.upskill-lock.json` by the `.github/…` strings, and deletes
  the `.copilot` directory. The hashes stay valid because the bytes do not
  change.

- [ ] **Step 3: Run them and confirm the failures**

  Run: `cargo test --test cli_scope global_`. Expected: every test fails on a
  missing `.copilot` path.

- [ ] **Step 4: Write the unit tests of `output_path` in
      `src/pipeline/output.rs`**

  One test asserts the three cells of the table above and the three resource
  bases (`.copilot/skills/x`, `.copilot/instructions/x`, `.copilot/agents/x`).
  One test asserts that for every `(kind, client)` with
  `scope == Scope::Project`, and for Claude and OpenCode with
  `scope == Scope::Global`, `output_path` equals `released_output_path`.
  Existing unit tests pass `Scope::Project`.

- [ ] **Step 5: Implement the scope-dependent paths and pass the scope through**

  - `install.rs` (near line 1321) passes `options.scope`.
  - `lifecycle.rs` `update` sets `scope: at.scope` in the `AddOptions` it builds
    (near line 766) and uses `at.root` where it used `target`.
  - `owned.rs` adoption and the no-record fallback of `doctor` call
    `released_output_path` and `released_resource_base_path`: a `schema: 1`
    lockfile was written by a released version in both scopes.
  - `check_source_outside_outputs` checks the paths of `at.scope`, and for
    `Scope::Global` also the released paths, because an update deletes owned
    files there.
  - `main.rs` sets `AddOptions.scope` and passes `&at` to `update`.

- [ ] **Step 6: Run `cargo test` — all pass. Commit**

  `feat: write global Copilot content to the personal Copilot directories`

---

### Task 3: `list` reports both scopes

**Files:**

- Modify: `src/pipeline/report.rs`, `src/pipeline/lifecycle.rs`,
  `src/pipeline/mod.rs`, `src/main.rs` (`run_list`, `print_list_report`,
  `print_list_json`), `tests/cli_list.rs`, `tests/cli_global_scope.rs`
- Test: `tests/cli_scope.rs`

**Interfaces:**

- Produces:

  ```rust
  // report.rs
  #[derive(Debug, Clone, Serialize)]
  pub struct ScopedList { pub scope: Scope, pub root: PathBuf, #[serde(flatten)] pub report: ListReport }
  // lifecycle.rs, re-exported from pipeline
  pub fn list_scopes(roots: &[ScopeRoot]) -> Result<Vec<ScopedList>>;
  // main.rs
  /// The scopes that `list` and `doctor` report, in the order project, global.
  fn report_scopes(global: bool, project: bool) -> anyhow::Result<Vec<ScopeRoot>>;
  ```

  `report_scopes`: `--global` gives the global scope; `--project` gives the
  project scope (current directory); no flag gives both. With no flag, when the
  canonical current directory equals the canonical home directory, the result is
  the global scope only; when `HOME` and `USERPROFILE` are not set, the result
  is the project scope only and `main.rs` prints
  `note: the global scope is not shown: HOME is not set` on stderr.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn list_shows_both_scopes_with_the_scope_of_each_item() {
      let e = env();
      let src = e.source.to_str().unwrap();
      run(&e, &e.repo, &["add", src, "api-conventions"]);
      run(&e, &e.repo, &["add", "--global", src, "create-api-endpoint"]);
      let text = stdout(&run(&e, &e.repo, &["list"]));
      let project = text.find("project (").expect(&text);
      let global = text.find(&format!("global ({})", e.home.display())).expect(&text);
      assert!(project < global);
      let rule = text.find("api-conventions").unwrap();
      let skill = text.find("create-api-endpoint").unwrap();
      assert!(project < rule && rule < global && global < skill, "{text}");
  }

  #[test]
  fn list_json_holds_one_entry_for_each_scope() {
      // Same installs. `list --json`:
      // v["scopes"][0]["scope"] == "project", v["scopes"][0]["rules"][0]["name"] == "api-conventions",
      // v["scopes"][1]["scope"] == "global",  v["scopes"][1]["skills"][0]["name"] == "create-api-endpoint",
      // v["scopes"][1]["root"] == home.
  }

  #[test]
  fn list_with_a_scope_flag_shows_that_scope_only() {
      // `list --global`: stdout has "global (" and not "project (";
      // `list --json --project`: v["scopes"].as_array().unwrap().len() == 1.
  }

  #[test]
  fn list_names_a_scope_without_items() {
      // Only the global install exists. `list` in the repository prints the
      // "project (" header followed by "  No items installed." and then the
      // global section.
  }

  #[test]
  fn list_in_the_home_directory_shows_the_global_scope_once() {
      // cwd = home, `add --global`, then `list --json`:
      // v["scopes"].as_array().unwrap().len() == 1 and its scope is "global".
  }
  ```

- [ ] **Step 2: Run them and confirm the failures**

- [ ] **Step 3: Implement `ScopedList`, `list_scopes`, `report_scopes` and the
      printing**

  Text format: one header line `<scope> (<root>)` for each scope, then the
  current sections indented by two more spaces, or `No items installed.` When
  every scope is empty, the output is the current empty message with its two
  hint lines, without headers. `list --json` prints `{"scopes": [...]}`.

- [ ] **Step 4: Update the existing `list` tests to the new shape**

  In `tests/cli_list.rs` and `tests/cli_global_scope.rs`, read the buckets from
  `v["scopes"][0]` after passing `--project` or `--global`. Do not change what a
  test asserts about the buckets.

- [ ] **Step 5: Run `cargo test` — all pass. Commit**

  `feat: list the project scope and the global scope together`

---

### Task 4: `doctor` reports both scopes

**Files:**

- Modify: `src/pipeline/report.rs`, `src/pipeline/lifecycle.rs`,
  `src/pipeline/mod.rs`, `src/main.rs` (`run_doctor`, `print_doctor_report`,
  `print_doctor_json`), `tests/cli_doctor*.rs`
- Test: `tests/cli_scope.rs`

**Interfaces:**

- Consumes: `report_scopes` (Task 3), `released_output_path` (Task 2).
- Produces:

  ```rust
  // report.rs
  #[derive(Debug, Clone, Serialize)]
  pub struct EarlierLayoutFile { pub kind: ItemKind, pub name: String, pub files: Vec<PathBuf> }
  #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
  pub struct BothScopesItem { pub kind: ItemKind, pub name: String }
  #[derive(Debug, Clone, Serialize)]
  pub struct ScopedDoctor { pub scope: Scope, pub root: PathBuf, #[serde(flatten)] pub report: DoctorReport }
  #[derive(Debug, Clone, Serialize)]
  pub struct DoctorScopes { pub scopes: Vec<ScopedDoctor>, pub in_both_scopes: Vec<BothScopesItem> }
  impl DoctorScopes { pub fn is_clean(&self) -> bool } // every scope is clean
  // DoctorReport gains: pub earlier_layout: Vec<EarlierLayoutFile>, and is_clean() requires it to be empty.
  // lifecycle.rs
  pub fn doctor(at: &ScopeRoot) -> Result<DoctorReport>;
  pub fn doctor_scopes(roots: &[ScopeRoot]) -> Result<DoctorScopes>;
  ```

  `earlier_layout` is filled for `Scope::Global` only. It holds, for each item,
  the Copilot paths where `released_output_path` differs from `output_path` and
  where either the item's record names the path, or the item has no record and a
  file exists at the path. Such a path is not reported a second time under
  `missing_outputs`.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn doctor_reports_each_scope_and_fails_when_one_is_not_clean() {
      let e = env();
      let src = e.source.to_str().unwrap();
      run(&e, &e.repo, &["add", src, "api-conventions"]);
      run(&e, &e.repo, &["add", "--global", src, "create-api-endpoint"]);
      fs::remove_file(e.home.join(".claude/skills/create-api-endpoint/SKILL.md")).unwrap();
      let out = run(&e, &e.repo, &["doctor", "--json"]);
      assert_eq!(out.status.code(), Some(1));
      let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
      assert_eq!(v["scopes"][0]["scope"], "project");
      assert_eq!(v["scopes"][0]["missing_outputs"].as_array().unwrap().len(), 0);
      assert_eq!(v["scopes"][1]["scope"], "global");
      assert_eq!(v["scopes"][1]["missing_outputs"][0]["name"], "create-api-endpoint");
      // The project scope alone is clean.
      assert_eq!(run(&e, &e.repo, &["doctor", "--project"]).status.code(), Some(0));
  }

  #[test]
  fn doctor_text_names_each_scope() {
      // Clean installs in both scopes: stdout has "project (" before "global (",
      // each followed by "doctor: clean"; exit code 0.
  }

  #[test]
  fn doctor_notes_an_item_installed_in_both_scopes() {
      // The same rule in both scopes. `doctor --json`:
      // v["in_both_scopes"] == [{"kind": "rule", "name": "api-conventions"}], exit code 0.
      // Text output contains "api-conventions" and "installed in both scopes".
      // `doctor --global`: v["in_both_scopes"] is empty.
  }

  #[test]
  fn doctor_reports_recorded_files_at_the_earlier_global_paths() {
      let e = env();
      released_global_install(&e);
      let out = run(&e, &e.outside, &["doctor", "--global", "--json"]);
      assert_eq!(out.status.code(), Some(1));
      let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
      let files: Vec<String> = /* every v["scopes"][0]["earlier_layout"][*]["files"][*] */;
      for rel in OLD { assert!(files.iter().any(|f| f == rel), "{rel}"); }
      assert_eq!(v["scopes"][0]["missing_outputs"].as_array().unwrap().len(), 0);
      let text = stdout(&run(&e, &e.outside, &["doctor", "--global"]));
      assert!(text.contains("upskill update --global"), "{text}");
      // After the update, the global scope is clean.
      run(&e, &e.outside, &["update", "--global", "--yes"]);
      assert_eq!(run(&e, &e.outside, &["doctor", "--global"]).status.code(), Some(0));
  }

  #[test]
  fn doctor_in_the_home_directory_reports_no_item_in_both_scopes() {
      // cwd = home, `add --global`, `doctor --json`: one scope, in_both_scopes empty.
  }
  ```

- [ ] **Step 2: Run them and confirm the failures**

- [ ] **Step 3: Implement the report types, `doctor(at)`, `doctor_scopes` and
      the printing**

  `in_both_scopes` holds each `(kind, name)` present in the lockfiles of two
  reported scopes, sorted by kind then name. Text format: the header line
  `<scope> (<root>)` for each scope, then the current report. The earlier-layout
  block, in the style of the existing blocks:

  ```text
  doctor: files at earlier global paths (N item(s)) — run 'upskill update --global' to move them
  ```

  The both-scopes note, after the last scope, one line for each item:

  ```text
  note: <kind> <name> is installed in both scopes
  ```

  `doctor --json` prints the `DoctorScopes` value. Exit code 1 when
  `!is_clean()`.

- [ ] **Step 4: Update the existing `doctor` tests to the new shape**

  `doctor_json` in `tests/cli_doctor.rs` passes `--project` and returns
  `v["scopes"][0]`; the helpers of the other `cli_doctor*` files do the same.
  Unit tests in `lifecycle.rs` build a `ScopeRoot` with `Scope::Project`.

- [ ] **Step 5: Run `cargo test` — all pass. Commit**

  `feat: report both scopes and earlier global paths in doctor`

---

### Task 5: `update` and `remove` say when an item is in the other scope

**Files:**

- Modify: `src/pipeline/lifecycle.rs`, `src/pipeline/mod.rs`, `src/main.rs`
  (`run_update`, `run_remove`)
- Test: `tests/cli_scope.rs`

**Interfaces:**

- Produces:
  `pub fn installed_names(root: &Path, names: &[String]) -> Vec<String>` — the
  names of `names` that the lockfile under `root` holds as an item; empty when
  the lockfile is missing or cannot be read.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn update_and_remove_say_when_the_item_is_in_the_other_scope() {
      let e = env();
      run(&e, &e.repo, &["add", "--global", e.source.to_str().unwrap(), "api-conventions"]);
      for cmd in ["update", "remove"] {
          let out = run(&e, &e.repo, &[cmd, "api-conventions"]);
          assert_eq!(out.status.code(), Some(1), "{cmd}");
          let err = stderr(&out);
          assert!(err.contains("not in lockfile: api-conventions"), "{err}");
          assert!(err.contains(&format!(
              "'api-conventions' is installed in the global scope ({}); pass --global",
              e.home.display())), "{err}");
      }
      assert!(e.home.join(".claude/rules/api-conventions.md").is_file());
  }

  #[test]
  fn the_hint_names_the_project_scope_for_a_global_command() {
      // Project install only; `update --global api-conventions` in the repository:
      // stderr contains "is installed in the project scope (" and "pass --project".
  }

  #[test]
  fn no_hint_for_a_name_that_no_scope_holds() {
      // `remove nothing-here`: stderr contains "not in lockfile" and not "is installed in".
  }
  ```

- [ ] **Step 2: Run them and confirm the failures**

- [ ] **Step 3: Implement `installed_names` and the hint**

  In the `Err` branch of `run_update` and `run_remove`, when names were passed,
  `main.rs` resolves the root of `at.scope.other()` with `scope_root`, calls
  `installed_names`, and prints one line for each returned name after the error
  chain, in place of the `run 'upskill doctor'` hint:

  ```text
  hint: '<name>' is installed in the <scope> scope (<root>); pass --<scope>
  ```

  No hint is printed when the other root equals the current root or cannot be
  resolved.

- [ ] **Step 4: Run `cargo test` — all pass. Commit**

  `feat: say when a named item is installed in the other scope`

---

### Task 6: Documentation and follow-up issues

**Files:**

- Modify: `docs/specification/specification.md` (§1.2, the scope paragraph of
  `add` near line 93, `update`, `remove`, `list`, `doctor`, the lockfile
  location table), `docs/specification/format-spec.md` (§7 output paths),
  `docs/book/commands.md` (lines 59-60, `list`, `doctor`, `remove-mcp` near line
  471), `docs/adr/0003-generation-pipeline.md` (output path table),
  `docs/roadmap.md` (step 5), `AGENTS.md` ("Install layout")

- [ ] **Step 1: Make each document describe the code of Tasks 1 to 5**

  State, with the exact values of this plan: the scope line; the refusal of
  `add` and that `update`, `remove` and `remove-mcp` use the global scope
  outside a git repository; that `list` and `doctor` report both scopes and what
  a scope flag does; the JSON shapes; the global Copilot path table; that
  `update --global` moves an install from the earlier global paths and keeps an
  edited file; the two `doctor` additions and their effect on the exit code; the
  other-scope hint. State the limits of decisions 2, 3 and 4 in the user guide.
  §1.2 of the specification names `<cwd>/.agents/` and `$HOME/.agents/` as where
  a scope lives; replace that with the scope roots and a link to the path table.
  Search every modified document for a sentence that the change makes false:
  `grep -n -i "fall\(s\|ing\)\? back\|auto-detect\|\$HOME/\.github\|~/\.github" docs/specification docs/book docs/adr/0003-generation-pipeline.md AGENTS.md src/cli.rs`.

- [ ] **Step 2: Build the docs**

  Run: `just fmt`, then `just book`. Expected: no warning.

- [ ] **Step 3: Open the follow-up issues (needs the user's confirmation)**

  Two debt issues under `Epic: #283` with the labels `debt`,
  `epic:customization-mvp`, `K2`, `S`, `P2`, and an entry in the epic task list:
  the hand-shake files at global scope (decision 4), and the confirmed deletion
  of unrecorded files at earlier paths (decision 9).

- [ ] **Step 4: Run `just verify` and `just sanity` — both pass. Commit**

  `docs: describe scope reporting and the global Copilot paths`

## After this step

The whole-branch review (`/review` passes 1 and 2) runs before the pull request.
Delivery step 6 (recorded commits and `update --check`) gets its own plan; its
check lists updates for both scopes and can reuse `report_scopes`.
