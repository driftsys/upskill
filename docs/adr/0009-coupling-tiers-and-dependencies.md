# Coupling tiers and directed dependencies

**Status**: Proposed (2026-06-03). Amends [ADR-0006](./0006-flat-item-layout.md).

## Context

Co-location — multiple entrypoints in one item directory sharing the directory
name ([ADR-0006](./0006-flat-item-layout.md),
[format-spec §2.1](../format-spec.md)) — is currently the only mechanism for
"install a rule + skill + agent together." Using co-location for that purpose
causes three failures (see the
[coupling redesign design](../superpowers/specs/2026-06-03-coupling-redesign-design.md)
§1):

- **Forced shared name.** All co-located kinds must share the folder name, even
  when their natural identities differ.
- **No copy scoping.** An item's whole supporting tree is copied with no way to
  exclude build artifacts, fixtures, or test scripts.
- **Ecosystem-invisible coupling.** Rules and agents do not exist in the Agent
  Skills standard, so a "these travel together" intent encoded by co-location
  cannot survive a round-trip through standard-only tooling.

Co-location is the right primitive for a _symmetric, inseparable_ unit. It is
the wrong primitive for a _directed_ "A needs B" relationship or a _curated,
optional_ set. This decision repositions co-location and adds the two missing
coupling tiers.

## Decision

### Three coupling tiers

upskill recognizes three non-overlapping coupling tiers:

| Tier                                 | Mechanism                                     | For                                           |
| ------------------------------------ | --------------------------------------------- | --------------------------------------------- |
| Symmetric unit (inseparable, mutual) | **co-location** — the folder _is_ the unit    | a mutual rule↔skill that must travel together |
| Directed prerequisite (A needs B)    | **`requires` / `preload-skills`** frontmatter | an item that needs an item in another unit    |
| Curated / optional set               | **bundles** (already exist)                   | a-la-carte packs                              |

A mutual dependency is expressed by **co-locating** (the folder is the
symmetric set), **never** by mutual `requires` — a `requires` cycle is an
error.

### Relaxed + optional naming

The model field `name` becomes optional for `Skill`, `Rule`, and `Agent`. The
**effective name** is resolved at discovery/parse time:

- **Skill**: `name` is optional. When present it MUST equal the folder name
  (the Agent Skills standard mandate). When absent, the effective name is the
  folder name.
- **Rule / Agent**: `name` comes from frontmatter and **MAY diverge** from the
  folder name. When absent, the effective name is the folder name.

Identity is `(kind, effective-name)` — layout-independent, never a path. The
folder name is the fallback default and the co-location grouping key; it
governs the skill when a skill is present, and otherwise serves only those two
roles. A co-located folder MAY hold **independently-named kinds** (e.g.
`markspec-trace/` with a skill named `markspec-trace` and a rule named
`markspec-trace-syntax`). A solo rule/agent whose frontmatter name diverges
from its folder is legal and emits **no lint diagnostic** — divergence is
silent, because identity is layout-independent.

### `requires` — directed dependencies

`requires` is a per-entrypoint frontmatter field common to all kinds (each item
owns its own edges — not a folder manifest, not skill-only). It mirrors the
bundle `items` vocabulary:

```yaml
requires:
  rules: [security-baseline]
  skills: [sarif-formatting]
  agents: []
```

- Dependencies are **hard** (auto-installed) and **acyclic** (a cycle is an
  error).
- Each `requires.<kind>` entry is a **bare string** (same-source, resolved by
  name) or a `{ name, source }` **map** (cross-source; `source` reuses the
  `upskill add` source DSL — `owner/repo@ref`, https, `gitlab:`, local).
- Resolution is by `(kind, name)`.
- **`preload-skills`** (agent) is a soft implies of `requires.skills` for skills
  **present in the same source**: those skills are auto-installed alongside the agent AND preloaded
  at startup. A preloaded skill absent from the same source is a runtime hint — neither
  auto-installed nor an error (unlike an explicit `requires` entry, which errors when missing).
  For GitHub Copilot and opencode (no native preload mechanism), the implementation renders
  the preload list as a prose `## Skills` section in the agent body.

### `ignore` — copy scoping

`ignore` is a per-entrypoint frontmatter field common to all kinds. It is
`.gitignore`-style and **subtractive only** — there is no `include`/allowlist
form, because an allowlist re-opens the under-copy footgun. A file matching any
`ignore` pattern is not copied; absent `ignore` means copy everything.
`ignore` is stripped from generated output.

```yaml
ignore: ["scripts/**", "fixtures/**"]
```

### Cross-source contract

- **`source`** reuses the `upskill add` DSL verbatim, parsed by the existing
  source machinery.
- **Conflict.** The same `(kind, name)` resolving to a different source/ref is
  an **error** — this reuses the existing same-name-different-source rule
  ([format-spec §3.7](../format-spec.md)). No version-range solving, no SAT.
- **Cycle detection** is keyed by `(canonical-source-label, kind, name)`.
- **Provenance.** A dependency-pulled item is recorded with its **own** source
  plus a **`required_by`** provenance list.
- **Removal never cascades** (the #196 "never auto-delete" ethos). `doctor`
  flags an item that is now present only as a removed item's dependency as an
  "orphaned dependency."
- **Staging.** Cross-source resolution ships in a follow-up release (Slice 2);
  same-source resolution ships now (Slice 1).

### Inherent limit

Co-located rules/agents are invisible to standard-only tooling: the Agent
Skills standard has no concept of rules or agents. The cross-kind "install
together" guarantee holds **only inside upskill** and cannot survive a
round-trip through the standard ecosystem. This is a property of the standard,
not a defect to engineer around.

## Consequences

**Positive.**

- The three co-location failures are resolved: directed coupling no longer
  forces a shared name (`requires` resolves by `(kind, name)` across folders);
  copy scoping is available via `ignore`; and a directed "A needs B" intent is
  carried by an explicit field rather than implied by folder membership.
- Co-location stays the right primitive for a symmetric, inseparable unit —
  repositioned, not removed.
- Rule/agent identity becomes layout-independent, so authors can name kinds
  naturally and group them in folders for convenience without coupling the two
  concerns.
- The cross-source contract reuses the established `add` DSL and the existing
  same-name-different-source conflict rule, adding no new resolution machinery.

**Negative / limits.**

- The inherent-limit note stands: cross-kind coupling is an upskill-only
  guarantee and does not survive a round-trip through standard-only tooling.
- Cross-source resolution is staged to a later release, so the full transitive
  closure across sources is not available in Slice 1.

## Alternatives considered (Rejected — do not revisit)

Lifted from the
[design spec §4](../superpowers/specs/2026-06-03-coupling-redesign-design.md):

- **Reference-aware / traced copy** — under-copies transitive deps hidden
  inside scripts/binaries (validated against Vercel NFT and
  vercel-labs/skills#810).
- **`include`/allowlist form in `ignore`** — same under-copy footgun;
  subtractive only.
- **Folder-file dependency manifest** — overlaps bundles, is
  ecosystem-invisible, and adds a third coupling construct.
- **Dependencies declared only in the skill** — skill-less units exist, so the
  skill is the wrong owner.
- **Path-based item references** — layout-brittle and conflicts with
  name-based identity.
- **Dropping co-location entirely** — it is the right primitive for symmetric
  units; reposition, don't delete.
- **`repo#name` single-token selector** — `#` collides with git/npm ref
  semantics and is a shell comment; positional name + `{ name, source }` cover
  it.
- **Naming the copy-scope field `resources`** — overloads the standard's
  vocabulary; use `ignore`.
- **Requiring rule/agent name to match the folder** — contradicts
  independently-named co-located kinds.
- **Lint warning on solo rule/agent name divergence** — identity is
  layout-independent, so divergence is silent.

## Migration

Per [no back-compat until 1.0](../../AGENTS.md) and consistent with
[ADR-0006](./0006-flat-item-layout.md)'s migration section, this is a pre-1.0
hard cut. New SSOT-only fields (`requires`, `ignore`) and the relaxed naming
rule are read by the new release only — no back-compat shim, no fallback
fields, no dual-support window.

## References

- Amends: [ADR-0006](./0006-flat-item-layout.md) — flat item layout.
- Authoritative spec: [`docs/format-spec.md`](../format-spec.md) §§2.1, 2.4,
  3.1, 3.4, 3.7, 3.8, 11.
- Design source:
  [coupling redesign design](../superpowers/specs/2026-06-03-coupling-redesign-design.md).
