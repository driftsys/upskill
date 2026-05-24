# Item Conflict Resolution & Source Locking — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent accidental item overrides from different sources by making items locked to their source by default, with `--force` and `--as` flags to resolve conflicts.

**Architecture:** Add conflict detection in `install_with_lockfile` (before writing), extend `LockedItem` with optional `source_name`, add `--force` and `--as` CLI flags, extend bundle install with `--exclude` and bundle-aware `--as`.

**Tech Stack:** Rust, clap 4 (derive), anyhow, serde

---

### Task 1: Add `source_name` field to `LockedItem`

**Files:**
- Modify: `src/lockfile.rs:37-54`
- Test: `src/lockfile.rs` (inline tests)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn locked_item_serializes_source_name_when_present() {
    let item = LockedItem {
        kind: "skill".to_string(),
        name: "brainstorming-v2".to_string(),
        source: "github:other-org/repo".to_string(),
        git_ref: None,
        hash: None,
        source_name: Some("brainstorming".to_string()),
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("\"source_name\":\"brainstorming\""), "{json}");
}

#[test]
fn locked_item_omits_source_name_when_none() {
    let item = LockedItem {
        kind: "skill".to_string(),
        name: "brainstorming".to_string(),
        source: "github:driftsys/superpowers".to_string(),
        git_ref: None,
        hash: None,
        source_name: None,
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(!json.contains("source_name"), "{json}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test locked_item_serializes_source_name`
Expected: FAIL — `source_name` field doesn't exist

- [ ] **Step 3: Add `source_name` field to `LockedItem`**

In `src/lockfile.rs`, add to the `LockedItem` struct after the `hash` field:

```rust
    /// Original item name in the source registry. Only present when the
    /// consumer installed with `--as <alias>` so `name` differs from the
    /// SSOT name. Used by `update` to locate the correct source file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
```

- [ ] **Step 4: Fix all struct literals that construct `LockedItem`**

Add `source_name: None` to:
- `items_from_report` (line ~246)
- All test helper `item()` functions in `src/lockfile.rs` and `tests/`

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/lockfile.rs
git commit -m "feat(lockfile): add source_name field for alias tracking"
```

---

### Task 2: Add `--force` and `--as` flags to `Add` command

**Files:**
- Modify: `src/cli.rs:50-64`

- [ ] **Step 1: Add flags to the `Add` variant in `cli.rs`**

```rust
    Add {
        /// Source: `owner/repo[@ref][:subfolder]`, full https URL, or local path.
        source: String,
        /// Optional subset filter — only items whose name matches one of
        /// these is installed. Empty means install everything in the
        /// source (the default).
        items: Vec<String>,
        /// Install into `$HOME` instead of the current directory.
        #[arg(short = 'g', long = "global", conflicts_with = "project")]
        global: bool,
        /// Force project scope (current directory). Overrides the auto-detect
        /// fallback to global when `cwd` is not inside a git repo.
        #[arg(short = 'p', long = "project")]
        project: bool,
        /// Replace existing items from a different source without error.
        #[arg(long = "force")]
        force: bool,
        /// Install under an alternate name to avoid conflicts.
        /// For direct installs: `--as alt-name`.
        /// For bundle installs: `--as original=alias` (repeatable).
        #[arg(long = "as", value_name = "ALIAS")]
        alias: Vec<String>,
        /// Skip specific items during bundle install (repeatable).
        #[arg(long = "exclude", value_name = "NAME")]
        exclude: Vec<String>,
    },
```

- [ ] **Step 2: Update the `Commands::Add` match in `main.rs`**

Update the destructuring at line ~46:

```rust
        Commands::Add {
            source,
            items,
            global,
            project,
            force,
            alias,
            exclude,
        } => run_add(&source, &items, global, project, force, &alias, &exclude),
```

And update `run_add` signature:

```rust
fn run_add(source: &str, items: &[String], global: bool, project: bool, force: bool, aliases: &[String], excludes: &[String]) -> i32 {
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles (unused params warning is fine for now)

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat(cli): add --force, --as, --exclude flags to add command"
```

---

### Task 3: Implement conflict detection

**Files:**
- Modify: `src/pipeline.rs`
- Create: `src/conflict.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `src/conflict.rs`:

```rust
//! Item conflict detection: checks whether incoming items collide with
//! existing lockfile entries from a different source.

use anyhow::{Result, bail};

use crate::lockfile::{LockedItem, Lockfile};
use crate::pipeline::ItemKind;

/// A detected conflict between an incoming item and an existing lockfile entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemConflict {
    pub kind: ItemKind,
    pub name: String,
    pub existing_source: String,
    pub incoming_source: String,
}

/// Check incoming items against the lockfile. Returns conflicts (items from
/// a different source that would be overwritten).
pub fn detect_conflicts(
    incoming: &[(ItemKind, String)],
    lockfile: &Lockfile,
    incoming_source: &str,
) -> Vec<ItemConflict> {
    let mut conflicts = Vec::new();
    for (kind, name) in incoming {
        let kind_str = match kind {
            ItemKind::Rule => "rule",
            ItemKind::Skill => "skill",
            ItemKind::Agent => "agent",
        };
        if let Some(existing) = lockfile.items.iter().find(|i| i.kind == kind_str && i.name == *name) {
            if existing.source != incoming_source {
                conflicts.push(ItemConflict {
                    kind: *kind,
                    name: name.clone(),
                    existing_source: existing.source.clone(),
                    incoming_source: incoming_source.to_string(),
                });
            }
        }
    }
    conflicts
}

/// Format conflicts into a user-facing error message.
pub fn format_conflict_error(conflicts: &[ItemConflict]) -> String {
    if conflicts.len() == 1 {
        let c = &conflicts[0];
        format!(
            "{:?} `{}` is already installed from `{}`.\n\
             Use --force to replace, or --as <alt-name> to keep both.",
            c.kind, c.name, c.existing_source
        )
    } else {
        let mut msg = format!("conflicts with {} installed items:\n", conflicts.len());
        for c in conflicts {
            msg.push_str(&format!("  - {:?} `{}` (from `{}`)\n", c.kind, c.name, c.existing_source));
        }
        msg.push_str("\nUse --force to replace all, or resolve individually with --exclude or --as.");
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{LockedItem, Lockfile};

    fn make_lockfile(items: Vec<LockedItem>) -> Lockfile {
        Lockfile {
            schema: 1,
            items,
            bundles: vec![],
            plugins: vec![],
        }
    }

    fn locked(kind: &str, name: &str, source: &str) -> LockedItem {
        LockedItem {
            kind: kind.to_string(),
            name: name.to_string(),
            source: source.to_string(),
            git_ref: None,
            hash: None,
            source_name: None,
        }
    }

    #[test]
    fn no_conflict_when_same_source() {
        let lf = make_lockfile(vec![locked("skill", "brainstorming", "github:driftsys/superpowers")]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_string())];
        let conflicts = detect_conflicts(&incoming, &lf, "github:driftsys/superpowers");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn conflict_when_different_source() {
        let lf = make_lockfile(vec![locked("skill", "brainstorming", "github:driftsys/superpowers")]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_string())];
        let conflicts = detect_conflicts(&incoming, &lf, "github:other-org/repo");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].name, "brainstorming");
        assert_eq!(conflicts[0].existing_source, "github:driftsys/superpowers");
    }

    #[test]
    fn no_conflict_when_item_not_in_lockfile() {
        let lf = make_lockfile(vec![]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_string())];
        let conflicts = detect_conflicts(&incoming, &lf, "github:other-org/repo");
        assert!(conflicts.is_empty());
    }

    #[test]
    fn same_name_different_kind_is_not_conflict() {
        let lf = make_lockfile(vec![locked("rule", "brainstorming", "github:driftsys/superpowers")]);
        let incoming = vec![(ItemKind::Skill, "brainstorming".to_string())];
        let conflicts = detect_conflicts(&incoming, &lf, "github:other-org/repo");
        assert!(conflicts.is_empty());
    }
}
```

- [ ] **Step 2: Register module in `lib.rs`**

Add `pub mod conflict;` to `src/lib.rs`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test conflict`
Expected: All 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add src/conflict.rs src/lib.rs
git commit -m "feat(conflict): add item conflict detection module"
```

---

### Task 4: Wire conflict detection into `install_with_lockfile`

**Files:**
- Modify: `src/pipeline.rs:284-380`
- Modify: `src/main.rs:127-158`

- [ ] **Step 1: Write integration test**

Create `tests/cli_conflict.rs`:

```rust
//! Integration tests for item conflict detection and resolution flags.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper: set up a temp dir with a lockfile containing an item from source A.
fn setup_with_existing_item(tmp: &TempDir) {
    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "test-skill",
            "source": "github:org-a/repo-a",
            "hash": "sha256:aaa"
        }],
        "bundles": []
    });
    fs::write(tmp.path().join(".upskill-lock.json"), serde_json::to_string_pretty(&lockfile).unwrap()).unwrap();
    // Create a minimal SSOT source with a skill named test-skill
    let skill_dir = tmp.path().join("source/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test-skill\ndescription: A test skill\n---\nContent here.\n").unwrap();
}

#[test]
fn add_from_different_source_errors_without_force() {
    let tmp = TempDir::new().unwrap();
    setup_with_existing_item(&tmp);

    // Init as git repo so upskill doesn't fall back to global
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already installed from"));
}

#[test]
fn add_from_different_source_succeeds_with_force() {
    let tmp = TempDir::new().unwrap();
    setup_with_existing_item(&tmp);

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source", "--force"])
        .assert()
        .success();
}

#[test]
fn add_from_same_source_succeeds_without_force() {
    let tmp = TempDir::new().unwrap();
    // Lockfile with source matching what we'll install from
    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "test-skill",
            "source": "local:source",
            "hash": "sha256:aaa"
        }],
        "bundles": []
    });
    fs::write(tmp.path().join(".upskill-lock.json"), serde_json::to_string_pretty(&lockfile).unwrap()).unwrap();

    let skill_dir = tmp.path().join("source/test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: test-skill\ndescription: A test skill\n---\nContent here.\n").unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source"])
        .assert()
        .success();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test cli_conflict`
Expected: `add_from_different_source_errors_without_force` passes (currently succeeds, should fail) — actually this will PASS currently because there's no conflict detection. We need the test to assert failure. Let me correct: currently `upskill add ./source` succeeds even with a conflicting lockfile entry, so `add_from_different_source_errors_without_force` will FAIL (it expects failure but gets success).

- [ ] **Step 3: Add `AddContext` struct and modify pipeline**

Add to `src/pipeline.rs`:

```rust
/// Options that control conflict resolution during install.
#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    /// When true, replace items from different sources without error.
    pub force: bool,
    /// Alias mappings: `source_name -> alias`. Items matching a key are
    /// installed under the alias instead.
    pub aliases: Vec<(String, String)>,
    /// Item names to skip during install.
    pub excludes: Vec<String>,
}
```

Change `install_with_lockfile` signature to accept `AddOptions`:

```rust
pub fn install_with_lockfile(
    source: &InstallSource,
    target: &Path,
    items: &[String],
    plugin_scope: crate::plugin::PluginScope,
    options: &AddOptions,
) -> Result<InstallReport> {
```

After resolving items but before writing, add conflict check:

```rust
    // -- Conflict detection --
    let lock = crate::lockfile::Lockfile::load(target)?;
    let label = source.to_string();
    let incoming: Vec<(ItemKind, String)> = report
        .items
        .iter()
        .map(|i| (i.kind, i.name.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Filter out excluded items
    let incoming: Vec<(ItemKind, String)> = incoming
        .into_iter()
        .filter(|(_, name)| !options.excludes.contains(name))
        .collect();

    let conflicts = crate::conflict::detect_conflicts(&incoming, &lock, &label);
    if !conflicts.is_empty() && !options.force {
        anyhow::bail!("{}", crate::conflict::format_conflict_error(&conflicts));
    }
```

- [ ] **Step 4: Update `main.rs` to pass `AddOptions`**

```rust
fn run_add(source: &str, items: &[String], global: bool, project: bool, force: bool, aliases: &[String], excludes: &[String]) -> i32 {
    let parsed = match parse_install_source(source) {
        Ok(s) => s,
        Err(err) => {
            print_error(&err);
            return EXIT_USAGE;
        }
    };

    let target = match install_target(global, project) {
        Ok(t) => t,
        Err(err) => {
            print_error(&err);
            return EXIT_ERROR;
        }
    };

    let plugin_scope = scope_to_plugin_scope(global, project);

    let options = upskill::pipeline::AddOptions {
        force,
        aliases: parse_alias_args(aliases),
        excludes: excludes.to_vec(),
    };

    print_install_progress(&parsed);
    match install_with_lockfile(&parsed, &target, items, plugin_scope, &options) {
        Ok(report) => {
            print_install_report(&report, source);
            print_plugin_results(&report);
            EXIT_SUCCESS
        }
        Err(err) => {
            print_error_chain(&err);
            EXIT_ERROR
        }
    }
}

/// Parse `--as` arguments. Direct: `"alt-name"`. Bundle: `"original=alias"`.
fn parse_alias_args(args: &[String]) -> Vec<(String, String)> {
    args.iter()
        .map(|a| {
            if let Some((from, to)) = a.split_once('=') {
                (from.to_string(), to.to_string())
            } else {
                // Direct add: alias applies to the single item being added
                // The source name will be filled in by the pipeline
                (String::new(), a.to_string())
            }
        })
        .collect()
}
```

- [ ] **Step 5: Run integration tests**

Run: `cargo test --test cli_conflict`
Expected: All 3 tests pass

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All pass (existing tests need `AddOptions::default()` added to calls)

- [ ] **Step 7: Commit**

```bash
git add src/pipeline.rs src/main.rs src/cli.rs tests/cli_conflict.rs
git commit -m "feat(pipeline): wire conflict detection into install_with_lockfile"
```

---

### Task 5: Implement `--as` alias installation

**Files:**
- Modify: `src/pipeline.rs`
- Modify: `src/lockfile.rs`

- [ ] **Step 1: Write integration test**

Add to `tests/cli_conflict.rs`:

```rust
#[test]
fn add_with_alias_installs_under_alternate_name() {
    let tmp = TempDir::new().unwrap();
    setup_with_existing_item(&tmp);

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source", "--as", "test-skill-v2"])
        .assert()
        .success();

    // Verify lockfile has the alias
    let lock_content = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(lock_content.contains("test-skill-v2"), "expected alias in lockfile: {lock_content}");
    assert!(lock_content.contains("\"source_name\":\"test-skill\"") || lock_content.contains("\"source_name\": \"test-skill\""),
        "expected source_name in lockfile: {lock_content}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_conflict add_with_alias`
Expected: FAIL

- [ ] **Step 3: Implement alias renaming in pipeline**

In `install_with_lockfile`, after conflict detection and before lockfile write, rename items that match aliases:

```rust
    // -- Apply aliases --
    // For direct --as (empty key): rename all items from this source
    // For bundle --as (key=value): rename specific items
    for item in &mut report.items {
        let alias = options.aliases.iter().find(|(from, _)| {
            from.is_empty() || *from == item.name
        });
        if let Some((original_name, alias_name)) = alias {
            // Rename the item in the report
            let source_name = item.name.clone();
            item.name = alias_name.clone();
            // Also rename the output path
            // (regeneration with new name handled below)
        }
    }
```

And in the lockfile write section, set `source_name` when aliased:

```rust
    let new_items = crate::lockfile::items_from_report(&report, &label, git_ref, |k, n| {
        hashes.get(&(k, n.to_string())).cloned().flatten()
    });

    // Set source_name for aliased items
    let new_items: Vec<_> = new_items.into_iter().map(|mut item| {
        if let Some((from, _)) = options.aliases.iter().find(|(_, to)| *to == item.name) {
            let original = if from.is_empty() {
                // Direct alias: the original name was whatever the source had
                // We need to track it from the report
                item.name.clone() // will be overridden below
            } else {
                from.clone()
            };
            item.source_name = Some(original);
        }
        item
    }).collect();
```

Note: The full implementation will need to regenerate client outputs with the alias name. This requires passing the alias through to `install_from_source` or renaming output files post-generation.

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_conflict`
Expected: All pass

- [ ] **Step 5: Commit**

```bash
git add src/pipeline.rs src/lockfile.rs tests/cli_conflict.rs
git commit -m "feat(pipeline): implement --as alias installation with source_name tracking"
```

---

### Task 6: Implement `--exclude` for bundle installs

**Files:**
- Modify: `src/pipeline.rs`

- [ ] **Step 1: Write integration test**

Add to `tests/cli_conflict.rs`:

```rust
#[test]
fn add_with_exclude_skips_named_item() {
    let tmp = TempDir::new().unwrap();

    // Create source with two skills
    let skill_a = tmp.path().join("source/skill-a");
    let skill_b = tmp.path().join("source/skill-b");
    fs::create_dir_all(&skill_a).unwrap();
    fs::create_dir_all(&skill_b).unwrap();
    fs::write(skill_a.join("SKILL.md"), "---\nname: skill-a\ndescription: Skill A\n---\nA\n").unwrap();
    fs::write(skill_b.join("SKILL.md"), "---\nname: skill-b\ndescription: Skill B\n---\nB\n").unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["add", "./source", "--exclude", "skill-b"])
        .assert()
        .success();

    let lock_content = fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap();
    assert!(lock_content.contains("skill-a"), "expected skill-a: {lock_content}");
    assert!(!lock_content.contains("skill-b"), "expected no skill-b: {lock_content}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_conflict add_with_exclude`
Expected: FAIL (skill-b is installed)

- [ ] **Step 3: Implement exclude filtering**

In `install_with_lockfile`, after the install but before lockfile write, filter out excluded items:

```rust
    // -- Apply excludes --
    if !options.excludes.is_empty() {
        report.items.retain(|item| !options.excludes.contains(&item.name));
        // Also remove generated output files for excluded items
        for exclude in &options.excludes {
            remove_generated_outputs(target, exclude);
        }
    }
```

The `remove_generated_outputs` helper deletes per-client files that were just written for excluded items. Alternatively, filter BEFORE generation by passing excludes into `install_from_source`.

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_conflict`
Expected: All pass

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/pipeline.rs tests/cli_conflict.rs
git commit -m "feat(pipeline): implement --exclude flag to skip items during install"
```

---

### Task 7: Update `upskill update` to use `source_name`

**Files:**
- Modify: `src/pipeline.rs` (the `update` function)

- [ ] **Step 1: Write the failing test**

Add to `tests/cli_conflict.rs`:

```rust
#[test]
fn update_uses_source_name_for_aliased_items() {
    let tmp = TempDir::new().unwrap();

    // Create source with a skill
    let skill_dir = tmp.path().join("source/original-name");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "---\nname: original-name\ndescription: Original\n---\nUpdated content.\n").unwrap();

    // Lockfile with an aliased item
    let lockfile = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "skill",
            "name": "my-alias",
            "source": "local:source",
            "source_name": "original-name",
            "hash": "sha256:old"
        }],
        "bundles": []
    });
    fs::write(tmp.path().join(".upskill-lock.json"), serde_json::to_string_pretty(&lockfile).unwrap()).unwrap();

    // Create generated output so update has something to compare
    let claude_dir = tmp.path().join(".claude/skills/my-alias");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(claude_dir.join("SKILL.md"), "old content").unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .assert()
        .success();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["update"])
        .assert()
        .success();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_conflict update_uses_source_name`
Expected: FAIL — update looks for `my-alias` in source, doesn't find it

- [ ] **Step 3: Modify `update` to use `source_name`**

In the `update` function in `src/pipeline.rs`, when fetching SSOT for a locked item, use `source_name` if present:

```rust
    let ssot_name = item.source_name.as_deref().unwrap_or(&item.name);
    // Use ssot_name to locate the item directory in the fetched source
```

And when regenerating, use `item.name` (the alias) as the output name.

- [ ] **Step 4: Run tests**

Run: `cargo test --test cli_conflict`
Expected: All pass

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add src/pipeline.rs tests/cli_conflict.rs
git commit -m "feat(update): use source_name to fetch aliased items correctly"
```

---

### Task 8: Final verification and lint

**Files:**
- All modified files

- [ ] **Step 1: Run full test suite**

Run: `just test`
Expected: All pass

- [ ] **Step 2: Run lints**

Run: `just lint`
Expected: No warnings

- [ ] **Step 3: Format**

Run: `just fmt`

- [ ] **Step 4: Run full verify**

Run: `just verify`
Expected: Clean

- [ ] **Step 5: Final commit (if fmt changed anything)**

```bash
git add -A
git commit -m "chore: format"
```
