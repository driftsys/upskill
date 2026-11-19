# upskill pack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** `upskill pack <bundle>` writes a Claude Code plugin directory and a
Copilot CLI plugin directory from one bundle, with the supporting changes to
`update --notice` (run lock), `add` (`--kind`) and `update` (keeps the bundle
and kind restriction).

**Architecture:** `pack` reuses the bundle closure and the generation pipeline
of `add` through one new planning function in `src/pipeline/`, and a new library
module `src/pack.rs` turns that plan into the file list of one plugin directory.
`main.rs` writes the files. The init command of a plugin is one fixed
`upskill add ... --kind rule` line, so `add` gains a kind filter and the
lockfile records the restriction for `update`.

**Tech Stack:** Rust 2024, MSRV 1.85, `clap` derive, `serde_json`, `assert_cmd`

- `tempfile` for acceptance tests. No new dependency.

**Spec:**
[docs/wip/specs/2026-11-18-bundle-plugin-output-design.md](../specs/2026-11-18-bundle-plugin-output-design.md).
Story: [driftsys/upskill#563](https://github.com/driftsys/upskill/issues/563).
Context only:
[hook item kind design](../../wip/specs/2026-11-18-hook-item-kind-design.md),
AD-0040, AD-0041, AD-0042.

## Global Constraints

- No new dependency. No `tokio`, `reqwest`, `git2`, `dialoguer`.
- Only `main.rs` writes to stdout and stderr. Library modules return data or
  `Result<T>`.
- Zero warnings: compiler, clippy (also on test code: run `just sanity`), docs.
- Integration tests build the binary with `common::upskill_cmd(&fake_home)`,
  never `Command::cargo_bin("upskill")`.
- Exit codes: success 0, general error 1, usage error 2.
- `update --notice` exits with code 0 in every case after argument parsing,
  writes nothing to stderr and never fetches.
- The bundle schema does not change. `pack` reads no lockfile and writes only
  below `--out`. upskill does not install or register the plugin.
- No back-compat work before 1.0: a new lockfile field is optional on read and
  needs no migration.
- Comments, commit messages and docs: plain literal English. Conventional
  Commits. The repository is on GitHub, so the GitLab commit trailer does not
  apply.
- Every branch works in its own worktree under `.claude/worktrees/<branch>`. Run
  `./bootstrap` once in a new worktree.

## Findings that change the spec's delivery order

1. **`update` widens a bundle install today.** Run with the released 0.9.0 on
   2026-11-19: after `upskill add ./src bun --claude` (bundle `bun` lists skill
   `a` and rule `r`; the source also holds skill `b`), `upskill update --yes`
   wrote `.claude/skills/b/SKILL.md`. Cause: for a source with a recorded
   bundle, `update` passes an empty item filter (`src/pipeline/lifecycle.rs`,
   `is_bundle_source`), and for a directory source an empty filter installs
   every item of the source.
2. **Nothing records a kind restriction.** With finding 1, the first
   `upskill update` after the init command would install the skills and agents
   of the source into the repository. Each skill would then be exposed twice,
   which the spec excludes (section 3).

Task 3 handles both. It adds one optional lockfile field, which the spec does
not name. **Decision needed from the user:** see "Open decisions", item 1.

## Review Focus

Each line names the task whose tests pin it.

1. A lock file whose modification time is later than the current time (the clock
   moved back): the run treats it as old, deletes it and prints the notice.
   Task 1.
2. The cache directory cannot be created, so no lock file can be created
   (`<cache>/upskill` is a regular file): the run continues without a lock, as
   it does today, and still exits with code 0. Task 1.
3. A `--source` value for `pack` that holds whitespace or a shell metacharacter:
   refused with exit code 2, because the value is written into a command line
   that a model runs in a shell. Task 4.
4. An item that the user installed directly from the same source as a bundle:
   `update` keeps it after Task 3 changes the item filter. Task 3.
5. A rule whose `audience` excludes the client: it does not count as a rule of
   that client, so a bundle whose only rules exclude Copilot gets no init
   command and no hook in the Copilot directory. Task 5.

---

## File Structure

| File                                                                                                                                                                                    | Change | Responsibility                                                    |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ----------------------------------------------------------------- |
| `src/notice.rs`                                                                                                                                                                         | modify | Run lock: acquire, age rule, release on drop                      |
| `src/main.rs`                                                                                                                                                                           | modify | Take the lock in `notice_lines`; `run_add` kinds; `run_pack`      |
| `src/cli.rs`                                                                                                                                                                            | modify | `add --kind`; `Commands::Pack`                                    |
| `src/pipeline/mod.rs`                                                                                                                                                                   | modify | `AddOptions.kinds`; export `plan_bundle_output`                   |
| `src/pipeline/plan.rs`                                                                                                                                                                  | modify | `PlanNames.kinds`, `PlanNames::takes`                             |
| `src/pipeline/install.rs`                                                                                                                                                               | modify | Kind filter in planning; `DependencyClosure::retain_kinds`        |
| `src/pipeline/prepared.rs`                                                                                                                                                              | modify | Kind filter in `prepare_install`; skip plugins and MCP in commit  |
| `src/pipeline/lifecycle.rs`                                                                                                                                                             | modify | `update` passes bundle names and recorded kinds                   |
| `src/lockfile.rs`                                                                                                                                                                       | modify | `LockedBundle.kinds`                                              |
| `src/pipeline/bundle_plan.rs`                                                                                                                                                           | create | Plan the generated files of a bundle closure for one client       |
| `src/pack.rs`                                                                                                                                                                           | create | Manifest, init text, hook file, file list of one plugin directory |
| `src/lib.rs`                                                                                                                                                                            | modify | `pub mod pack;`                                                   |
| `tests/cli_update_notice.rs`                                                                                                                                                            | modify | Run lock acceptance tests                                         |
| `tests/cli_add.rs`                                                                                                                                                                      | modify | `--kind` acceptance tests                                         |
| `tests/cli_update_bundle.rs`                                                                                                                                                            | modify | `update` keeps bundle and kinds                                   |
| `tests/cli_pack.rs`                                                                                                                                                                     | create | `pack` acceptance tests                                           |
| `tests/fixtures/pack/`                                                                                                                                                                  | create | Source registry and golden directories                            |
| `docs/adr/0044-plugin-output-of-a-bundle.md`                                                                                                                                            | create | Decision record (Decision 5 of the spec)                          |
| `docs/book/commands.md`, `docs/book/recipes.md`, `docs/book/support-levels.md`, `docs/specification/specification.md`, `docs/adr/README.md`, `AGENTS.md`, `skills/upskill-cli/SKILL.md` | modify | As-built documentation                                            |

---

### Task 0: Prepare the branch

**Files:** none.

- [ ] **Step 1:** In the worktree
      `.claude/worktrees/docs-bundle-plugin-output-spec`, bring in `origin/main`
      (2 commits ahead, including a change to
      `plugins/upskill-copilot/hooks.json`):
      `git fetch origin && git merge -X theirs origin/main -m "chore: merge main into the pack branch"`.
      The git-std commit hook can leave the worktree with a detached HEAD;
      reattach with `git checkout -- . && git checkout <branch>`.
- [ ] **Step 2:** Create the implementation branch and worktree named in "Open
      decisions", item 2, from the merged branch. Run `./bootstrap` in it.
- [ ] **Step 3:** Run `just check`. Expected: all tests pass before any change.
      Record the commit as the base of Task 1.

### Task 1: Run lock in `update --notice`

**Files:**

- Modify: `src/notice.rs` (new items after `CACHE_FILE_NAME`; module doc)
- Modify: `src/main.rs` (`notice_lines`, about line 1164)
- Test: `tests/cli_update_notice.rs`, unit tests in `src/notice.rs`

**Interfaces:**

- Produces, in `src/notice.rs`:

```rust
pub const LOCK_FILE_NAME: &str = "notice.lock";
pub const LOCK_MAX_AGE_SECS: u64 = 60;

/// Deletes the lock file when dropped.
pub struct RunLock { path: PathBuf }

pub enum LockAttempt {
    /// This run created the lock file.
    Held(RunLock),
    /// Another run holds a lock younger than `LOCK_MAX_AGE_SECS`.
    Busy,
    /// The lock file cannot be created for another reason.
    Unavailable,
}

pub fn acquire_run_lock(path: &Path, now: std::time::SystemTime) -> LockAttempt;
```

- [ ] **Step 1: Write the failing unit tests** in `src/notice.rs`:

```rust
fn lock_path(dir: &tempfile::TempDir) -> PathBuf { dir.path().join("cache").join(LOCK_FILE_NAME) }
fn set_age(path: &Path, now: SystemTime, secs: u64) {
    fs::File::options().write(true).open(path).unwrap()
        .set_modified(now - Duration::from_secs(secs)).unwrap();
}

#[test] fn the_first_run_holds_the_lock_and_creates_the_directory()
// acquire -> Held; lock_path exists.
#[test] fn dropping_the_lock_deletes_the_file()
#[test] fn a_lock_younger_than_60_seconds_is_busy()
// file exists with age 59 -> Busy; file still exists.
#[test] fn a_lock_of_60_seconds_is_replaced()
// age 60 -> Held.
#[test] fn a_lock_dated_after_now_is_replaced()
// set_modified(now + 3600 s) -> Held.
#[test] fn a_directory_that_cannot_be_created_is_unavailable()
// parent of the lock path is a regular file -> Unavailable.
```

- [ ] **Step 2:** Run `cargo test --lib notice::`. Expected: compile error,
      `acquire_run_lock` not found.
- [ ] **Step 3: Implement `acquire_run_lock`.** Create the parent directory,
      then open with `OpenOptions::new().write(true).create_new(true)`. On
      `AlreadyExists`: read the modification time; when
      `now.duration_since(mtime)` is `Ok(age)` with `age < 60 s`, return `Busy`;
      otherwise delete the file and try `create_new` once more, and return
      `Busy` when that second attempt finds a file. Any other error returns
      `Unavailable`. Update the module doc: it states that only `load` and
      `save` perform file I/O.
- [ ] **Step 4:** Run `cargo test --lib notice::`. Expected: PASS.
- [ ] **Step 5: Write the failing acceptance tests** in
      `tests/cli_update_notice.rs`. Use the setup of
      `notice_prints_one_line_once_for_a_set_of_updates` (one unreported
      update), and `let lock = cache_path(&e).with_file_name("notice.lock");`.

```rust
#[test] fn notice_with_a_fresh_lock_file_prints_nothing_and_leaves_the_cache_unchanged()
// Create the lock file. Record the bytes of notice.json (or its absence).
// Run: stdout "", stderr "", exit 0, notice.json bytes equal, lock file
// still exists. Assert no contact with the source by the same means as
// `notice_does_not_contact_the_source_again_within_the_period`.
#[test] fn notice_with_a_lock_file_of_60_seconds_prints_the_notice_and_deletes_the_lock()
// Lock file with modification time now - 60 s. Run: stdout is the notice
// line, exit 0, lock file absent.
#[test] fn a_completed_notice_run_leaves_no_lock_file()
#[test] fn notice_with_a_lock_file_dated_in_the_future_prints_the_notice()
#[test] fn notice_prints_when_the_cache_directory_is_a_regular_file()
// XDG_CACHE_HOME/upskill is a regular file (use `run_xdg`): the notice line
// prints, exit 0, stderr "".
```

Extend `notice_leaves_only_the_notice_file_in_the_cache_directory` only if it
fails; the lock file is deleted before the process ends.

- [ ] **Step 6:** Run `cargo test --test cli_update_notice`. Expected: the two
      lock tests with a present lock file FAIL (the notice prints).
- [ ] **Step 7: Use the lock in `notice_lines`.** Directly after
      `notice_cache_path()?`, before any read of `notice.json`: `Held(lock)`
      keeps `lock` alive to the end of the function; `Busy` returns `Some(())`
      at once; `Unavailable` continues without a lock. The lock path is
      `cache_path.with_file_name(notice::LOCK_FILE_NAME)`. Do not change
      `NOTICE_TIME_LIMIT` (5 seconds for all lookups, which is below the
      60-second age limit). Do not change the interrupt handler: an interrupted
      run leaves the lock file, which the spec lists as a known limit.
- [ ] **Step 8:** Run `cargo test --test cli_update_notice` and
      `cargo test --test cli_interrupted`. Expected: PASS.
- [ ] **Step 9: Commit:**
      `feat(update): add a run lock to update --notice (#563)`.

### Task 2: `add --kind`

**Files:**

- Modify: `src/cli.rs` (`Commands::Add`), `src/main.rs` (`AddArgs`, `run_add`)
- Modify: `src/pipeline/mod.rs` (`AddOptions`), `src/pipeline/plan.rs`,
  `src/pipeline/install.rs`, `src/pipeline/prepared.rs`,
  `src/pipeline/lifecycle.rs` (the `AddOptions` literal only)
- Test: `tests/cli_add.rs`

**Interfaces:**

- Produces:

```rust
// src/pipeline/mod.rs, in AddOptions
/// Item kinds to install. Empty means every kind. When it is not empty, the
/// install runs no plugin step and no MCP step.
pub kinds: Vec<ItemKind>,

// src/pipeline/plan.rs, in PlanNames
pub kinds: Vec<ItemKind>,
impl PlanNames { pub fn takes(&self, kind: ItemKind) -> bool; } // true when `kinds` is empty

// src/pipeline/install.rs
impl DependencyClosure {
    /// Remove every item whose kind is not in `kinds` from `by_source`,
    /// `item_source` and the keys of `required_by`. No-op for an empty slice.
    pub(super) fn retain_kinds(&mut self, kinds: &[ItemKind]);
}
```

- CLI:
  `#[arg(short = 'k', long = "kind", value_name = "KIND")] kind: Vec<String>` on
  `Add`, parsed in `run_add` with `ItemKind::from_str`; an unknown value prints
  `unknown item kind: <value>` and returns exit code 2.

- [ ] **Step 1: Write the failing acceptance tests** in `tests/cli_add.rs`. The
      fixture is a local source with rule `r`, skill `s`, agent `g`, and a
      bundle `bun` that lists all three, declares one Claude plugin and one MCP
      server.

```rust
#[test] fn add_kind_rule_installs_only_the_rules_of_a_bundle()
// add <src> bun --kind rule --project --claude: .claude/rules/r.md exists;
// .claude/skills and .claude/agents do not exist; the lockfile `items` holds
// only rule:r.
#[test] fn add_kind_is_repeatable()
// --kind rule --kind agent: rule and agent, no skill.
#[test] fn add_kind_skips_the_plugins_and_mcp_servers_of_a_bundle()
// The lockfile has no `plugins` and no `mcps` entry, stdout and stderr name
// neither, and no `.mcp.json` is written. Put a fake `claude` script first
// on PATH that writes a marker file; assert that the marker is absent.
#[test] fn add_kind_applies_to_a_source_without_item_names()
// add <src> --kind skill: only the skill.
#[test] fn add_kind_does_not_install_a_required_item_of_another_kind()
// Rule r2 `requires` skill s. add <src> r2 --kind rule: no skill file, exit 0,
// and `upskill doctor --project` exits 0.
#[test] fn add_with_an_unknown_kind_is_a_usage_error()
// --kind hook: exit 2, stderr contains "unknown item kind: hook", no file.
```

- [ ] **Step 2:** Run `cargo test --test cli_add add_kind`. Expected: FAIL, clap
      rejects `--kind`.
- [ ] **Step 3: Add the flag and the option fields.** `run_add` fills
      `AddOptions.kinds`. `prepare_install` copies it into `entry_names` and
      into the two `PlanNames` values for dependency sources. `lifecycle.rs`
      sets `kinds: vec![]` (Task 3 changes it).
- [ ] **Step 4: Apply the filter.** The filter follows the places where
      `excludes` is applied:
  - `install_items_of_kind`: return at once when `!names.takes(kind)`.
  - `collect_source_items`: an item of another kind gets `installed: None`, so
    its directory stays protected by the overlap check.
  - `prepare_install`: filter `requested` and the `incoming` list of the
    conflict check by kind, and call `closure.retain_kinds(&options.kinds)`
    directly after the closure is built, before the conflict check.
  - `commit_adopted`: when `options.kinds` is not empty, skip
    `install_plugins_from_bundles` and `install_mcps_from_bundles`. The
    recording loops then record nothing.
- [ ] **Step 5:** Run `cargo test --test cli_add` and `just check`. Expected:
      PASS.
- [ ] **Step 6: Commit:**
      `feat(add): add --kind to restrict an install to item kinds (#563)`.

### Task 3: `update` keeps the bundle and the kind restriction

**Files:**

- Modify: `src/lockfile.rs` (`LockedBundle`), `src/pipeline/prepared.rs` (the
  two `upsert_bundle` calls), `src/pipeline/lifecycle.rs` (`update`, about lines
  940-972)
- Test: `tests/cli_update_bundle.rs`, unit test in `src/lockfile.rs`

**Interfaces:**

- Consumes: `AddOptions.kinds` (Task 2).
- Produces, in `LockedBundle`:

```rust
/// The kinds that `add --kind` restricted this bundle to. Empty: every kind.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub kinds: Vec<ItemKind>,
```

- [ ] **Step 1: Write the failing tests** in `tests/cli_update_bundle.rs`:

```rust
#[test] fn update_does_not_install_an_item_of_the_source_that_the_bundle_does_not_list()
// The 2026-11-19 reproduction: source with skills a, b, rule r; bundle lists
// a and r. add <src> bun --claude, then update --yes: .claude/skills/b does
// not exist.
#[test] fn update_keeps_the_kind_restriction_of_add()
// add <src> bun --kind rule --project --claude; change the body of rule r and
// of skill a in the source; update --yes: the rule file has the new body, no
// skill file exists, the lockfile bundle entry holds "kinds": ["rule"], and
// the lockfile has no `plugins` and no `mcps` entry.
#[test] fn update_installs_a_rule_that_the_bundle_gained()
// After add --kind rule, add rule r2 to the bundle; update --yes writes it.
#[test] fn update_keeps_an_item_installed_directly_beside_a_bundle_of_the_same_source()
// add <src> bun --claude, then add <src> b --claude; update --yes keeps
// .claude/skills/b/SKILL.md and its lockfile entry.
```

Unit test in `src/lockfile.rs`: a bundle entry without `kinds` loads with an
empty list, and an empty list is not written.

- [ ] **Step 2:** Run `cargo test --test cli_update_bundle`. Expected: the first
      three new tests FAIL.
- [ ] **Step 3: Record the kinds.** Both `upsert_bundle` calls in
      `commit_adopted` set `kinds: options.kinds.clone()`.
- [ ] **Step 4: Change the item filter of `update`.** For a source whose label
      does not end with `BUNDLE_SUFFIX` and that has recorded bundles, the
      filter is: the names of the recorded bundles of that source, then the
      source name of each entry of that source that has an empty `required_by`
      and that no recorded bundle of that source lists in `items`. A
      `.bundle.yaml` source keeps the empty filter. `AddOptions.kinds` is the
      union of the `kinds` of the recorded bundles of the source when every one
      of them has a non-empty list, else empty.
- [ ] **Step 5:** Run `cargo test --test cli_update_bundle`, then `just check`
      (the `cli_update*` and `pipeline_*` suites pin the existing behaviour).
      Expected: PASS. An existing test that fails because it relied on the
      widened install is reported to the user before its expectation changes.
- [ ] **Step 6: Commit:**
      `fix(update): keep the bundle selection and the kind restriction of add (#563)`.

### Task 4: `upskill pack` for Claude Code

**Files:**

- Create: `src/pipeline/bundle_plan.rs`, `src/pack.rs`, `tests/cli_pack.rs`,
  `tests/fixtures/pack/registry/**`,
  `tests/fixtures/pack/expected/demo-claude/**`
- Modify: `src/pipeline/mod.rs`, `src/lib.rs`, `src/cli.rs`, `src/main.rs`

**Interfaces:**

- Produces:

```rust
// src/pipeline/bundle_plan.rs, re-exported from src/pipeline/mod.rs
pub struct BundleFile {
    pub kind: ItemKind,
    pub name: String,
    /// Relative to the client root: the planned path without its first
    /// component (`.claude/skills/a/SKILL.md` -> `skills/a/SKILL.md`).
    pub rel: PathBuf,
    pub bytes: Vec<u8>,
    pub mode: Option<u32>,
}
pub struct BundleOutput {
    /// The closure, dependencies before dependents; the last is the entry.
    pub bundles: Vec<crate::model::Bundle>,
    /// Skill and agent files for the client. No rule file.
    pub files: Vec<BundleFile>,
    /// Names of the rules that generate output for the client, sorted.
    pub rules: Vec<String>,
}
pub fn plan_bundle_output(registry_root: &Path, bundle: &str, client: Client) -> Result<BundleOutput>;

// src/pack.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackClient { Claude, Copilot }
pub struct PackFile { pub rel: PathBuf, pub bytes: Vec<u8>, pub mode: Option<u32> }
pub struct PluginDir { pub dir_name: String /* "<bundle>-claude" */, pub files: Vec<PackFile> }
pub fn validate_source(source: &str) -> Result<()>;
pub fn build(registry_root: &Path, bundle: &str, source: &str, client: PackClient) -> Result<PluginDir>;
```

- CLI:
  `Commands::Pack { bundle: String, source: String (--source, required),
  out: PathBuf (--out, required), force: bool, claude: bool, copilot: bool }`,
  `display_order = 13`, described as an author command. In this task the Copilot
  client returns the error `pack for Copilot CLI is not implemented` from
  `build`, and `main.rs` selects only Claude when no flag is given; Task 5
  removes both.

- [ ] **Step 1: Create the fixture registry** `tests/fixtures/pack/registry/`:
      bundle `demo` (`description: Demo bundle`, `license: MIT`,
      `metadata.version: 1.2.0`, items: skill `alpha` with one resource file
      `reference.md`, agent `helper`, rule `style`, `requires: [base]`, one
      Claude plugin `tool` with `install_url: https://example.com/tool`, one
      Copilot plugin `cotool` without `install_url`, one MCP server `srv`),
      bundle `base` (skill `beta`, rule `naming`), bundle `norules` (skill
      `alpha` only), and a skill `stray` that no bundle lists.
- [ ] **Step 2: Create the golden directory** `expected/demo-claude/`. Write by
      hand: `.claude-plugin/plugin.json`, `commands/init.md`, and
      `hooks/hooks.json` as a copy of `plugins/upskill-claude/hooks/hooks.json`.
      Produce `skills/**` and `agents/**` by running the released
      `upskill add
  <registry> demo --claude` in a temporary project and
      copying `.claude/skills` and `.claude/agents`.

  `.claude-plugin/plugin.json` (two-space indent, final newline, this key order,
  `version` and `license` omitted when the bundle has none):

```json
{
  "name": "demo",
  "description": "Demo bundle",
  "version": "1.2.0",
  "license": "MIT"
}
```

`commands/init.md` for `--source driftsys/demo@v1`:

````markdown
---
description: Install the rules of the demo bundle into this repository
---

Run this command in the root directory of the repository. Then report its
output to the user.

```text
upskill add driftsys/demo@v1 demo --kind rule --project --claude
```

If the command fails, report the error text. Do not run another command in its
place.

The command does not install the following. After the install, give this list
to the user:

- plugin `tool`: https://example.com/tool
- MCP server `srv`
````

The last paragraph and the list are absent when the closure declares no plugin
for the client and no MCP server. Plugins come first, then MCP servers, each
sorted by name. A plugin whose descriptor has no `install_url` is
`- plugin \`<name>\``.

- [ ] **Step 3: Write the failing acceptance tests** in `tests/cli_pack.rs`.
      Each test runs in a copy of the fixture registry as the current directory.

```rust
#[test] fn pack_writes_the_claude_plugin_directory()
// pack demo --source driftsys/demo@v1 --out <tmp>/out --claude: the tree
// <out>/demo-claude equals the golden directory (same file set, same bytes).
#[test] fn pack_output_holds_no_rule_and_no_item_outside_the_closure()
// No path contains "rules/", "style", "naming" or "stray"; skills/beta
// (from `base`) exists.
#[test] fn pack_hook_file_equals_the_notice_plugin()
// Bytes of demo-claude/hooks/hooks.json equal plugins/upskill-claude/hooks/hooks.json.
#[test] fn pack_of_a_bundle_without_rules_writes_no_init_command_and_no_hook()
// pack norules: no commands/ and no hooks/ directory; the manifest has no
// "version" and no "license" key.
#[test] fn pack_refuses_a_non_empty_output_directory()
// <out>/demo-claude holds a file: exit 1, stderr names the directory and
// `--force`, the file is unchanged, nothing else is written.
#[test] fn pack_with_force_replaces_the_output_directory()
// The earlier file is absent afterwards; the tree equals the golden one.
#[test] fn pack_of_an_unknown_bundle_fails()
// exit 1, stderr contains "nope", <out> is not created.
#[test] fn pack_refuses_a_source_with_whitespace_or_a_shell_metacharacter()
// --source "a/b; rm -rf x", "a/b c", "$(x)", "a/b`x`": exit 2, no file.
#[test] fn pack_reads_no_lockfile_and_writes_only_below_out()
// The registry holds an invalid `.upskill-lock.json` ("not json"): pack
// succeeds, and the tree of the registry and of the fake home is unchanged.
#[test] fn pack_requires_source_and_out()
// Missing --source or --out: exit 2.
```

- [ ] **Step 4:** Run `cargo test --test cli_pack`. Expected: FAIL, clap rejects
      `pack`.
- [ ] **Step 5: Implement `plan_bundle_output`.** Call
      `resolve_bundle_closure(&label, registry_root, None, None, &[bundle])`
      with the label of the local path source, then
      `install_from_local_path_selective(&si.root, Some(&si.items),
  &ClientSelection::restrict(..client..), &PlanNames::default(),
  Scope::Project)`
      for each source of the closure, as `prepare_install` does. Keep the
      `DependencyClosure` alive until the planned bytes are collected. An
      unknown bundle name returns the error
      `no bundle named '<name>' in <registry_root>`. The function loads no
      lockfile. A bundle of another source in the `requires` closure is fetched,
      as for `add`.
- [ ] **Step 6: Implement `src/pack.rs`.** `validate_source`:
      `parse_install_source` must accept the value, and the value must hold only
      ASCII letters, digits and `/ @ : . _ - ~ +`. The hook file is
      `include_str!("../plugins/upskill-claude/hooks/hooks.json")`. The manifest
      is a `Serialize` struct with the four fields in the order above. Unit
      tests in the module cover the init text with and without the list and the
      manifest with and without the optional fields.
- [ ] **Step 7: Implement `run_pack` in `main.rs`.** Order: validate the source
      (exit 2); build every selected client in memory; check every target
      directory (`<out>/<dir_name>`: absent or empty, else exit 1 unless
      `--force`; a path that is not a directory is always refused); then write.
      With `--force`, delete the target directory before writing. Print one line
      for each directory to stdout unless `--quiet`: `Wrote <path> (<n> files)`.
      On Unix, apply `mode` to a copied resource.
- [ ] **Step 8:** Run `cargo package --list --allow-dirty | grep plugins/`.
      Expected: both hook files are listed, so `include_str!` works for a build
      from the published crate. If they are not listed, add `plugins/` to the
      `include` list in `Cargo.toml`.
- [ ] **Step 9:** Run `cargo test --test cli_pack`,
      `cargo test --test
  cli_mangen`, `just check`. Expected: PASS.
- [ ] **Step 10: Commit:**
      `feat(pack): build a bundle as a Claude Code plugin (#563)`.

### Task 5: `upskill pack` for Copilot CLI

**Files:**

- Modify: `src/pack.rs`, `src/main.rs`, `tests/cli_pack.rs`
- Create: `tests/fixtures/pack/expected/demo-copilot/**`

**Interfaces:**

- Consumes: everything of Task 4. No new public item.

- [ ] **Step 1: Create the golden directory** `expected/demo-copilot/`:
      `plugin.json` at the root (same content as the Claude manifest),
      `hooks.json` at the root as a copy of
      `plugins/upskill-copilot/hooks.json`, `commands/init.md` with the flag
      `--copilot` and the list `- plugin \`cotool\``and`- MCP server
      \`srv\``,`skills/**`and`agents/<name>.agent.md`copied from`.github/skills`and`.github/agents`of
  an`upskill
      add <registry> demo --copilot` run.
- [ ] **Step 2: Write the failing tests:**

```rust
#[test] fn pack_writes_the_copilot_plugin_directory()        // --copilot, golden compare
#[test] fn pack_without_a_client_flag_writes_both_directories()
#[test] fn pack_copilot_hook_file_equals_the_notice_plugin()
#[test] fn pack_checks_both_directories_before_it_writes_one()
// demo-copilot is not empty, no flag: exit 1 and demo-claude is not created.
#[test] fn pack_gives_no_init_command_to_a_client_that_no_rule_targets()
// Add the fixture bundle `claudeonly` to the registry: one skill, and one
// rule with `audience: [claude]`. claudeonly-copilot has no commands/ and no
// hooks.json; claudeonly-claude has both.
```

- [ ] **Step 3:** Run `cargo test --test cli_pack`. Expected: the new tests FAIL
      with `pack for Copilot CLI is not implemented`.
- [ ] **Step 4: Implement the Copilot layout** in `pack::build` and make
      `run_pack` select both clients when no flag is given. Remove the temporary
      error.
- [ ] **Step 5:** Run `cargo test --test cli_pack`, `just check`. Expected:
      PASS.
- [ ] **Step 6: Commit:**
      `feat(pack): build a bundle as a Copilot CLI plugin (#563)`.

### Task 6: Documentation and decision record

**Files:**

- Create: `docs/adr/0044-plugin-output-of-a-bundle.md`
- Modify: `docs/adr/README.md`, `docs/book/commands.md`, `docs/book/recipes.md`,
  `docs/book/support-levels.md`, `docs/specification/specification.md`,
  `AGENTS.md`, `skills/upskill-cli/SKILL.md`

- [ ] **Step 1: Write ADR-0044.** Content: a plugin directory is one output of a
      bundle, and ADR-0013 keeps the bundle and the native plugin package as
      distinct concepts; upskill does not install the plugin (AD-0042, Decision
      8); two directories (AD-0041); rules through the init command; the
      embedded notice hook with the run lock; the `kinds` record of a bundle in
      the lockfile; alternatives from the spec.
- [ ] **Step 2: Update the user guide and the specification** as built: `pack`
      (flags, output layout, refusal rule, `--source` character rule, no README
      and no LICENSE file is written, so `claude plugin validate` reports both
      as missing), `add --kind` (no plugin step, no MCP step), the `kinds` field
      of a lockfile bundle entry, `update` for a bundle of a directory source,
      the run lock of `update --notice` with its two known limits, and the known
      limits of the spec. State in `support-levels.md` that plugin output is
      experimental until a maintainer runs a smoke test with a marketplace
      install. `doctor` gains no row.
- [ ] **Step 3: Update `AGENTS.md`:** module layout (`pack.rs`), test list
      (`cli_pack`), the `update --notice` paragraph (lock file).
- [ ] **Step 4:** Run `just fmt`, `just verify`, `just sanity`, `just book`.
      Expected: no error and no warning.
- [ ] **Step 5: Commit:**
      `docs: describe pack, add --kind and the notice run lock (#563)`.
- [ ] **Step 6: Whole-branch review:** `/review` passes 1 and 2.
- [ ] **Step 7: Garden:** run the `sdd-gardening` skill for the bundle plugin
      output spec and this plan. The hook item kind spec stays in `docs/wip/`:
      it describes a later slice.
- [ ] **Step 8 (maintainer, paid model calls):** smoke run with a marketplace
      install in Claude Code and Copilot CLI: install both directories from a
      marketplace repository, check that the skills and the agent are visible,
      run the init command, start a session with an available update. Record the
      result in a technote. This step does not block the pull request.

---

## Execution

- **Implementer of each task:** Codex CLI with the `sol` or `astra` model, as
  the user proposed on 2026-11-19, one task for each run, in the task order
  above. Tasks 1 and 2 are independent of each other. Task 3 needs Task 2, Task
  4 needs Task 2 for its init text only, Task 5 needs Task 4.
- **Review gate after each task:** a Claude subagent reviews the diff of the
  task, computed from the base commit recorded before the task started, and
  returns two verdicts: the task meets its brief, and the code is sound. It
  checks every doc statement that the diff adds, changes or makes false. A Codex
  review seat can run beside it and does not replace it.
- **Pull request:** one for the story, squash merge.

## Open decisions

1. **Task 3 and the lockfile field.** Recommended: accept Task 3 as written
   (optional `kinds` list on a lockfile bundle entry; `update` passes the
   recorded bundle names). Alternative: move the fix of finding 1 to a separate
   bug issue and pull request that lands first, and keep only the `kinds` part
   here. Without either, the init command of the spec does not hold across
   `update`.
2. **Branch.** Recommended: a new branch `feat/upskill-pack` and worktree from
   the merged `docs/bundle-plugin-output-spec`, so that the spec, the plan and
   the code land in one pull request. Alternative: continue on
   `docs/bundle-plugin-output-spec`.
3. **`--force`.** Recommended: delete `<out>/<bundle>-<client>` and write it
   again, so that a file of an earlier run does not stay in the plugin.
   Alternative: overwrite in place and keep other files.
4. **README and LICENSE.** Recommended: `pack` writes neither in this slice; the
   manifest carries the `license` value. The spec leaves this to the builder.
5. **`pack` from a remote source.** Recommended: no. `<bundle>` is resolved in
   the current directory only. The spec leaves this to the builder.

## Spec coverage

| Spec section                 | Task                     |
| ---------------------------- | ------------------------ |
| 1 Command                    | 4, 5                     |
| 2 Output directories         | 4, 5                     |
| 3 Content split              | 4 (no rule file)         |
| 4 Init command, `add --kind` | 2, 3, 4, 5               |
| 5 Updates, run lock          | 1 (lock), 4 and 5 (hook) |
| 6 Support level              | 6                        |
| 7 Code and tests             | 1, 2, 4, 5               |
| Decision 5 (ADR)             | 6                        |
| Delivery order step 5        | 6                        |
