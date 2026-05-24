# Item Conflict Resolution & Source Locking

**Date**: 2026-05-24
**Status**: Draft
**Scope**: `upskill add`, bundle install, lockfile, alias mechanism

---

## Problem

When two source registries publish items with the same `(kind, name)`, consumers
hit conflicts with no graceful resolution path. Today:

- Direct `upskill add` silently overwrites an existing item from a different source.
- Bundle resolution hard-errors on `(kind, name)` duplicates within the dependency
  graph, but offers no resolution mechanism.
- There is no way to keep two items with the same short name from different sources.

This leads to accidental overrides, unintended shadowing, and poor DX when
composing content from multiple registries.

## Design Principles

1. **Short names are the only user-facing handle.** Agents trigger skills by short
   name; users type `/skill-name`. No namespaces, scopes, or qualified IDs in
   daily usage.
2. **Locked by default.** Once an item is installed, its `(kind, name)` is bound to
   its source. Replacement from a different source requires explicit opt-in.
3. **Deterministic behavior.** No interactive prompts — errors are clear and
   actionable, with flags to resolve. Works identically in CI and local.
4. **Aliases for coexistence.** When two items with the same upstream name are both
   needed, one is installed under an alias.

## Design

### 1. Source Affinity (Implicit Locking)

Every `LockedItem` in `.upskill-lock.json` already records a `source` field. This
field becomes the lock: any subsequent `upskill add` that would write a `(kind,
name)` already present in the lockfile from a **different source** is an error.

No new lockfile fields are needed for locking. The existing `source` field is
sufficient.

### 2. Conflict Detection

On `upskill add <source>`, after resolving which items the source provides:

1. For each resolved `(kind, name)`, check the lockfile.
2. If the `(kind, name)` exists and `source` matches → this is an **update**, proceed
   normally.
3. If the `(kind, name)` exists and `source` differs → **conflict error**.

Error message:

```
Error: skill `brainstorming` is already installed from `driftsys/superpowers`.
       Use --force to replace, or --as <alt-name> to keep both.
```

### 3. Resolution Flags

#### `--force`

Overrides the source lock. Removes the existing item and installs the new one in
its place. The lockfile entry is updated to the new source.

```bash
upskill add other-org/repo:brainstorming --force --claude
```

#### `--as <alias>`

Installs the item under an alternate short name, avoiding the conflict entirely.

```bash
upskill add other-org/repo:brainstorming --as brainstorming-v2 --claude
```

- The alias becomes the item's name in all generated client outputs.
- The alias must follow standard naming rules (lowercase `a-z`, digits `0-9`,
  hyphens, max 64 chars, no leading/trailing hyphen).
- The alias must not conflict with another installed `(kind, name)`.

### 4. Lockfile: Aliased Items

When `--as` is used, the lockfile records the original source name so that
`upskill update` can fetch the correct SSOT:

```json
{
  "kind": "skill",
  "name": "brainstorming-v2",
  "source": "other-org/repo",
  "source_name": "brainstorming",
  "ref": "main",
  "hash": "def456..."
}
```

- `source_name` (optional, string): the item's name in the source registry. Only
  present when it differs from `name` (i.e., when aliased).
- `upskill update` uses `source_name` (falling back to `name`) to locate the SSOT
  file in the source.

Schema remains `1` — this is an additive, backwards-compatible field.

### 5. Bundle Install Conflicts

When a bundle install resolves items that conflict with existing locked entries,
the same rules apply per-item.

**Single conflict:**

```
Error: bundle `android-baseline` provides skill `systematic-debugging`
       which is already installed from `driftsys/superpowers`.
       Use --force to replace, or --exclude systematic-debugging to skip it,
       or --as systematic-debugging=android-debugging to alias it.
```

**Multiple conflicts:**

```
Error: bundle `android-baseline` conflicts with 3 installed items:
  - skill `systematic-debugging` (from `driftsys/superpowers`)
  - rule `license-awareness` (from `company/legal-bundle`)
  - agent `security-reviewer` (from `driftsys/superpowers`)

Use --force to replace all, or resolve individually with --exclude or --as.
```

**Bundle flags:**

| Flag                         | Behavior                          |
| ---------------------------- | --------------------------------- |
| `--force`                    | Replace all conflicting items     |
| `--exclude <name>`           | Skip specific items (repeatable)  |
| `--as <source-name>=<alias>` | Alias specific items (repeatable) |

**Bundle-to-bundle conflict** (two bundles in a `requires` chain list the same
`(kind, name)`): remains a hard error as today. This is an authoring bug in the
bundle graph, not a consumer-side conflict.

### 6. Same-Source Reinstall

When `(kind, name)` exists and the source matches, `upskill add` treats it as an
update — fetches the latest, regenerates outputs, updates the hash. No error, no
prompt.

### 7. CI Behavior

No special CI mode needed. The behavior is deterministic everywhere:

- Conflict → error with actionable message
- `--force` or `--as` resolves it
- No interactive prompts in any mode

## Changes Summary

| Area             | Change                                                                  |
| ---------------- | ----------------------------------------------------------------------- |
| Lockfile         | Add optional `source_name` field. No schema bump.                       |
| `upskill add`    | Conflict detection. New flags: `--force`, `--as`.                       |
| Bundle install   | Per-item conflict detection. New flags: `--force`, `--exclude`, `--as`. |
| `upskill update` | Use `source_name` when fetching aliased items.                          |
| Error messages   | Actionable guidance: what conflicted, from where, how to resolve.       |
| No new commands  | Locking is implicit via source affinity.                                |

## Out of Scope

- Namespace/scope syntax (rejected — DX tax too high for the problem).
- Interactive prompts (rejected — deterministic errors are simpler and CI-safe).
- `pin`/`unpin` commands (rejected — locked-by-default eliminates the need).
- Canonical qualified IDs in user-facing flows (internal bookkeeping only).
