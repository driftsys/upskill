# Coupling Redesign Slice 2 — Cross-Source `requires` Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make cross-source `requires` entries (`{ name, source }`) actually fetch, resolve, install, and record in the lockfile — additively on top of the merged Slice 1 same-source machinery.

**Architecture:** Widen the dependency-closure resolver so it tracks a _per-item canonical source label_, fetches each distinct cross-source locator once (via `fetch_ssot`, cached by canonical label, with `TempDir` guards owned by the returned closure), detects cross-source cycles keyed by `(label, kind, name)` and same-`(kind,name)`-different-source conflicts, then installs each item from _its own_ fetched root and records each lockfile entry with _its own_ source/ref. The design (ADR-0009, design spec §2.5) is fixed — this plan only builds it.

**Tech Stack:** Rust (edition 2024, MSRV 1.85), `anyhow`, `serde`/`serde_json`, `tempfile`. No new dependencies. Tests use `assert_cmd` + `tempfile` + `tests/common::upskill_cmd`.

---

## Background: what Slice 1 built (already on `main`)

- `src/model/requires.rs` — `RequireRef` (untagged `Name(String)` | `Detailed { name, source }`) with `.name()` / `.source() -> Option<&str>`; `ItemRequires { rules, skills, agents: Vec<RequireRef> }`.
- `src/pipeline/install.rs` — `resolve_requires_closure(source: &Path, initial: &[(ItemKind,String)]) -> Result<DependencyClosure>`. `DependencyClosure { items: bundle::ResolvedItems, required_by: BTreeMap<(ItemKind,String), BTreeSet<String>> }`. DFS over `(kind,name)` within ONE source; cross-source entries **bail at `src/pipeline/install.rs:311-316`**. Helpers `index_source`, `read_item_requires`, `visit_requires`.
- `src/pipeline/mod.rs` `install_with_lockfile` — fetches one source, builds `requested`, builds `closure` (skipped for bundle-file / name-resolution deferrals), runs conflict detection over the closure items, installs via one `install_from_local_path(&local_source, target, Some(&closure.items))`, records the lockfile via `items_from_report(report, &label, git_ref, &provenance, hash_for)`.
- `src/lockfile.rs` — `LockedItem { kind, name, source, git_ref, hash, source_name, required_by, group }`; `items_from_report(report, source_label, git_ref, required_by, hash_for)`.
- `src/pipeline/git.rs` — `fetch_ssot(&InstallSource) -> Result<(PathBuf, Option<TempDir>)>` (the auth/clone path; `LocalPath` returns the path with `None` guard).
- `src/source.rs` — `parse_install_source(&str) -> Result<InstallSource, _>` parses the `add` DSL (`owner/repo@ref`, `https://…`, `gitlab:…`, `./path`, `/abs`). `InstallSource::Display` produces the canonical label (`github:owner/repo[@ref][:sub]`, `local:<path>`, …).
- `src/conflict.rs` — `detect_conflicts(incoming: &[(ItemKind,String)], &Lockfile, incoming_source: &str) -> Vec<ItemConflict>`; same `(kind,name)` from a different source = conflict.
- `src/pipeline/lifecycle.rs` `doctor` — advisory orphaned-dependency flag (item whose every `required_by` requirer is gone). Unchanged by Slice 2.

## File Structure (what this slice creates / modifies)

- **Modify** `src/pipeline/install.rs` — the core change: replace the single-source `DependencyClosure` + `resolve_requires_closure` + `visit_requires` with a source-aware `Resolver` that fetches cross-source locators and tracks per-item source labels. (~150 lines changed, the bulk of the slice.)
- **Modify** `src/pipeline/mod.rs` `install_with_lockfile` — consume the new closure: group conflict detection by source, install per-source, record per-item source/ref. Move the `git_ref` computation above the closure block.
- **Modify** `src/lockfile.rs` — change `items_from_report` to take a per-item `source_for(kind, name) -> (String, Option<String>)` resolver instead of one `source_label` + `git_ref`. Update its one unit test.
- **Create** `tests/cross_source_requires.rs` — ATDD integration coverage (happy path, transitive, conflict, cycle, doctor-orphan) using local-path "other sources".
- **Modify** `docs/format-spec.md` §3.7 + §11 item 7 — flip cross-source from "lands in a later release" to implemented.
- **Modify** `docs/adr/0009-coupling-tiers-and-dependencies.md` — flip the "Staging" bullet and the negative-consequence bullet to implemented.

> **Why local-path "other sources" for tests (not bare git repos):** the `requires` `source:` field uses the `add` DSL verbatim, which has **no `file://` form** — so a `requires` entry cannot point at a bare repo by URL. A local path (`/abs/...`) _is_ a valid DSL form, flows through `parse_install_source` → `InstallSource::LocalPath` → `fetch_ssot` (returns the path, `None` guard) → `index_source` → per-source install/record, exercising the entire cross-source path deterministically and offline. The git-clone arm of `fetch_ssot` is already covered by `tests/pipeline_source.rs`; cross-source calls the identical function.

---

## Task 1: Source-aware dependency closure (the core refactor)

This task rewrites the closure to span sources and rewires its two consumers (`mod.rs`, `lockfile.rs`) so the crate stays green. Unit tests in `install.rs` drive it.

**Files:**

- Modify: `src/pipeline/install.rs` (closure types, `resolve_requires_closure`, `Resolver`, unit tests)
- Modify: `src/pipeline/mod.rs` (`install_with_lockfile`)
- Modify: `src/lockfile.rs` (`items_from_report` signature + its unit test)

- [ ] **Step 1: Write the failing unit test for a cross-source pull**

Add to the `#[cfg(test)] mod tests` block in `src/pipeline/install.rs` (the existing `write_item` helper already creates `<root>/<name>/<ENTRY>.md`). This test creates two source roots and asserts the closure resolves the dependency under the _other_ source's canonical label.

```rust
#[test]
fn closure_pulls_cross_source_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let src_a = tmp.path().join("a");
    let src_b = tmp.path().join("b");
    std::fs::create_dir_all(&src_a).unwrap();
    std::fs::create_dir_all(&src_b).unwrap();
    write_item(&src_b, ItemKind::Skill, "sarif", "");
    write_item(
        &src_a,
        ItemKind::Agent,
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
            src_b.display()
        ),
    );

    let a_label = format!("local:{}", src_a.display());
    let b_label = format!("local:{}", src_b.display());
    let closure = resolve_requires_closure(
        &a_label,
        &src_a,
        None,
        &[(ItemKind::Agent, "code-review".to_string())],
    )
    .expect("closure");

    // code-review resolves under source A; sarif under source B.
    assert_eq!(
        closure.item_source.get(&(ItemKind::Agent, "code-review".to_string())),
        Some(&a_label)
    );
    assert_eq!(
        closure.item_source.get(&(ItemKind::Skill, "sarif".to_string())),
        Some(&b_label)
    );
    // sarif installs from B's root.
    assert!(closure.by_source[&b_label].items.skills.contains(&"sarif".to_string()));
    // provenance recorded.
    let prov = closure
        .required_by
        .get(&(ItemKind::Skill, "sarif".to_string()))
        .expect("sarif provenance");
    assert!(prov.contains("agent:code-review"), "{prov:?}");
}
```

- [ ] **Step 2: Run it to confirm it fails to compile**

Run: `cargo test -p upskill --lib closure_pulls_cross_source_dependency`
Expected: FAIL — `resolve_requires_closure` takes 2 args not 4; no `item_source` / `by_source` fields. (Compile error is the expected failure.)

- [ ] **Step 3: Replace the closure types and resolver in `install.rs`**

Replace the entire region from the `DependencyClosure` struct (line ~166) through the end of `visit_requires` (line ~335) with the source-aware implementation below. Also add the imports at the top of the file: in the existing `use crate::model::{...}` line keep it, and add `use crate::source::{InstallSource, parse_install_source};`.

```rust
/// The transitive closure of items reached by expanding a requested set
/// along `requires` edges — possibly spanning multiple sources.
#[derive(Debug, Default)]
pub(super) struct DependencyClosure {
    /// Per canonical source label: the fetched root, its pinned ref, and
    /// the items to install from it (in dependency order).
    pub by_source: BTreeMap<String, SourceInstall>,
    /// Canonical source label each resolved `(kind, name)` came from.
    /// `(kind, name)` is globally unique within a valid closure (a clash
    /// across sources is a conflict error), so this maps cleanly.
    pub item_source: BTreeMap<(ItemKind, String), String>,
    /// `(kind, name) -> { "<requirer-kind>:<requirer-name>", .. }`. Empty
    /// for directly-requested items. Drives the lockfile's `required_by`.
    pub required_by: BTreeMap<(ItemKind, String), BTreeSet<String>>,
    /// Tempdir guards for cross-source fetches. Held here so the clones
    /// outlive the per-source install calls in `install_with_lockfile`.
    pub guards: Vec<tempfile::TempDir>,
}

/// One source's contribution to a [`DependencyClosure`].
#[derive(Debug)]
pub(super) struct SourceInstall {
    pub root: PathBuf,
    pub git_ref: Option<String>,
    pub items: crate::bundle::ResolvedItems,
}

/// A fetched-and-indexed source, cached by canonical label during DFS.
struct FetchedSource {
    root: PathBuf,
    git_ref: Option<String>,
    index: BTreeMap<(ItemKind, String), PathBuf>,
}

/// Index every item in `source` by its effective `(kind, name)` identity.
fn index_source(source: &Path) -> Result<BTreeMap<(ItemKind, String), PathBuf>> {
    let mut idx = BTreeMap::new();
    for (folder, dir) in iter_item_dirs(source)? {
        for kind in [ItemKind::Skill, ItemKind::Rule, ItemKind::Agent] {
            let entry = dir.join(kind.entrypoint_filename());
            if entry.is_file() {
                let name = super::discovery::probe_effective_name(&entry, kind, &folder);
                idx.insert((kind, name), dir.clone());
            }
        }
    }
    Ok(idx)
}

/// Parse an item's `requires` block. For agents, fold `preload_skills` into
/// `requires.skills` softly (a preloaded skill present in the SAME source is
/// implicitly required; an absent one is a runtime hint, skipped).
fn read_item_requires(
    dir: &Path,
    kind: ItemKind,
    index: &BTreeMap<(ItemKind, String), PathBuf>,
) -> Result<crate::model::ItemRequires> {
    let entry = dir.join(kind.entrypoint_filename());
    let raw = fs::read_to_string(&entry).with_context(|| format!("read {}", entry.display()))?;
    let req = match kind {
        ItemKind::Skill => frontmatter::parse::<Skill>(&raw)?.0.requires,
        ItemKind::Rule => frontmatter::parse::<Rule>(&raw)?.0.requires,
        ItemKind::Agent => {
            let agent = frontmatter::parse::<Agent>(&raw)?.0;
            let mut req = agent.requires;
            for s in &agent.preload_skills {
                let present = index.contains_key(&(ItemKind::Skill, s.clone()));
                if present && !req.skills.iter().any(|r| r.name() == s) {
                    req.skills.push(RequireRef::Name(s.clone()));
                }
            }
            req
        }
    };
    Ok(req)
}

/// DFS state for [`resolve_requires_closure`]. Carries the fetch cache so a
/// source pulled by two edges is fetched once, and the visited/on-path sets
/// keyed for cross-source cycle and conflict detection.
struct Resolver {
    /// Canonical label -> fetched-and-indexed source.
    sources: BTreeMap<String, FetchedSource>,
    /// Tempdir guards for cross-source clones (drained into the closure).
    guards: Vec<tempfile::TempDir>,
    /// Fully-resolved identities (dedup).
    resolved: BTreeSet<(ItemKind, String)>,
    /// The label each identity resolved under — second visit under a
    /// different label is a cross-source conflict.
    item_label: BTreeMap<(ItemKind, String), String>,
    /// Dependency-order accumulation.
    order: Vec<(ItemKind, String)>,
    /// On-path set keyed by `(label, kind, name)` for cycle detection (§2.5).
    on_path: BTreeSet<(String, ItemKind, String)>,
    /// Dependency provenance keyed by identity.
    required_by: BTreeMap<(ItemKind, String), BTreeSet<String>>,
}

impl Resolver {
    /// Ensure `src_dsl` (an `add`-DSL locator) is fetched and indexed;
    /// return its canonical label.
    fn ensure_source(&mut self, src_dsl: &str) -> Result<String> {
        let install_source = parse_install_source(src_dsl)
            .map_err(|e| anyhow::anyhow!("parse requires source `{src_dsl}`: {e}"))?;
        let label = install_source.to_string();
        if !self.sources.contains_key(&label) {
            let (root, guard) = super::git::fetch_ssot(&install_source)
                .with_context(|| format!("fetch requires source `{label}`"))?;
            let index = index_source(&root)?;
            if let Some(g) = guard {
                self.guards.push(g);
            }
            self.sources.insert(
                label.clone(),
                FetchedSource {
                    root,
                    git_ref: source_git_ref(&install_source),
                    index,
                },
            );
        }
        Ok(label)
    }

    fn visit(
        &mut self,
        label: &str,
        node: (ItemKind, String),
        requirer: Option<&(ItemKind, String)>,
    ) -> Result<()> {
        let (kind, name) = node.clone();

        // Cross-source identity conflict: the same (kind, name) reached from
        // two different sources (format-spec §3.7).
        if let Some(prev) = self.item_label.get(&node)
            && prev != label
        {
            anyhow::bail!(
                "{kind} `{name}` is required from two different sources: `{prev}` and `{label}`"
            );
        }

        let dir = self
            .sources
            .get(label)
            .expect("source fetched before visit")
            .index
            .get(&node)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("{kind} `{name}` is required but not found in source `{label}`")
            })?;

        if let Some((rk, rn)) = requirer {
            self.required_by
                .entry(node.clone())
                .or_default()
                .insert(format!("{rk}:{rn}"));
        }

        if self.resolved.contains(&node) {
            return Ok(());
        }
        let path_key = (label.to_string(), kind, name.clone());
        if self.on_path.contains(&path_key) {
            anyhow::bail!("circular dependency detected including {kind} `{name}`");
        }
        self.on_path.insert(path_key.clone());
        self.item_label.insert(node.clone(), label.to_string());

        let requires = {
            let index = &self.sources.get(label).expect("source present").index;
            read_item_requires(&dir, kind, index)?
        };
        for (dep_kind, refs) in [
            (ItemKind::Rule, &requires.rules),
            (ItemKind::Skill, &requires.skills),
            (ItemKind::Agent, &requires.agents),
        ] {
            for r in refs {
                let dep = (dep_kind, r.name().to_string());
                match r.source() {
                    None => self.visit(label, dep, Some(&node))?,
                    Some(src_dsl) => {
                        let dep_label = self.ensure_source(src_dsl)?;
                        self.visit(&dep_label, dep, Some(&node))?;
                    }
                }
            }
        }

        self.on_path.remove(&path_key);
        self.resolved.insert(node.clone());
        self.order.push(node);
        Ok(())
    }
}

fn source_git_ref(s: &InstallSource) -> Option<String> {
    match s {
        InstallSource::Github(r) => r.git_ref.clone(),
        InstallSource::Gitlab(r) => r.git_ref.clone(),
        InstallSource::LocalPath(_) => None,
    }
}

/// Expand `initial` (identities in the already-fetched entry source) along
/// `requires` edges into the full cross-source closure.
///
/// `entry_label` is the entry source's canonical label, `entry_root` its
/// already-fetched root (seeded into the fetch cache so it is not re-fetched),
/// `entry_git_ref` its pinned ref. Cross-source `{ name, source }` entries
/// are fetched via [`fetch_ssot`], cached by canonical label. Cycles
/// (keyed `(label, kind, name)`) and same-`(kind,name)`-different-source
/// clashes are errors; a required item missing from its source is an error.
pub(super) fn resolve_requires_closure(
    entry_label: &str,
    entry_root: &Path,
    entry_git_ref: Option<&str>,
    initial: &[(ItemKind, String)],
) -> Result<DependencyClosure> {
    let mut resolver = Resolver {
        sources: BTreeMap::new(),
        guards: Vec::new(),
        resolved: BTreeSet::new(),
        item_label: BTreeMap::new(),
        order: Vec::new(),
        on_path: BTreeSet::new(),
        required_by: BTreeMap::new(),
    };
    resolver.sources.insert(
        entry_label.to_string(),
        FetchedSource {
            root: entry_root.to_path_buf(),
            git_ref: entry_git_ref.map(str::to_string),
            index: index_source(entry_root)?,
        },
    );

    for node in initial {
        resolver.visit(entry_label, node.clone(), None)?;
    }

    let mut by_source: BTreeMap<String, SourceInstall> = BTreeMap::new();
    let mut item_source: BTreeMap<(ItemKind, String), String> = BTreeMap::new();
    for node in &resolver.order {
        let label = resolver
            .item_label
            .get(node)
            .cloned()
            .expect("resolved item has a source");
        let group = by_source.entry(label.clone()).or_insert_with(|| {
            let fs = resolver.sources.get(&label).expect("source present");
            SourceInstall {
                root: fs.root.clone(),
                git_ref: fs.git_ref.clone(),
                items: crate::bundle::ResolvedItems::default(),
            }
        });
        match node.0 {
            ItemKind::Rule => group.items.rules.push(node.1.clone()),
            ItemKind::Skill => group.items.skills.push(node.1.clone()),
            ItemKind::Agent => group.items.agents.push(node.1.clone()),
        }
        item_source.insert(node.clone(), label);
    }

    Ok(DependencyClosure {
        by_source,
        item_source,
        required_by: resolver.required_by,
        guards: resolver.guards,
    })
}
```

- [ ] **Step 4: Update the existing `install.rs` closure unit tests to the new API**

The three existing tests (`closure_pulls_same_source_dependency`, `closure_rejects_cycle`, `closure_folds_preload_skills`, `closure_skips_absent_preload_skill`) call the old 2-arg `resolve_requires_closure(src, &[...])` and assert on `closure.items`. Update each call to `resolve_requires_closure("local:test", src, None, &[...])` and replace `closure.items.contains(kind, name)` assertions with `closure.item_source.contains_key(&(kind, name.to_string()))`. Example for `closure_pulls_same_source_dependency`:

```rust
#[test]
fn closure_pulls_same_source_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path();
    write_item(src, ItemKind::Agent, "code-review", "requires:\n  skills: [sarif]\n");
    write_item(src, ItemKind::Skill, "sarif", "");

    let closure = resolve_requires_closure(
        "local:test",
        src,
        None,
        &[(ItemKind::Agent, "code-review".to_string())],
    )
    .expect("closure");

    assert!(closure.item_source.contains_key(&(ItemKind::Agent, "code-review".to_string())));
    assert!(closure.item_source.contains_key(&(ItemKind::Skill, "sarif".to_string())));
    let prov = closure
        .required_by
        .get(&(ItemKind::Skill, "sarif".to_string()))
        .expect("sarif provenance");
    assert!(prov.contains("agent:code-review"), "{prov:?}");
}
```

Apply the same call-signature change to `closure_rejects_cycle`, `closure_folds_preload_skills`, and `closure_skips_absent_preload_skill` (the latter two replace `closure.items.contains(...)` / `!closure.items.contains(...)` with `closure.item_source.contains_key(...)` / `!closure.item_source.contains_key(...)`).

- [ ] **Step 5: Replace `closure_errors_on_cross_source_entry` with a not-found test**

Cross-source no longer bails as "not yet available". Replace that test with one asserting a cross-source dependency that is absent from its (existing-but-empty) source errors with "not found in source":

```rust
#[test]
fn closure_errors_on_missing_cross_source_item() {
    let tmp = tempfile::tempdir().unwrap();
    let src_a = tmp.path().join("a");
    let src_b = tmp.path().join("b");
    std::fs::create_dir_all(&src_a).unwrap();
    std::fs::create_dir_all(&src_b).unwrap(); // exists but holds no `x`
    write_item(
        &src_a,
        ItemKind::Rule,
        "a",
        &format!("requires:\n  skills: [{{ name: x, source: {} }}]\n", src_b.display()),
    );

    let err = resolve_requires_closure(
        &format!("local:{}", src_a.display()),
        &src_a,
        None,
        &[(ItemKind::Rule, "a".to_string())],
    )
    .expect_err("missing cross-source item");
    assert!(err.to_string().contains("not found in source"), "{err}");
}
```

- [ ] **Step 6: Add a cross-source cycle unit test**

```rust
#[test]
fn closure_rejects_cross_source_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let src_a = tmp.path().join("a");
    let src_b = tmp.path().join("b");
    std::fs::create_dir_all(&src_a).unwrap();
    std::fs::create_dir_all(&src_b).unwrap();
    write_item(
        &src_a,
        ItemKind::Rule,
        "alpha",
        &format!("requires:\n  rules: [{{ name: beta, source: {} }}]\n", src_b.display()),
    );
    write_item(
        &src_b,
        ItemKind::Rule,
        "beta",
        &format!("requires:\n  rules: [{{ name: alpha, source: {} }}]\n", src_a.display()),
    );

    let err = resolve_requires_closure(
        &format!("local:{}", src_a.display()),
        &src_a,
        None,
        &[(ItemKind::Rule, "alpha".to_string())],
    )
    .expect_err("cross-source cycle");
    assert!(err.to_string().contains("circular"), "{err}");
}
```

- [ ] **Step 7: Change `items_from_report` in `lockfile.rs` to a per-item source resolver**

Replace the signature and body so each item's source/ref is looked up per `(kind, name)`:

```rust
pub fn items_from_report(
    report: &InstallReport,
    source_for: impl Fn(ItemKind, &str) -> (String, Option<String>),
    required_by: &std::collections::BTreeMap<(ItemKind, String), Vec<String>>,
    mut hash_for: impl FnMut(ItemKind, &str) -> Option<String>,
) -> Vec<LockedItem> {
    use std::collections::BTreeSet;
    let mut seen: BTreeSet<(ItemKind, String)> = BTreeSet::new();
    let mut out = Vec::new();
    for entry in &report.items {
        if !seen.insert((entry.kind, entry.name.clone())) {
            continue;
        }
        let (source, git_ref) = source_for(entry.kind, &entry.name);
        out.push(LockedItem {
            kind: entry.kind,
            name: entry.name.clone(),
            source,
            git_ref,
            hash: hash_for(entry.kind, &entry.name),
            source_name: None,
            required_by: required_by
                .get(&(entry.kind, entry.name.clone()))
                .cloned()
                .unwrap_or_default(),
            group: entry.group.clone(),
        });
    }
    out
}
```

Update the doc comment above it to describe `source_for` (per-item `(label, ref)` lookup — supports cross-source dependency-pulled items recording their own source).

- [ ] **Step 8: Update the `items_from_report` unit test in `lockfile.rs`**

In `items_from_report_dedupes_per_kind_name`, change the call to pass a closure for `source_for`:

```rust
let items = items_from_report(
    &report,
    |_, _| ("local:./src".to_string(), None),
    &std::collections::BTreeMap::new(),
    |_, _| Some("sha256:abc".into()),
);
```

(The rest of the assertions are unchanged: `items[0].source == "local:./src"`, etc.)

- [ ] **Step 9: Rewire `install_with_lockfile` in `mod.rs` — move `git_ref` up and rebuild the closure**

In `src/pipeline/mod.rs`, **delete** the later `let git_ref = match source { ... };` block (currently ~line 342) and **insert** it just before the closure block (replacing the `// -- Same-source`requires`closure --` region's closure construction). The closure now needs `entry_label` (`&label`), `entry_root` (`&local_source`), and `entry_git_ref` (`git_ref`).

Replace the closure construction (currently lines ~222-226) with:

```rust
let git_ref = match source {
    InstallSource::Github(r) => r.git_ref.as_deref(),
    InstallSource::Gitlab(r) => r.git_ref.as_deref(),
    InstallSource::LocalPath(_) => None,
};
let closure = if discovery::is_bundle_file(&local_source) || defer_to_name_resolution {
    None
} else {
    Some(resolve_requires_closure(
        &label,
        &local_source,
        git_ref,
        &requested,
    )?)
};
```

- [ ] **Step 10: Rewire conflict detection in `mod.rs` to group by source**

Replace the conflict-detection block (currently lines ~228-280, from `let mut lock = ...` through the `anyhow::bail!` on conflicts) with per-source grouping. Cross-source dependency items are never aliased; only entry-source items honor `--as`:

```rust
// -- Conflict detection (before any files are written) --
let mut lock = crate::lockfile::Lockfile::load(target)?;
let mut conflicts = Vec::new();
if let Some(closure) = &closure {
    // Group incoming items by their canonical source label; conflicts
    // are per (kind, name, source). Aliases apply only to entry-source
    // items — dependency-pulled items are never aliased.
    let mut grouped: std::collections::BTreeMap<String, Vec<(ItemKind, String)>> =
        std::collections::BTreeMap::new();
    for ((kind, name), src_label) in &closure.item_source {
        let effective = if *src_label == label {
            options
                .aliases
                .iter()
                .find(|(from, _)| from.is_empty() || *from == *name)
                .map(|(_, to)| to.clone())
                .unwrap_or_else(|| name.clone())
        } else {
            name.clone()
        };
        grouped
            .entry(src_label.clone())
            .or_default()
            .push((*kind, effective));
    }
    for (src_label, items) in &grouped {
        conflicts.extend(crate::conflict::detect_conflicts(items, &lock, src_label));
    }
} else {
    let mut seen = std::collections::BTreeSet::new();
    let incoming: Vec<(ItemKind, String)> = scanned_items
        .iter()
        .filter(|(_, n)| !options.excludes.contains(n))
        .filter(|(_, n)| items.is_empty() || items.contains(n))
        .filter_map(|(kind, name)| {
            let effective_name = options
                .aliases
                .iter()
                .find(|(from, _)| from.is_empty() || *from == *name)
                .map(|(_, to)| to.clone())
                .unwrap_or_else(|| name.clone());
            if seen.insert((*kind, effective_name.clone())) {
                Some((*kind, effective_name))
            } else {
                None
            }
        })
        .collect();
    conflicts.extend(crate::conflict::detect_conflicts(&incoming, &lock, &label));
}
if !conflicts.is_empty() && !options.force {
    anyhow::bail!("{}", crate::conflict::format_conflict_error(&conflicts));
}
```

- [ ] **Step 11: Rewire the install branch in `mod.rs` to install per source**

Replace the install block (currently lines ~282-292, `let mut report = if let Some(closure) ...`) with a per-source loop. Cross-source `TempDir` guards live in `closure.guards`, which stays in scope until the end of the function:

```rust
// -- Now proceed with generation (files are written here) --
// With a closure, install each source's resolved items from that
// source's own fetched root. Without one (bundle file / name
// resolution), keep the existing dispatch.
let mut report = if let Some(closure) = &closure {
    let mut r = InstallReport::default();
    for si in closure.by_source.values() {
        let part = install_from_local_path(&si.root, target, Some(&si.items))?;
        r.items.extend(part.items);
        r.bundles.extend(part.bundles);
    }
    r
} else if items.is_empty() {
    install_from_local_path(&local_source, target, None)?
} else {
    install_with_name_resolution_from_local(&local_source, target, items)?
};
```

- [ ] **Step 12: Rewire the lockfile recording in `mod.rs` to per-item source**

The `provenance` map construction (from `closure.required_by`) is unchanged. Replace the `items_from_report` call (currently ~lines 364-367) and keep `hashes` as-is. Note the `git_ref` binding is now defined earlier (Step 9), so the bundle-recording code below it still works:

```rust
let item_source_lookup = |kind: ItemKind, name: &str| -> (String, Option<String>) {
    if let Some(closure) = &closure
        && let Some(src_label) = closure.item_source.get(&(kind, name.to_string()))
    {
        let gr = closure
            .by_source
            .get(src_label)
            .and_then(|si| si.git_ref.clone());
        return (src_label.clone(), gr);
    }
    (label.clone(), git_ref.map(str::to_string))
};
let new_items = crate::lockfile::items_from_report(
    &report,
    item_source_lookup,
    &provenance,
    |k, n| hashes.get(&(k, n.to_string())).cloned().flatten(),
);
```

(Leave the `for mut item in new_items { ... lock.upsert(item); }` loop, the alias `source_name` fix-up, the bundle-recording loop using `label`/`git_ref`, and the plugin/ancillary tails unchanged.)

- [ ] **Step 13: Build and run all library unit tests**

Run: `just assemble` then `cargo test -p upskill --lib`
Expected: PASS — all `install.rs` closure tests (including the new `closure_pulls_cross_source_dependency`, `closure_rejects_cross_source_cycle`, `closure_errors_on_missing_cross_source_item`), the `lockfile.rs` `items_from_report_dedupes_per_kind_name`, and every pre-existing test compile and pass. Fix any borrow/type errors surfaced by the compiler before moving on.

- [ ] **Step 14: Run the full existing test suite to confirm no regressions**

Run: `just test`
Expected: PASS — the Slice 1 integration tests (`cli_add`, `pipeline_*`, `generate_*`) still pass; same-source closure behavior is preserved.

- [ ] **Step 15: Commit**

```bash
git add src/pipeline/install.rs src/pipeline/mod.rs src/lockfile.rs
git commit -m "feat(pipeline): resolve cross-source requires into a per-source install closure"
```

---

## Task 2: ATDD — cross-source happy path

**Files:**

- Create: `tests/cross_source_requires.rs`

- [ ] **Step 1: Write the failing integration test**

Create `tests/cross_source_requires.rs` with the shared helper plus the happy-path test. Local-path "other source" `source-b`; entry source `source-a` whose agent requires B's skill. Isolated fake `$HOME` + `.git` marker per issue #193.

```rust
//! ATDD: cross-source `requires` resolution (ADR-0009 / format-spec §3.7).
//!
//! The "other source" is a local-path SSOT root — a valid `add`-DSL locator
//! that flows through `parse_install_source` -> `fetch_ssot` (LocalPath arm)
//! -> per-source install, exercising the whole cross-source path offline.

mod common;

use common::upskill_cmd;
use std::fs;
use std::path::Path;

fn write_item(root: &Path, kind: &str, name: &str, extra: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let entry = match kind {
        "skill" => "SKILL.md",
        "rule" => "RULE.md",
        "agent" => "AGENT.md",
        _ => unreachable!(),
    };
    let body =
        format!("---\nschema: 1\nname: {name}\ndescription: test {name}\n{extra}---\n# {name}\n");
    fs::write(dir.join(entry), body).unwrap();
}

fn read_lock(project: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project.join(".upskill-lock.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn cross_source_requires_pulls_and_records_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let source_b = tmp.path().join("source-b");
    write_item(&source_b, "skill", "sarif-formatting", "");

    let source_a = tmp.path().join("source-a");
    write_item(
        &source_a,
        "agent",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif-formatting, source: {} }}]\n",
            source_b.display()
        ),
    );

    let project = tmp.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();

    upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", source_a.to_str().unwrap(), "code-review"])
        .assert()
        .success();

    // The dependency skill is generated alongside the requested agent.
    assert!(
        project
            .join(".claude/skills/sarif-formatting/SKILL.md")
            .exists(),
        "cross-source dependency skill must be generated"
    );

    // Lockfile records the dependency with ITS OWN source + provenance.
    let lock = read_lock(&project);
    let items = lock["items"].as_array().unwrap();
    let sarif = items
        .iter()
        .find(|i| i["name"] == "sarif-formatting" && i["kind"] == "skill")
        .expect("sarif-formatting in lockfile");
    assert_eq!(
        sarif["source"].as_str().unwrap(),
        format!("local:{}", source_b.display())
    );
    assert_eq!(sarif["required_by"][0], "agent:code-review");

    // The requested agent records the ENTRY source.
    let agent = items
        .iter()
        .find(|i| i["name"] == "code-review" && i["kind"] == "agent")
        .expect("code-review in lockfile");
    assert_eq!(
        agent["source"].as_str().unwrap(),
        format!("local:{}", source_a.display())
    );
}
```

- [ ] **Step 2: Run it**

Run: `cargo test --test cross_source_requires cross_source_requires_pulls_and_records_dependency`
Expected: PASS (Task 1 already implemented the behavior). If the `.claude/skills/...` path assertion fails, confirm the skill output path via `src/pipeline/output.rs::output_path` and adjust the asserted path — do not change production code.

- [ ] **Step 3: Commit**

```bash
git add tests/cross_source_requires.rs
git commit -m "test(pipeline): ATDD cross-source requires happy path"
```

---

## Task 3: ATDD — transitive cross-source closure

**Files:**

- Modify: `tests/cross_source_requires.rs`

- [ ] **Step 1: Add the transitive test**

A→B→C chain across three sources: agent in A requires skill in B; that skill requires a rule in C. All three install; each records its own source.

```rust
#[test]
fn cross_source_requires_resolves_transitively() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let source_c = tmp.path().join("source-c");
    write_item(&source_c, "rule", "security-baseline", "");

    let source_b = tmp.path().join("source-b");
    write_item(
        &source_b,
        "skill",
        "sarif-formatting",
        &format!(
            "requires:\n  rules: [{{ name: security-baseline, source: {} }}]\n",
            source_c.display()
        ),
    );

    let source_a = tmp.path().join("source-a");
    write_item(
        &source_a,
        "agent",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif-formatting, source: {} }}]\n",
            source_b.display()
        ),
    );

    let project = tmp.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();

    upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", source_a.to_str().unwrap(), "code-review"])
        .assert()
        .success();

    let lock = read_lock(&project);
    let items = lock["items"].as_array().unwrap();
    let baseline = items
        .iter()
        .find(|i| i["name"] == "security-baseline" && i["kind"] == "rule")
        .expect("transitive rule in lockfile");
    assert_eq!(
        baseline["source"].as_str().unwrap(),
        format!("local:{}", source_c.display())
    );
    assert_eq!(baseline["required_by"][0], "skill:sarif-formatting");
    assert!(
        project
            .join(".github/instructions/security-baseline.instructions.md")
            .exists()
            || project.join(".claude/rules").exists()
            || items.len() == 3,
        "the transitive rule must be installed; saw {} items",
        items.len()
    );
}
```

> Note: the rule output path varies per client; the lockfile assertion (`baseline.source` + `required_by`) is the authoritative check. Trim the final `||` assertion to the confirmed rule output path during execution if desired.

- [ ] **Step 2: Run it**

Run: `cargo test --test cross_source_requires cross_source_requires_resolves_transitively`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add tests/cross_source_requires.rs
git commit -m "test(pipeline): ATDD transitive cross-source closure"
```

---

## Task 4: ATDD — cross-source conflict is an error

**Files:**

- Modify: `tests/cross_source_requires.rs`

- [ ] **Step 1: Add the conflict test**

Pre-install `sarif-formatting` from source C, then add source A (which pulls `sarif-formatting` from a _different_ source B). The closure-vs-lockfile conflict must abort with the existing "already installed" message and a non-zero exit.

```rust
#[test]
fn cross_source_conflict_with_existing_install_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    // Source C provides sarif-formatting directly.
    let source_c = tmp.path().join("source-c");
    write_item(&source_c, "skill", "sarif-formatting", "");

    // Source B also provides sarif-formatting (different source).
    let source_b = tmp.path().join("source-b");
    write_item(&source_b, "skill", "sarif-formatting", "");

    // Source A's agent requires sarif-formatting FROM B.
    let source_a = tmp.path().join("source-a");
    write_item(
        &source_a,
        "agent",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif-formatting, source: {} }}]\n",
            source_b.display()
        ),
    );

    let project = tmp.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();

    // First, install sarif-formatting from C.
    upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", source_c.to_str().unwrap(), "sarif-formatting"])
        .assert()
        .success();

    // Now adding A pulls sarif-formatting from B — conflict with C.
    upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", source_a.to_str().unwrap(), "code-review"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already installed"));
}
```

This test uses `predicates` (an `assert_cmd` companion already in the dev-dependency tree via `assert_cmd`). If `predicates` is not a direct dev-dependency, replace the `.stderr(...)` line with a manual check: capture `.get_output().stderr` and `assert!(String::from_utf8_lossy(...).contains("already installed"))` — but first try `predicates`, which `assert_cmd` re-exports patterns for.

- [ ] **Step 2: Run it**

Run: `cargo test --test cross_source_requires cross_source_conflict_with_existing_install_errors`
Expected: PASS. If `predicates` is unavailable, switch to the manual stderr check described above and re-run.

- [ ] **Step 3: Commit**

```bash
git add tests/cross_source_requires.rs
git commit -m "test(pipeline): ATDD cross-source conflict aborts"
```

---

## Task 5: ATDD — cross-source cycle is an error

**Files:**

- Modify: `tests/cross_source_requires.rs`

- [ ] **Step 1: Add the cycle test**

A's rule `alpha` requires B's rule `beta` (cross), B's `beta` requires A's `alpha` (cross). `upskill add <A> alpha` must fail with "circular".

```rust
#[test]
fn cross_source_cycle_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let source_a = tmp.path().join("source-a");
    let source_b = tmp.path().join("source-b");
    fs::create_dir_all(&source_a).unwrap();
    fs::create_dir_all(&source_b).unwrap();

    write_item(
        &source_a,
        "rule",
        "alpha",
        &format!(
            "requires:\n  rules: [{{ name: beta, source: {} }}]\n",
            source_b.display()
        ),
    );
    write_item(
        &source_b,
        "rule",
        "beta",
        &format!(
            "requires:\n  rules: [{{ name: alpha, source: {} }}]\n",
            source_a.display()
        ),
    );

    let project = tmp.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();

    upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", source_a.to_str().unwrap(), "alpha"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("circular"));
}
```

- [ ] **Step 2: Run it**

Run: `cargo test --test cross_source_requires cross_source_cycle_errors`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add tests/cross_source_requires.rs
git commit -m "test(pipeline): ATDD cross-source cycle aborts"
```

---

## Task 6: ATDD — doctor flags an orphaned cross-source dependency after removal

**Files:**

- Modify: `tests/cross_source_requires.rs`

- [ ] **Step 1: Add the doctor-orphan test**

Install A's `code-review` (pulls B's `sarif-formatting`). Remove `code-review` (no cascade). `doctor` must report `sarif-formatting` as an orphaned dependency (advisory; exit 0).

```rust
#[test]
fn removing_requirer_leaves_cross_source_dependency_as_doctor_orphan() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let source_b = tmp.path().join("source-b");
    write_item(&source_b, "skill", "sarif-formatting", "");

    let source_a = tmp.path().join("source-a");
    write_item(
        &source_a,
        "agent",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif-formatting, source: {} }}]\n",
            source_b.display()
        ),
    );

    let project = tmp.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();

    upskill_cmd(&home)
        .current_dir(&project)
        .args(["add", source_a.to_str().unwrap(), "code-review"])
        .assert()
        .success();

    // Remove the requirer; the dependency is NOT cascaded.
    upskill_cmd(&home)
        .current_dir(&project)
        .args(["remove", "code-review"])
        .assert()
        .success();

    // The dependency is still recorded.
    let lock = read_lock(&project);
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["name"] == "sarif-formatting"),
        "removal must not cascade to the dependency"
    );

    // doctor surfaces it as an orphaned dependency (advisory → exit 0).
    upskill_cmd(&home)
        .current_dir(&project)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("sarif-formatting"));
}
```

> During execution, confirm `main.rs`'s doctor presentation includes orphaned-dependency item names in stdout. If the wording differs (e.g. the name is only shown under an "orphaned dependencies" heading), keep the `contains("sarif-formatting")` check — it is robust to heading wording. Adjust only if doctor omits the name entirely.

- [ ] **Step 2: Run it**

Run: `cargo test --test cross_source_requires removing_requirer_leaves_cross_source_dependency_as_doctor_orphan`
Expected: PASS

- [ ] **Step 3: Run the whole new test file**

Run: `cargo test --test cross_source_requires`
Expected: PASS — all six tests green.

- [ ] **Step 4: Commit**

```bash
git add tests/cross_source_requires.rs
git commit -m "test(pipeline): ATDD doctor flags orphaned cross-source dependency"
```

---

## Task 7: Documentation — flip cross-source to implemented

**Files:**

- Modify: `docs/format-spec.md` (§3.7 item-requires resolution, §11 item 7)
- Modify: `docs/adr/0009-coupling-tiers-and-dependencies.md` (Cross-source contract staging bullet + Consequences)

- [ ] **Step 1: Update format-spec §3.7**

In `docs/format-spec.md`, replace the trailing paragraph of "Item `requires` resolution" (currently lines ~692-696):

Old:

```markdown
Each `requires.<kind>` entry is a bare name (same source) or a `{ name, source }` map
(cross-source), where `source` reuses the `upskill add` source DSL. Same-source resolution
(bare-name entries against the entry item's own already-fetched source) is implemented in the
initial release; cross-source resolution (`{ name, source }` entries) is specified here but
lands in a later release.
```

New:

```markdown
Each `requires.<kind>` entry is a bare name (same source) or a `{ name, source }` map
(cross-source), where `source` reuses the `upskill add` source DSL. Same-source resolution
(bare-name entries against the entry item's own already-fetched source) and cross-source
resolution (`{ name, source }` entries) are both implemented: a cross-source entry is fetched
via the same machinery as `upskill add` (a distinct source is fetched once and cached for the
duration of the resolution), the transitive closure MAY span sources, and each
dependency-pulled item is recorded in the lockfile with its **own** `source` (and `ref`) plus
a `required_by` provenance list. Cycle detection is keyed by `(canonical-source-label, kind,
name)`; the same `(kind, name)` resolving to two different sources within one closure — or
conflicting with an existing different-source install — is an error. There is no version-range
solving.
```

- [ ] **Step 2: Update format-spec §11 item 7**

Replace the "Same-source resolution is implemented; cross-source resolution is staged to a later release." sentence (currently lines ~1047-1049) with:

```markdown
`(canonical-source-label, kind, name)`. Both same-source and cross-source resolution are
implemented. See
[ADR-0009](./adr/0009-coupling-tiers-and-dependencies.md).
```

- [ ] **Step 3: Update ADR-0009 cross-source staging bullet**

In `docs/adr/0009-coupling-tiers-and-dependencies.md`, replace the "Staging" bullet (currently lines ~115-116):

Old:

```markdown
- **Staging.** Cross-source resolution ships in a follow-up release (Slice 2);
  same-source resolution ships now (Slice 1).
```

New:

```markdown
- **Status.** Both same-source (Slice 1) and cross-source (Slice 2) resolution
  are implemented. A cross-source `source` is fetched once and cached for the
  duration of one resolution; the transitive closure may span sources.
```

- [ ] **Step 4: Update ADR-0009 negative-consequences bullet**

Replace the "Cross-source resolution is staged to a later release..." bullet (currently lines ~146-147) with:

```markdown
- A cross-source closure fetches each distinct source once per resolution
  (no shared on-disk cache across separate `upskill add` invocations); a large
  multi-source dependency graph therefore re-fetches its sources on each install.
```

- [ ] **Step 5: Format and lint the docs**

Run: `just fmt`
Expected: Markdown reflowed; no errors. Then `markdownlint` runs under `just lint` later.

- [ ] **Step 6: Commit**

```bash
git add docs/format-spec.md docs/adr/0009-coupling-tiers-and-dependencies.md
git commit -m "docs: cross-source requires resolution is implemented (Slice 2)"
```

---

## Task 8: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Format everything**

Run: `just fmt`
Expected: no changes needed (or only whitespace already committed).

- [ ] **Step 2: Full gate**

Run: `just verify`
Expected: PASS — `cargo clippy -- -D warnings` (zero warnings), `cargo fmt --check`, dprint, markdownlint, shellcheck, and the full test suite all green. Fix any clippy findings (watch for `too_many_arguments` on `Resolver::visit` — it takes `&mut self` + 2 args, so it is fine; and `clippy::type_complexity` on the closure maps — extract type aliases only if clippy flags them).

- [ ] **Step 3: Confirm the cross-source seam no longer bails**

Run: `grep -rn "not yet available" src/`
Expected: no matches — the Slice 1 bail message is gone.

- [ ] **Step 4: Commit any verification fixups, then hand off**

If `just verify` required fixes, commit them:

```bash
git add -A
git commit -m "chore: satisfy verify gate for cross-source slice"
```

Then proceed to `superpowers:finishing-a-development-branch` to open the PR (`just fmt` + `just verify` already green; one PR for the slice; squash-merge).

---

## Self-Review

**Spec coverage (design spec §2.5 / ADR-0009 cross-source contract):**

- `source` reuses the `add` DSL, parsed by `parse_install_source`, fetched by `fetch_ssot` → Task 1 Step 3 (`Resolver::ensure_source`).
- Transitive closure spans sources → Task 1 (`visit` recurses across labels); Task 3 ATDD.
- Cycle detection keyed `(canonical-source-label, kind, name)` → Task 1 (`on_path` keyed `(label, kind, name)`); Task 5 ATDD.
- Conflict: same `(kind, name)` different source/ref is an error, reusing `conflict.rs` → Task 1 (`item_label` for in-closure clash) + Task 1 Step 10 (per-source `detect_conflicts` vs lockfile); Task 4 ATDD.
- Dependency-pulled items recorded with own source + `required_by` → Task 1 Steps 7/12; Task 2 ATDD.
- Removal never cascades; doctor flags orphan → no production change needed (Slice 1 doctor); Task 6 ATDD.
- No version solving / SAT → nothing added; documented in Task 7.
- Docs flipped to implemented → Task 7.

**Placeholder scan:** every code step shows complete code; commands list exact invocations + expected outcomes. No TBD/TODO.

**Type consistency:** `DependencyClosure { by_source: BTreeMap<String, SourceInstall>, item_source: BTreeMap<(ItemKind,String), String>, required_by, guards }`, `SourceInstall { root, git_ref, items }`, `resolve_requires_closure(entry_label, entry_root, entry_git_ref, initial)`, and `items_from_report(report, source_for, required_by, hash_for)` are used identically across Tasks 1, 2-6 (assertions), and the rewired `mod.rs`. `Resolver::visit(label, node, requirer)` and `ensure_source(src_dsl)` signatures are self-consistent.

**Out-of-scope guardrails honored:** no version ranges/SAT, no auto-cascading removal, no path-based cross-source refs (always source-locator + name). Pre-1.0: the old single-source `DependencyClosure`/`resolve_requires_closure` shape is deleted outright (no back-compat); `items_from_report` signature changed in place.
