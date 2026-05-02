# upskill — Specification

> Upskill your coding agents.

**Status**: v0.2 design (in progress). Phases 0–2 implemented; Phase 3+ pending
— see [§7 Implementation status](#7-implementation-status).

## 1. Overview

`upskill` is a single-binary CLI that lets authors **create, manage, and
prolong** AI-assistance content — rules, skills, agents — across multiple AI
coding clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth.

The central abstraction is **generation**: the SSOT (Single Source of Truth)
on disk is portable; per-client output is produced on demand by a generation
pipeline. v0.1's fetch-and-copy installer model is replaced with this
SSOT-to-client pipeline; see [ADR-0001](./adr/0001-multi-kind-compiler-architecture.md)
for the umbrella decision.

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

Per [ADR-0004](./adr/0004-cli-surface.md):

- **Source registry** — repository where SSOT items are authored. Holds the
  canonical `RULE.md` / `SKILL.md` / `AGENT.md` files. Author commands
  (`new`, `lint`, `fmt`) operate here.
- **Consumer project** — repository where generated outputs are installed
  for use by AI clients. Holds only generated per-client files. Consumer
  commands (`add`, `remove`, `update`, `list`, `info`, `doctor`) operate
  here.

The same repository can be both, but SSOT and generated outputs do not mix
in the same tree.

## 2. CLI surface

Per [ADR-0004](./adr/0004-cli-surface.md). Nine commands, no overlap.

| Command  | Role     | Purpose                                                 |
| -------- | -------- | ------------------------------------------------------- |
| `add`    | Consumer | Install content from any source.                        |
| `remove` | Consumer | Remove installed content.                               |
| `update` | Consumer | Pull latest, regenerate changed items.                  |
| `list`   | Consumer | Show installed content (or `--available` from sources). |
| `info`   | Consumer | Show item or bundle details.                            |
| `doctor` | Consumer | Verify installation consistency.                        |
| `new`    | Author   | Scaffold a new rule, skill, or agent.                   |
| `lint`   | Author   | Validate SSOT files against the format spec.            |
| `fmt`    | Author   | Canonicalise YAML frontmatter (key order, quoting).     |

### 2.1 `add`

```text
upskill add <source> [items...] [--global|--project]
```

`<source>` accepts (per [ADR-0005](./adr/0005-skills-sh-ecosystem-interop.md),
parity with `npx skills add`):

- `owner/repo` — GitHub shorthand
- `owner/repo@ref` — pinned ref (branch, tag, or commit SHA)
- `owner/repo:path/to/item` — subfolder
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `gitlab:owner/repo[...]` or `https://gitlab.com/[...]` — GitLab
- `./path`, `../path`, `/abs/path`, `~/path` — local paths
- `<bundle-name>` — a bundle resolved from configured registries

Source resolution order: registry index first; if no match, treat as git repo
or local path.

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
upskill remove [name...] [--global|--project]
```

Removes installed items from the matching scope. Without arguments,
implementations may prompt or require an explicit name list.

### 2.4 `list` / `info` / `doctor`

- `list [--available]` — show installed content, or available content from
  configured sources.
- `info <name>` — show item or bundle details (resolved source, version,
  hash, generated paths).
- `doctor` — verify on-disk state matches the lock file; report drift.

### 2.5 `new` / `lint` / `fmt`

Author commands. Run inside a source-registry working tree.

- `new <kind> <name>` — scaffold a new SSOT item directory (`<kind>` is one
  of `rule`, `skill`, `agent`).
- `lint [paths...] [--strict]` — validate SSOT files. `--strict` is the CI
  mode (warnings become errors).
- `fmt [paths...]` — canonicalise YAML frontmatter only. Markdown body
  formatting is dprint's responsibility.

### 2.6 Aliases

`add` does **not** alias to `install`. `remove` does **not** alias to
`uninstall`. v0.1's `install` and `uninstall` verbs are dropped — the unified
verb names are canonical. Migration is covered in v0.2.0 release notes.

## 3. Generation pipeline

Per [ADR-0003](./adr/0003-generation-pipeline.md). SSOT → per-client output
runs on the developer's machine, on every install or update. No CI generation
step. No committed `dist/` artifacts.

### 3.1 Per-client output paths

| Item kind | Claude Code                             | GitHub Copilot                                                | opencode                                           |
| --------- | --------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | `.agents/rules/<name>/RULE.md` (canonical-store)   |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (canonical-store) |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                       |

Per-client frontmatter mapping is defined in [format-spec §7](./format-spec.md#7-generation-client-specific-output)
and Appendix B; behaviour rationale is in
[ADR-0003](./adr/0003-generation-pipeline.md).

### 3.2 Copy, not symlink

All installation is file copy or generated output. No symlinks anywhere. One
code path; Windows portability without Developer Mode or
`core.symlinks=true`. The deliberate divergence from skills.sh's symlink
default is documented in
[ADR-0005](./adr/0005-skills-sh-ecosystem-interop.md).

### 3.3 Markdown formatting

Generated output passes through embedded `dprint-plugin-markdown`
(exact-pinned at `=0.21.1`) before writing. This guarantees idempotence under
the same formatter in `dprint.json`. Authors who run `dprint` locally see no
diffs on generated files. Rationale and version-pin discipline are in
[ADR-0003](./adr/0003-generation-pipeline.md).

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

Per [ADR-0003](./adr/0003-generation-pipeline.md). State is split between
two files, both schema-versioned (`schema: 1`).

### 4.1 `.upskill-lock.json` (per-project, committed)

Lives at the consumer-project root. Records what was installed, from where,
at what resolved git ref, with what content hashes. Plays the same role as
`package-lock.json`: deterministic regeneration on another developer's
machine and in CI.

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

### 4.2 `~/.upskill/installed.json` (per-user)

Tracks the user-global view: items installed at the global scope, drift
state for the global location, and source-registry caches. Not committed.

### 4.3 v0.1 lockfile migration

The lockfile filename does not change between v0.1 and v0.2 — both versions
use `.upskill-lock.json`. Per
[ADR-0003](./adr/0003-generation-pipeline.md), v0.2 stamps a top-level
`schema: 2` field; v0.1 lockfiles (no `schema` field) are read once on first
invocation of a v0.2 command and rewritten in place with the v0.2 entry shape.
Any user-global state moves into `~/.upskill/installed.json`. Existing v0.1
users are not silently broken.

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

## 7. Implementation status

Per [ADR-0001](./adr/0001-multi-kind-compiler-architecture.md).

| Phase | Scope                                                                                             | Status       |
| ----- | ------------------------------------------------------------------------------------------------- | ------------ |
| 0     | Tag, branch, deps, ADRs, model skeleton.                                                          | Done.        |
| 1     | SSOT parser + generation pipeline for skills × all 3 clients.                                     | Done.        |
| 2     | Pipeline extension to rules and agents.                                                           | Done.        |
| 3     | `add` / `update` / `remove` over the pipeline; bundles; v0.1 lockfile migration; ancillary files. | In progress. |
| 5     | `lint` + `fmt`.                                                                                   | Pending.     |
| 6     | `new` (scaffolding).                                                                              | Pending.     |
| 7     | Polish + v0.2.0 release.                                                                          | Pending.     |

The v0.1 CLI surface (`add` / `list` / `remove` / `check` / `search` /
`update` against `.upskill-lock.json`) remains the **shipped** behaviour
until v0.2.0. v0.1 is supported via patch releases of the `v0.1.x` line.

## 8. Out of scope

- Runtime invocation of installed content (each AI client's responsibility).
- Hosting a registry server or central marketplace.
- Content quality review (delegated to AI workflows outside upskill).
- Telemetry, analytics, or usage tracking.
- Publishing to non-Git destinations (npm, crates.io, PyPI).

## 9. References

- Format spec: [`docs/format-spec.md`](./format-spec.md)
- Architecture decisions:
  [ADR-0001](./adr/0001-multi-kind-compiler-architecture.md) (umbrella),
  [ADR-0002](./adr/0002-portable-content-format.md) (format),
  [ADR-0003](./adr/0003-generation-pipeline.md) (pipeline),
  [ADR-0004](./adr/0004-cli-surface.md) (CLI),
  [ADR-0005](./adr/0005-skills-sh-ecosystem-interop.md) (interop)
- Implementation guide: [`docs/architecture.md`](./architecture.md)
- User guide: [`docs/usage.md`](./usage.md)
- Agent Skills open standard: <https://agentskills.io>
- skills.sh ecosystem: <https://skills.sh>
