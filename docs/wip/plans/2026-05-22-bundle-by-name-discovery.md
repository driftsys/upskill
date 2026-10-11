# Bundle-by-name Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow `upskill add owner/repo bundle-name` to discover and install bundles by name, not just by full file path.

**Architecture:** Add a `find_bundle_by_name` helper that recursively searches a source directory for `<name>.bundle.yaml`. Wire it into `install_with_lockfile` so that when positional item names are provided and no items match, it falls back to bundle-by-name discovery. Add a lint rule that detects name collisions between bundles and items.

**Tech Stack:** Rust, existing `pipeline.rs` + `lint.rs` modules, existing `parse::bundle::discover` for recursive bundle walking.

---

## Tasks

### Task 1: Add `find_bundle_by_name` helper to pipeline.rs

**Files:**

- Modify: `src/pipeline.rs` (add function near `is_bundle_file`)

- [ ] **Step 1: Write the failing test**

In `src/pipeline.rs` inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn find_bundle_by_name_finds_nested_bundle_file() {
    let tmp = tempfile::tempdir().unwrap();
    let bundles_dir = tmp.path().join("bundles");
    std::fs::create_dir_all(&bundles_dir).unwrap();
    std::fs::write(
        bundles_dir.join("baseline.bundle.yaml"),
        "schema: 1\nname: baseline\ndescription: test\nitems:\n  rules: []\n",
    )
    .unwrap();

    let result = find_bundle_by_name(tmp.path(), "baseline");
    assert_eq!(
        result,
        Some(bundles_dir.join("baseline.bundle.yaml"))
    );
}

#[test]
fn find_bundle_by_name_returns_none_when_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("skills/foo")).unwrap();
    std::fs::write(tmp.path().join("skills/foo/SKILL.md"), "---\nschema: 1\nname: foo\n---\n# body\n").unwrap();

    let result = find_bundle_by_name(tmp.path(), "foo");
    assert!(result.is_none());
}

#[test]
fn find_bundle_by_name_skips_hidden_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let hidden = tmp.path().join(".hidden");
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::write(hidden.join("secret.bundle.yaml"), "schema: 1\nname: secret\ndescription: x\nitems:\n  rules: []\n").unwrap();

    let result = find_bundle_by_name(tmp.path(), "secret");
    assert!(result.is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib find_bundle_by_name`
Expected: FAIL — `find_bundle_by_name` not found.

- [ ] **Step 3: Implement `find_bundle_by_name`**

Add to `src/pipeline.rs` near `is_bundle_file` (around line 155):

```rust
/// Search `root` recursively for a file named `<name>.bundle.yaml`.
/// Skips hidden directories. Returns the first match or `None`.
fn find_bundle_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let target_filename = format!("{}{}", name, crate::parse::bundle::BUNDLE_SUFFIX);
    find_bundle_recursive(root, &target_filename)
}

fn find_bundle_recursive(dir: &Path, target: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let entry_name = entry.file_name();
        let name_str = entry_name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_file() && name_str == target {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_bundle_recursive(&path, target) {
                return Some(found);
            }
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib find_bundle_by_name`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): add find_bundle_by_name recursive search helper"
```

---

### Task 2: Add `has_matching_items` helper to pipeline.rs

**Files:**

- Modify: `src/pipeline.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn has_matching_items_true_when_skill_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let skill_dir = tmp.path().join("code-review");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "---\nschema: 1\nname: code-review\n---\n# body\n").unwrap();

    assert!(has_matching_items(tmp.path(), "code-review"));
}

#[test]
fn has_matching_items_false_when_no_item_exists() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("other")).unwrap();

    assert!(!has_matching_items(tmp.path(), "nonexistent"));
}

#[test]
fn has_matching_items_true_for_rules_and_agents() {
    let tmp = tempfile::tempdir().unwrap();
    let rule_dir = tmp.path().join("my-rule");
    std::fs::create_dir_all(&rule_dir).unwrap();
    std::fs::write(rule_dir.join("RULE.md"), "---\nschema: 1\nname: my-rule\n---\n# body\n").unwrap();

    assert!(has_matching_items(tmp.path(), "my-rule"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib has_matching_items`
Expected: FAIL — function not defined.

- [ ] **Step 3: Implement `has_matching_items`**

Add to `src/pipeline.rs`:

```rust
/// Check whether a source directory contains any item (skill, rule, or
/// agent) with the given name. An item exists when a subdirectory named
/// `name` contains at least one of `SKILL.md`, `RULE.md`, or `AGENT.md`.
fn has_matching_items(source: &Path, name: &str) -> bool {
    let item_dir = source.join(name);
    if !item_dir.is_dir() {
        return false;
    }
    item_dir.join("SKILL.md").is_file()
        || item_dir.join("RULE.md").is_file()
        || item_dir.join("AGENT.md").is_file()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib has_matching_items`
Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): add has_matching_items existence check"
```

---

### Task 3: Wire bundle-by-name into `install_with_lockfile`

**Files:**

- Modify: `src/pipeline.rs` (the `install_with_lockfile` function)

- [ ] **Step 1: Write the failing integration test**

In `tests/pipeline_lockfile.rs`, add:

```rust
#[test]
fn install_by_name_discovers_bundle_when_no_item_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    // "with-plugins" is a bundle name, not an item name.
    let report = install_with_lockfile(
        &InstallSource::LocalPath(registry.clone()),
        &target,
        &["with-plugins".into()],
        PluginScope::Project,
    )
    .expect("install");

    // Bundle dispatch should have fired — items from the bundle are installed.
    assert!(
        !report.items.is_empty(),
        "expected bundle items to be installed"
    );
    assert!(
        !report.bundles.is_empty(),
        "expected bundle to appear in report"
    );
    assert_eq!(report.bundles[0].name, "with-plugins");
}

#[test]
fn install_by_name_errors_on_ambiguity() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    // Create an item directory that collides with bundle name "with-plugins"
    let collision_dir = registry.join("with-plugins");
    fs::create_dir_all(&collision_dir).unwrap();
    fs::write(
        collision_dir.join("SKILL.md"),
        "---\nschema: 1\nname: with-plugins\ndescription: collision\n---\n# body\n",
    )
    .unwrap();

    let err = install_with_lockfile(
        &InstallSource::LocalPath(registry.clone()),
        &target,
        &["with-plugins".into()],
        PluginScope::Project,
    )
    .expect_err("should error on ambiguity");

    let msg = format!("{:#}", err);
    assert!(msg.contains("matches both"), "error mentions ambiguity: {msg}");
    assert!(msg.contains(".bundle.yaml"), "error shows bundle path: {msg}");
}

#[test]
fn install_by_name_prefers_items_when_only_items_match() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = tmp.path().join("registry");
    let target = tmp.path().join("target");
    stage_registry(&registry);
    fs::create_dir_all(&target).unwrap();

    // "license-awareness" is a rule item, not a bundle.
    let report = install_with_lockfile(
        &InstallSource::LocalPath(registry.clone()),
        &target,
        &["license-awareness".into()],
        PluginScope::Project,
    )
    .expect("install");

    assert!(!report.items.is_empty());
    assert!(report.bundles.is_empty(), "no bundle dispatch for item-only match");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test pipeline_lockfile install_by_name`
Expected: First test FAILS with "no matching items in source for: with-plugins".

- [ ] **Step 3: Implement bundle-by-name dispatch in `install_with_lockfile`**

Modify the `install_with_lockfile` function in `src/pipeline.rs`. Replace the current logic between the item filter construction and the "no matching items" bail:

```rust
pub fn install_with_lockfile(
    source: &InstallSource,
    target: &Path,
    items: &[String],
    plugin_scope: crate::plugin::PluginScope,
) -> Result<InstallReport> {
    let mut report = if items.is_empty() {
        // No names → install everything (existing default).
        install_from_source(source, target, None)?
    } else {
        // Names provided → try item-filter first, then bundle-by-name.
        install_with_name_resolution(source, target, items)?
    };

    // -- Plugin installation (ADR-0008) --
    let plugin_results = install_plugins_from_bundles(&report.bundles, plugin_scope);
    report.plugin_results = plugin_results;

    // ... rest unchanged (lockfile recording, ancillary files) ...
```

Add the new helper:

```rust
/// Resolve positional names against both items and bundles.
///
/// For each name:
/// - If it matches items AND a bundle → error (ambiguity).
/// - If it matches only a bundle → install via bundle dispatch.
/// - If it matches only items → install via item filter.
/// - If it matches neither → error.
///
/// When the list contains a mix (some names are bundles, some are items),
/// all bundle installs run first, then item installs run with the
/// remaining names as a filter.
fn install_with_name_resolution(
    source: &InstallSource,
    target: &Path,
    names: &[String],
) -> Result<InstallReport> {
    // We need the source as a local path to check for bundles/items.
    // For git sources, clone first and then inspect.
    let (local_source, _tmp) = resolve_to_local(source)?;

    let mut bundle_paths: Vec<PathBuf> = Vec::new();
    let mut item_names: Vec<String> = Vec::new();

    for name in names {
        let has_items = has_matching_items(&local_source, name);
        let bundle_path = find_bundle_by_name(&local_source, name);

        match (has_items, bundle_path) {
            (true, Some(bp)) => {
                anyhow::bail!(
                    "'{}' matches both an item and a bundle\n\n  \
                     item:   {}/{}\n  \
                     bundle: {}\n\n\
                     Disambiguate with the full path:\n  \
                     upskill add <source>:{}\n  \
                     upskill add <source>:{}",
                    name,
                    name,
                    detect_item_entrypoint(&local_source, name),
                    bp.strip_prefix(&local_source).unwrap_or(&bp).display(),
                    bp.strip_prefix(&local_source).unwrap_or(&bp).display(),
                    name,
                );
            }
            (false, Some(bp)) => bundle_paths.push(bp),
            (true, None) => item_names.push(name.clone()),
            (false, None) => {
                anyhow::bail!("no matching items or bundles in source for: {}", name);
            }
        }
    }

    let mut report = InstallReport::default();

    // Install bundles first.
    for bp in &bundle_paths {
        let bundle_report = install_bundle_file(bp, target)?;
        report.items.extend(bundle_report.items);
        report.bundles.extend(bundle_report.bundles);
    }

    // Install remaining items via the standard filter path.
    if !item_names.is_empty() {
        let filter = crate::bundle::ResolvedItems {
            rules: item_names.clone(),
            skills: item_names.clone(),
            agents: item_names.clone(),
        };
        let item_report = install_from_local_path(&local_source, target, Some(&filter))?;
        report.items.extend(item_report.items);
    }

    Ok(report)
}

/// Resolve an InstallSource to a local path (cloning if needed).
/// Returns the path and an optional TempDir guard (dropped = cleaned up).
fn resolve_to_local(source: &InstallSource) -> Result<(PathBuf, Option<tempfile::TempDir>)> {
    match source {
        InstallSource::LocalPath(p) => Ok((p.clone(), None)),
        InstallSource::Github(repo) => {
            let url = github_authenticated_url(repo)?;
            let tmp = tempfile::tempdir().context("create temp dir for clone")?;
            fetch::shallow_clone(&url, repo.git_ref.as_deref(), "clone", tmp.path())
                .map_err(|e| anyhow!("git clone: {}", e))?;
            let source = fetch::resolve_subfolder(
                &tmp.path().join("clone"),
                repo.subfolder.as_deref(),
                &repo.owner,
                &repo.name,
            )
            .map_err(|e| anyhow!("{}", e))?;
            Ok((source, Some(tmp)))
        }
        InstallSource::Gitlab(repo) => {
            let url = gitlab_authenticated_url(repo)?;
            let tmp = tempfile::tempdir().context("create temp dir for clone")?;
            fetch::shallow_clone(&url, repo.git_ref.as_deref(), "clone", tmp.path())
                .map_err(|e| anyhow!("git clone: {}", e))?;
            let source = fetch::resolve_subfolder(
                &tmp.path().join("clone"),
                repo.subfolder.as_deref(),
                &repo.owner,
                &repo.name,
            )
            .map_err(|e| anyhow!("{}", e))?;
            Ok((source, Some(tmp)))
        }
    }
}

/// Detect which entrypoint file (SKILL.md, RULE.md, AGENT.md) exists for
/// an item, for use in error messages.
fn detect_item_entrypoint(source: &Path, name: &str) -> &'static str {
    let dir = source.join(name);
    if dir.join("SKILL.md").is_file() {
        "SKILL.md"
    } else if dir.join("RULE.md").is_file() {
        "RULE.md"
    } else if dir.join("AGENT.md").is_file() {
        "AGENT.md"
    } else {
        "SKILL.md" // fallback for error message
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test pipeline_lockfile install_by_name`
Expected: 3 tests PASS.

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All existing tests still pass (the empty-items path is unchanged).

- [ ] **Step 6: Commit**

```bash
git add src/pipeline.rs tests/pipeline_lockfile.rs
git commit -m "feat(pipeline): wire bundle-by-name discovery into install_with_lockfile"
```

---

### Task 4: Add lint rule for bundle-item-name-collision

**Files:**

- Modify: `src/lint.rs`
- Test: `tests/cli_lint.rs`

- [ ] **Step 1: Write the failing test**

In `tests/cli_lint.rs`, add:

```rust
#[test]
fn lint_flags_bundle_item_name_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create a skill named "baseline"
    let skill_dir = root.join("baseline");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: baseline\ndescription: a skill\n---\n# body\n",
    )
    .unwrap();

    // Create a bundle also named "baseline"
    let bundles_dir = root.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(
        bundles_dir.join("baseline.bundle.yaml"),
        "schema: 1\nname: baseline\ndescription: a bundle\nitems:\n  skills:\n    - baseline\n",
    )
    .unwrap();

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(root)
        .args(["lint"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("name-collision"))
        .stderr(predicates::str::contains("baseline"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test cli_lint lint_flags_bundle_item_name_collision`
Expected: FAIL — no "name-collision" in output.

- [ ] **Step 3: Implement the lint rule**

In `src/lint.rs`, add a new check function and wire it into `lint()`:

After `check_directives`, add a new cross-file check. Modify the `lint` function to call it after per-file checks complete:

```rust
pub fn lint(paths: &[PathBuf], strict: bool) -> Result<LintReport> {
    // ... existing code through per-file loop ...

    // Cross-file checks: bundle-item name collisions.
    for root in roots {
        if root.is_dir() {
            check_bundle_item_name_collisions(root, &mut report.findings)?;
        }
    }

    if strict {
        // ... existing strict promotion ...
    }

    Ok(report)
}
```

Add the implementation:

```rust
/// Cross-file lint: detect when a bundle's `name:` collides with an item
/// directory name in the same registry. This ambiguity causes
/// `upskill add <source> <name>` to fail at install time — better to
/// catch it during authoring.
fn check_bundle_item_name_collisions(root: &Path, out: &mut Vec<Finding>) -> Result<()> {
    // Collect item names (directories containing SKILL.md/RULE.md/AGENT.md).
    let mut item_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dirname = entry.file_name().to_string_lossy().to_string();
            if dirname.starts_with('.') {
                continue;
            }
            if path.join("SKILL.md").is_file()
                || path.join("RULE.md").is_file()
                || path.join("AGENT.md").is_file()
            {
                item_names.insert(dirname);
            }
        }
    }

    // Discover bundles and check for collisions.
    let mut bundle_files: Vec<PathBuf> = Vec::new();
    walk_bundles(root, &mut bundle_files)?;

    for bundle_path in bundle_files {
        let raw = fs::read_to_string(&bundle_path)
            .with_context(|| format!("read {}", bundle_path.display()))?;
        // Quick extraction of the `name:` field without full parse.
        let bundle_name = extract_bundle_name(&raw);
        if let Some(name) = bundle_name {
            if item_names.contains(&name) {
                let item_kind = detect_item_kind_in_root(root, &name);
                out.push(Finding {
                    rule_id: "name-collision",
                    severity: Severity::Error,
                    path: bundle_path,
                    line: None,
                    message: format!(
                        "bundle '{}' collides with {} '{}'",
                        name, item_kind, name
                    ),
                });
            }
        }
    }

    Ok(())
}

/// Extract the `name:` field from a bundle YAML without full parse.
fn extract_bundle_name(raw: &str) -> Option<String> {
    // Use serde for reliability — the bundle schema is known.
    let parsed: Result<crate::model::Bundle, _> = serde_yaml_ng::from_str(raw);
    parsed.ok().map(|b| b.name)
}

/// Detect what kind of item a name corresponds to in the root, for error messages.
fn detect_item_kind_in_root(root: &Path, name: &str) -> &'static str {
    let dir = root.join(name);
    if dir.join("SKILL.md").is_file() {
        "skill"
    } else if dir.join("RULE.md").is_file() {
        "rule"
    } else if dir.join("AGENT.md").is_file() {
        "agent"
    } else {
        "item"
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test cli_lint lint_flags_bundle_item_name_collision`
Expected: PASS.

- [ ] **Step 5: Run full lint test suite**

Run: `cargo test --test cli_lint`
Expected: All lint tests pass (existing clean fixtures have no collisions).

- [ ] **Step 6: Commit**

```bash
git add src/lint.rs tests/cli_lint.rs
git commit -m "feat(lint): add name-collision rule for bundle-item name conflicts"
```

---

### Task 5: Run clippy + full verification

**Files:** None (verification only)

- [ ] **Step 1: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: No warnings.

- [ ] **Step 2: Run full verification**

Run: `just verify`
Expected: All checks pass.

- [ ] **Step 3: Fix any issues found**

If clippy or tests fail, fix and re-run.

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address clippy/test issues from bundle-by-name"
```

---

### Task 6: Create PR

**Files:** None

- [ ] **Step 1: Push branch**

```bash
git push origin feat/bundle-by-name-discovery
```

- [ ] **Step 2: Create PR**

```bash
gh pr create \
  --title "feat(pipeline): bundle-by-name discovery for upskill add" \
  --base main \
  --body "..." \
  --label "story"
```
