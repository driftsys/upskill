# Multi-Registry Search with Local Index

## Summary

Extend `upskill search` to query configured git-based registries in addition
to skills.sh. Introduce a general config file and a local index cache with
HEAD-based invalidation for fast, offline-capable discovery.

## Motivation

Users with corporate registries (or well-known public repos like Anthropic,
Microsoft) cannot discover content via `upskill search` today — it only
queries skills.sh. They must know the exact `owner/repo:path` to install.
Multi-registry search closes this gap.

## Design

### 1. Configuration

**First upskill config file.** General-purpose, extensible for future settings.

| Scope   | Path                            | Committed? |
| ------- | ------------------------------- | ---------- |
| Global  | `~/.config/upskill/config.yaml` | No         |
| Project | `.upskill/config.yaml`          | Yes        |

Merge order: built-in → global → project. Duplicate registry names: project
overrides global overrides built-in.

```yaml
# ~/.config/upskill/config.yaml
registries:
  - name: corp
    source: gitlab:mycompany/ai-skills
  - name: anthropic
    source: anthropics/skills
```

**Model:**

```rust
struct Config {
    registries: Vec<RegistryEntry>,
}

struct RegistryEntry {
    name: String,
    source: String, // parsed via existing source.rs
}
```

The `source` field accepts the same syntax as `upskill add` (owner/repo,
gitlab:owner/repo, full URLs, local paths).

**Built-in registries:** skills.sh remains the only built-in for now. Well-known
public git registries (Anthropic, Microsoft) can be added later as the ecosystem
matures — no code change needed, just add entries to the hardcoded list.

### 2. Local Index

**Location:** `~/.cache/upskill/index/<registry-name>.json`

**Schema:**

```json
{
  "schema": 1,
  "registry": "corp",
  "source": "gitlab:mycompany/ai-skills",
  "head": "abc123def456...",
  "indexed_at": "2026-05-24T10:00:00Z",
  "items": [
    {
      "name": "code-review",
      "kind": "skill",
      "description": "Structured code review workflow with checklists",
      "path": "skills/code-review"
    }
  ]
}
```

**Description extraction:** First non-empty paragraph from the SSOT body (after
YAML frontmatter). Uses existing frontmatter parsing.

**Freshness check:**

1. `git ls-remote <url> HEAD` → current HEAD sha (~100ms)
2. Compare with `head` field in cached index
3. Same → serve from cache
4. Different → shallow clone into temp dir, scan, rebuild index, delete clone

No persistent clone storage — only the index JSON persists.

**Local-path registries:** When `source` is a local path (`./`, `../`, `/`,
`~/`), skip the clone entirely — scan the directory in place. No HEAD check;
always re-scan (local FS is fast). Still write an index file for consistency
but without a `head` field.

### 3. Search UX

**Unified search:**

```
upskill search foo
```

1. Query skills.sh API (existing, unchanged)
2. Query local index for each configured registry (substring match on name +
   description)
3. Merge results, grouped by source

**Output:**

```
── skills.sh ──────────────────────────────────────────
  code-review          stasson/skills:skills/code-review (12 installs)

── corp ───────────────────────────────────────────────
  code-review          gitlab:mycompany/ai-skills:skills/code-review
  code-review-strict   gitlab:mycompany/ai-skills:skills/code-review-strict

── anthropic ──────────────────────────────────────────
  code-review          anthropics/skills:skills/code-review
```

**Targeted search:**

```
upskill search foo --registry corp
```

Only searches the named registry. Skips skills.sh and others.

**Flags:**

- `--registry <name>` — search a specific registry only
- `--kind <skill|rule|agent|bundle>` — filter by item kind
- `--limit N` — existing, applies to skills.sh results

**Install from results:** User copies the source path and runs
`upskill add <source>` as today. No new install syntax.

### 4. Index Lifecycle

**When indexing happens:**

- First search after adding a registry → clone + index (foreground with
  progress)
- Subsequent searches → HEAD check, serve from cache or re-index
- `upskill index` → force rebuild all
- `upskill index --registry corp` → rebuild one

**Error handling:**

- Network failure + cache exists → serve stale with warning
  (`"using cached index for 'corp' (offline)"`)
- Network failure + no cache → skip registry, warn
- Invalid/empty registry → warn, produce empty index
- Auth failure → hint about tokens (reuses `auth.rs`)

**Cache management:**

- `upskill index --clear` removes all cached indexes
- Removing a registry from config does not auto-delete its cache

### 5. Implementation

**New modules:**

- `src/config.rs` — parse + merge config layers
- `src/index.rs` — clone, scan, build/read/write index JSON, HEAD check

**Modified modules:**

- `src/search.rs` — extend to query local indexes
- `src/main.rs` — add `index` subcommand, `--registry`/`--kind` flags on
  `search`

**New CLI surface:**

```
upskill search <query> [--registry <name>] [--kind <kind>] [--limit N]
upskill index [--registry <name>] [--clear]
```

**Dependencies:** None new. Uses `serde_yaml_ng` (config), `serde_json`
(index), git shell-out (clone/ls-remote via existing `fetch.rs` patterns).

### 6. Out of Scope

- Named registry install syntax (`upskill add corp --skill foo`) — the
  `owner/repo:path` syntax already works
- Full-text body search — name + description covers discovery needs
- Background re-indexing — foreground is acceptable given shallow clone speed
- Registry publishing/hosting — remains out of scope per spec §7
