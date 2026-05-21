# upskill — Specification

> Upskill your coding agents.

## 1. Overview

`upskill` is a single-binary CLI that lets authors **create, manage, and
prolong** AI-assistance content — rules, skills, agents — across multiple AI
coding clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth.

The central abstraction is **generation**: the SSOT (Single Source of Truth)
on disk is portable; per-client output is produced on demand by a generation
pipeline.

### 1.1 Three item kinds

| Kind  | Purpose                             | Entrypoint file |
| ----- | ----------------------------------- | --------------- |
| Rule  | Always-on behavioural guidance.     | `RULE.md`       |
| Skill | On-demand procedural content.       | `SKILL.md`      |
| Agent | Persona definition with tool scope. | `AGENT.md`      |

The on-disk contract is defined by the [portable format spec](./format-spec.md).
This document describes upskill's behaviour against that contract.

### 1.2 Two scopes

| Scope   | Lives at         | Use                                            |
| ------- | ---------------- | ---------------------------------------------- |
| Project | `<cwd>/.agents/` | Content shared across the team via git.        |
| Global  | `$HOME/.agents/` | Content available to a single user, all repos. |

### 1.3 Two roles

- **Source registry** — repository where SSOT items are authored. Holds the
  canonical `RULE.md` / `SKILL.md` / `AGENT.md` files and `*.bundle.yaml`
  manifests. Author commands (`new`, `lint`, `fmt`) operate here.
- **Consumer project** — repository where generated outputs are installed
  for use by AI clients. Holds only generated per-client files. Consumer
  commands (`add`, `remove`, `update`, `list`, `doctor`, `search`) operate
  here.

The same repository can be both, but SSOT and generated outputs do not mix
in the same tree.

## 2. CLI surface

No overlap between verbs.

| Command  | Role     | Purpose                                             |
| -------- | -------- | --------------------------------------------------- |
| `add`    | Consumer | Install items or bundles from any source.           |
| `remove` | Consumer | Remove installed content.                           |
| `update` | Consumer | Pull latest, regenerate changed items.              |
| `list`   | Consumer | Show installed content from the lock file.          |
| `doctor` | Consumer | Verify installation consistency.                    |
| `search` | Consumer | Look up skills via the public registry.             |
| `new`    | Author   | Scaffold a new rule, skill, or agent.               |
| `lint`   | Author   | Validate SSOT files against the format spec.        |
| `fmt`    | Author   | Canonicalise YAML frontmatter (key order, quoting). |

### 2.1 `add`

```text
upskill add <source> [items...] [--global|--project]
```

`<source>` accepts (parity with `npx skills add`):

- `owner/repo` — GitHub shorthand
- `owner/repo@ref` — pinned ref (branch, tag, or commit SHA)
- `owner/repo:path/to/item` — subfolder
- `owner/repo:path/to/name.bundle.yaml` — bundle file (resolves transitively, see §2.6)
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `gitlab:owner/repo[...]` or `https://gitlab.com/[...]` — GitLab
- `./path`, `../path`, `/abs/path`, `~/path` — local paths

`upskill add <source>` installs everything the source contains. Optional
`items...` after the source filter to a subset. There is no interactive
picker; install-all-by-default fits bundle curation.

Default scope is `--project` (write into the current repo's `.agents/...`),
falling back to `--global` (`$HOME/.agents/...`) when the current directory
is not inside a git repo. Either flag can be passed explicitly.

### 2.2 `update`

```text
upskill update [name...] [--global|--project] [--dry-run]
```

Always fetches the latest source before regenerating. Absorbs the role of a
separate `sync` command — there is no `--offline` flag in v1; offline use is
covered by passing local paths to `add` / `update`.

`update` covers two distinct behaviours:

- **Absorb client drift.** When Copilot or Claude Code change a path or
  frontmatter field, `update` regenerates outputs from the same SSOT.
- **Track upstream evolution.** `update` git-pulls source repos and
  regenerates items whose source hash changed.

### 2.3 `remove`

```text
upskill remove [name...] [--source <label>] [--global]
```

Removes installed items from the matching scope. Either name one or more
items, or pass `--source <label>` to remove every item that came from a
single source. Bare `upskill remove` is rejected — be explicit. Ancillary
files (`CLAUDE.md`, `opencode.json`, `.vscode/settings.json`) are not
touched.

### 2.4 `list` / `doctor`

- `list` — show installed content from `.upskill-lock.json`, grouped by
  kind. Bundles are surfaced in a separate section. The `--available`
  view (items discoverable from configured sources) is deferred for a
  future release.
- `doctor` — verify on-disk state matches the lock file. Three
  independent buckets: missing per-client outputs (reinstall fixes),
  SSOT hash drift on `local:` sources (`update` fixes), and lockfile
  entries with no recoverable source (`remove` fixes). Exit 0 when
  clean, 1 when any drift is found.

### 2.5 `new` / `lint` / `fmt`

Author commands. Run inside a source-registry working tree.

- `new <kind> <name>` — scaffold a new SSOT item directory (`<kind>` is one
  of `rule`, `skill`, `agent`).
- `lint [paths...] [--strict]` — validate SSOT files. `--strict` is the CI
  mode (warnings become errors).
- `fmt [paths...]` — canonicalise YAML frontmatter only. Markdown body
  formatting is dprint's responsibility.

### 2.6 Bundles

A **bundle** is a `*.bundle.yaml` manifest that names a curated set of items
and (optionally) other bundles to install together. Bundles contain no
content of their own; they reference items by `name`. Frontmatter shape is
defined in [format-spec §3.7](./format-spec.md#37-bundle-schema).

`upskill add <source>:path/to/foo.bundle.yaml` installs a bundle.
Implementation behaviour:

- **Items.** Every entry in `items.rules`, `items.skills`, `items.agents` is
  resolved against the same source repository and installed. Unresolved
  names error out before any write.
- **Transitive `requires:`.** When the bundle declares `requires:`, every
  bundle in the transitive closure is resolved against the same source.
  Cross-source bundle resolution is not supported; bundles and their
  dependencies must live in one source repository. Circular `requires:`
  is rejected.
- **Lockfile.** Each installed bundle is recorded as a top-level `bundles[]`
  entry in `.upskill-lock.json` alongside its resolved items. The same item
  may appear in multiple bundle `items[]` lists; it is installed once.
- **Removal.** `upskill remove --source <label>` removes every item that
  came from that source, including those introduced via bundles. A
  bundle-name-targeted removal (`remove <bundle-name>` removing only items
  exclusive to that bundle) is not yet implemented.

### 2.7 Aliases

`add` does **not** alias to `install`. `remove` does **not** alias to
`uninstall`. The unified verb names are canonical.

## 3. Generation pipeline

SSOT → per-client output runs on the developer's machine, on every install
or update. No CI generation step. No committed `dist/` artifacts.

### 3.1 Per-client output paths

| Item kind | Claude Code                             | GitHub Copilot                                                | opencode                                           |
| --------- | --------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | `.agents/rules/<name>/RULE.md` (canonical-store)   |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (canonical-store) |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                       |

Per-client frontmatter mapping is defined in [format-spec §7](./format-spec.md#7-generation-client-specific-output)
and Appendix B.

### 3.2 Copy, not symlink

All installation is file copy or generated output. No symlinks anywhere. One
code path; Windows portability without Developer Mode or
`core.symlinks=true`.

### 3.3 Markdown formatting

Generated output passes through embedded `dprint-plugin-markdown`
(exact-pinned at `=0.21.1`) before writing. This guarantees idempotence under
the same formatter in `dprint.json`. Authors who run `dprint` locally see no
diffs on generated files.

### 3.4 Ancillary file handling

Three files outside the per-item path tree are managed idempotently:

- **`CLAUDE.md`** at repo root — created once with `@AGENTS.md` content if
  absent. Never overwritten.
- **`.vscode/settings.json`** — `chat.instructionsFilesLocations` is set;
  other keys preserved.
- **`opencode.json`** — managed only on **first** opencode-rule install.
  The entry `".agents/rules/**/RULE.md"` is added to `instructions[]` if not
  already present, then the file is never mutated again. opencode's glob
  picks up subsequent rule additions and removals automatically.

## 4. State files

`.upskill-lock.json` records what was installed, from where, at what
resolved git ref, with what content hashes. Same schema (`schema: 1`),
two possible locations:

| Scope   | Location                   | Committed? |
| ------- | -------------------------- | ---------- |
| Project | `<cwd>/.upskill-lock.json` | Yes        |
| Global  | `$HOME/.upskill-lock.json` | No         |

The lockfile plays the same role as `package-lock.json`: deterministic
regeneration on another developer's machine and in CI for project scope,
and across-repo continuity for global scope.

```json
{
  "schema": 1,
  "items": [
    {
      "kind": "skill",
      "name": "code-review",
      "source": "github:driftsys/skills",
      "ref": "v1.2.0",
      "hash": "sha256:..."
    }
  ],
  "bundles": [
    {
      "name": "platform-defaults",
      "source": "github:driftsys/bundles",
      "ref": "v0.4.0",
      "items": ["code-review", "secret-scanner"]
    }
  ]
}
```

## 5. Source format and authentication

Source format: see [§2.1](#21-add).

Token resolution order:

| Host   | Order                                                             |
| ------ | ----------------------------------------------------------------- |
| GitHub | `GITHUB_TOKEN` → `GH_TOKEN` → `gh auth token` → unauthenticated   |
| GitLab | `GITLAB_TOKEN` → `GL_TOKEN` → `glab auth token` → unauthenticated |

Self-hosted GitLab is supported via full URL form
(`https://gitlab.mycompany.com/team/repo`).

## 6. CLI compliance

### 6.1 Exit codes

| Code  | Meaning                              |
| ----- | ------------------------------------ |
| `0`   | Success.                             |
| `1`   | General error (network, parse, I/O). |
| `2`   | Usage error (invalid args or flags). |
| `130` | Interrupted (Ctrl+C / SIGINT).       |

### 6.2 Conventions

- POSIX/GNU long flags (`--flag-name`) and short flags (`-f`).
- Stdout for data, stderr for progress and errors.
- Honors `NO_COLOR` and TTY detection (no color in pipes/CI).
- Global `-q` / `--quiet` suppresses informational stdout. Errors on
  stderr and exit codes are unaffected.
- Single static binary, no runtime dependencies.

### 6.3 Environment variables

| Variable       | Purpose                                       |
| -------------- | --------------------------------------------- |
| `NO_COLOR`     | Disable colored output.                       |
| `GITHUB_TOKEN` | Authenticate GitHub requests (private repos). |
| `GH_TOKEN`     | Fallback for `GITHUB_TOKEN`.                  |
| `GITLAB_TOKEN` | Authenticate GitLab requests (private repos). |
| `GL_TOKEN`     | Fallback for `GITLAB_TOKEN`.                  |
| `HTTPS_PROXY`  | HTTP proxy for network requests.              |

## 7. Out of scope

- Runtime invocation of installed content (each AI client's responsibility).
- Hosting a registry server or central marketplace.
- Content quality review (delegated to AI workflows outside upskill).
- Telemetry, analytics, or usage tracking.
- Publishing to non-Git destinations (npm, crates.io, PyPI).

## 8. References

- Format spec: [`docs/format-spec.md`](./format-spec.md)
- User guide: [Getting started](./getting-started.md), [Commands](./commands.md), [Recipes](./recipes.md)
- Agent Skills open standard: <https://agentskills.io>
- skills.sh ecosystem: <https://skills.sh>
