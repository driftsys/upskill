# Design: copy item supporting resources into rendered output (#199)

**Date**: 2026-06-02
**Issue**: [#199](https://github.com/driftsys/upskill/issues/199) — `bug(add)`: item
supporting resources (sibling files) are not copied into rendered output
**Status**: Approved (brainstorming)

## Problem

`upskill add` renders only an item's **entrypoint** (`SKILL.md` / `RULE.md` /
`AGENT.md`) and silently drops every other file in the item directory. Any skill,
rule, or agent that ships a helper script or reference file is delivered broken —
the entrypoint body references a file that was never installed.

This violates the portable format specification, which is unambiguous:

- **format-spec §2.4**: "Implementations MUST preserve supporting files and their
  directory structure during generation (copy them alongside the generated
  entrypoint into the client-expected location)."
- **format-spec §9 (Conformance #6)**: "It preserves supporting files from item
  directories in generated output per §2.4."

So this is a **conformance bug**, not a feature request. The motivating real case
is a _rule_: the metapowers `working-memory-lifecycle` rule ships a `wip-gate.sh`
CI script that never reaches consumers.

### Root cause

The install path reads only the entrypoint file and never enumerates the item
directory. In `pipeline.rs`, `install_skills` / `install_rules` / `install_agents`
each do `dir.join("SKILL.md")` (etc.), parse its frontmatter, render, and write —
with no `fs::read_dir(dir)` of the item directory. There is no code path that
copies non-entrypoint files.

## Prior art considered

- **Anthropic Agent Skills** (agentskills.io) — a skill _is_ a folder; `SKILL.md`
  plus optional `scripts/`, `references/`, `assets/` travel together verbatim,
  referenced by relative markdown links. No static analysis: the directory is the
  unit of distribution. Our format-spec §2.4 is modelled directly on this.
- **Vercel / Next.js Output File Tracing** (`@vercel/nft`) — static analysis of
  `import`/`require`/`fs` to compute the exact closure of referenced files, with
  `outputFileTracingIncludes/Excludes` escape hatches. Powerful, but built for a
  JS import graph we do not have; our bodies use plain markdown relative links.

We adopt the **Anthropic copy-the-tree model**: it is what the spec mandates
("preserve supporting files **and their directory structure**"), and it needs no
link-graph machinery.

## Design

### Placement — every resource tree is namespaced by item name

After rendering an item's entrypoint for a target client, copy every
non-entrypoint, non-override file from the source item directory (preserving
sub-directory structure) into a per-item namespace within that client's output.

| Kind / client       | Entrypoint output                             | Resource output                      | Body rewrite |
| ------------------- | --------------------------------------------- | ------------------------------------ | ------------ |
| Skill (all clients) | `.../skills/<name>/SKILL.md`                  | `.../skills/<name>/<tree>`           | No           |
| Rule (opencode)     | `.agents/rules/<name>/RULE.md`                | `.agents/rules/<name>/<tree>`        | No           |
| Rule (Claude)       | `.claude/rules/<name>.md`                     | `.claude/rules/<name>/<tree>`        | **Yes**      |
| Rule (Copilot)      | `.github/instructions/<name>.instructions.md` | `.github/instructions/<name>/<tree>` | **Yes**      |
| Agent (Claude)      | `.claude/agents/<name>.md`                    | `.claude/agents/<name>/<tree>`       | **Yes**      |
| Agent (Copilot)     | `.github/agents/<name>.agent.md`              | `.github/agents/<name>/<tree>`       | **Yes**      |
| Agent (opencode)    | `.opencode/agents/<name>.md`                  | `.opencode/agents/<name>/<tree>`     | **Yes**      |

where `<tree>` is the resource's path relative to the source item directory
(e.g. `scripts/gate.sh`, `references/patterns.md`).

**Two layout families:**

- **Directory-backed kinds** — every skill, and opencode rules — already render
  their entrypoint _inside_ a `<name>/` directory. Resources land beside it and the
  body's relative links (`./scripts/gate.sh`) resolve unchanged. **No rewrite.**
- **Flat kinds** — Claude/Copilot rules and all agents — render their entrypoint as
  a single flat file (`<name>.md`) with no directory. Resources go into a sibling
  namespace directory `<name>/`, and the entrypoint body's relative resource links
  are **rewritten** to prefix `<name>/`. The flat file `<name>.md` and the sibling
  `<name>/` directory coexist without conflict.

### Collisions — eliminated by construction

Because every resource tree is namespaced by the item's `<name>` within its output
directory, and item names are unique per kind, two items can never write the same
resource path. No collision detection, no clobbering, no warn/error policy needed.

### Body link rewrite (flat kinds only)

The rewrite runs **only** for a flat-kind item that actually ships resources. It is
deliberately narrow.

- Parse the rendered body with **pulldown-cmark** (already a dependency) so that
  link/image destinations are identified structurally and **code spans / fenced
  code blocks are never modified**.
- Rewrite a destination **only if** it is a relative path that resolves, relative
  to the source item directory, to a file that was actually copied. Prefix such
  destinations with `<name>/`.
- **Leave untouched:** absolute URLs (`http:`, `https:`, `mailto:` …), absolute
  filesystem paths, bare fragments (`#section`), and any `../`-prefixed target that
  escapes the item directory.
- **Preserve** a trailing `#fragment` and any `"title"` suffix on the link;
  rewrite only the path portion.

Ordering: the rewrite is applied to the resolved body **before** the final dprint
formatting pass, so output stays idempotent under the formatter (§7.4).

### Excluded from copying

- Entrypoint files: `SKILL.md`, `RULE.md`, `AGENT.md` (already rendered).
- Per-client override files: `*.claude.md`, `*.copilot.md`, `*.opencode.md`
  (§2.3 — SSOT-only, never emitted as-is).

Everything else in the item directory is copied byte-for-byte with its
sub-directory structure preserved. (Override files are matched by the
`<KIND>.<client>.md` pattern so an unrelated resource that merely ends in
`.claude.md` is still copied — the exclusion is anchored to entrypoint stems.)

### Copy fidelity

Resources are content, not generated markdown: they are copied verbatim and
MUST NOT pass through dprint or any markdown transform — including resource files
that happen to be `.md` (e.g. `references/patterns.md`). On Unix the copy MUST
preserve the file mode, so a `scripts/gate.sh` arrives **executable**; a script
delivered without its execute bit is as broken as one not delivered at all.

### Removal & drift safety

- The lockfile records each copied resource's **output path**, per item, alongside
  the entrypoint output paths it already tracks. `upskill remove` then deletes
  exactly that item's resource tree — no orphans, no over-deletion.
- `hash_item_dir` already hashes the entire source item directory (all files), so
  `update` / `doctor` drift detection already accounts for resource changes; no
  change needed there.

## Components touched

- `pipeline.rs` — `install_skills` / `install_rules` / `install_agents` (or their
  unified successor): after writing the entrypoint, enumerate item-directory
  resources, compute per-client output paths, copy them, and record paths in the
  install report → lockfile.
- `generate/` — a new resource-link-rewrite step in the body-generation path for
  flat-kind items, gated on the item having ≥1 resource.
- `lockfile.rs` — extend the per-item entry to carry resource output paths
  (`schema: 1`; additive field, no version bump — see below).
- `pipeline.rs` removal path — delete recorded resource paths on `remove`.

## Lockfile shape

Add an optional `resources: [<output-path>, ...]` array to each installed item's
entry. Absent ⇒ no resources (backward-compatible with existing `schema: 1`
lockfiles; no schema bump). Pre-1.0 we do not add migration shims — old lockfiles
simply have no `resources` key and read as empty.

## Testing (ATDD → TDD)

Acceptance criteria are the §2.4 / §9-#6 conformance contract. Tests are written
**first** (failing), then the implementation makes them pass. The link-rewrite is
the highest-risk unit and gets the densest coverage.

### Unit (TDD) — the rewrite is exhaustively table-tested

**Link rewrite** (`generate/`) — a single table test feeding `(body, copied-set,
name) → expected body`, one row per case:

_Rewritten_ (relative path resolving to a copied resource, prefixed with `<name>/`):

- inline link `[t](./scripts/gate.sh)`
- inline link without `./` → `[t](scripts/gate.sh)`
- image `![alt](./assets/logo.png)`
- reference definition `[id]: ./scripts/gate.sh` (with `[t][id]` usage)
- link with title `[t](./scripts/gate.sh "run it")` → path rewritten, title kept
- link with fragment `[t](./references/patterns.md#section)` → path rewritten,
  `#section` kept

_Left untouched_:

- absolute URLs `https://…`, `http://…`, `mailto:…`
- absolute filesystem path `/usr/local/bin/x`
- bare fragment `#section`
- `../escapes/out.sh` (target escapes the item directory)
- a relative link whose target was **not** copied (e.g. points at a sibling item
  or a nonexistent file) — only resolving-to-a-copied-resource links are rewritten
- a link string appearing inside an **inline code span** `` `./scripts/gate.sh` ``
- a link string appearing inside a **fenced code block**

_Properties_:

- **idempotency** — rewriting an already-rewritten body is a no-op (no
  double-prefix)
- **no-resources fast path** — body returned byte-identical when the copied set is
  empty (and the pass is skipped entirely)

**Resource enumeration** (`pipeline.rs`):

- excludes `SKILL.md` / `RULE.md` / `AGENT.md`
- excludes override files `*.{claude,copilot,opencode}.md` matched by entrypoint
  stem; **includes** a same-suffix non-override like `notes.claude.md`
- includes nested trees (`scripts/`, `references/sub/`), preserving structure
- empty set when the directory holds only an entrypoint

**Per-client resource output-path computation** — directory-backed (skill,
opencode-rule) vs flat (Claude/Copilot rule, all agents); asserts the namespace
segment and the rewrite-needed flag per (kind, client).

**Lockfile** (`lockfile.rs`):

- round-trip of an item entry with a `resources` array
- reads a pre-existing entry **without** the key as empty resources (no schema
  bump)

### Integration (ATDD, `tests/`) — observable CLI behavior

Add fixtures under `tests/fixtures/` for one skill, one rule, and one agent, each
shipping `scripts/<x>.sh` + `references/notes.md`, plus a body that links them.

1. **Skill, all clients** — files land at `.../skills/<name>/scripts/<x>.sh` and
   `.../references/notes.md`; body unchanged; `scripts/<x>.sh` is executable.
2. **Rule, motivating case** — for Claude: `.claude/rules/<name>.md` flat **and**
   `.claude/rules/<name>/scripts/gate.sh`, body link rewritten to
   `./<name>/scripts/gate.sh`; for Copilot the analogous
   `.github/instructions/<name>/…`; for opencode `.agents/rules/<name>/scripts/gate.sh`
   with the body **un**rewritten.
3. **Agent, all clients** — namespaced copy + rewrite for Claude/Copilot/opencode
   (all flat).
4. **`audience` scoping** — an item targeting only `claude` copies resources for
   Claude and for no other client.
5. **Co-located multi-kind item** (§2.1, `SKILL.md` + `AGENT.md` sharing
   `references/`) — resources are copied alongside **each** emitted entrypoint.
6. **Bundle install** — a bundle-sourced item (the real metapowers shape) copies
   its resources, exercising the `add <owner/repo:bundle.yaml>` path, not only
   local `add`.
7. **Idempotency** — running `add` twice yields byte-identical output and no
   lockfile churn.
8. **`remove`** — deletes the resource tree and prunes the now-empty namespace
   directory; `doctor` reports clean afterward.
9. **`update` reconciliation** — adding a resource file in source then `update`
   copies it; deleting a resource in source then `update` removes the stale output
   file (guarding the #196-class "stale output" failure mode).

### Formatting guarantee (§7.4)

A test asserts a rewritten flat-kind body is idempotent under dprint and passes
the pinned markdownlint ruleset — the rewrite must not produce output that the
formatter would then change.

### Golden fixtures

Store expected output trees under `tests/fixtures/` so the per-client layout,
rewritten bodies, and verbatim resource contents are diffable in review.

## Out of scope

- Static link-graph tracing (Vercel-style) — not needed; copy-the-tree is the spec
  model.
- Changing client entrypoint output paths (rules stay flat per §7) — only resource
  placement is added.
- Validating that body links actually point at existing resources (lint concern,
  separate follow-up).
