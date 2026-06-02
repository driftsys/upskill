# Supporting Resources Copy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `upskill add` copy an item's supporting resource files (everything in the item directory except the entrypoint and per-client override files) into each client's rendered output, per format-spec §2.4 / §9-#6 (fixes #199).

**Architecture:** Enumerate non-entrypoint files per item; copy them into a per-item namespace directory in each client's output (`resource_base_path`). Directory-backed kinds (all skills, opencode rules) land resources beside their entrypoint with no body change. Flat kinds (Claude/Copilot rules, all agents) land resources in a sibling `<name>/` directory and have their entrypoint's relative resource links rewritten to prefix `<name>/`. Removal recomputes the same locations — no lockfile change. `--as` aliasing on resource-bearing items is guarded (deferred to debt).

**Tech Stack:** Rust 2024, `pulldown-cmark` 0.11 (link detection), `anyhow`, `assert_cmd` + `tempfile` (integration tests).

**Spec:** `docs/superpowers/specs/2026-06-02-supporting-resources-design.md`

---

## File structure

- `src/pipeline/discovery.rs` — add `iter_item_resources(dir)` (enumerate resources).
- `src/pipeline/output.rs` — add `is_dir_backed`, `resource_base_path`, `copy_item_resources`; extend `remove_item_outputs`.
- `src/generate/link_rewrite.rs` — **new module**: `rewrite_resource_links`.
- `src/generate/mod.rs` — register `pub mod link_rewrite;`.
- `src/pipeline/install.rs` — wire resource copy + rewrite into `install_items_of_kind`.
- `src/pipeline/mod.rs` — `--as` pre-flight guard in `install_with_lockfile`.
- `src/pipeline/lifecycle.rs` — route `remove` through the extended `remove_item_outputs`.
- `tests/cli_resources.rs` — **new** integration tests.

---

## Task 1: Enumerate item resources (`discovery.rs`)

**Files:**

- Modify: `src/pipeline/discovery.rs`
- Test: `src/pipeline/discovery.rs` (`#[cfg(test)]` module at bottom)

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block at the end of `src/pipeline/discovery.rs`:

```rust
    #[test]
    fn iter_item_resources_excludes_entrypoint_and_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(dir.join("RULE.md"), "x").unwrap();
        std::fs::write(dir.join("RULE.claude.md"), "x").unwrap(); // override
        std::fs::write(dir.join("SKILL.md"), "x").unwrap(); // co-located entrypoint
        std::fs::write(dir.join("notes.claude.md"), "x").unwrap(); // NOT an override
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/gate.sh"), "x").unwrap();

        let res = iter_item_resources(dir);
        assert_eq!(
            res,
            vec![
                std::path::PathBuf::from("notes.claude.md"),
                std::path::PathBuf::from("scripts/gate.sh"),
            ],
            "entrypoints and <KIND>.<client>.md overrides are excluded; \
             nested files and same-suffix non-overrides are kept"
        );
    }

    #[test]
    fn iter_item_resources_empty_when_only_entrypoint() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("SKILL.md"), "x").unwrap();
        assert!(iter_item_resources(tmp.path()).is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib iter_item_resources`
Expected: FAIL — `cannot find function iter_item_resources`.

- [ ] **Step 3: Implement `iter_item_resources`**

Add to `src/pipeline/discovery.rs` (after `iter_item_dirs`):

```rust
/// Relative paths (from `dir`) of every file under an item directory that
/// is neither an entrypoint (`SKILL.md`/`RULE.md`/`AGENT.md`) nor a
/// per-client override file (`<KIND>.<client>.md`, format-spec §2.3).
/// Recursive; sub-directory structure is preserved in the returned
/// relative paths. Sorted for deterministic output.
pub(super) fn iter_item_resources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_resource_files(dir, dir, &mut out);
    out.sort();
    out
}

fn collect_resource_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_resource_files(root, &path, out);
            continue;
        }
        // Entrypoints and override files only ever sit at the top level of
        // an item directory; a nested file with the same name is content.
        let top_level = path.parent() == Some(root);
        let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if top_level && is_entrypoint_or_override(fname) {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}

fn is_entrypoint_or_override(fname: &str) -> bool {
    matches!(fname, "SKILL.md" | "RULE.md" | "AGENT.md") || is_override_file(fname)
}

/// `<KIND>.<client>.md` where KIND ∈ {SKILL,RULE,AGENT} and client ∈
/// {claude,copilot,opencode}. Anchored to entrypoint stems so an
/// unrelated file like `notes.claude.md` is NOT treated as an override.
fn is_override_file(fname: &str) -> bool {
    let Some(stem) = fname.strip_suffix(".md") else {
        return false;
    };
    let mut parts = stem.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(kind), Some(client), None) => {
            matches!(kind, "SKILL" | "RULE" | "AGENT")
                && matches!(client, "claude" | "copilot" | "opencode")
        }
        _ => false,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib iter_item_resources`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add src/pipeline/discovery.rs
git commit -m "feat(pipeline): enumerate item supporting resources (#199)"
```

---

## Task 2: Resource output paths + copy (`output.rs`)

**Files:**

- Modify: `src/pipeline/output.rs`
- Test: `src/pipeline/output.rs` (`#[cfg(test)]` module)

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `src/pipeline/output.rs`:

```rust
    #[test]
    fn is_dir_backed_matches_layout() {
        // Skills are directory-backed on every client.
        for c in ALL_CLIENTS {
            assert!(is_dir_backed(ItemKind::Skill, c));
        }
        // Rules: only opencode is directory-backed.
        assert!(is_dir_backed(ItemKind::Rule, Client::OpenCode));
        assert!(!is_dir_backed(ItemKind::Rule, Client::Claude));
        assert!(!is_dir_backed(ItemKind::Rule, Client::Copilot));
        // Agents are flat on every client.
        for c in ALL_CLIENTS {
            assert!(!is_dir_backed(ItemKind::Agent, c));
        }
    }

    #[test]
    fn resource_base_dir_backed_is_entrypoint_dir() {
        assert_eq!(
            resource_base_path(ItemKind::Skill, Client::Claude, "x"),
            PathBuf::from(".claude/skills/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::OpenCode, "x"),
            PathBuf::from(".agents/rules/x")
        );
    }

    #[test]
    fn resource_base_flat_is_sibling_namespace_dir() {
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Claude, "x"),
            PathBuf::from(".claude/rules/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Rule, Client::Copilot, "x"),
            PathBuf::from(".github/instructions/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Agent, Client::Claude, "x"),
            PathBuf::from(".claude/agents/x")
        );
        assert_eq!(
            resource_base_path(ItemKind::Agent, Client::OpenCode, "x"),
            PathBuf::from(".opencode/agents/x")
        );
    }

    #[test]
    fn copy_item_resources_preserves_tree() {
        let src = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(src.path().join("scripts")).unwrap();
        std::fs::write(src.path().join("scripts/gate.sh"), b"#!/bin/sh\n").unwrap();
        let target = tempfile::tempdir().unwrap();

        copy_item_resources(
            target.path(),
            src.path(),
            ItemKind::Rule,
            Client::Claude,
            "demo",
            &[PathBuf::from("scripts/gate.sh")],
        )
        .unwrap();

        let dest = target.path().join(".claude/rules/demo/scripts/gate.sh");
        assert!(dest.is_file());
        assert_eq!(std::fs::read(&dest).unwrap(), b"#!/bin/sh\n");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib --package upskill output::tests`
Expected: FAIL — `is_dir_backed` / `resource_base_path` / `copy_item_resources` not found.

- [ ] **Step 3: Implement the helpers**

Add to `src/pipeline/output.rs`. First extend the imports at the top:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use super::{ALL_CLIENTS, ItemKind};
use crate::generate::Client;
```

Then add the functions (after `output_path`):

```rust
/// True when an item's entrypoint for `client` lives inside its own
/// `<name>/` directory (all skills; opencode rules). Such items hold
/// resources beside the entrypoint and need no link rewrite. Flat kinds
/// (Claude/Copilot rules, all agents) return false.
pub(super) fn is_dir_backed(kind: ItemKind, client: Client) -> bool {
    matches!(
        (kind, client),
        (ItemKind::Skill, _) | (ItemKind::Rule, Client::OpenCode)
    )
}

/// Directory (relative to the install target) under which an item's
/// supporting resources are copied for `client`. Directory-backed items
/// use the entrypoint's own `<name>/` directory; flat items use a sibling
/// `<name>/` namespace directory next to the flat entrypoint file.
pub(super) fn resource_base_path(kind: ItemKind, client: Client, name: &str) -> PathBuf {
    let entry = output_path(kind, client, name);
    let parent = entry
        .parent()
        .expect("output path always has a parent directory");
    if is_dir_backed(kind, client) {
        parent.to_path_buf()
    } else {
        parent.join(name)
    }
}

/// Copy each resource (a path relative to the SSOT item directory
/// `source_dir`) into the client's [`resource_base_path`], preserving
/// sub-structure. `fs::copy` preserves the file mode on Unix, so an
/// executable script stays executable.
pub(super) fn copy_item_resources(
    target: &Path,
    source_dir: &Path,
    kind: ItemKind,
    client: Client,
    name: &str,
    resources: &[PathBuf],
) -> Result<()> {
    if resources.is_empty() {
        return Ok(());
    }
    let base = resource_base_path(kind, client, name);
    for rel in resources {
        let from = source_dir.join(rel);
        let to = target.join(&base).join(rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }
        fs::copy(&from, &to)
            .with_context(|| format!("copy {} to {}", from.display(), to.display()))?;
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib --package upskill output::tests`
Expected: PASS (existing + 4 new tests).

- [ ] **Step 5: Commit**

```bash
git add src/pipeline/output.rs
git commit -m "feat(pipeline): resource base paths and copy helper (#199)"
```

---

## Task 3: Link rewrite module (`generate/link_rewrite.rs`)

**Files:**

- Create: `src/generate/link_rewrite.rs`
- Modify: `src/generate/mod.rs` (register module)
- Test: `src/generate/link_rewrite.rs` (`#[cfg(test)]` module)

- [ ] **Step 1: Register the module**

In `src/generate/mod.rs`, add to the module declarations near the top (alongside `pub mod directives;` etc.):

```rust
pub mod link_rewrite;
```

- [ ] **Step 2: Write the failing table test**

Create `src/generate/link_rewrite.rs` with this content (impl stub returns input unchanged so the file compiles and the test fails on assertions):

````rust
//! Rewrite relative resource links in a rendered flat-kind body so they
//! address the per-item `<name>/` namespace directory.
//!
//! Used only for flat outputs (Claude/Copilot rules, all agents).
//! Directory-backed kinds (all skills, opencode rules) render the
//! entrypoint inside the resource directory, so their relative links
//! already resolve and this module is not invoked for them.

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn rewrite_resource_links(
    _rendered: &str,
    _name: &str,
    _copied: &HashSet<PathBuf>,
) -> Result<String> {
    todo!("implemented in Step 4")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copied(paths: &[&str]) -> HashSet<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    fn run(body: &str, paths: &[&str]) -> String {
        rewrite_resource_links(body, "demo", &copied(paths)).unwrap()
    }

    #[test]
    fn rewrites_inline_link_with_dot_slash() {
        assert_eq!(
            run("See [g](./scripts/gate.sh).", &["scripts/gate.sh"]),
            "See [g](./demo/scripts/gate.sh)."
        );
    }

    #[test]
    fn rewrites_inline_link_without_dot_slash() {
        assert_eq!(
            run("See [g](scripts/gate.sh).", &["scripts/gate.sh"]),
            "See [g](demo/scripts/gate.sh)."
        );
    }

    #[test]
    fn rewrites_image() {
        assert_eq!(
            run("![logo](./assets/logo.png)", &["assets/logo.png"]),
            "![logo](./demo/assets/logo.png)"
        );
    }

    #[test]
    fn rewrites_reference_definition() {
        let body = "Use [the gate][g].\n\n[g]: ./scripts/gate.sh\n";
        assert_eq!(
            run(body, &["scripts/gate.sh"]),
            "Use [the gate][g].\n\n[g]: ./demo/scripts/gate.sh\n"
        );
    }

    #[test]
    fn preserves_title_and_fragment() {
        assert_eq!(
            run(
                "[p](./refs/patterns.md#sec \"Patterns\")",
                &["refs/patterns.md"]
            ),
            "[p](./demo/refs/patterns.md#sec \"Patterns\")"
        );
    }

    #[test]
    fn leaves_urls_untouched() {
        let body = "[site](https://example.com/scripts/gate.sh) and [m](mailto:x@y.z)";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn leaves_absolute_path_untouched() {
        let body = "[g](/usr/local/bin/gate.sh)";
        assert_eq!(run(body, &["usr/local/bin/gate.sh"]), body);
    }

    #[test]
    fn leaves_bare_fragment_untouched() {
        let body = "[top](#section)";
        assert_eq!(run(body, &["section"]), body);
    }

    #[test]
    fn leaves_parent_escape_untouched() {
        let body = "[x](../other/gate.sh)";
        assert_eq!(run(body, &["other/gate.sh"]), body);
    }

    #[test]
    fn leaves_uncopied_target_untouched() {
        // Link resolves to a relative path, but it was not among the copied
        // resources (e.g. points at a sibling item) — do not rewrite.
        let body = "[x](./scripts/missing.sh)";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn leaves_inline_code_untouched() {
        let body = "Run `./scripts/gate.sh` manually.";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn leaves_fenced_code_untouched() {
        let body = "```sh\n./scripts/gate.sh\n```\n";
        assert_eq!(run(body, &["scripts/gate.sh"]), body);
    }

    #[test]
    fn empty_copied_set_is_identity() {
        let body = "[g](./scripts/gate.sh)";
        assert_eq!(rewrite_resource_links(body, "demo", &copied(&[])).unwrap(), body);
    }

    #[test]
    fn idempotent() {
        let once = run("[g](./scripts/gate.sh)", &["scripts/gate.sh"]);
        let twice = run(&once, &["scripts/gate.sh"]);
        assert_eq!(once, twice);
    }
}
````

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --lib --package upskill link_rewrite`
Expected: FAIL — every test panics on `todo!`.

- [ ] **Step 4: Implement `rewrite_resource_links`**

Replace the stub `rewrite_resource_links` (and its `use` lines) in `src/generate/link_rewrite.rs` with:

```rust
use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::HashSet;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

/// Prefix every relative link/image destination that resolves to one of
/// `copied` (paths relative to the item directory) with `name/`, so the
/// destination addresses the namespaced resource directory. Destinations
/// that are URLs, absolute paths, bare fragments, `../`-escaping, or not
/// among `copied` are left unchanged — as is any link-like text inside
/// inline-code spans or fenced code blocks (pulldown-cmark reports those
/// as non-link events, so they never enter `edits`).
pub fn rewrite_resource_links(
    rendered: &str,
    name: &str,
    copied: &HashSet<PathBuf>,
) -> Result<String> {
    if copied.is_empty() {
        return Ok(rendered.to_string());
    }

    // (absolute byte range of the destination token, replacement string).
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();

    // Reference definitions (`[id]: dest "title"`) are consumed during the
    // initial scan and are not emitted as link events, so collect them
    // separately. `RefDef.span` covers the whole definition line.
    let ref_parser = Parser::new_ext(rendered, Options::empty());
    for (_label, def) in ref_parser.reference_definitions().iter() {
        if let Some(new_dest) = rewritten_dest(&def.dest, name, copied)
            && let Some(off) = rendered[def.span.clone()].rfind(def.dest.as_ref())
        {
            let abs = def.span.start + off;
            edits.push((abs..abs + def.dest.len(), new_dest));
        }
    }

    // Inline links and images.
    let parser = Parser::new_ext(rendered, Options::empty());
    for (event, range) in parser.into_offset_iter() {
        let dest = match &event {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => dest_url.to_string(),
            _ => continue,
        };
        if let Some(new_dest) = rewritten_dest(&dest, name, copied)
            && let Some(off) = rendered[range.clone()].rfind(&dest)
        {
            let abs = range.start + off;
            edits.push((abs..abs + dest.len(), new_dest));
        }
    }

    // Apply right-to-left so earlier ranges stay valid.
    edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));
    let mut out = rendered.to_string();
    for (range, replacement) in edits {
        out.replace_range(range, &replacement);
    }
    Ok(out)
}

/// Returns `Some(new_dest)` when `dest` is a relative path (optionally
/// with a `#fragment`) that resolves to a copied resource, else `None`.
fn rewritten_dest(dest: &str, name: &str, copied: &HashSet<PathBuf>) -> Option<String> {
    let (path_part, frag) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (dest, None),
    };
    if path_part.is_empty() || has_scheme(path_part) || path_part.starts_with('/') {
        return None;
    }
    let normalized = path_part.strip_prefix("./").unwrap_or(path_part);
    if Path::new(normalized)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }
    if !copied.contains(&PathBuf::from(normalized)) {
        return None;
    }
    let prefixed = if path_part.starts_with("./") {
        format!("./{name}/{normalized}")
    } else {
        format!("{name}/{normalized}")
    };
    Some(match frag {
        Some(f) => format!("{prefixed}#{f}"),
        None => prefixed,
    })
}

/// True for `scheme:` URLs (`https:`, `mailto:`, …). A relative file path
/// has no leading `scheme:` segment.
fn has_scheme(s: &str) -> bool {
    match s.find(':') {
        Some(idx) if idx > 0 => s[..idx]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')),
        _ => false,
    }
}
```

> Note: field names `def.dest` / `def.span` follow pulldown-cmark 0.11's `RefDef`. If a minor API mismatch surfaces, adjust to the crate's actual field names — the logic is unchanged.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --lib --package upskill link_rewrite`
Expected: PASS (14 tests).

- [ ] **Step 6: Commit**

```bash
git add src/generate/link_rewrite.rs src/generate/mod.rs
git commit -m "feat(generate): rewrite relative resource links for flat kinds (#199)"
```

---

## Task 4: Copy resources + rewrite during install (`install.rs`)

**Files:**

- Modify: `src/pipeline/install.rs:352-464` (`install_items_of_kind`)
- Test: covered end-to-end by Task 7 (no isolated unit test for the wiring).

- [ ] **Step 1: Extend the imports**

In `src/pipeline/install.rs`, update the `use super::output::...` line and discovery import:

```rust
use super::discovery::{
    detect_item_entrypoint, find_bundle_by_name, find_registry_root, has_matching_items,
    is_bundle_file, iter_item_dirs, iter_item_resources,
};
use super::output::{
    copy_item_resources, is_dir_backed, output_path, remove_item_outputs, write_output,
};
```

- [ ] **Step 2: Wire enumeration, rewrite, and copy into the write loop**

In `install_items_of_kind`, replace the block from `let source_hash = hash_item_dir(&dir);` through the `for (client, rendered) in &renders { ... }` loop (currently `src/pipeline/install.rs:423-440`) with:

```rust
        let source_hash = hash_item_dir(&dir);
        let resources = iter_item_resources(&dir);
        let copied: std::collections::HashSet<std::path::PathBuf> =
            resources.iter().cloned().collect();

        // Clean existing output directories for this specific item before
        // writing new outputs. This removes stale sibling files while keeping
        // other items' outputs intact if generation fails later.
        remove_item_outputs(target, kind, &name);

        for (client, rendered) in &renders {
            // Flat-kind entrypoints (Claude/Copilot rules, all agents) live
            // beside a `<name>/` resource directory; rewrite their relative
            // resource links to point into it. Directory-backed kinds need
            // no rewrite (entrypoint and resources share the directory).
            let body = if !copied.is_empty() && !is_dir_backed(kind, *client) {
                crate::generate::link_rewrite::rewrite_resource_links(rendered, &name, &copied)
                    .with_context(|| format!("rewrite resource links for {name} ({client:?})"))?
            } else {
                rendered.clone()
            };
            let rel = output_path(kind, *client, &name);
            write_output(target, &rel, &body)?;
            copy_item_resources(target, &dir, kind, *client, &name, &resources)?;
            report.items.push(InstalledItem {
                kind,
                name: name.clone(),
                client: *client,
                output_path: rel,
                source_hash: source_hash.clone(),
            });
        }
```

- [ ] **Step 3: Build to verify it compiles**

Run: `just assemble`
Expected: compiles with zero warnings.

- [ ] **Step 4: Run the existing pipeline + generate suites (no regressions)**

Run: `cargo test --test pipeline_local --test pipeline_source --test generate_skills --test generate_rules --test generate_agents`
Expected: PASS (resourceless fixtures are unaffected — `copied` is empty, so no rewrite and no copy).

- [ ] **Step 5: Commit**

```bash
git add src/pipeline/install.rs
git commit -m "feat(pipeline): copy item resources and rewrite flat-kind links on install (#199)"
```

---

## Task 5: Remove resource trees on `remove`/`update` (`output.rs`, `lifecycle.rs`)

**Files:**

- Modify: `src/pipeline/output.rs` (`remove_item_outputs`)
- Modify: `src/pipeline/lifecycle.rs:64-85` (`remove` loop)
- Test: `src/pipeline/output.rs` (`#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

Add to `src/pipeline/output.rs` tests:

```rust
    #[test]
    fn remove_item_outputs_deletes_flat_kind_resource_dir() {
        let target = tempfile::tempdir().unwrap();
        // Flat-kind (Claude rule) entrypoint + sibling resource namespace dir.
        std::fs::create_dir_all(target.path().join(".claude/rules/demo/scripts")).unwrap();
        std::fs::write(target.path().join(".claude/rules/demo.md"), "x").unwrap();
        std::fs::write(
            target.path().join(".claude/rules/demo/scripts/gate.sh"),
            "x",
        )
        .unwrap();

        remove_item_outputs(target.path(), ItemKind::Rule, "demo");

        assert!(!target.path().join(".claude/rules/demo.md").exists());
        assert!(
            !target.path().join(".claude/rules/demo").exists(),
            "the sibling resource namespace dir must be removed too"
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib --package upskill remove_item_outputs_deletes_flat_kind_resource_dir`
Expected: FAIL — `.claude/rules/demo` still exists.

- [ ] **Step 3: Extend `remove_item_outputs`**

In `src/pipeline/output.rs`, replace the body of `remove_item_outputs` with:

```rust
pub(super) fn remove_item_outputs(target: &Path, kind: ItemKind, name: &str) {
    for client in ALL_CLIENTS {
        let rel = output_path(kind, client, name);
        let full = target.join(&rel);
        if full.exists() {
            let _ = fs::remove_file(&full);
        }
        // Directory-backed items: remove the item's own `<name>/` directory
        // (which holds both the entrypoint and its resources).
        if let Some(parent) = full.parent()
            && parent
                .file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f == name)
            && parent.is_dir()
        {
            let _ = fs::remove_dir_all(parent);
        }
        // Flat kinds: remove the sibling `<name>/` resource namespace dir.
        if !is_dir_backed(kind, client) {
            let res = target.join(resource_base_path(kind, client, name));
            if res.is_dir() {
                let _ = fs::remove_dir_all(&res);
            }
        }
    }
}
```

- [ ] **Step 4: Route `lifecycle::remove` through `remove_item_outputs`**

In `src/pipeline/lifecycle.rs`, replace the per-entry loop body (currently the `for client in ALL_CLIENTS { ... }` inner block plus `lock.remove(...)` at `src/pipeline/lifecycle.rs:65-85`) with:

```rust
let mut report = RemoveReport::default();
for entry in &to_remove {
    let kind = entry.kind;
    // Record which entrypoint outputs existed (for the user-facing
    // report) before deleting everything for this item.
    let mut deleted_files = Vec::new();
    for client in ALL_CLIENTS {
        let rel = output_path(kind, client, &entry.name);
        if target.join(&rel).exists() {
            deleted_files.push(rel);
        }
    }
    remove_item_outputs(target, kind, &entry.name);
    lock.remove(entry.kind, &entry.name);
    report.items.push(RemovedItem {
        kind,
        name: entry.name.clone(),
        deleted_files,
    });
}
```

- [ ] **Step 5: Run the unit test and the lockfile suite**

Run: `cargo test --lib --package upskill remove_item_outputs_deletes_flat_kind_resource_dir && cargo test --test pipeline_lockfile`
Expected: PASS (new unit test passes; existing remove/lockfile tests still pass — entrypoint-level `deleted_files` reporting is preserved).

- [ ] **Step 6: Commit**

```bash
git add src/pipeline/output.rs src/pipeline/lifecycle.rs
git commit -m "fix(pipeline): delete resource trees on remove and update-orphan (#199)"
```

---

## Task 6: Guard `--as` aliasing on resource-bearing items (`mod.rs`)

**Files:**

- Modify: `src/pipeline/mod.rs` (`install_with_lockfile`, pre-flight section ~line 167)
- Test: covered by integration Task 7 (scenario 10).

- [ ] **Step 1: Add the pre-flight guard**

In `src/pipeline/mod.rs`, inside `install_with_lockfile`, immediately after the `// -- Validate bare --as with multi-item source --` block (before `// -- Conflict detection`), insert:

```rust
// -- Guard: aliasing an item that ships supporting resources is not yet
// supported (the resource namespace dir and rewritten `<name>/` link
// prefix would not be relocated to the alias). Tracked as debt; abort
// before writing rather than emit broken output.
if !options.aliases.is_empty() {
    for (name, dir) in discovery::iter_item_dirs(&local_source)? {
        let aliased = options
            .aliases
            .iter()
            .any(|(from, _)| from.is_empty() || *from == name);
        if aliased && !discovery::iter_item_resources(&dir).is_empty() {
            anyhow::bail!(
                "aliasing items with supporting resources is not yet supported \
                 ('{name}' ships resource files). Install it without --as. \
                 (See format-spec §2.4.)"
            );
        }
    }
}
```

- [ ] **Step 2: Build to verify it compiles**

Run: `just assemble`
Expected: compiles (note `discovery::iter_item_dirs` and `iter_item_resources` are `pub(super)`, visible from the parent `mod.rs`).

- [ ] **Step 3: Commit**

```bash
git add src/pipeline/mod.rs
git commit -m "feat(pipeline): guard --as against resource-bearing items (#199)"
```

---

## Task 7: End-to-end integration tests (`tests/cli_resources.rs`)

**Files:**

- Create: `tests/cli_resources.rs`

These build the SSOT source inline in a tempdir (matching the existing inline-tempdir test style) so they do not perturb the shared `tests/fixtures/items/` golden corpus.

- [ ] **Step 1: Write the integration tests**

Create `tests/cli_resources.rs`:

```rust
//! Integration coverage for format-spec §2.4 supporting-resource copying
//! (#199): resources travel with each rendered entrypoint; flat-kind
//! bodies are link-rewritten; remove/update reconcile; `--as` is guarded.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

fn upskill() -> Command {
    Command::cargo_bin("upskill").unwrap()
}

/// Write `SKILL.md`/`RULE.md`/`AGENT.md` + a script + a reference file into
/// `<root>/<name>/`. `entry` is the entrypoint filename.
fn write_item(root: &Path, name: &str, entry: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("references")).unwrap();
    let fm = format!("---\nschema: 1\nname: {name}\ndescription: demo {name}\n---\n\n{body}");
    fs::write(dir.join(entry), fm).unwrap();
    fs::write(dir.join("scripts/gate.sh"), "#!/bin/sh\necho gate\n").unwrap();
    fs::write(dir.join("references/notes.md"), "# Notes\n\nstuff\n").unwrap();
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap()
}

#[test]
fn skill_resources_copied_for_all_clients_no_rewrite() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(
        src.path(),
        "demo-skill",
        "SKILL.md",
        "## Demo\n\nRun [gate](./scripts/gate.sh). See [notes](./references/notes.md).\n",
    );

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();

    for base in [".claude/skills", ".github/skills", ".agents/skills"] {
        let dir = proj.path().join(base).join("demo-skill");
        assert!(dir.join("scripts/gate.sh").is_file(), "{base}: script copied");
        assert!(dir.join("references/notes.md").is_file(), "{base}: ref copied");
        // Directory-backed: body link is unchanged.
        assert!(
            read(&dir.join("SKILL.md")).contains("(./scripts/gate.sh)"),
            "{base}: skill body link must NOT be rewritten"
        );
    }
}

#[test]
fn rule_resources_namespaced_and_rewritten_per_client() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(
        src.path(),
        "demo-rule",
        "RULE.md",
        "## Gate\n\nRun [the gate](./scripts/gate.sh) in CI.\n",
    );

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();

    // Claude: flat entrypoint + sibling namespace dir + rewritten link.
    let claude_md = proj.path().join(".claude/rules/demo-rule.md");
    assert!(claude_md.is_file());
    assert!(
        proj.path().join(".claude/rules/demo-rule/scripts/gate.sh").is_file(),
        "claude resource in sibling namespace dir"
    );
    assert!(
        read(&claude_md).contains("(./demo-rule/scripts/gate.sh)"),
        "claude rule link must be rewritten to the namespace dir"
    );

    // Copilot: same shape under .github/instructions/.
    assert!(
        proj.path()
            .join(".github/instructions/demo-rule/scripts/gate.sh")
            .is_file()
    );
    assert!(
        read(&proj.path().join(".github/instructions/demo-rule.instructions.md"))
            .contains("(./demo-rule/scripts/gate.sh)")
    );

    // opencode: directory-backed — resources beside RULE.md, link unchanged.
    let oc = proj.path().join(".agents/rules/demo-rule");
    assert!(oc.join("scripts/gate.sh").is_file());
    assert!(
        read(&oc.join("RULE.md")).contains("(./scripts/gate.sh)"),
        "opencode rule link must NOT be rewritten"
    );
}

#[test]
fn agent_resources_namespaced_and_rewritten_all_clients() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(
        src.path(),
        "demo-agent",
        "AGENT.md",
        "## Agent\n\nUses [gate](./scripts/gate.sh).\n",
    );

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();

    for (entry, dir) in [
        (".claude/agents/demo-agent.md", ".claude/agents/demo-agent"),
        (
            ".github/agents/demo-agent.agent.md",
            ".github/agents/demo-agent",
        ),
        (".opencode/agents/demo-agent.md", ".opencode/agents/demo-agent"),
    ] {
        assert!(
            proj.path().join(dir).join("scripts/gate.sh").is_file(),
            "{dir}: resource copied"
        );
        assert!(
            read(&proj.path().join(entry)).contains("(./demo-agent/scripts/gate.sh)"),
            "{entry}: agent link rewritten"
        );
    }
}

#[test]
fn audience_scopes_resource_copy() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let dir = src.path().join("only-claude");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: only-claude\ndescription: d\naudience:\n  - claude\n---\n\n## X\n\n[g](./scripts/gate.sh)\n",
    )
    .unwrap();
    fs::write(dir.join("scripts/gate.sh"), "x").unwrap();

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();

    assert!(proj.path().join(".claude/skills/only-claude/scripts/gate.sh").is_file());
    assert!(!proj.path().join(".github/skills/only-claude").exists());
    assert!(!proj.path().join(".agents/skills/only-claude").exists());
}

#[test]
fn remove_deletes_resource_tree() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(src.path(), "demo-rule", "RULE.md", "## G\n\n[g](./scripts/gate.sh)\n");

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();
    assert!(proj.path().join(".claude/rules/demo-rule/scripts/gate.sh").is_file());

    upskill()
        .current_dir(proj.path())
        .args(["remove", "demo-rule"])
        .assert()
        .success();

    assert!(!proj.path().join(".claude/rules/demo-rule.md").exists());
    assert!(
        !proj.path().join(".claude/rules/demo-rule").exists(),
        "resource namespace dir removed"
    );
}

#[test]
fn readd_is_idempotent() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(src.path(), "demo-skill", "SKILL.md", "## X\n\n[g](./scripts/gate.sh)\n");

    let run = || {
        upskill()
            .current_dir(proj.path())
            .args(["add", src.path().to_str().unwrap(), "--force"])
            .assert()
            .success();
    };
    run();
    let first = read(&proj.path().join(".claude/skills/demo-skill/SKILL.md"));
    run();
    let second = read(&proj.path().join(".claude/skills/demo-skill/SKILL.md"));
    assert_eq!(first, second, "re-add must be byte-identical");
}

#[test]
fn update_removes_stale_resource_after_source_deletes_it() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(src.path(), "demo-skill", "SKILL.md", "## X\n\n[g](./scripts/gate.sh)\n");

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();
    let copied = proj.path().join(".claude/skills/demo-skill/references/notes.md");
    assert!(copied.is_file());

    // Delete a resource from the SSOT source, then update.
    fs::remove_file(src.path().join("demo-skill/references/notes.md")).unwrap();
    upskill()
        .current_dir(proj.path())
        .args(["update"])
        .assert()
        .success();

    assert!(
        !copied.exists(),
        "stale resource must be cleaned (remove_item_outputs runs before re-copy)"
    );
}

#[test]
fn alias_on_resource_item_is_rejected() {
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(src.path(), "demo-skill", "SKILL.md", "## X\n\n[g](./scripts/gate.sh)\n");

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap(), "--as", "renamed"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "aliasing items with supporting resources is not yet supported",
        ));

    // Nothing written.
    assert!(!proj.path().join(".claude/skills/renamed").exists());
    assert!(!proj.path().join(".claude/skills/demo-skill").exists());
}
```

> The `predicates` crate is already a dev-dependency via `assert_cmd`; if a direct import is needed, add `predicates` to `[dev-dependencies]` (check `Cargo.toml` first — `cli_add.rs` already uses it).

- [ ] **Step 2: Run the integration suite**

Run: `cargo test --test cli_resources`
Expected: PASS (8 tests).

- [ ] **Step 3: Verify the re-add idempotence holds under dprint (§7.4)**

The `readd_is_idempotent` test already asserts byte-identical re-render. Additionally confirm a rewritten flat-kind body is dprint-clean by running the formatter check:

Run: `cargo test --test cli_resources && just lint`
Expected: PASS — no formatting drift on generated output.

- [ ] **Step 4: Commit**

```bash
git add tests/cli_resources.rs
git commit -m "test(cli): end-to-end supporting-resource copy coverage (#199)"
```

---

## Task 8: Multi-kind co-location + bundle install coverage

**Files:**

- Modify: `tests/cli_resources.rs`

- [ ] **Step 1: Write the co-location and bundle tests**

Append to `tests/cli_resources.rs`:

```rust
#[test]
fn colocated_kinds_share_resources() {
    // One directory holding SKILL.md + AGENT.md (same name), sharing
    // references/. Each emitted entrypoint must get the resource.
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    let dir = src.path().join("paired");
    fs::create_dir_all(dir.join("references")).unwrap();
    for entry in ["SKILL.md", "AGENT.md"] {
        fs::write(
            dir.join(entry),
            "---\nschema: 1\nname: paired\ndescription: d\n---\n\n## P\n\n[n](./references/notes.md)\n",
        )
        .unwrap();
    }
    fs::write(dir.join("references/notes.md"), "# n\n").unwrap();

    upskill()
        .current_dir(proj.path())
        .args(["add", src.path().to_str().unwrap()])
        .assert()
        .success();

    // Skill (dir-backed) gets it beside SKILL.md.
    assert!(proj.path().join(".claude/skills/paired/references/notes.md").is_file());
    // Agent (flat) gets it in the namespace dir.
    assert!(proj.path().join(".claude/agents/paired/references/notes.md").is_file());
}

#[test]
fn bundle_install_copies_resources() {
    // Registry with one resource-bearing skill and a bundle that names it.
    let src = tempfile::tempdir().unwrap();
    let proj = tempfile::tempdir().unwrap();
    write_item(src.path(), "demo-skill", "SKILL.md", "## X\n\n[g](./scripts/gate.sh)\n");
    fs::write(
        src.path().join("demo.bundle.yaml"),
        "schema: 1\nname: demo\ndescription: d\nitems:\n  skills:\n    - demo-skill\n",
    )
    .unwrap();

    upskill()
        .current_dir(proj.path())
        .args([
            "add",
            &format!("{}:demo.bundle.yaml", src.path().to_str().unwrap()),
        ])
        .assert()
        .success();

    assert!(
        proj.path().join(".claude/skills/demo-skill/scripts/gate.sh").is_file(),
        "bundle-installed item must carry its resources"
    );
}
```

> If the local bundle source syntax differs, mirror the form used in `tests/pipeline_source.rs` for a local `:bundle.yaml` source.

- [ ] **Step 2: Run the suite**

Run: `cargo test --test cli_resources`
Expected: PASS (10 tests total).

- [ ] **Step 3: Commit**

```bash
git add tests/cli_resources.rs
git commit -m "test(cli): multi-kind and bundle resource coverage (#199)"
```

---

## Task 9: Docs, debt issue, and final verification

**Files:**

- Modify: `docs/commands.md` (or `docs/recipes.md`) — note resource copying.

- [ ] **Step 1: Document the behavior**

Add a short note to `docs/commands.md` under `upskill add` (match the surrounding prose style):

```markdown
Supporting files in an item directory (anything besides the entrypoint and
per-client override files — e.g. `scripts/`, `references/`, `assets/`) are
copied into each client's output alongside the rendered item. For rules and
agents that render to a flat file (Claude Code, GitHub Copilot), resources go
into a sibling `<name>/` directory and the body's relative links are rewritten
to match. See [format-spec §2.4](./format-spec.md).
```

- [ ] **Step 2: Open the alias debt issue**

```bash
gh issue create \
  --title "debt: support --as aliasing for items with supporting resources" \
  --label debt --label K1 --label S \
  --body "Epic: #176

\`upskill add --as <alias>\` currently aborts when the target item ships
supporting resources (#199): the resource namespace directory and the
rewritten \`<name>/\` flat-kind link prefix are not relocated to the alias.

Implement relocation: move \`resource_base_path(kind, client, <orig>)\` to
\`<alias>\` and re-prefix flat-kind entrypoint links from \`<orig>/\` to
\`<alias>/\`, then drop the pre-flight guard in
\`src/pipeline/mod.rs::install_with_lockfile\`.

Add coverage in \`tests/cli_resources.rs\`."
```

Record the returned issue number and update the guard message in `src/pipeline/mod.rs` to reference it (`(see #<n>)`), then re-commit that one line.

- [ ] **Step 3: Format, then run the full gate**

Run: `just fmt && just verify`
Expected: PASS — all tests, clippy with `-D warnings`, fmt check, docs tooling clean.

- [ ] **Step 4: Commit any formatting changes and open the PR**

```bash
git add -A
git commit -m "docs(commands): document supporting-resource copying (#199)"
git push -u origin fix/199-supporting-resources
gh pr create --base main \
  --title "fix(add): copy item supporting resources into rendered output (#199)" \
  --body "$(cat <<'EOF'
Closes #199.

Copies an item's supporting files (everything except the entrypoint and
per-client override files) into each client's rendered output, per
format-spec §2.4 / §9-#6.

- Directory-backed kinds (all skills, opencode rules): resources land beside
  the entrypoint; bodies unchanged.
- Flat kinds (Claude/Copilot rules, all agents): resources land in a sibling
  `<name>/` namespace dir and relative resource links are rewritten to prefix
  `<name>/` (pulldown-cmark; code spans/fences never touched).
- `remove`/`update` reconcile resource trees (recomputed from kind/client/name
  — no lockfile change).
- `--as` on a resource-bearing item is guarded; relocation tracked as debt.

Design: docs/superpowers/specs/2026-06-02-supporting-resources-design.md

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Self-review

- **Spec coverage:** placement table → Tasks 2/4; rewrite (all cases) → Task 3; copy fidelity / exec bit → Task 2 (`fs::copy`) asserted in Task 7; exclusions → Task 1; removal → Task 5; update reconciliation → Task 7; alias guard → Tasks 6/7; collisions (none by construction) → no task needed; multi-kind + bundle + audience + idempotency → Tasks 7/8; §7.4 → Task 7 Step 3.
- **Type consistency:** `rewrite_resource_links(&str, &str, &HashSet<PathBuf>) -> Result<String>`, `is_dir_backed(ItemKind, Client) -> bool`, `resource_base_path(ItemKind, Client, &str) -> PathBuf`, `copy_item_resources(&Path, &Path, ItemKind, Client, &str, &[PathBuf]) -> Result<()>`, `iter_item_resources(&Path) -> Vec<PathBuf>` — used identically across tasks.
- **Placeholders:** none — every code step is complete. The two `>` notes (pulldown `RefDef` field names; local bundle-source syntax) are verification reminders, not missing code.
