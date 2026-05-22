# Bundle-by-name Discovery

Date: 2026-05-22

## Problem

`upskill add owner/repo bundle-name` only searches for items
(skills/rules/agents). Bundle files (`.bundle.yaml`) are only installed
when the source path literally ends in `.bundle.yaml`. There is no
ergonomic way to install a bundle by name from a remote repo.

Current workaround: specify the full path to the bundle file:

```bash
upskill add driftsys/upskill:skills/prompt-engineering.bundle.yaml
```

This is not discoverable and forces users to know the internal layout.

## Design

### Bundle-by-name resolution

**Trigger:** `upskill add <source> <name>` (positional item-filter arg
only). No change to the `:subfolder` colon syntax — that remains a
literal path.

**Resolution order for each positional name:**

1. Search for items (skills/rules/agents) matching the name (existing
   behavior).
2. Search for `<name>.bundle.yaml` files anywhere in the source
   directory (recursive walk).
3. Dispatch based on results:
   - **Both exist** — error listing both matches, suggest `:path`
     disambiguation.
   - **Only bundle** — use bundle dispatch (existing
     `install_bundle_file` path).
   - **Only items** — install items (existing behavior).
   - **Neither** — error (existing "no matching items" message).

**Bare `upskill add owner/repo`** (no positional names) keeps current
behavior: install all items, ignore bundles. Bundles are opt-in by
name.

### Lint rule: bundle-item-name-collision

`upskill lint` gains a new check at **error** severity. If a bundle's
`name:` field collides with any item directory name (skill, rule, or
agent) in the same registry root, emit an error.

This is a static guardrail that prevents the ambiguity case from ever
reaching end users. Registries that follow the convention "bundles and
items have distinct names" will never trigger the install-time
ambiguity error.

### Convention

Registries SHOULD use distinct names for bundles and items:

- `prompt-engineering` — bundle (references items + plugins + deps)
- `prompt-design` — skill (the actual SSOT content)

This makes `upskill add owner/repo prompt-engineering` unambiguous.

## Error messages

### Install-time ambiguity

```
error: 'foo' matches both a skill and a bundle

  skill:  skills/foo/SKILL.md
  bundle: bundles/foo.bundle.yaml

Disambiguate with the full path:
  upskill add owner/repo:bundles/foo.bundle.yaml
  upskill add owner/repo:skills/foo
```

### Lint collision

```
error[name-collision]: bundle 'foo' collides with skill 'foo'
  --> bundles/foo.bundle.yaml
  = help: rename the bundle or item so names are unique
```

## Implementation scope

### pipeline.rs

- Add `find_bundle_by_name(source_dir: &Path, name: &str) ->
  Option<PathBuf>` — walks the source tree for
  `<name>.bundle.yaml`.
- In `install_with_lockfile`, when positional `items` are provided:
  after cloning, for each name check both item existence and bundle
  existence. Dispatch accordingly or error on ambiguity.
- The `install_from_local_path` function's existing item-filter path
  is unchanged; the new bundle-by-name logic sits above it at the
  `install_with_lockfile` / `install_from_source` level.

### lint.rs

- Add `check_bundle_item_name_collision` — iterates discovered bundles
  and item directories, reports collisions.
- Severity: error (not warning). This is a hard rule for registry
  authors.

### No changes to

- `source.rs` — `:subfolder` parsing untouched.
- `fetch.rs` — `resolve_subfolder` untouched.
- Lockfile schema — no new fields needed.

## Testing

### Pipeline tests

- `upskill add <local-path> bundle-name` where only a bundle matches
  → bundle dispatch fires, items from bundle are installed.
- `upskill add <local-path> item-name` where only an item matches →
  existing behavior preserved.
- `upskill add <local-path> ambiguous-name` where both exist → error
  with disambiguation message.
- `upskill add <local-path> unknown-name` → existing "no matching
  items" error.

### Lint tests

- Registry with colliding bundle/item name → error finding.
- Registry with distinct names → clean.

### Integration (CLI) tests

- `upskill add owner/repo bundle-name` (via local git clone) →
  success, lockfile records bundle.
