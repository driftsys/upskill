# Coupling Redesign — Slice 1 (same-source) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reposition co-location as the "symmetric unit" primitive and add the directed-dependency tier: optional/relaxed item naming, `ignore` copy-scoping, and same-source `requires` auto-install — plus the lockfile/doctor bookkeeping that makes co-located units removable and orphaned dependencies visible.

**Architecture:** SSOT models gain two stripped-from-output fields (`requires`, `ignore`) and an optional `name`. Item _identity_ becomes `(kind, effective-name)` where the effective name is the frontmatter `name` for rules/agents (may diverge from the folder) or the folder name for skills (Agent Skills standard mandate) and as a fallback. The install pipeline resolves the same-source `requires` transitive closure before conflict detection, copies resources minus `ignore` globs, and records co-location grouping + `required_by` provenance in the lockfile.

**Tech Stack:** Rust (edition 2024, MSRV 1.85), `serde` + `serde_yaml_ng`, `anyhow`, `assert_cmd` + `tempfile` for integration tests. No new dependencies (the `ignore` glob matcher is hand-rolled to protect the ~3 MB binary target).

**Reference spec:** [docs/superpowers/specs/2026-06-03-coupling-redesign-design.md](../specs/2026-06-03-coupling-redesign-design.md)

**Out of scope (Slice 2, separate plan):** cross-source `requires` resolution (`{ name, source }` fetched via `fetch_ssot`), cross-source cycle/conflict keyed by canonical source label. Slice 1 parses the `{ name, source }` form but errors clearly when asked to resolve one.

---

## Conventions for every task

- **Worktree:** all work happens in `.claude/worktrees/feat-coupling-redesign/` (branch `feat/coupling-redesign`, already bootstrapped, off `origin/main`). Use full worktree-prefixed absolute paths for every Write/Edit.
- **TDD:** write the failing test, run it red, implement, run it green, commit. One Conventional Commit per task (or per sub-behavior).
- **Test isolation (issue #193 — has bitten twice):** integration tests MUST use `tests/common::upskill_cmd(fake_home)` and create a `.git` marker dir in the project dir, or `upskill add` defaults to global `$HOME` scope and pollutes the real `~`. Inspect `tests/common/mod.rs` before writing the first integration test and follow the existing pattern.
- **Before the PR:** `just fmt` then `just verify`. Squash-merge.
- **Pre-1.0:** no back-compat shims. Lockfile field additions are additive (`#[serde(default)]`), no schema bump.

---

## File structure

**Docs (Task 1):**

- Create: `docs/adr/0009-coupling-tiers-and-dependencies.md`
- Modify: `docs/format-spec.md` (§2.1, §2.4, §3.1, §3.4, §3.7, §3.8, §11)

**Model (Tasks 2, 4):**

- Create: `src/model/requires.rs` — `ItemRequires` + `RequireRef`
- Modify: `src/model/mod.rs` — re-export the new types
- Modify: `src/model/skill.rs`, `src/model/rule.rs`, `src/model/agent.rs` — `name: Option<String>`, add `requires`, `ignore`

**Generation (Task 4):**

- Modify: `src/generate/mod.rs` — `render_*` and `build_skill_frontmatter` take `name: &str`
- Modify: `src/generate/claude.rs`, `src/generate/copilot.rs`, `src/generate/opencode.rs` — `rule_frontmatter`/`agent_frontmatter`/`skill_frontmatter` take `name: &str`

**Resources (Task 3):**

- Create: `src/pipeline/ignore.rs` — `filter_ignored` + the glob matcher
- Modify: `src/pipeline/mod.rs` — `mod ignore;`
- Modify: `src/pipeline/install.rs` — apply `ignore` before copy

**Naming + discovery (Task 4):**

- Modify: `src/pipeline/discovery.rs` — `effective_name`, `probe_effective_name`, `scan_source_items` returns effective names
- Modify: `src/pipeline/install.rs` — compute + use effective name
- Modify: `src/lint.rs` — relax `check_name_matches_dir`; `parse_kind` returns `Option<String>` name

**Dependencies (Task 5):**

- Modify: `src/pipeline/install.rs` — `resolve_requires_closure`
- Modify: `src/pipeline/mod.rs` — wire closure into `install_with_lockfile`
- Modify: `src/lockfile.rs` — `LockedItem.required_by`, `items_from_report` takes a provenance map

**Grouping + lifecycle (Tasks 6, 7):**

- Modify: `src/lockfile.rs` — `LockedItem.group`
- Modify: `src/pipeline/install.rs` / `src/pipeline/mod.rs` — record `group`
- Modify: `src/pipeline/lifecycle.rs` — `remove` acts on the co-located unit; `doctor` orphaned-dependency flag
- Modify: `src/pipeline/report.rs` — `OrphanedDependency` + `DoctorReport.orphaned_dependencies`

**Integration tests:**

- Create/extend: `tests/cli_requires.rs`, `tests/generate_*` (strip), `tests/pipeline_*` (grouping, doctor)

---

## Task 1: ADR-0009 + format-spec edits

**Files:**

- Create: `docs/adr/0009-coupling-tiers-and-dependencies.md`
- Modify: `docs/format-spec.md`

This task is documentation; its "test" is `just fmt` (markdownlint + dprint) passing and the internal links resolving. No Rust tests.

- [ ] **Step 1: Write ADR-0009**

Create `docs/adr/0009-coupling-tiers-and-dependencies.md` following the structure of `docs/adr/0006-flat-item-layout.md` (Status / Context / Decision / Consequences / Alternatives considered / References). Content, drawn verbatim in substance from the design spec:

- **Status**: Proposed (2026-06-03). Amends ADR-0006.
- **Context**: co-location is currently the only "install together" mechanism; the three failures (forced shared name, no copy scoping, ecosystem-invisible coupling). Reference the spec.
- **Decision**:
  - Three coupling tiers (co-location = symmetric unit; `requires`/`preload-skills` = directed; bundles = curated). A mutual dependency is co-location, never mutual `requires` (cycle = error).
  - Relaxed + optional naming: skill `name` optional → folder fallback, must match folder if present; rule/agent `name` from frontmatter, may diverge, folder is fallback + grouping key only; identity is `(kind, name)`, layout-independent; no lint diagnostic on rule/agent divergence.
  - `requires` (per-entrypoint, hard, acyclic, resolved by `(kind, name)`, string-or-`{name,source}`); `preload-skills` implies `requires.skills`. `ignore` (subtractive copy scope).
  - Cross-source contract (source reuses `add` DSL; conflict = same `(kind,name)` different source/ref is an error reusing §3.7; cycle keyed by `(canonical-source, kind, name)`; `required_by` provenance; removal never cascades; doctor flags orphaned deps). Note that cross-source _resolution_ ships in a follow-up.
  - Inherent limit: co-located rules/agents are invisible to standard-only tooling; the cross-kind guarantee holds only inside upskill.
- **Consequences** and **Alternatives considered**: lift the "Rejected" list from the design spec §4.
- **References**: ADR-0006, format-spec, design spec.

- [ ] **Step 2: Edit format-spec §2.1 — relaxed/optional naming**

In `docs/format-spec.md` §2.1, replace the constraint "`<name>` MUST match the `name` field in every entrypoint within the directory" and the co-location "every entrypoint's `name:` field MUST equal the directory name" with:

> - A `SKILL.md`'s `name` field, **when present**, MUST equal the directory name (Agent Skills standard). When absent, the effective skill name is the directory name.
> - A `RULE.md`'s or `AGENT.md`'s `name` field, when present, is that item's identity and MAY differ from the directory name. When absent, the effective name is the directory name.
> - Item identity is `(kind, effective-name)` and is independent of on-disk layout.
> - A directory MAY hold independently-named kinds (e.g. a skill named after the directory plus a rule with its own name). The directory name is the co-location grouping key.

Keep the lowercase/hyphen/length constraints on the effective name. Keep the Agent Skills compatibility paragraph.

- [ ] **Step 3: Edit format-spec §2.4 — `ignore` copy scope**

In §2.4, after the "Implementations MUST preserve supporting files" bullet, add:

> An item MAY declare an `ignore` list in its frontmatter (§3.1) to exclude supporting files from the copy. Patterns are `.gitignore`-style and **subtractive only** (there is no allowlist form). A file matching any `ignore` pattern is not copied to client output. `ignore` itself is stripped from generated output.

- [ ] **Step 4: Edit format-spec §3.1 — `requires` + `ignore` common fields**

Add two rows to the §3.1 common-fields table:

| Field      | Type     | Required | Description                                                                             |
| ---------- | -------- | -------- | --------------------------------------------------------------------------------------- |
| `requires` | map      | no       | Directed item dependencies (`rules`/`skills`/`agents`). See §3.7. Stripped from output. |
| `ignore`   | string[] | no       | `.gitignore`-style subtractive copy-scope patterns (§2.4). Stripped from output.        |

Add field semantics prose: each `requires.<kind>` entry is a bare name (same source) or a `{ name, source }` map (cross-source; `source` uses the `add` source DSL). Both `requires` and `ignore` are stripped from every client output (like `schema`, `metadata`, `license`).

- [ ] **Step 5: Edit format-spec §3.4 — `preload-skills` implies `requires`**

In §3.4, add to the `preload-skills` description: "Listing a skill in `preload-skills` also implies `requires.skills` for that skill (it is both required for install and preloaded at agent startup)."

- [ ] **Step 6: Edit format-spec §3.7 — item `requires` cycle + conflict**

Add a subsection "Item `requires` resolution" stating: installing an item implies installing its transitive `requires` closure; resolution is by `(kind, name)`; circular item `requires` MUST be rejected as an error; the same `(kind, name)` resolving to a different source or ref MUST be an error (the same rule already stated for bundle item conflicts). Note cross-source resolution is specified here but lands in a later release.

- [ ] **Step 7: Edit format-spec §3.8 — canonical key order**

Add `requires` and `ignore` to each item kind's key order (after `metadata`, before the passthrough blocks). Rule/Skill/Agent rows get `…, metadata, requires, ignore, claude, copilot, opencode, extras`. Bundle row unchanged.

- [ ] **Step 8: Edit format-spec §11 — mark multi-repo sources resolved**

Change open-question 7 ("Multi-repo item sources") to RESOLVED, noting `requires` with a `source` locator (§3.7) defines the resolution contract; same-source resolution is implemented and cross-source resolution is staged.

- [ ] **Step 9: Format and commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add docs/adr/0009-coupling-tiers-and-dependencies.md docs/format-spec.md
git commit -m "docs(adr): ADR-0009 coupling tiers + dependencies; format-spec edits"
```

Expected: `just fmt` reports formatted/clean; commit succeeds (the `git std lint` commit-message hook passes for a Conventional Commit).

---

## Task 2: Model — `requires` + `ignore` fields (additive, stripped)

**Files:**

- Create: `src/model/requires.rs`
- Modify: `src/model/mod.rs`
- Modify: `src/model/skill.rs:7-33`, `src/model/rule.rs:14-39`, `src/model/agent.rs:31-68`
- Test: unit tests in `src/model/requires.rs` + `src/parse/frontmatter.rs`; integration strip test in `tests/`

`name` stays `String` in this task — only `requires`/`ignore` are added, so the build and all generation stay green. Optional `name` is Task 4.

- [ ] **Step 1: Write the failing model test**

Create `src/model/requires.rs` with the types and a unit test module:

```rust
//! Item-level `requires` (§3.7) — directed dependencies declared per
//! entrypoint. Distinct from the bundle-level `Requires` (which pins a
//! semver constraint); item requires resolve by `(kind, name)`.

use serde::{Deserialize, Serialize};

/// One `requires` reference. A bare string targets the same source by
/// name; a `{ name, source }` map targets another source (the `source`
/// uses the `upskill add` source DSL). Cross-source *resolution* ships in
/// a later release; this type captures the stable on-disk form now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequireRef {
    Name(String),
    Detailed { name: String, source: String },
}

impl RequireRef {
    pub fn name(&self) -> &str {
        match self {
            RequireRef::Name(n) => n,
            RequireRef::Detailed { name, .. } => name,
        }
    }
    /// The cross-source locator, if this is a `{ name, source }` entry.
    pub fn source(&self) -> Option<&str> {
        match self {
            RequireRef::Name(_) => None,
            RequireRef::Detailed { source, .. } => Some(source),
        }
    }
}

/// `requires:` block — mirrors the bundle `items` vocabulary.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRequires {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RequireRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<RequireRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<RequireRef>,
}

impl ItemRequires {
    /// True when no dependency is declared. Used by `skip_serializing_if`.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty() && self.skills.is_empty() && self.agents.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_string_and_map_entries() {
        let yaml = "rules: [security-baseline]\nskills: [{ name: sarif, source: org/repo }]\n";
        let req: ItemRequires = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(req.rules, vec![RequireRef::Name("security-baseline".into())]);
        assert_eq!(req.skills[0].name(), "sarif");
        assert_eq!(req.skills[0].source(), Some("org/repo"));
        assert!(req.agents.is_empty());
    }

    #[test]
    fn empty_when_no_kinds() {
        assert!(ItemRequires::default().is_empty());
    }
}
```

- [ ] **Step 2: Run it red**

Run: `cargo test -p upskill model::requires`
Expected: FAIL to compile — module not declared in `src/model/mod.rs`.

- [ ] **Step 3: Declare and re-export the module**

In `src/model/mod.rs`, add `pub mod requires;` (after `pub mod rule;`) and to the re-export block add:

```rust
pub use requires::{ItemRequires, RequireRef};
```

- [ ] **Step 4: Run it green**

Run: `cargo test -p upskill model::requires`
Expected: PASS (2 tests).

- [ ] **Step 5: Add `requires` + `ignore` to the three item models**

In each of `src/model/skill.rs`, `src/model/rule.rs`, `src/model/agent.rs`, add these two fields immediately after the `metadata` field and before the passthrough blocks (`claude`/`copilot`/`opencode`). Add `ItemRequires` to the `use crate::model::...` import line in each file.

```rust
/// §3.7 directed dependencies. SSOT-only — stripped from output.
#[serde(default, skip_serializing_if = "ItemRequires::is_empty")]
pub requires: crate::model::ItemRequires,
/// §2.4 subtractive copy-scope patterns. SSOT-only — stripped from output.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub ignore: Vec<String>,
```

(Using the fully-qualified `crate::model::ItemRequires` avoids touching each file's `use` line if preferred; either form compiles.)

- [ ] **Step 6: Confirm fields land outside `extra` (round-trip test)**

Add to `src/parse/frontmatter.rs` tests:

```rust
#[test]
fn requires_and_ignore_parse_into_typed_fields_not_extra() {
    let input = concat!(
        "---\n",
        "schema: 1\n",
        "name: code-review\n",
        "description: x\n",
        "requires:\n",
        "  skills: [sarif-formatting]\n",
        "ignore:\n",
        "  - \"scripts/**\"\n",
        "---\nbody\n",
    );
    let (skill, _) = parse::<Skill>(input).unwrap();
    assert_eq!(skill.requires.skills[0].name(), "sarif-formatting");
    assert_eq!(skill.ignore, vec!["scripts/**".to_string()]);
    // They must NOT have leaked into the pass-through `extra` map (which
    // skills emit verbatim into output).
    assert!(!skill.extra.contains_key("requires"));
    assert!(!skill.extra.contains_key("ignore"));
}
```

Run: `cargo test -p upskill requires_and_ignore_parse_into_typed_fields_not_extra`
Expected: PASS.

- [ ] **Step 7: Write the failing generation-strip integration test**

Skills emit their `extra` map verbatim, so this guards the real regression. Add `tests/generate_strip_ssot_fields.rs`:

```rust
use std::fs;
use std::process::Command;

mod common;

#[test]
fn requires_and_ignore_are_stripped_from_skill_output() {
    let env = common::TestEnv::new(); // adapt to tests/common API (see existing tests)
    let src = env.project.join("src-registry/code-review");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("SKILL.md"),
        "---\nschema: 1\nname: code-review\ndescription: x\nrequires:\n  skills: [other]\nignore:\n  - \"scripts/**\"\n---\n\n## Body\n\ntext\n",
    )
    .unwrap();
    // Also create the required item so install does not fail on resolution.
    let other = env.project.join("src-registry/other");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("SKILL.md"), "---\nschema: 1\nname: other\ndescription: y\n---\n\n## B\n\nt\n").unwrap();

    common::upskill_cmd(&env)
        .args(["add", "./src-registry", "--claude"])
        .assert()
        .success();

    let out = fs::read_to_string(env.project.join(".claude/skills/code-review/SKILL.md")).unwrap();
    assert!(!out.contains("requires"), "requires must be stripped:\n{out}");
    assert!(!out.contains("ignore"), "ignore must be stripped:\n{out}");
}
```

NOTE: adapt the `common::` helper calls to the actual `tests/common/mod.rs` surface (read it first). The behavioral asserts are what matter.

- [ ] **Step 8: Run it — verify it already passes**

Run: `cargo test --test generate_strip_ssot_fields`
Expected: PASS without any generation change — because the typed fields are not in `extra` and the builders never insert them. If it FAILS, the fields leaked into `extra` (revisit Step 5). This test is a regression guard, not a driver for new code.

- [ ] **Step 9: Commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add src/model tests/generate_strip_ssot_fields.rs src/parse/frontmatter.rs
git commit -m "feat(model): add item requires + ignore fields, stripped from output"
```

---

## Task 3: `ignore` copy-scope filter

**Files:**

- Create: `src/pipeline/ignore.rs`
- Modify: `src/pipeline/mod.rs:31-37` (add `mod ignore;`)
- Modify: `src/pipeline/install.rs:425-428` (filter resources before copy)

- [ ] **Step 1: Write the failing matcher test**

Create `src/pipeline/ignore.rs`:

```rust
//! `.gitignore`-style subtractive filtering of an item's supporting
//! resources (format-spec §2.4). Hand-rolled to avoid a glob dependency
//! (protects the ~3 MB binary target). Supports `*` (any run of
//! non-`/` chars), `**` (any run including `/`), `?` (one non-`/` char),
//! and literal segments. A pattern with no `/` matches the basename at any
//! depth; a pattern with a `/` matches the full relative path. A trailing
//! `/**` (or a bare directory-name pattern) matches everything under that
//! directory.

use std::path::PathBuf;

/// Drop every resource (path relative to the item dir) that matches any
/// `ignore` glob. Returns the kept resources, order preserved.
pub(super) fn filter_ignored(resources: Vec<PathBuf>, patterns: &[String]) -> Vec<PathBuf> {
    if patterns.is_empty() {
        return resources;
    }
    resources
        .into_iter()
        .filter(|rel| {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            !patterns.iter().any(|p| matches_pattern(&rel_str, p))
        })
        .collect()
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');
    // A pattern with no slash matches the basename at any depth.
    if !pattern.contains('/') {
        let base = path.rsplit('/').next().unwrap_or(path);
        return glob_match(base, pattern);
    }
    // Directory-prefix shorthand: `scripts` or `scripts/**` ignores the
    // whole subtree.
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    glob_match(path, pattern)
}

/// Match `text` against a glob `pat` where `*`/`?` do not cross `/` and
/// `**` does. Recursive backtracking — patterns are short and few.
fn glob_match(text: &str, pat: &str) -> bool {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = pat.chars().collect();
    m(&t, 0, &p, 0)
}

fn m(t: &[char], ti: usize, p: &[char], pi: usize) -> bool {
    if pi == p.len() {
        return ti == t.len();
    }
    match p[pi] {
        '*' if pi + 1 < p.len() && p[pi + 1] == '*' => {
            // `**` — consume any chars including `/`.
            let mut k = ti;
            loop {
                if m(t, k, p, pi + 2) {
                    return true;
                }
                if k == t.len() {
                    return false;
                }
                k += 1;
            }
        }
        '*' => {
            let mut k = ti;
            loop {
                if m(t, k, p, pi + 1) {
                    return true;
                }
                if k == t.len() || t[k] == '/' {
                    return false;
                }
                k += 1;
            }
        }
        '?' => ti < t.len() && t[ti] != '/' && m(t, ti + 1, p, pi + 1),
        c => ti < t.len() && t[ti] == c && m(t, ti + 1, p, pi + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn paths(v: &[&str]) -> Vec<PathBuf> {
        v.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn empty_patterns_keep_everything() {
        let r = paths(&["scripts/gate.sh", "refs/p.md"]);
        assert_eq!(filter_ignored(r.clone(), &[]), r);
    }

    #[test]
    fn double_star_prefix_drops_subtree() {
        let kept = filter_ignored(
            paths(&["scripts/gate.sh", "scripts/lib/x.sh", "refs/p.md"]),
            &["scripts/**".to_string()],
        );
        assert_eq!(kept, paths(&["refs/p.md"]));
    }

    #[test]
    fn bare_name_matches_basename_at_any_depth() {
        let kept = filter_ignored(
            paths(&["a/b/notes.log", "keep.md"]),
            &["*.log".to_string()],
        );
        assert_eq!(kept, paths(&["keep.md"]));
    }

    #[test]
    fn single_star_does_not_cross_slash() {
        let kept = filter_ignored(
            paths(&["a/x.txt", "a/b/x.txt"]),
            &["a/*.txt".to_string()],
        );
        assert_eq!(kept, paths(&["a/b/x.txt"]));
    }

    #[test]
    fn bare_dir_name_ignores_subtree() {
        let kept = filter_ignored(
            paths(&["fixtures/data.json", "main.md"]),
            &["fixtures".to_string()],
        );
        assert_eq!(kept, paths(&["main.md"]));
    }
}
```

- [ ] **Step 2: Run it red**

Run: `cargo test -p upskill pipeline::ignore`
Expected: FAIL — `mod ignore;` not declared.

- [ ] **Step 3: Declare the module**

In `src/pipeline/mod.rs`, add `mod ignore;` alongside the other `mod` lines (around line 31-37).

- [ ] **Step 4: Run it green**

Run: `cargo test -p upskill pipeline::ignore`
Expected: PASS (5 tests).

- [ ] **Step 5: Apply the filter in install**

In `src/pipeline/install.rs`, the resource collection currently reads (around line 425-428):

```rust
let source_hash = hash_item_dir(&dir);
let resources = iter_item_resources(&dir);
```

The `ignore` list lives on the parsed model. The parse happens inside the per-kind `match` (Step in Task 4 restructures this). For Task 3, capture the `ignore` list out of the match. Change the `match` block to also yield the `ignore` vec, e.g. extend the tuple it binds:

```rust
        let (audience, ignore, renders): (Option<Vec<Audience>>, Vec<String>, Vec<(Client, String)>) =
            match kind {
                ItemKind::Skill => {
                    let (skill, body) = frontmatter::parse::<Skill>(&raw)
                        .with_context(|| format!("parse {}", entry_path.display()))?;
                    let aud = skill.audience.clone();
                    let ign = skill.ignore.clone();
                    let mut out = Vec::new();
                    /* …existing render loop… */
                    (aud, ign, out)
                }
                /* Rule, Agent arms: same shape, binding rule.ignore / agent.ignore */
            };

        let source_hash = hash_item_dir(&dir);
        let resources = super::ignore::filter_ignored(iter_item_resources(&dir), &ignore);
```

Repeat the `ign = X.ignore.clone();` and the `(aud, ign, out)` return in all three arms (Rule binds `rule.ignore`, Agent binds `agent.ignore`).

- [ ] **Step 6: Write the failing install integration test**

Add `tests/pipeline_ignore.rs` — install an item with a `scripts/**` ignore and assert the script is NOT copied while a non-ignored resource IS. Use the `tests/common` helpers (read `tests/common/mod.rs` first). Skeleton:

```rust
mod common;
use std::fs;

#[test]
fn ignored_resources_are_not_copied() {
    let env = common::TestEnv::new();
    let item = env.project.join("reg/demo");
    fs::create_dir_all(item.join("scripts")).unwrap();
    fs::create_dir_all(item.join("refs")).unwrap();
    fs::write(item.join("SKILL.md"),
        "---\nschema: 1\nname: demo\ndescription: x\nignore:\n  - \"scripts/**\"\n---\n\n## B\n\n[r](./refs/p.md)\n").unwrap();
    fs::write(item.join("scripts/gate.sh"), "#!/bin/sh\n").unwrap();
    fs::write(item.join("refs/p.md"), "p\n").unwrap();

    common::upskill_cmd(&env).args(["add", "./reg", "--claude"]).assert().success();

    let base = env.project.join(".claude/skills/demo");
    assert!(base.join("refs/p.md").exists(), "non-ignored resource must be copied");
    assert!(!base.join("scripts/gate.sh").exists(), "ignored resource must be skipped");
}
```

Run: `cargo test --test pipeline_ignore`
Expected: PASS (after Step 5).

- [ ] **Step 7: Commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add src/pipeline/ignore.rs src/pipeline/mod.rs src/pipeline/install.rs tests/pipeline_ignore.rs
git commit -m "feat(pipeline): subtractive ignore filter for item resources"
```

---

## Task 4: Optional + relaxed naming (effective-name identity)

**Files:**

- Modify: `src/model/skill.rs`, `src/model/rule.rs`, `src/model/agent.rs` — `name: Option<String>`
- Modify: `src/generate/mod.rs:51-146` — `render_*` + `build_skill_frontmatter` take `name: &str`
- Modify: `src/generate/claude.rs:9-49`, `src/generate/copilot.rs:9-46`, `src/generate/opencode.rs:10-52`
- Modify: `src/pipeline/discovery.rs` — `effective_name`, `probe_effective_name`, `scan_source_items`
- Modify: `src/pipeline/install.rs` — compute + use effective name
- Modify: `src/lint.rs:283`, `:331-372`, `parse_kind` — relax matching

This is the largest task. The build only goes green at the end of Step 8, so commit once at the end.

- [ ] **Step 1: Write the failing render test (effective name from folder)**

Add to `src/generate/mod.rs` tests (create a `#[cfg(test)] mod tests` if absent) — but first the signature must change, so write the test against the target signature:

```rust
#[cfg(test)]
mod name_tests {
    use super::*;
    use crate::model::{Rule, SchemaVersion};

    fn minimal_rule(name: Option<&str>) -> Rule {
        Rule {
            schema: SchemaVersion::new(1).unwrap(),
            name: name.map(str::to_string),
            description: "d".into(),
            audience: None,
            license: None,
            scope: None,
            metadata: None,
            requires: Default::default(),
            ignore: vec![],
            claude: None,
            copilot: None,
            opencode: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn rule_render_uses_passed_effective_name_not_frontmatter() {
        let rule = minimal_rule(None); // name absent
        let out = render_rule(&rule, "security-baseline", "## B\n\nt\n", Client::Claude).unwrap();
        assert!(out.contains("name: security-baseline"));
    }
}
```

- [ ] **Step 2: Run it red**

Run: `cargo test -p upskill name_tests`
Expected: FAIL to compile — `render_rule` takes 3 args, `Rule.name` is `String`.

- [ ] **Step 3: Make `name` optional in the three models**

In `src/model/skill.rs`, `src/model/rule.rs`, `src/model/agent.rs`, change `pub name: String,` to:

```rust
/// Effective name resolution is layout-dependent (§2.1): absent means
/// the directory name is used. Resolved by the pipeline/lint layer.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub name: Option<String>,
```

- [ ] **Step 4: Thread `name: &str` through `generate/mod.rs`**

Change the three `render_*` signatures and `build_skill_frontmatter`:

```rust
pub fn render_skill(skill: &Skill, name: &str, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::skill_frontmatter(skill, name)?,
        Client::Copilot => copilot::skill_frontmatter(skill, name)?,
        Client::OpenCode => opencode::skill_frontmatter(skill, name)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

pub fn render_rule(rule: &Rule, name: &str, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::rule_frontmatter(rule, name)?,
        Client::Copilot => copilot::rule_frontmatter(rule, name)?,
        Client::OpenCode => opencode::rule_frontmatter(rule, name)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}

pub fn render_agent(agent: &Agent, name: &str, body: &str, client: Client) -> Result<String> {
    let processed_body = directives::process(body, client)?;
    let frontmatter = match client {
        Client::Claude => claude::agent_frontmatter(agent, name)?,
        Client::Copilot => copilot::agent_frontmatter(agent, name)?,
        Client::OpenCode => opencode::agent_frontmatter(agent, name)?,
    };
    let combined = assemble(&frontmatter, &processed_body);
    format::format_markdown(&combined)
}
```

And `build_skill_frontmatter`:

```rust
pub(crate) fn build_skill_frontmatter(
    skill: &Skill,
    name: &str,
    passthrough: Option<&Value>,
) -> Result<String> {
    let mut map = Mapping::new();
    map.insert(Value::from("name"), Value::from(name.to_string()));
    map.insert(Value::from("description"), Value::from(skill.description.clone()));
    for (k, v) in &skill.extra {
        map.insert(Value::from(k.clone()), v.clone());
    }
    if let Some(Value::Mapping(m)) = passthrough {
        for (k, v) in m {
            map.insert(k.clone(), v.clone());
        }
    }
    serde_yaml_ng::to_string(&map).context("serializing skill frontmatter")
}
```

- [ ] **Step 5: Thread `name` through the three client builders**

In `src/generate/claude.rs`:

- `skill_frontmatter(skill: &Skill, name: &str)` → `super::build_skill_frontmatter(skill, name, skill.claude.as_ref())`
- `rule_frontmatter(rule: &Rule, name: &str)` → replace `Value::from(rule.name.clone())` with `Value::from(name.to_string())`
- `agent_frontmatter(agent: &Agent, name: &str)` → replace `Value::from(agent.name.clone())` with `Value::from(name.to_string())`

Apply the identical change in `src/generate/copilot.rs` (`skill_frontmatter`/`rule_frontmatter`/`agent_frontmatter`, each gains `name: &str`; the `rule.name`/`agent.name` reads become `name.to_string()`) and in `src/generate/opencode.rs` (same three functions; `rule.name`→`name`, `agent.name`→`name`). These are mechanical: every `X.name.clone()` in a frontmatter builder becomes `name.to_string()`, and `skill_frontmatter` forwards `name`.

- [ ] **Step 6: Run the render test green**

Run: `cargo test -p upskill name_tests`
Expected: PASS. (Library may still fail to build until install.rs is updated — if so, temporarily this test still compiles within the crate; proceed to Step 7 and run after.)

- [ ] **Step 7: Add the effective-name resolver + restructure install**

In `src/pipeline/discovery.rs`, add:

```rust
use super::ItemKind;
use crate::parse::frontmatter;

/// Effective identity name for an item (§2.1). Skills always take the
/// folder name (Agent Skills standard mandates name == dir); rules/agents
/// take their frontmatter name when present, else the folder name.
pub(super) fn effective_name(kind: ItemKind, frontmatter_name: Option<&str>, folder: &str) -> String {
    match kind {
        ItemKind::Skill => folder.to_string(),
        ItemKind::Rule | ItemKind::Agent => frontmatter_name.unwrap_or(folder).to_string(),
    }
}

/// Probe an entrypoint's frontmatter `name` cheaply, falling back to the
/// folder name on any read/parse failure (validation belongs to lint, not
/// to discovery). Used so `(kind, name)` identity is consistent everywhere.
pub(super) fn probe_effective_name(entry_path: &std::path::Path, kind: ItemKind, folder: &str) -> String {
    if kind == ItemKind::Skill {
        return folder.to_string();
    }
    #[derive(serde::Deserialize)]
    struct NameProbe {
        #[serde(default)]
        name: Option<String>,
    }
    match std::fs::read_to_string(entry_path) {
        Ok(raw) => match frontmatter::parse::<NameProbe>(&raw) {
            Ok((p, _)) => p.name.unwrap_or_else(|| folder.to_string()),
            Err(_) => folder.to_string(),
        },
        Err(_) => folder.to_string(),
    }
}
```

Update `scan_source_items` so each pushed pair uses the effective name (it currently pushes the folder name). For each detected entrypoint, compute:

```rust
if path.join("SKILL.md").is_file() {
    items.push((ItemKind::Skill, name.to_string())); // skill == folder, unchanged
}
if path.join("RULE.md").is_file() {
    let n = probe_effective_name(&path.join("RULE.md"), ItemKind::Rule, name);
    items.push((ItemKind::Rule, n));
}
if path.join("AGENT.md").is_file() {
    let n = probe_effective_name(&path.join("AGENT.md"), ItemKind::Agent, name);
    items.push((ItemKind::Agent, n));
}
```

Apply the same to the category-subdir branch.

In `src/pipeline/install.rs` `install_items_of_kind`, rename the loop var `name` → `folder`, parse the model first, compute the effective name, filter on it, render with it, and use it for output path / resources / lockfile. The restructured loop body:

```rust
    for (folder, dir) in iter_item_dirs(source)? {
        let entry_path = dir.join(entrypoint);
        if !entry_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&entry_path)
            .with_context(|| format!("read {}", entry_path.display()))?;

        // Parse, resolve the effective name, then filter on it.
        let (name, audience, ignore, renders) = match kind {
            ItemKind::Skill => {
                let (skill, body) = frontmatter::parse::<Skill>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let name = super::discovery::effective_name(kind, skill.name.as_deref(), &folder);
                let renders = render_all(|client| generate::render_skill(&skill, &name, body, client),
                                         skill.audience.as_deref(), &name, kind)?;
                (name, skill.audience.clone(), skill.ignore.clone(), renders)
            }
            ItemKind::Rule => {
                let (rule, body) = frontmatter::parse::<Rule>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let name = super::discovery::effective_name(kind, rule.name.as_deref(), &folder);
                let renders = render_all(|client| generate::render_rule(&rule, &name, body, client),
                                         rule.audience.as_deref(), &name, kind)?;
                (name, rule.audience.clone(), rule.ignore.clone(), renders)
            }
            ItemKind::Agent => {
                let (agent, body) = frontmatter::parse::<Agent>(&raw)
                    .with_context(|| format!("parse {}", entry_path.display()))?;
                let name = super::discovery::effective_name(kind, agent.name.as_deref(), &folder);
                let renders = render_all(|client| generate::render_agent(&agent, &name, body, client),
                                         agent.audience.as_deref(), &name, kind)?;
                (name, agent.audience.clone(), agent.ignore.clone(), renders)
            }
        };

        if let Some(items) = filter
            && !items.contains(kind, &name)
        {
            continue;
        }

        let source_hash = hash_item_dir(&dir);
        let resources = super::ignore::filter_ignored(iter_item_resources(&dir), &ignore);
        /* …unchanged from here: copied set, remove_item_outputs(target, kind, &name),
           the render write loop, audience cleanup — all already keyed on `name`… */
    }
```

Add the small `render_all` helper near the top of `install.rs` to avoid repeating the audience/render loop three times:

```rust
/// Render one item for every targeted client, returning `(client, body)`
/// pairs. `render_one` is the per-client renderer closure.
fn render_all(
    mut render_one: impl FnMut(Client) -> Result<String>,
    audience: Option<&[Audience]>,
    name: &str,
    kind: ItemKind,
) -> Result<Vec<(Client, String)>> {
    let mut out = Vec::new();
    for client in ALL_CLIENTS {
        if !targets(client, audience) {
            continue;
        }
        out.push((
            client,
            render_one(client).with_context(|| format!("render {kind} {name} for {client:?}"))?,
        ));
    }
    Ok(out)
}
```

NOTE: the closure borrows the parsed model and `name` by reference; if the borrow checker objects to `name` moving into the tuple, clone `name` for the tuple and pass `&name` to `render_all`. Adjust as the compiler directs — the behavior (render per targeted client) is fixed.

- [ ] **Step 8: Relax lint name-matching**

In `src/lint.rs`, change `parse_kind` to return `(Option<String>, &str)`:

```rust
fn parse_kind(raw: &str, kind: FileKind) -> Result<(Option<String>, &str)> {
    match kind {
        FileKind::Skill => { let (i, b) = frontmatter::parse::<Skill>(raw)?; Ok((i.name, b)) }
        FileKind::Rule => { let (i, b) = frontmatter::parse::<Rule>(raw)?; Ok((i.name, b)) }
        FileKind::Agent => { let (i, b) = frontmatter::parse::<Agent>(raw)?; Ok((i.name, b)) }
        FileKind::Bundle => { let b: crate::model::Bundle = serde_yaml_ng::from_str(raw)?; Ok((Some(b.name), "")) }
    }
}
```

Update the caller at `src/lint.rs:283` to pass the `Option<String>`:

```rust
Ok((parsed_name, body)) => {
    check_name_matches_dir(file, parsed_name.as_deref(), kind, out);
    body
}
```

Rewrite `check_name_matches_dir(file, name: Option<&str>, kind, out)`:

```rust
fn check_name_matches_dir(file: &Path, name: Option<&str>, kind: FileKind, out: &mut Vec<Finding>) {
    match kind {
        // Bundles: filename stem must match the (required) name.
        FileKind::Bundle => {
            let Some(name) = name else { return };
            let Some(stem) = file.file_name().and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(crate::parse::bundle::BUNDLE_SUFFIX)) else { return };
            if stem != name {
                out.push(Finding { rule_id: "name-matches-dir", severity: Severity::Error,
                    path: file.to_path_buf(), line: None,
                    message: format!("frontmatter `name: {name}` does not match filename stem `{stem}`") });
            }
        }
        // Skills: name is optional, but if present MUST equal the directory
        // (Agent Skills standard). Absent → folder name is used; OK.
        FileKind::Skill => {
            let (Some(name), Some(dir)) = (name, file.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str())) else { return };
            if dir != name {
                out.push(Finding { rule_id: "name-matches-dir", severity: Severity::Error,
                    path: file.to_path_buf(), line: None,
                    message: format!("skill `name: {name}` must equal directory `{dir}` (Agent Skills standard)") });
            }
        }
        // Rules/agents: name is free and layout-independent (§2.1). No check.
        FileKind::Rule | FileKind::Agent => {}
    }
}
```

- [ ] **Step 9: Build and run the full suite green**

Run: `just assemble && cargo test -p upskill`
Expected: PASS. Fix any remaining call sites the compiler flags (e.g. other `render_*` callers, the `lint.rs:741` test that asserted solo `name-matches-dir` for a rule — update or delete that test since rule divergence is now legal and silent).

- [ ] **Step 10: Add the relaxed-naming integration tests**

Add `tests/cli_relaxed_naming.rs`:

- A skill folder with no `name:` → output at `.claude/skills/<folder>/SKILL.md` with `name: <folder>`.
- A rule folder `stuff/` whose `RULE.md` has `name: security-baseline` → output at `.claude/rules/security-baseline.md` containing `name: security-baseline` (folder name `stuff` absent from output path).
- `upskill lint` on a skill whose `name` ≠ folder → exits non-zero with the standard-mandate message.
- `upskill lint` on a rule whose `name` ≠ folder → exits zero (no finding).

Use the `tests/common` helpers. Run: `cargo test --test cli_relaxed_naming` → PASS.

- [ ] **Step 11: Commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add -A
git commit -m "feat(naming): optional + relaxed item naming with layout-independent identity"
```

---

## Task 5: Same-source `requires` closure + `required_by` provenance

**Files:**

- Modify: `src/pipeline/install.rs` — `resolve_requires_closure`
- Modify: `src/pipeline/mod.rs:139-349` — expand the install set before conflict detection; pass provenance to the lockfile
- Modify: `src/lockfile.rs:37-59` — `LockedItem.required_by`; `items_from_report` signature
- Test: `src/pipeline/install.rs` unit tests + `tests/cli_requires.rs`

- [ ] **Step 1: Write the failing closure unit test**

Add to `src/pipeline/install.rs` tests:

```rust
#[test]
fn closure_pulls_same_source_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // agent `code-review` requires skill `sarif`.
    std::fs::create_dir_all(root.join("code-review")).unwrap();
    std::fs::write(root.join("code-review/AGENT.md"),
        "---\nschema: 1\nname: code-review\ndescription: a\nrequires:\n  skills: [sarif]\n---\n\n## B\n\nt\n").unwrap();
    std::fs::create_dir_all(root.join("sarif")).unwrap();
    std::fs::write(root.join("sarif/SKILL.md"),
        "---\nschema: 1\nname: sarif\ndescription: s\n---\n\n## B\n\nt\n").unwrap();

    let closure = resolve_requires_closure(root, &[(ItemKind::Agent, "code-review".into())]).unwrap();
    assert!(closure.items.contains(ItemKind::Agent, "code-review"));
    assert!(closure.items.contains(ItemKind::Skill, "sarif"));
    assert_eq!(
        closure.required_by.get(&(ItemKind::Skill, "sarif".into())).map(|s| s.iter().cloned().collect::<Vec<_>>()),
        Some(vec!["agent:code-review".to_string()])
    );
}

#[test]
fn closure_rejects_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::write(root.join("a/RULE.md"),
        "---\nschema: 1\nname: a\ndescription: a\nrequires:\n  rules: [b]\n---\n\n## B\n\nt\n").unwrap();
    std::fs::create_dir_all(root.join("b")).unwrap();
    std::fs::write(root.join("b/RULE.md"),
        "---\nschema: 1\nname: b\ndescription: b\nrequires:\n  rules: [a]\n---\n\n## B\n\nt\n").unwrap();

    let err = resolve_requires_closure(root, &[(ItemKind::Rule, "a".into())]).unwrap_err();
    assert!(format!("{err:#}").contains("circular"), "{err:#}");
}

#[test]
fn closure_errors_on_cross_source_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::write(root.join("a/RULE.md"),
        "---\nschema: 1\nname: a\ndescription: a\nrequires:\n  skills: [{ name: x, source: org/repo }]\n---\n\n## B\n\nt\n").unwrap();

    let err = resolve_requires_closure(root, &[(ItemKind::Rule, "a".into())]).unwrap_err();
    assert!(format!("{err:#}").contains("cross-source"), "{err:#}");
}
```

- [ ] **Step 2: Run red**

Run: `cargo test -p upskill closure_`
Expected: FAIL — `resolve_requires_closure`, `DependencyClosure` undefined.

- [ ] **Step 3: Implement `resolve_requires_closure`**

Add to `src/pipeline/install.rs`. It needs a `(kind, name) → (folder, entry_path)` index over the source (because a rule's name may diverge from its folder), built by scanning + probing:

```rust
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use crate::model::{ItemRequires, RequireRef};

/// Transitive same-source `requires` expansion of an initial item set.
pub(super) struct DependencyClosure {
    pub items: crate::bundle::ResolvedItems,
    /// `(kind, name)` → set of `"kind:name"` requirers (provenance).
    pub required_by: BTreeMap<(ItemKind, String), BTreeSet<String>>,
}

/// Build a `(kind, effective-name) → item directory` index for `source`.
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

/// Read an item's `requires` block by re-parsing its entrypoint.
fn read_requires(dir: &Path, kind: ItemKind) -> Result<ItemRequires> {
    let entry = dir.join(kind.entrypoint_filename());
    let raw = fs::read_to_string(&entry).with_context(|| format!("read {}", entry.display()))?;
    let req = match kind {
        ItemKind::Skill => frontmatter::parse::<Skill>(&raw)?.0.requires,
        ItemKind::Rule => frontmatter::parse::<Rule>(&raw)?.0.requires,
        ItemKind::Agent => {
            let agent = frontmatter::parse::<Agent>(&raw)?.0;
            // preload-skills implies requires.skills (format-spec §3.4).
            let mut req = agent.requires;
            for s in &agent.preload_skills {
                if !req.skills.iter().any(|r| r.name() == s) {
                    req.skills.push(RequireRef::Name(s.clone()));
                }
            }
            req
        }
    };
    Ok(req)
}

pub(super) fn resolve_requires_closure(
    source: &Path,
    initial: &[(ItemKind, String)],
) -> Result<DependencyClosure> {
    let index = index_source(source)?;
    let mut required_by: BTreeMap<(ItemKind, String), BTreeSet<String>> = BTreeMap::new();
    let mut resolved: BTreeSet<(ItemKind, String)> = BTreeSet::new();
    let mut on_path: BTreeSet<(ItemKind, String)> = BTreeSet::new();
    let mut order: Vec<(ItemKind, String)> = Vec::new();

    // Depth-first so a back-edge to an item already on the current path is
    // a cycle. `initial` items have no requirer.
    fn visit(
        node: (ItemKind, String),
        requirer: Option<&str>,
        index: &BTreeMap<(ItemKind, String), PathBuf>,
        required_by: &mut BTreeMap<(ItemKind, String), BTreeSet<String>>,
        resolved: &mut BTreeSet<(ItemKind, String)>,
        on_path: &mut BTreeSet<(ItemKind, String)>,
        order: &mut Vec<(ItemKind, String)>,
    ) -> Result<()> {
        if let Some(r) = requirer {
            required_by.entry(node.clone()).or_default().insert(r.to_string());
        }
        if resolved.contains(&node) {
            return Ok(());
        }
        if !on_path.insert(node.clone()) {
            anyhow::bail!("circular requires detected at {} `{}`", node.0, node.1);
        }
        let dir = index.get(&node).ok_or_else(|| anyhow::anyhow!(
            "{} `{}` is required but not found in the source", node.0, node.1))?;
        let req = read_requires(dir, node.0)?;
        let label = format!("{}:{}", node.0, node.1);
        for (kind, refs) in [
            (ItemKind::Rule, &req.rules),
            (ItemKind::Skill, &req.skills),
            (ItemKind::Agent, &req.agents),
        ] {
            for r in refs {
                if let Some(src) = r.source() {
                    anyhow::bail!(
                        "{} `{}` requires {} `{}` from cross-source `{}`: \
                         cross-source dependency resolution is not yet available in this release",
                        node.0, node.1, kind, r.name(), src);
                }
                visit((kind, r.name().to_string()), Some(&label), index, required_by, resolved, on_path, order)?;
            }
        }
        on_path.remove(&node);
        resolved.insert(node.clone());
        order.push(node);
        Ok(())
    }

    for item in initial {
        visit(item.clone(), None, &index, &mut required_by, &mut resolved, &mut on_path, &mut order)?;
    }

    let mut items = crate::bundle::ResolvedItems::default();
    for (kind, name) in order {
        match kind {
            ItemKind::Rule => items.rules.push(name),
            ItemKind::Skill => items.skills.push(name),
            ItemKind::Agent => items.agents.push(name),
        }
    }
    Ok(DependencyClosure { items, required_by })
}
```

(If `crate::bundle::ResolvedItems` does not derive `Default`, add `#[derive(Default)]` to it — confirm its definition in `src/bundle.rs` first.)

- [ ] **Step 4: Run the closure tests green**

Run: `cargo test -p upskill closure_`
Expected: PASS (3 tests).

- [ ] **Step 5: Add `required_by` to the lockfile**

In `src/lockfile.rs`, add to `LockedItem` (after `source_name`):

```rust
/// `"kind:name"` of every installed item that pulled this one in as a
/// dependency. Empty for directly-requested items. Drives `doctor`'s
/// orphaned-dependency check; never triggers auto-removal (#196).
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub required_by: Vec<String>,
```

Update every `LockedItem { … }` literal in the crate (the `conflict.rs` test helper, `lockfile.rs` tests, `pipeline/mod.rs`) to add `required_by: vec![]` — the compiler lists them.

Change `items_from_report` to accept a provenance map and populate the field:

```rust
pub fn items_from_report(
    report: &InstallReport,
    source_label: &str,
    git_ref: Option<&str>,
    required_by: &std::collections::BTreeMap<(ItemKind, String), Vec<String>>,
    mut hash_for: impl FnMut(ItemKind, &str) -> Option<String>,
) -> Vec<LockedItem> {
    /* …existing dedupe loop… */
        out.push(LockedItem {
            kind: entry.kind,
            name: entry.name.clone(),
            source: source_label.to_string(),
            git_ref: git_ref.map(str::to_string),
            hash: hash_for(entry.kind, &entry.name),
            source_name: None,
            required_by: required_by.get(&(entry.kind, entry.name.clone())).cloned().unwrap_or_default(),
        });
    /* … */
}
```

Update the `items_from_report` unit test to pass `&BTreeMap::new()`.

- [ ] **Step 6: Wire the closure into `install_with_lockfile`**

In `src/pipeline/mod.rs`, after `scan_source_items` and before conflict detection, expand the requested set through the closure (skip when the source is a bundle file — bundles already resolve their own items):

```rust
let requested: Vec<(ItemKind, String)> = if items.is_empty() {
    scanned_items.iter().filter(|(_, n)| !options.excludes.contains(n)).cloned().collect()
} else {
    scanned_items.iter()
        .filter(|(_, n)| items.contains(n))
        .filter(|(_, n)| !options.excludes.contains(n))
        .cloned().collect()
};
let closure = if discovery::is_bundle_file(&local_source) {
    None
} else {
    Some(install::resolve_requires_closure(&local_source, &requested)?)
};
```

Use `closure`'s `items` as the install filter, and feed `required_by` to the lockfile. Where the code currently branches on `items.is_empty()` to call `install_from_local_path(.., None)` vs `install_with_name_resolution_from_local`, replace with a single filtered install when a closure exists:

```rust
let mut report = match &closure {
    Some(c) => install_from_local_path(&local_source, target, Some(&c.items))?,
    None => install_from_local_path(&local_source, target, None)?, // bundle path
};
```

(For the bundle-file branch the existing `install_bundle_file` dispatch inside `install_from_local_path` still runs.) Build the provenance map for the lockfile:

```rust
let required_by: std::collections::BTreeMap<(ItemKind, String), Vec<String>> = closure
    .as_ref()
    .map(|c| c.required_by.iter()
        .map(|(k, v)| (k.clone(), v.iter().cloned().collect()))
        .collect())
    .unwrap_or_default();
```

and pass `&required_by` to `items_from_report`.

Conflict detection: build `incoming` from `closure.items` (so a pulled dependency conflicting with an existing different-source install is also caught) instead of the current `scanned_items` filter. Keep the existing `--as`/alias handling intact for the directly-named items.

- [ ] **Step 7: Add the requires integration test**

Add `tests/cli_requires.rs`:

- Registry with agent `code-review` (`requires.skills: [sarif]`) and skill `sarif`. `upskill add ./reg code-review --claude` installs BOTH; the lockfile records `sarif` with `required_by: ["agent:code-review"]`.
- `preload-skills: [sarif]` on the agent (no explicit `requires`) also pulls `sarif`.
- A cross-source `{ name, source }` requires entry → `upskill add` exits non-zero mentioning "cross-source".

Use `tests/common` helpers and read the lockfile JSON to assert `required_by`. Run: `cargo test --test cli_requires` → PASS.

- [ ] **Step 8: Commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add -A
git commit -m "feat(pipeline): same-source requires closure with required_by provenance"
```

---

## Task 6: Co-location grouping + unit removal

**Files:**

- Modify: `src/lockfile.rs` — `LockedItem.group`
- Modify: `src/pipeline/install.rs` / `src/pipeline/mod.rs` — record `group` (source folder)
- Modify: `src/pipeline/lifecycle.rs:32-120` — `remove` acts on the co-located unit

- [ ] **Step 1: Write the failing removal integration test**

Add `tests/cli_colocation_remove.rs`: a single folder `markspec-trace/` holding `SKILL.md` (`name: markspec-trace`) and `RULE.md` (`name: markspec-trace-syntax`). `upskill add ./reg --claude` installs both. `upskill remove markspec-trace` removes BOTH the skill and the co-located rule (same source + folder group), and the lockfile is empty afterwards.

```rust
mod common;
use std::fs;

#[test]
fn remove_acts_on_colocated_unit() {
    let env = common::TestEnv::new();
    let item = env.project.join("reg/markspec-trace");
    fs::create_dir_all(&item).unwrap();
    fs::write(item.join("SKILL.md"),
        "---\nschema: 1\nname: markspec-trace\ndescription: s\n---\n\n## B\n\nt\n").unwrap();
    fs::write(item.join("RULE.md"),
        "---\nschema: 1\nname: markspec-trace-syntax\ndescription: r\n---\n\n## B\n\nt\n").unwrap();

    common::upskill_cmd(&env).args(["add", "./reg", "--claude"]).assert().success();
    common::upskill_cmd(&env).args(["remove", "markspec-trace"]).assert().success();

    assert!(!env.project.join(".claude/skills/markspec-trace/SKILL.md").exists());
    assert!(!env.project.join(".claude/rules/markspec-trace-syntax.md").exists(),
        "co-located rule must be removed with the unit");
}
```

Run: `cargo test --test cli_colocation_remove` → FAIL (only the named skill is removed).

- [ ] **Step 2: Add `group` to the lockfile**

In `src/lockfile.rs` `LockedItem`, add:

```rust
/// Source folder this item was discovered in — the co-location
/// grouping key (§2.1). `remove <name>` removes every item sharing the
/// same `(source, group)` so a co-located unit travels as one.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub group: Option<String>,
```

Add `group: None` (or the folder) to every `LockedItem` literal the compiler flags.

- [ ] **Step 3: Thread the folder through install → report → lockfile**

`InstalledItem` (in `report.rs`) needs the source folder. Add `pub group: Option<String>` to `InstalledItem`, set it in `install_items_of_kind` to `Some(folder.clone())`. In `items_from_report`, set `group` from the first report entry for each `(kind, name)` (capture it alongside the hash). Simplest: extend the dedupe loop to read `entry.group`.

- [ ] **Step 4: Make `remove` expand to the unit**

In `src/pipeline/lifecycle.rs` `remove`, after computing `to_remove` for `RemoveFilter::ByNames`, expand it to include every lockfile item sharing a `(source, group)` with a matched item:

```rust
if let RemoveFilter::ByNames(_) = &filter {
    let groups: std::collections::BTreeSet<(String, String)> = to_remove.iter()
        .filter_map(|i| i.group.clone().map(|g| (i.source.clone(), g)))
        .collect();
    for it in &lock.items {
        if let Some(g) = &it.group {
            if groups.contains(&(it.source.clone(), g.clone()))
                && !to_remove.iter().any(|r| r.kind == it.kind && r.name == it.name)
            {
                to_remove.push(it.clone());
            }
        }
    }
}
```

(Make `to_remove` `mut`. Keep the existing unknown-name error check on the _originally_ matched names, before expansion.)

- [ ] **Step 5: Run green**

Run: `cargo test --test cli_colocation_remove && cargo test -p upskill`
Expected: PASS. Fix any `LockedItem`/`InstalledItem` literal sites the compiler flags.

- [ ] **Step 6: Commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add -A
git commit -m "feat(pipeline): track co-location group; remove acts on the unit"
```

---

## Task 7: `doctor` orphaned-dependency flag

**Files:**

- Modify: `src/pipeline/report.rs` — `OrphanedDependency` + `DoctorReport.orphaned_dependencies`
- Modify: `src/pipeline/lifecycle.rs:135-252` — populate it
- Modify: `src/main.rs` — print the new bucket (find the existing doctor printer)

- [ ] **Step 1: Write the failing doctor test**

Add `tests/cli_doctor_orphan.rs`: install agent `code-review` + its required skill `sarif`; `upskill remove code-review` (the requirer) leaving `sarif` (whose `required_by: ["agent:code-review"]` now points at an absent item); `upskill doctor` reports `sarif` as an orphaned dependency. Assert via doctor's stdout (read `main.rs`'s doctor output format first). The orphaned-dependency bucket is **advisory** — like `skipped_plugins` it does NOT flip the exit code, so assert `doctor` still exits 0 but the message names `sarif`.

(Note: removing `code-review`, which is co-located only with itself, does not cascade to `sarif` because `sarif` lives in its own folder — §2.5 "removal never cascades". That is exactly the state doctor surfaces.)

- [ ] **Step 2: Add the report type**

In `src/pipeline/report.rs`:

```rust
/// A dependency-pulled item whose every requirer is no longer installed.
/// Advisory only — upskill never auto-removes (#196); the user decides.
#[derive(Debug, Clone, Serialize)]
pub struct OrphanedDependency {
    pub kind: ItemKind,
    pub name: String,
    /// The now-absent requirers (`"kind:name"`) recorded at install time.
    pub former_requirers: Vec<String>,
}
```

Add to `DoctorReport`:

```rust
/// Items present only as a dependency of an item that is no longer
/// installed. Advisory — does NOT affect `is_clean()`.
pub orphaned_dependencies: Vec<OrphanedDependency>,
```

`is_clean()` is unchanged (the new bucket is informational, like `skipped_plugins`). Re-export `OrphanedDependency` from `pipeline/mod.rs` if `report::*` is re-exported (it is — line 45 `pub use report::*;`).

- [ ] **Step 3: Populate it in `doctor`**

In `src/pipeline/lifecycle.rs` `doctor`, after the per-item loop, build the set of installed `(kind, name)` and flag any item whose `required_by` is non-empty and whose every requirer is absent:

```rust
let installed: std::collections::BTreeSet<String> = lock.items.iter()
    .map(|i| format!("{}:{}", i.kind, i.name)).collect();
for entry in &lock.items {
    if entry.required_by.is_empty() {
        continue;
    }
    if entry.required_by.iter().all(|r| !installed.contains(r)) {
        report.orphaned_dependencies.push(OrphanedDependency {
            kind: entry.kind,
            name: entry.name.clone(),
            former_requirers: entry.required_by.clone(),
        });
    }
}
```

Import `OrphanedDependency` in the `use super::{…}` list.

- [ ] **Step 4: Print the bucket in `main.rs`**

Find the doctor-report printer in `src/main.rs` (search for `orphan_entries` / `skipped_plugins`). Add a section that, when `!report.orphaned_dependencies.is_empty()`, prints each as e.g.:

```text
orphaned dependency: skill `sarif` (was required by agent:code-review, now absent)
  remove with `upskill remove sarif` if no longer needed
```

Keep it under the advisory (non-failing) part of the output, matching how `skipped_plugins` is presented.

- [ ] **Step 5: Run green**

Run: `cargo test --test cli_doctor_orphan && cargo test -p upskill`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
git add -A
git commit -m "feat(doctor): flag orphaned dependencies (advisory, never auto-removes)"
```

---

## Final: verify + PR

- [ ] **Step 1: Full verification**

```bash
cd .claude/worktrees/feat-coupling-redesign
just fmt
just verify
```

Expected: clean — zero warnings (compiler + clippy), all tests pass, docs tooling clean.

- [ ] **Step 2: Open the PR**

```bash
git push -u origin feat/coupling-redesign
gh pr create --base main --title "feat: coupling redesign slice 1 — optional naming, ignore, same-source requires" \
  --body "Implements ADR-0009 (amends ADR-0006): three-tier coupling model. Slice 1 = same-source. Adds optional/relaxed item naming (layout-independent identity), \`ignore\` copy-scope, same-source \`requires\` auto-install with \`required_by\` provenance, co-location grouping + unit removal, and a \`doctor\` orphaned-dependency flag. Cross-source resolution is Slice 2 (separate plan). See docs/superpowers/specs/2026-06-03-coupling-redesign-design.md.

🤖 Generated with [Claude Code](https://claude.com/claude-code)"
```

- [ ] **Step 3: After opening — fix CI first, then review comments.** Per AGENTS.md: K0 findings fixed in-PR; K1/K2 tracked as debt issues.

---

## Self-review (completed by plan author)

**Spec coverage:** §2.1 relaxed/optional naming → Tasks 1, 4. §2.2 three tiers → Tasks 1, 5 (+ co-location grouping Task 6). §2.3 `requires` → Tasks 1, 2, 5. §2.4 `ignore` → Tasks 1, 2, 3. §2.5 cross-source contract → Task 1 (doc) + Task 5 (errors on cross-source; resolution deferred to Slice 2 by design). §2.6 identity → Task 4. §3 inherent limit → Task 1 (ADR/spec). Lockfile `required_by` + group → Tasks 5, 6. doctor → Task 7. Strip-from-output → Task 2. **Slice 2 (cross-source resolution) is explicitly deferred to its own plan.**

**Placeholder scan:** No "TBD"/"implement later". Two NOTE callouts (tests/common API surface; borrow-checker on `name`) point the executor at a real file to read rather than hiding work — the _behavior_ is fully specified in each.

**Type consistency:** `effective_name`/`probe_effective_name` (discovery), `render_skill/rule/agent(item, name, body, client)`, `build_skill_frontmatter(skill, name, passthrough)`, `ItemRequires`/`RequireRef`, `DependencyClosure { items, required_by }`, `resolve_requires_closure(source, initial)`, `LockedItem.{required_by, group}`, `OrphanedDependency` — names used consistently across tasks.
