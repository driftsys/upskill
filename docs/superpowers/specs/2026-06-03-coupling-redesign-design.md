# Design — co-location → dependency redesign

**Date**: 2026-06-03
**Status**: Approved (brainstorming output) — implementation pending
**Amends**: [ADR-0006](../../adr/0006-flat-item-layout.md)
**Will produce**: ADR-0009 + `format-spec.md` edits + two implementation slices

---

## 1. Problem

Co-location — multiple entrypoints in one folder sharing the directory name
([format-spec §2.1](../../format-spec.md), [ADR-0006](../../adr/0006-flat-item-layout.md))
— is currently the only way to "install a rule + skill + agent together."
Using it for that purpose causes three failures:

- **Forced shared name.** All co-located kinds must share the folder name, even
  when their natural identities differ.
- **Resource duplication / no copy scoping.** An item's whole supporting tree is
  copied with no way to exclude build artifacts, fixtures, or test scripts.
- **Ecosystem-invisible coupling.** Rules and agents do not exist in the Agent
  Skills standard, so a "these travel together" intent encoded by co-location
  cannot survive a round-trip through standard-only tooling.

Co-location is the right primitive for a _symmetric, inseparable_ unit. It is
the wrong primitive for a _directed_ "A needs B" relationship or a _curated_
optional set. This redesign repositions co-location and adds the two missing
tiers.

## 2. Decided model

### 2.1 Three non-overlapping coupling tiers

| Tier                                 | Mechanism                                       | For                                         |
| ------------------------------------ | ----------------------------------------------- | ------------------------------------------- |
| Symmetric unit (inseparable, mutual) | **co-location** — the folder _is_ the unit      | mutual rule↔skill that must travel together |
| Directed prerequisite (A needs B)    | **`requires:` / `preload-skills:`** frontmatter | an item needs an item in another unit       |
| Curated / optional set               | **bundles** (already exist)                     | a-la-carte packs                            |

A mutual dependency is expressed by **co-locating** (the folder is the symmetric
set), **never** by mutual `requires` (which would be a cycle = error).

### 2.2 Name resolution — relaxed + optional naming

The model field `name` becomes optional for `Skill`, `Rule`, and `Agent`
(`name: String` → `name: Option<String>`). The **effective name** is resolved
at discovery/parse time:

| Kind             | `name` present in frontmatter                                                | `name` absent                |
| ---------------- | ---------------------------------------------------------------------------- | ---------------------------- |
| **Skill**        | MUST equal the folder name (Agent Skills standard mandate) — else lint error | effective name = folder name |
| **Rule / Agent** | effective name = the frontmatter name (**MAY diverge** from the folder)      | effective name = folder name |

Consequences:

- The **folder name** governs the **skill** when one is present, and otherwise
  serves only as the **fallback default** and the **co-location grouping key**.
- **Rule/agent identity is layout-independent.** A solo rule/agent whose
  frontmatter name diverges from its folder is legal and emits **no lint
  diagnostic** (decided: silent — identity is `(kind, name)`, never a path).
- A co-located folder MAY hold **independently-named kinds**: e.g.
  `markspec-trace/` with a skill named `markspec-trace` (folder fallback) and a
  rule named `markspec-trace-syntax` (diverges).
- **Generation always emits a complete name.** `build_skill_frontmatter` (and
  the rule/agent equivalents) read the resolved effective name, so generated
  client output is always standard-complete even when the SSOT omits `name`.
- **Bundles are unchanged**: `name` stays **required** and matches the filename
  stem. This relaxation does not apply to bundles.

### 2.3 `requires:` — directed dependencies (common to all kinds)

Per-entrypoint (each item owns its own edges — NOT a folder manifest, NOT
skill-only). Mirrors the bundle `items` vocabulary. **Hard** (auto-installed),
**acyclic** (cycle = error). Resolved by `(kind, name)`.

```yaml
requires:
  rules: [security-baseline]
  skills: [sarif-formatting]
  agents: []
```

- Each entry is **string-or-map** (serde untagged):
  - bare string → same source, by name;
  - `{ name, source }` → cross-source (`source` reuses the `add` DSL).
- **`preload-skills`** (agent) = `requires.skills` **plus** the Claude-Code
  runtime `skills:` emission. Preload implies require.
- **Same-source resolution** (decided): a same-source `requires` entry resolves
  by `(kind, name)` against the **same already-fetched registry root** the
  entry item came from (reusing `has_matching_items` / `iter_item_dirs`), then
  auto-installs it. No second fetch. This mirrors how bundle `items` already
  resolve.

### 2.4 `ignore:` — copy scoping (common to all kinds)

`.gitignore`-style, **subtractive only**. No `include`/allowlist form — an
allowlist re-opens the under-copy footgun that reference-tracing also has.
Default absent = copy everything. Author-declared, stripped from generated
output.

```yaml
ignore: ["scripts/**", "fixtures/**"]
```

Filters the result of `iter_item_resources` before the copy step.

### 2.5 Cross-source resolution (contract fixed now; slice 2)

- `source` reuses the `add` DSL verbatim (`owner/repo@ref`, https, `gitlab:`,
  local), parsed by `source.rs` (`InstallSource`), fetched by `fetch_ssot`
  (same auth path).
- The transitive closure MAY span sources.
- **Cycle detection** keyed by `(canonical-source-label, kind, name)`.
- **Conflict policy:** the same `(kind, name)` resolving to a different
  source/ref is an **error**, reusing the existing `conflict.rs` rule
  (format-spec §3.7). No version-range solving, no SAT.
- Dependency-pulled items are recorded in the lockfile with their **own**
  `source` plus a **`required_by`** provenance list.
- **Removal does NOT auto-cascade** (the #196 "never auto-delete" ethos).
  `doctor` flags items now installed only as a removed item's dependency
  ("orphaned dependency").

### 2.6 Identity

`(kind, name)`, never a path. On `add`: name = the positional argument
(`upskill add owner/repo code-review`); `:path` is only "where to scan"; `@ref`
is the version. Cross-source = source-locator + name, never a foreign
filesystem path.

## 3. Inherent limit (document, do not fix)

Co-located rules/agents are invisible to standard-only tooling (the Agent
Skills standard has no concept of rules or agents). The "install
together" / cross-kind-coupling guarantee holds **only inside upskill** and
cannot survive a round-trip through the standard ecosystem. This is a property
of the standard, not a defect to engineer around.

## 4. Rejected alternatives (do not revisit)

- **Reference-aware / traced copy** — under-copies transitive deps inside
  scripts/binaries (validated against Vercel NFT + vercel-labs/skills#810).
- **`include`/allowlist in `ignore`** — same under-copy footgun; subtractive
  only.
- **Folder-file manifest for deps** — overlaps bundles, ecosystem-invisible,
  third construct.
- **Deps "just in the skill"** — skill-less units exist; wrong owner.
- **Path-based item references** — layout-brittle; conflicts with name identity.
- **Dropping co-location entirely** — it is the right primitive for symmetric
  units; reposition, don't delete.
- **`repo#name` single-token selector** — `#` collides with git/npm ref
  semantics and is a shell comment; positional name + `{name, source}` cover it.
- **Naming the copy-scope field `resources`** — overloads the standard's
  vocabulary; use `ignore`.
- **Requiring rule/agent name to match the folder** — contradicts
  independently-named co-located kinds (§2.2). Rejected in brainstorming.
- **Lint warning on solo rule/agent name divergence** — rejected in
  brainstorming; identity is layout-independent, so divergence is silent.

## 5. Touch points in the current code (`origin/main`)

- `src/model/{skill,rule,agent}.rs` — `name` → `Option<String>`; add `requires`,
  `ignore` (skip-serialize-if-empty); strip both at generation.
- `src/parse/frontmatter.rs` — unchanged mechanics; new fields ride the model.
- `src/pipeline/discovery.rs` — `iter_item_dirs` keeps returning folder name;
  add an effective-name resolver that reads the entrypoint frontmatter; track
  folder-group membership.
- `src/pipeline/install.rs` — use the **effective name** (not folder name) for
  output paths, link-rewrite namespace, lockfile identity; apply the `ignore`
  filter to `iter_item_resources`; resolve + install the same-source `requires`
  closure; record `required_by`.
- `src/generate/mod.rs` (+ `claude.rs`/`copilot.rs`/`opencode.rs`) — read the
  effective name; strip `requires`/`ignore` (alongside `schema`/`metadata`/
  `license`).
- `src/lockfile.rs` — add folder-group membership + `required_by` provenance to
  `LockedItem`.
- `src/pipeline/lifecycle.rs` — `doctor` orphaned-dependency flag; `remove`
  acts on the folder-group unit.
- `src/conflict.rs` — reuse for cross-source `(kind, name)` conflict (slice 2).
- `src/lint.rs` — relax `check_name_matches_dir`: skill keeps the match rule;
  rule/agent name is free; folder is fallback when name is absent.

## 6. Deliverables & staging

1. **ADR-0009** amending ADR-0006: repositioning + relaxed/optional naming +
   three-tier model + `requires`/`ignore` + cross-source + conflict/cycle +
   the inherent-limit note. (Produced in Slice 1's PR.)
2. **format-spec edits**: §2.1 (optional/relaxed naming for skill vs
   rule/agent), §2.4 (`ignore` copy-scope), §3.1 (`requires` + `ignore` common
   fields), §3.4 (`preload-skills` implies `requires`), §3.7 (item-`requires`
   cycle + conflict, reusing the same-name-different-source rule), §3.8
   (canonical key order: add `requires`, `ignore`), §11 (mark "multi-repo item
   sources" RESOLVED). (Produced in Slice 1's PR.)
3. **Implementation (TDD), two slices, one PR each, off `origin/main`:**
   - **Slice 1 (same-source):** optional/relaxed naming + folder-group tracking
     - `ignore` filter + same-source `requires` closure + lockfile
       group/`required_by` + `doctor` orphaned-dependency flag. Ships the ADR +
       format-spec edits.
   - **Slice 2 (cross-source, additive):** wire `{name, source}` through
     `fetch_ssot`; cross-source cycle/conflict keyed by the canonical source
     label.

## 7. Conventions

- **Pre-1.0:** delete deprecated shapes outright — no back-compat, no migration
  shims, no fallback fields.
- All new SSOT-only fields (`requires`, `ignore`) stripped from generated client
  output.
- TDD (failing test first), Conventional Commits, `just fmt` then `just verify`
  before PR, squash-merge.
- **Test isolation:** integration tests MUST use `tests/common::upskill_cmd`
  with an isolated fake `$HOME` + a `.git` marker in the project dir (issue
  #193) — `upskill add` defaults to global `$HOME` scope outside a git repo and
  will otherwise pollute the developer's real `~`.
- Each branch in its own worktree under `.claude/worktrees/<branch>`; run
  `./bootstrap` after `git worktree add`; use full worktree-prefixed paths for
  edits.
