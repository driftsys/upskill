---
schema: 1
name: upskill-writing-bundles
description: Use when authoring or editing upskill .bundle.yaml manifests — declaring items, plugins, requires dependencies, naming conventions, or troubleshooting bundle resolution errors. Also use when adding plugin install declarations for client CLIs (Claude Code, Copilot, VS Code, opencode).
metadata:
  version: 0.1.0
  author: driftsys
---

Author upskill `.bundle.yaml` manifests — the distribution unit for
shipping related skills, rules, and agents together with their plugin
dependencies.

## When to Use

- Creating a new bundle to group related items for one-shot install
- Adding `plugins:` declarations so `upskill doctor` can reconcile
  client-native dependencies
- Declaring `requires:` dependencies between bundles
- Debugging resolution failures (cycles, missing items, name collisions)
- Converting a loose collection of skills into a distributable bundle

## When NOT to Use

- Authoring individual skill content → `writing-skills`
- Authoring rules → `upskill-writing-rules`
- Authoring subagents → `upskill-writing-subagents`
- Lifecycle operations (add/update/remove) → `upskill-cli`

## Bundle YAML Schema

File must be named `<name>.bundle.yaml` (pure YAML, no Markdown
frontmatter).

```yaml
schema: 1 # required — always 1
name: my-bundle # required — must match filename stem
description: >- # required — human-readable purpose
  One-line description of what this bundle provides.
license: MIT # optional — SPDX identifier or "proprietary"

items: # required — may be empty {} for meta-bundles
  skills:
    - skill-one
    - skill-two
  rules:
    - rule-one
  agents:
    - agent-one

requires: # optional — dependencies on other bundles
  - name: other-bundle
    version: ">=0.2.0" # opaque, not enforced yet (future C2)

plugins: # optional — client-native plugin deps
  superpowers:
    claude:
      source: claude-plugins-official
      plugin: superpowers
      install_url: https://github.com/obra/superpowers
    copilot:
      source: superpowers-marketplace
      plugin: superpowers
      install_url: https://github.com/obra/superpowers
    opencode:
      module: superpowers
      install_url: https://github.com/obra/superpowers

metadata: # optional — freeform
  version: 0.1.0
  author: your-org
```

## Naming Rules

| Rule                          | Example                                     |
| ----------------------------- | ------------------------------------------- |
| Pattern: `[a-z0-9-]{1,64}`    | `my-bundle`                                 |
| No leading/trailing hyphen    | `my-bundle` not `-my-bundle-`               |
| Filename stem = `name:` field | `my-bundle.bundle.yaml` → `name: my-bundle` |
| No collision with item names  | Bundle `foo` + skill `foo/` → lint error    |

**Name collision prevention:** If your bundle is called `prompt-engineering`,
don't have a skill directory also called `prompt-engineering/`. Use a
distinct name for one of them (e.g., rename the skill to `upskill-prompt-design`).

### Choosing the namespace prefix

Every item name (skill, rule, agent) shares one flat per-client directory
per kind, so two items with the same `(kind, name)` collide on disk when
co-installed from different sources. Prefix every item name with a
**namespace** to keep it globally unique. Pick the scope by how the content
is distributed:

| Scope        | Prefix form             | Use when                                                                                                                                                      |
| ------------ | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Repo/org** | `<org>-<descriptor>`    | The repo ships one cohesive suite of first-party content. Shortest; reads as provenance.                                                                      |
| **Bundle**   | `<bundle>-<descriptor>` | One repo/org ships **multiple** bundles that could reuse generic descriptors (`using`, `evaluating`). The bundle is the collision unit — strongest guarantee. |

Rules of thumb:

- **Default to repo/org scope** — it is shorter and signals who ships the
  content. This repo uses the `upskill-` prefix (e.g. `upskill-writing-rules`).
- The namespace token SHOULD be short (≤ 10 characters) and stable. Prefer the
  project name over a long topic name (`upskill-`, not `prompt-engineering-`).
- Co-located entrypoints keep one prefixed base name and differ only by kind
  (`upskill-writing-rules` skill + rule) — the resolver allows same `name`
  across different kinds.
- Pre-1.0, if a second bundle later collides on a descriptor, just rename one.
  No migration path needed.

## Plugin Declarations

Plugins declare client-native dependencies that `upskill doctor`
reconciles after install.

### Per-Client Fields

**Claude Code:**

```yaml
plugins:
  plugin-name:
    claude:
      source: <marketplace-source> # required
      plugin: <plugin-id> # required
      install_url: <url> # optional — shown when CLI missing
```

**Copilot CLI:**

```yaml
plugins:
  plugin-name:
    copilot:
      source: <marketplace-source> # required
      plugin: <plugin-id> # required
      install_url: <url> # optional
```

**VS Code:**

```yaml
plugins:
  plugin-name:
    vscode:
      extension: <extension-id> # required — e.g. publisher.name
      install_url: <url> # optional
```

**opencode:**

```yaml
plugins:
  plugin-name:
    opencode:
      module: <module-name> # required
      install_url: <url> # optional
```

A plugin entry MAY target one client, a subset, or all four.

### How Doctor Reconciles Plugins

After `upskill add`, `upskill doctor` checks each declared plugin:

| Outcome                        | Doctor bucket       | Exit code     |
| ------------------------------ | ------------------- | ------------- |
| CLI present + plugin installed | `installed_plugins` | 0             |
| CLI present + plugin missing   | `missing_plugins`   | **1** (drift) |
| CLI not found on PATH          | `skipped_plugins`   | 0 (warning)   |
| Check command failed           | `failed_plugins`    | 0 (warning)   |

Only `missing_plugins` causes a non-zero exit — skipped/failed are
informational.

### Lockfile Recording

When a plugin's CLI is not found, `upskill add` records it in the
lockfile with `"status": "skipped"`. Pre-existing lockfiles without a
`status` field default to `"installed"`.

## The `requires` Dependency Graph

Bundles can depend on other bundles in the same registry:

```yaml
requires:
  - name: base-bundle
  - name: other-bundle
    version: ">=1.0.0" # stored but not enforced yet
```

**Resolution rules:**

- Topological (post-order): dependencies install before dependents
- Cycles are a hard error
- Missing requirements are a hard error
- Item conflicts (two bundles declaring same `(kind, name)`) are a hard
  error
- Same name in different kinds is allowed (rule `shared` + skill `shared`)

## Validation Workflow

Always lint after authoring or editing a bundle:

```bash
upskill lint <registry-dir> --strict
```

Common findings:

- `name-collision` — bundle name matches an item directory name
- Schema parse error — missing required field or invalid YAML
- Filename/name mismatch — stem doesn't match `name:` field

## Scaffold Checklist

`upskill new bundle` does not exist yet. Author manually:

1. **Choose a name** — `[a-z0-9-]{1,64}`, distinct from any item in the
   same registry
2. **Create the file** — `skills/<name>.bundle.yaml`
3. **Fill required fields** — `schema: 1`, `name`, `description`, `items`
4. **Add items** — list skill/rule/agent names that exist in the registry
5. **Add plugins** (if any) — declare per-client plugin dependencies
6. **Add requires** (if any) — declare bundle dependencies
7. **Lint** — `upskill lint <dir> --strict`
8. **Test install** — `upskill add ./<dir>/<name>.bundle.yaml --claude`
   (or whichever clients you target)
9. **Run doctor** — `upskill doctor` to verify plugin reconciliation

## Common Mistakes

| Mistake                           | Fix                                      |
| --------------------------------- | ---------------------------------------- |
| Bundle name = item name           | Rename one; lint catches this            |
| Missing `schema: 1`               | Parser silently skips file without it    |
| `name:` doesn't match filename    | `foo.bundle.yaml` must have `name: foo`  |
| Referencing items not in registry | Resolution fails at install time         |
| Circular `requires`               | Flatten or restructure dependencies      |
| Quoting version as bare float     | Use `"0.1.0"` not `0.1.0` in strict mode |

## Honest Caveats

This skill has not been through a RED-GREEN-REFACTOR evaluation cycle.
It is reference documentation distilled from ADR-0007, ADR-0008, and
the format-spec. Gaps may exist in edge-case coverage.

## You Are Done When

- The `.bundle.yaml` manifest exists with valid schema
- All declared items resolve to existing SSOT files
- `upskill lint` passes on the bundle
