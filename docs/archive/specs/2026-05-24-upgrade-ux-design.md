# Upgrade UX Design

Date: 2026-05-24

## Problem

`upskill update` has three gaps that leave stale artifacts on disk:

1. **File-level staleness** — when a file inside a skill is renamed or removed
   upstream (e.g., `scripts/old.js` → `scripts/new.js`), the old output file
   lingers because update overwrites but never deletes.

2. **Item-level staleness** — when an item is renamed or removed from a source,
   the lockfile entry becomes an orphan. The user must manually discover this
   (via `doctor`) and run `upskill remove`. There is no automatic cleanup.

3. **No confirmation** — `update` applies changes silently. Now that update will
   perform removals, a confirmation step is needed for safety.

## Design

### 1. Clean-and-regenerate

Before writing new output for an item, delete the entire per-client output
directory for that item.

**Affected paths per item (example: skill named `foo`):**

- `.claude/skills/foo/`
- `.github/skills/foo/`
- `.agents/skills/foo/`

**Behavior:**

- Applies during `update` (Apply mode) and `add` (re-install of existing item).
- Does NOT apply during `--dry-run` (no side effects).
- If deletion fails (permissions, etc.), report the error and continue with
  other items.

**Rationale:** Output directories are generated artifacts. Users should not add
files to them. Clean-and-regenerate is correct by construction with zero
additional state.

### 2. Auto-remove orphans on update

After fetching a source, compare lockfile entries for that source against the
items actually discovered in the fetched SSOT.

**Detection:**

- For each lockfile entry whose `source` matches the fetched source:
  - If the item `(kind, name)` is NOT present in the fetched SSOT → orphan.

**Action (Apply mode):**

- Delete per-client output directories for the orphan.
- Remove the lockfile entry.
- Include in the update report as `Removed` with reason.

**Action (DryRun mode):**

- Include in the report as `WouldRemove`.

**Edge cases:**

- Item-filtered update (`upskill update foo bar`): only the named items are
  checked. Orphan detection applies only to the subset the user asked to update.
  Rationale: if the user said "update foo", they didn't ask us to remove `baz`.
- Full update (`upskill update`): orphan detection applies to all entries for
  each fetched source.

### 3. Interactive confirmation

After computing the full update plan (updated, removed, unchanged), display a
summary and prompt before applying.

**Format:**

```
upskill update — github:owner/repo

  updated:   2 items (code-review, secret-scanner)
  removed:   1 item  (old-lint-rule — no longer in source)
  unchanged: 3 items

Apply? [y/N]
```

**Skip conditions (no prompt, apply immediately):**

- `--yes` / `-y` flag passed.
- stdin is not a TTY (CI, pipes).
- `--dry-run` (never applies, so no prompt needed).
- Nothing to do (all items unchanged, no removals).

**CLI change:**

Add `--yes` / `-y` flag to the `Update` command (same pattern as `Remove`
already has).

### Non-goals

- **Auto-update** — no hooks, daemons, or staleness nudges. Users run
  `upskill update` manually or in CI.
- **Rename detection** — no attempt to match old→new names. The effect is
  correct: old item removed, new item installed.
- **Per-file tracking in lockfile** — clean-and-regenerate eliminates the need.
- **Warning before wiping user-added files** — output dirs are generated; adding
  files there is unsupported.

## Implementation notes

### Modules affected

- `src/pipeline.rs` — `update()` function: add orphan detection, add
  clean-before-regenerate, compute plan before applying, return enriched report.
- `src/cli.rs` — add `--yes` flag to `Update` command.
- `src/main.rs` — `run_update()`: display plan summary, prompt, then call apply.
- `src/lockfile.rs` — add `remove_item()` method if not already present.

### UpdateReport changes

Add a new variant to `UpdateStatus`:

```rust
pub enum UpdateStatus {
    UpToDate,
    Updated { old_hash: Option<String>, new_hash: Option<String> },
    WouldChange { old_hash: Option<String>, new_hash: Option<String> },
    Removed,       // NEW
    WouldRemove,   // NEW
}
```

### Clean-and-regenerate implementation

In the install/regenerate path, before writing output files for an item:

```rust
fn clean_item_outputs(target: &Path, kind: ItemKind, name: &str) {
    for client in ALL_CLIENTS {
        let dir = target.join(output_dir(kind, client, name));
        if dir.is_dir() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
```

Call this in `install_from_source` (or the per-item render step) before writing.

### Lockfile schema

No schema change. `schema: 1` is preserved. Removal is just deleting entries.
