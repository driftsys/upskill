# Upgrade UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `upskill update` auto-remove orphaned items, clean output before regenerating, and prompt before applying destructive changes.

**Architecture:** Three changes to the update pipeline: (1) detect items in lockfile that no longer exist in the fetched source and remove them, (2) delete per-client output files before regenerating to prevent stale file accumulation, (3) show a summary and confirm interactively before applying.

**Tech Stack:** Rust, clap 4, anyhow, serde_json, std::io::IsTerminal

---

### Task 1: Add `Removed` / `WouldRemove` variants to `UpdateStatus`

**Files:**

- Modify: `src/pipeline.rs:1049-1078`

- [ ] **Step 1: Add variants to `UpdateStatus`**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// SSOT hash matches the lockfile — nothing to do.
    UpToDate,
    /// `Apply` mode: the lockfile hash changed (or was previously unset
    /// and now resolved). Outputs were rewritten.
    Updated {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
    /// `DryRun` mode: SSOT hash differs from the lockfile entry; an
    /// `update` (without `--dry-run`) would rewrite outputs.
    WouldChange {
        old_hash: Option<String>,
        new_hash: Option<String>,
    },
    /// `Apply` mode: item no longer exists in the source. Outputs deleted
    /// and lockfile entry removed.
    Removed,
    /// `DryRun` mode: item no longer exists in the source; an `update`
    /// (without `--dry-run`) would remove it.
    WouldRemove,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: warnings about unmatched variants in `main.rs` pattern matches (non-exhaustive), but compilation succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/pipeline.rs
git commit -m "refactor: add Removed/WouldRemove variants to UpdateStatus"
```

---

### Task 2: Add `--yes` flag to the `Update` CLI command

**Files:**

- Modify: `src/cli.rs:109-122`
- Modify: `src/main.rs:64` (destructure the new field)

- [ ] **Step 1: Add `yes` field to `Update` variant in `cli.rs`**

In `src/cli.rs`, inside the `Update` struct, after the `project` field:

```rust
/// Skip the confirmation prompt. Already implicit when stdin is
/// not a terminal (CI / pipes) or when `--dry-run` is used.
#[arg(short = 'y', long = "yes")]
yes: bool,
```

- [ ] **Step 2: Update the destructure in `main.rs`**

In `src/main.rs`, change the `Update` match arm (around line 60-64) to destructure `yes`:

```rust
Commands::Update {
    names,
    dry_run,
    global,
    project,
    yes,
} => run_update(&names, dry_run, yes, global, project),
```

- [ ] **Step 3: Update `run_update` signature to accept `yes`**

```rust
fn run_update(names: &[String], dry_run: bool, yes: bool, global: bool, project: bool) -> i32 {
```

(For now, `yes` is unused — we'll wire it in Task 5.)

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: warning about unused `yes` parameter, no errors.

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat(cli): add --yes flag to update command"
```

---

### Task 3: Implement orphan detection and removal in `update()`

**Files:**

- Modify: `src/pipeline.rs` — the `update()` function (line 1090–1194)
- Test: `tests/cli_update.rs`

- [ ] **Step 1: Write failing ATDD test — orphan removed on update**

Append to `tests/cli_update.rs`:

```rust
#[test]
fn update_removes_orphaned_item_when_source_no_longer_contains_it() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    // Verify the item exists before removal.
    assert!(target.join(".claude/skills/create-api-endpoint/SKILL.md").exists());

    // Remove an item from the source (simulating upstream rename/delete).
    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "--yes"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("removed"),
        "expected 'removed' in output, got:\n{out}"
    );

    // Output files should be gone.
    assert!(!target.join(".claude/skills/create-api-endpoint/SKILL.md").exists());
    assert!(!target.join(".github/skills/create-api-endpoint/SKILL.md").exists());
    assert!(!target.join(".agents/skills/create-api-endpoint/SKILL.md").exists());

    // Lockfile should no longer contain the item.
    assert!(lockfile_hash_for(&target, "create-api-endpoint").is_none());
}

#[test]
fn update_dry_run_reports_would_remove_without_deleting() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "--dry-run"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("would remove"),
        "expected 'would remove' in dry-run output, got:\n{out}"
    );

    // Files must still exist.
    assert!(target.join(".claude/skills/create-api-endpoint/SKILL.md").exists());
    // Lockfile must still contain the item.
    assert!(lockfile_hash_for(&target, "create-api-endpoint").is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_update update_removes_orphaned_item update_dry_run_reports_would_remove`
Expected: FAIL (no `removed` / `would remove` in output)

- [ ] **Step 3: Implement orphan detection in `update()`**

In `src/pipeline.rs`, modify the `update()` function. After fetching the source and computing hashes, detect entries that are no longer in the source:

Replace the inner loop body for both `Apply` and `DryRun` modes. The key change: after computing `new_hashes` (in DryRun) or `new_hashes` from the install report (in Apply), check if the entry's `(kind, name)` is absent from the discovered items.

In the `DryRun` branch (starting at line 1166), replace the `for entry in &source_entries` loop:

```rust
UpdateMode::DryRun => {
    let (root, _guard) = fetch_ssot(&source)?;
    let new_hashes = hash_source_items(&root);
    for entry in &source_entries {
        let kind = parse_kind(&entry.kind)?;
        let key = (kind, entry.name.clone());
        if !new_hashes.contains_key(&key) {
            // Item no longer in source — would be removed.
            report.items.push(UpdatedItem {
                kind,
                name: entry.name.clone(),
                source: source_label.clone(),
                status: UpdateStatus::WouldRemove,
            });
            continue;
        }
        let new_hash = new_hashes.get(&key).cloned().flatten();
        let status = if new_hash == entry.hash {
            UpdateStatus::UpToDate
        } else {
            UpdateStatus::WouldChange {
                old_hash: entry.hash.clone(),
                new_hash,
            }
        };
        report.items.push(UpdatedItem {
            kind,
            name: entry.name.clone(),
            source: source_label.clone(),
            status,
        });
    }
}
```

In the `Apply` branch (starting at line 1137), replace the `for entry in &source_entries` loop:

```rust
UpdateMode::Apply => {
    let install_report = install_with_lockfile(&source, target, &[], plugin_scope)?;
    let mut new_hashes: std::collections::BTreeMap<(ItemKind, String), Option<String>> =
        std::collections::BTreeMap::new();
    for it in &install_report.items {
        new_hashes.insert((it.kind, it.name.clone()), it.source_hash.clone());
    }
    for entry in &source_entries {
        let kind = parse_kind(&entry.kind)?;
        let key = (kind, entry.name.clone());
        if !new_hashes.contains_key(&key) {
            // Item no longer in source — remove it.
            remove_item_outputs(target, kind, &entry.name);
            let mut lock = crate::lockfile::Lockfile::load(target)?;
            lock.remove(&entry.kind, &entry.name);
            lock.save(target)?;
            report.items.push(UpdatedItem {
                kind,
                name: entry.name.clone(),
                source: source_label.clone(),
                status: UpdateStatus::Removed,
            });
            continue;
        }
        let new_hash = new_hashes
            .get(&key)
            .cloned()
            .flatten();
        let status = if new_hash == entry.hash {
            UpdateStatus::UpToDate
        } else {
            UpdateStatus::Updated {
                old_hash: entry.hash.clone(),
                new_hash,
            }
        };
        report.items.push(UpdatedItem {
            kind,
            name: entry.name.clone(),
            source: source_label.clone(),
            status,
        });
    }
}
```

- [ ] **Step 4: Add `remove_item_outputs` helper**

Add this helper function in `src/pipeline.rs` (near the existing `output_path` function):

```rust
/// Delete all per-client output files for an item. Best-effort: ignores
/// errors (file may already be gone). Also removes empty parent dirs.
fn remove_item_outputs(target: &Path, kind: ItemKind, name: &str) {
    for client in ALL_CLIENTS {
        let rel = output_path(kind, client, name);
        let full = target.join(&rel);
        if full.exists() {
            let _ = fs::remove_file(&full);
            if let Some(parent) = full.parent() {
                let _ = fs::remove_dir(parent);
            }
        }
    }
}
```

- [ ] **Step 5: Update `print_update_report` in `main.rs` to handle new variants**

In `src/main.rs`, in the `print_update_report` function, add arms for the new variants:

```rust
UpdateStatus::Removed => style::error("removed — no longer in source"),
UpdateStatus::WouldRemove => style::warn("would remove — no longer in source"),
```

Also update the changes count filter to include removals:

```rust
let changes = report
    .items
    .iter()
    .filter(|i| !matches!(i.status, UpdateStatus::UpToDate))
    .count();
```

(This already works since `Removed`/`WouldRemove` don't match `UpToDate`.)

- [ ] **Step 6: Run tests**

Run: `cargo test --test cli_update`
Expected: all tests pass, including the two new ones.

- [ ] **Step 7: Commit**

```bash
git add src/pipeline.rs src/main.rs tests/cli_update.rs
git commit -m "feat: auto-remove orphaned items during update"
```

---

### Task 4: Clean output before regenerating (prevent stale files)

**Files:**

- Modify: `src/pipeline.rs` — the per-kind install loops (around lines 1534–1549, 1577–1592, 1620–1635)
- Test: `tests/cli_update.rs`

- [ ] **Step 1: Write failing test — stale output file removed on update**

Append to `tests/cli_update.rs`:

```rust
#[test]
fn update_removes_stale_output_file_from_previous_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    // Simulate a stale file that was previously generated but no longer
    // should be (e.g., output from a previous version of the generator).
    let stale_file = target.join(".claude/skills/create-api-endpoint/OLD_FILE.md");
    fs::write(&stale_file, "stale content").unwrap();
    assert!(stale_file.exists());

    // Mutate the skill so update actually regenerates it.
    mutate_skill(&source);

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update", "--yes"])
        .assert()
        .success();

    // The stale file should be gone after regeneration.
    assert!(
        !stale_file.exists(),
        "stale file should have been removed during clean-and-regenerate"
    );
    // But the real output should still exist.
    assert!(target.join(".claude/skills/create-api-endpoint/SKILL.md").exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_update update_removes_stale_output`
Expected: FAIL (stale file still exists)

- [ ] **Step 3: Add clean step before writing output**

In `src/pipeline.rs`, in the skill install loop (around line 1534), before writing output for each item, clean the parent directory. The cleanest approach is to call `remove_item_outputs` before the per-client render loop for each item:

Find the skill loop (starts around line 1523 with `let entry_path = dir.join("SKILL.md");`). Before the `for client in ALL_CLIENTS` loop at line 1534, insert:

```rust
// Clean previous output to prevent stale files from lingering
// after upstream renames/removes files within the item.
remove_item_outputs(target, ItemKind::Skill, &name);
```

Do the same for the rule loop (before the `for client in ALL_CLIENTS` around line 1577):

```rust
remove_item_outputs(target, ItemKind::Rule, &name);
```

And the agent loop (before the `for client in ALL_CLIENTS` around line 1620):

```rust
remove_item_outputs(target, ItemKind::Agent, &name);
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_update`
Expected: all tests pass including the new one.

- [ ] **Step 5: Run full test suite to check for regressions**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/pipeline.rs tests/cli_update.rs
git commit -m "feat: clean output before regenerating to prevent stale files"
```

---

### Task 5: Interactive confirmation prompt before applying

**Files:**

- Modify: `src/main.rs` — `run_update()` function
- Modify: `src/pipeline.rs` — split update into plan + apply phases
- Test: `tests/cli_update.rs`

- [ ] **Step 1: Write failing test — update without --yes prompts (non-TTY auto-proceeds)**

Append to `tests/cli_update.rs`:

```rust
#[test]
fn update_without_yes_proceeds_in_non_tty() {
    // In non-TTY (like test harness), update should proceed without
    // blocking — same as remove's behavior.
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source);

    mutate_skill(&source);

    // No --yes flag, but stdin is not a TTY so it should proceed.
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&target)
        .args(["update"])
        .assert()
        .success();

    let out_path = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    let content = fs::read_to_string(&out_path).unwrap();
    assert!(
        content.contains("mutated by test"),
        "update should have applied without prompt in non-TTY"
    );
}
```

- [ ] **Step 2: Refactor `update()` to support a two-phase plan-then-apply pattern**

The confirmation prompt needs to show the plan before applying. The cleanest approach: `update()` already computes the report. We change the flow in `main.rs` so that:

1. First call `update()` with `DryRun` to get the plan.
2. If nothing to do → print and exit.
3. If changes exist and TTY and no `--yes` → show summary and prompt.
4. If confirmed → call `update()` with `Apply`.

Modify `run_update` in `src/main.rs`:

```rust
fn run_update(names: &[String], dry_run: bool, yes: bool, global: bool, project: bool) -> i32 {
    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let plugin_scope = scope_to_plugin_scope(global, project);

    if dry_run {
        // Pure dry-run: report and exit.
        match update(&target, names, UpdateMode::DryRun, plugin_scope) {
            Ok(report) => {
                print_update_report(&report, true);
                EXIT_SUCCESS
            }
            Err(err) => {
                print_error_chain(&err);
                EXIT_ERROR
            }
        }
    } else {
        // Plan phase: dry-run to compute what would change.
        let plan = match update(&target, names, UpdateMode::DryRun, plugin_scope) {
            Ok(r) => r,
            Err(err) => {
                print_error_chain(&err);
                return EXIT_ERROR;
            }
        };

        let has_changes = plan.items.iter().any(|i| !matches!(i.status, UpdateStatus::UpToDate));
        if !has_changes {
            print_update_report(&plan, false);
            return EXIT_SUCCESS;
        }

        // Show plan and confirm.
        print_update_plan(&plan);
        if !yes && !confirm_update() {
            eprintln!("aborted.");
            return EXIT_SUCCESS;
        }

        // Apply phase.
        match update(&target, names, UpdateMode::Apply, plugin_scope) {
            Ok(report) => {
                print_update_report(&report, false);
                EXIT_SUCCESS
            }
            Err(err) => {
                print_error_chain(&err);
                EXIT_ERROR
            }
        }
    }
}
```

- [ ] **Step 3: Add `print_update_plan` and `confirm_update` in `main.rs`**

```rust
fn print_update_plan(report: &UpdateReport) {
    if style::is_quiet() {
        return;
    }
    let updated = report
        .items
        .iter()
        .filter(|i| matches!(i.status, UpdateStatus::WouldChange { .. }))
        .count();
    let removed = report
        .items
        .iter()
        .filter(|i| matches!(i.status, UpdateStatus::WouldRemove))
        .count();
    let unchanged = report
        .items
        .iter()
        .filter(|i| matches!(i.status, UpdateStatus::UpToDate))
        .count();

    println!();
    if updated > 0 {
        println!(
            "  {}  {} item(s)",
            style::success("update:"),
            updated
        );
    }
    if removed > 0 {
        println!(
            "  {}  {} item(s) (no longer in source)",
            style::error("remove:"),
            removed
        );
    }
    if unchanged > 0 {
        println!(
            "  {} {} item(s)",
            style::dim("unchanged:"),
            unchanged
        );
    }
    println!();
}

/// Prompt user to confirm update. Returns `true` to proceed.
/// Non-TTY stdin → returns `true` (same convention as `confirm_bulk_remove`).
fn confirm_update() -> bool {
    use std::io::{BufRead, IsTerminal, Write};

    if !std::io::stdin().is_terminal() {
        return true;
    }

    eprint!("Apply? [y/N] ");
    let _ = std::io::stderr().flush();

    let stdin = std::io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_update`
Expected: all tests pass. The existing tests pass `--yes` or run in non-TTY (test harness), so they proceed without blocking.

- [ ] **Step 5: Fix existing tests that don't pass `--yes`**

The existing tests (`update_no_change_reports_up_to_date`, `update_after_ssot_mutation_rewrites_outputs_and_lockfile_hash`) run in non-TTY so `confirm_update()` returns `true`. They should still pass. If any fail, add `--yes` to their args.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/pipeline.rs tests/cli_update.rs
git commit -m "feat: interactive confirmation prompt before update applies"
```

---

### Task 6: Lint, format, and verify

**Files:**

- All modified files

- [ ] **Step 1: Format**

Run: `just fmt`

- [ ] **Step 2: Lint**

Run: `just lint`
Expected: no warnings, no errors.

- [ ] **Step 3: Full verification**

Run: `just verify`
Expected: all checks pass.

- [ ] **Step 4: Final commit if formatting changed anything**

```bash
git add -A
git commit -m "chore: fmt"
```
