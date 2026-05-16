# Git-native registry standard — design

**Status**: Draft (2026-05-16)
**Epic**: #62 — v0.4 Registry standard
**Supersedes**: the Pages/`.well-known` approach described in issue #63

## Context

`upskill search` today is hardwired to the skills.sh API (`src/search.rs`,
`https://skills.sh/api/search`). v0.4 (#62) introduces _custom registries_:
teams hosting their own curated multi-kind content.

Issue #63 originally proposed publishing a CI-generated
`.well-known/agent-skills/index.json` to GitHub/GitLab Pages. Review found
this inadequate:

1. **`.well-known` is a misnomer.** RFC 8615 well-known URIs are
   host-root-relative (`https://{authority}/.well-known/{name}`). A project
   Pages site lives at `owner.github.io/repo/` — a path, not an authority —
   so `…/repo/.well-known/…` carries none of the RFC's semantics. It would
   only be valid if every registry had its own domain, which contradicts
   the "fork a template repo" goal.
2. **Skills-only.** An index built from `SKILL.md` frontmatter cannot
   represent rules, agents, or bundles — but upskill is multi-kind.
3. **Stale layout.** #63's `skills/{name}/SKILL.md` predates #133, which
   dropped kind subdirectories (kind is now the entrypoint filename).
4. **Private registries.** Pages for private repos requires GitHub
   Enterprise / GitLab paid tier.

Separately, the format-spec and #63 text drifted from what shipped (#133
flat layout, the #105 CLI-compliance changes) and need refining.

## Requirements

1. A registry is a **git repo** (public or private) holding multi-kind
   items (flat layout per #133) plus bundles.
2. Discovery must **not** depend on Pages or `.well-known`.
3. **skills.sh compatibility is minimal**: `upskill add`/`search` against
   skills.sh git sources keeps working unchanged (no regression).
   Publishing `npx skills`-compatible prebuilt content _from_ a registry
   is a **future feature, out of scope**.
4. `.upskill-registries.yaml` maps names → registries (#29);
   `add <registry> <item>` (#30) and cached search (#31) build on the
   chosen discovery mechanism.
5. Deliverable includes refining the stale format-spec + #63 text to
   match what shipped.

## Decision: generated, committed manifest with live-scan fallback

A registry author runs `upskill registry build`, which scans the repo
tree and writes a committed `.upskill-registry.json`. CI verifies it is
up to date (`--check`, the lockfile / `cargo fmt --check` pattern).
Consumers fetch that one file via the git provider's raw/contents API
(token auth for private). If a registry has **no** manifest, discovery
falls back to a live tree scan — so the manifest is the precomputed
result of the same scan, one code path with two triggers.

Rejected alternatives: a pure live-scan-only model (N API calls / clone
per search, unauth GitHub rate limit 60/h); Pages static index (private
tier, RFC misuse, two sources of truth).

## Files and naming

| File                          | Role                                            | Author                                                 | Location                   | Format                                   |
| ----------------------------- | ----------------------------------------------- | ------------------------------------------------------ | -------------------------- | ---------------------------------------- |
| `REGISTRY.md`                 | Registry identity                               | **Authored**                                           | registry root              | Markdown + YAML frontmatter, `schema: 1` |
| `.upskill-registry.json`      | Generated manifest (identity + items + bundles) | **Generated** by `upskill registry build`; CI-verified | registry root              | JSON, `schema: 1`                        |
| `.upskill-registries.yaml`    | Consumer map of named registries → URLs (#29)   | **Authored**                                           | consumer project root      | YAML                                     |
| `registry-<sha256(url)>.json` | Per-registry search cache, 1h TTL (#31)         | Generated                                              | `$XDG_CACHE_HOME/upskill/` | JSON                                     |

**`.upskill-registry.json`** mirrors `.upskill-lock.json`: flat
dotfile-at-root, `schema: 1`, deterministic pretty JSON, CI-verified
fresh. One predictable path consumers fetch via the provider API.

**`.upskill-registries.yaml`** is YAML, _not_ TOML or `.upskill/…`:

- The project depends on `serde`, `serde_json`, `serde_yaml_ng` — there
  is **no `toml` crate**. TOML would add a dependency to a project that
  gates dependencies behind an ADR (ADR-0001 §3).
- YAML is comment-friendly and hand-editable (TOML's real ergonomic
  win) at **zero new dependency**.
- **Every authored file in this project is already YAML** (all
  `RULE/SKILL/AGENT.md` + `*.bundle.md` frontmatter). Intra-project
  consistency beats the generic Cargo.toml-vs-lock analogy.
- Flat dotfile, not `.upskill/registries.yaml`: the project explicitly
  **struck** the `~/.upskill/` directory (ADR-0003 §4.2, issue #110 /
  PR #120). Authored = YAML, generated state = JSON keeps the
  authored-vs-generated signal intact.

`REGISTRY.md` matches the existing uppercase-entrypoint + YAML
frontmatter convention; `registry build` lifts its frontmatter into the
manifest header so identity has one source of truth.

## Schemas

### `REGISTRY.md` frontmatter

```yaml
---
schema: 1
name: platform-registry
description: Baseline rules, skills, agents for all repositories
maintainer: platform-dx
homepage: https://github.com/acme/registry   # optional
---
```

### `.upskill-registry.json`

```json
{
  "schema": 1,
  "registry": {
    "name": "platform-registry",
    "description": "Baseline rules, skills, agents for all repositories",
    "maintainer": "platform-dx",
    "homepage": "https://github.com/acme/registry"
  },
  "items": [
    {
      "name": "license-awareness",
      "kind": "rule",
      "path": "license-awareness",
      "description": "Flag unlicensed external code",
      "version": "1.2.0"
    }
  ],
  "bundles": [
    {
      "name": "platform-baseline",
      "description": "Baseline for all repos",
      "path": "platform-baseline.bundle.md"
    }
  ]
}
```

- `kind` ∈ `rule|skill|agent`, derived from the entrypoint filename
  (#133 flat layout).
- `path` is **repo-root-relative** (the resolved location), so a
  consumer can sparse-fetch it without knowing the registry's
  `<item-root>`.
- `version` from item `metadata.version` (nullable).
- `bundles` listed separately — they are manifests, not items.

### `.upskill-registries.yaml` (consumer)

```yaml
schema: 1
registries:
  platform:
    url: https://github.com/acme/registry
  community:
    url: https://gitlab.com/foo/skills
    ref: v2          # optional pin
```

## Data flow

**`upskill add <registry> <item>`**

1. Resolve `<registry>` via `.upskill-registries.yaml` → git URL (+ ref).
2. Fetch `.upskill-registry.json` via the provider raw/contents API
   (token auth for private; existing `auth.rs` resolution).
3. Look up `<item>` → `{kind, path}`. Unknown item → typed error.
4. Sparse-fetch that path (this is what debt #60 becomes) and generate
   per-client output exactly as today.
5. If no manifest present → live tree scan, then continue from step 3.

**`upskill search <query>`**

- The existing skills.sh path is unchanged (no regression).
- Additionally, for each configured registry: load cached
  `.upskill-registry.json` (cache key `sha256(url)`, 1h TTL — #31) or
  fetch; filter items by query; merge results labeled per registry.
- skills.sh remains one source; custom registries are additive.

## `upskill registry build`

`upskill registry build [--check]` (a `registry` subcommand group, room
for `validate`/`show` later):

- Scans the repo tree from `<item-root>`, reads each entrypoint's
  frontmatter, lifts `REGISTRY.md` frontmatter into the manifest header.
- Writes deterministic, sorted, pretty JSON to `.upskill-registry.json`.
- `--check` exits non-zero if the file is stale — identical guard to
  `cargo fmt --check` / the lockfile.
- The template registry repo ships a ~10-line CI workflow running
  `upskill registry build --check`.

## Scope

**In**: `REGISTRY.md`, generated `.upskill-registry.json`,
`upskill registry build [--check]`, `.upskill-registries.yaml`
resolution, manifest-driven `add`/`search` with live-scan fallback,
per-registry search cache.

**Out (explicit)**: Pages / `.well-known` anything; publishing
`npx skills`-compatible prebuilt content from a registry (**future
feature**); deeper skills.sh API integration beyond the existing search.

## Spec/docs refinement

- **New ADR-0007** "Git-native registry standard" — records: no
  Pages/well-known, generated+committed manifest, B-with-A-fallback,
  the naming decisions and their rationale.
- **`docs/format-spec.md`** — add a "Registry manifest" section after
  Bundles defining `.upskill-registry.json` + `REGISTRY.md`; scrub
  `skills/{name}/SKILL.md` examples implying kind subdirs (stale
  post-#133).
- **`docs/commands.md` / `docs/recipes.md`** — document
  `upskill registry build` and the named-registry workflow.
- **Issues** — rewrite #63 (drop Pages/well-known/index.json AC → this
  standard); update #29 to `.upskill-registries.yaml`; point #30/#31 at
  the manifest + cache; fold #60 in as the `add <registry> <item>`
  fetch mechanism.

## Testing

Per AGENTS.md (ATDD first, then TDD):

- **Integration** (`tests/cli_registry.rs`, new): `registry build`
  produces a deterministic manifest from a fixture registry;
  `--check` fails on a stale manifest; `add <registry> <item>` resolves
  via a local fixture registry (both with and without a committed
  manifest, exercising the live-scan fallback); `search` merges
  registry + skills.sh results without regressing the skills.sh path.
- **Unit**: manifest (de)serialization round-trip + `schema`
  validation; `.upskill-registries.yaml` parsing incl. ref pin and
  malformed input; cache key + TTL logic.
- Golden manifest fixture under `tests/fixtures/`.

## Future work (out of scope here)

- Publishing `npx skills`-compatible prebuilt content from an upskill
  registry, so skills.sh users can `npx skills add` curated upskill
  content directly.
