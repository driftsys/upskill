# upskill — Specification

> Upskill your coding agents.

Production-architecture direction is specified by the
[customization delivery contract](https://github.com/driftsys/upskill/blob/main/docs/topics/0009-customization-delivery.md).
Independent Codex/OpenCode rules use a selected native or hook route beside
unchanged team `AGENTS.md`. The native-first amendment selects a trusted Codex
hook with its declared recorded-history limitation and permits explicit OpenCode
paths only after Q5 PASS; it does not mandate a plugin where a native route
works. Experimental contribution APIs and generated-file tests do not activate
that route or schema 2. Current released command behavior below remains separate
from those planned gates
([AD-0040 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md#native-first-rules-and-implementation-order-2026-10-24)).

## 1. Overview

`upskill` is a single-binary CLI that lets authors **create, manage, and
prolong** AI-assistance content — rules, skills, agents — across multiple AI
coding clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth.

The central abstraction is **generation**: the SSOT (Single Source of Truth) on
disk is portable; per-client output is produced on demand by a generation
pipeline.

### 1.1 Three item kinds

| Kind  | Purpose                             | Entrypoint file |
| ----- | ----------------------------------- | --------------- |
| Rule  | Always-on behavioural guidance.     | `RULE.md`       |
| Skill | On-demand procedural content.       | `SKILL.md`      |
| Agent | Persona definition with tool scope. | `AGENT.md`      |

The on-disk contract is defined by the
[portable format spec](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md).
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
- **Consumer project** — repository where generated outputs are installed for
  use by AI clients. Holds only generated per-client files. Consumer commands
  (`add`, `remove`, `update`, `list`, `doctor`, `search`) operate here.

The same repository can be both, but SSOT and generated outputs do not mix in
the same tree.

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
| `index`  | Consumer | Build or manage the local registry index cache.     |
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
- `owner/repo:path/to/name.bundle.yaml` — bundle file (resolves transitively,
  see §2.6)
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `https://<host>/<path>[...]` — any other https git host: GitLab (including
  self-hosted instances and subgroups at any nesting depth), Bitbucket, Gitea,
  or a plain git server
- `./path`, `../path`, `/abs/path`, `~/path` — local paths

`upskill add <source>` installs everything the source contains. Optional
`items...` after the source filter to a subset. There is no interactive picker;
install-all-by-default fits bundle curation.

Default scope is `--project` (write into the current repo's `.agents/...`),
falling back to `--global` (`$HOME/.agents/...`) when the current directory is
not inside a git repo. Either flag can be passed explicitly.

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
- **Track upstream evolution.** `update` git-pulls source repos and regenerates
  items whose source hash changed.

Persisted schema-1 `gitlab:` and `gitlab+<host>:` source labels remain readable
for updates. Unambiguous historical labels decode to generic HTTPS sources,
preserving namespace, ref and subfolder. Applying `update` canonicalizes the
source labels of reinstalled items and bundles, even when their content is
unchanged. `update --dry-run`, `list` and `doctor` do not rewrite lockfiles or
generated output. Aliases, recorded selection, bundle membership and unrelated
item/plugin/MCP records remain intact. This does not introduce schema 2 or infer
artifact ownership.

The supported historical authority grammar and reconciliation limits are defined
in the
[ADR-0011 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12).
Unsupported or ambiguous labels fail before fetch with the recorded label and
guidance to reconcile a verified HTTPS URL, ref and subfolder. Modern `add`
still rejects the removed GitLab shorthands with exit 2; use a full HTTPS URL.

### 2.3 `remove`

```text
upskill remove [name...] [--source <label>] [--global]
```

Removes installed items from the matching scope. Either name one or more items,
or pass `--source <label>` to remove every item that came from a single source.
Bare `upskill remove` is rejected — be explicit. Ancillary files (`CLAUDE.md`,
`opencode.json`, `.vscode/settings.json`) are not touched.

### 2.4 `list` / `doctor`

- `list` — show installed content from `.upskill-lock.json`, grouped by kind.
  Bundles are surfaced in a separate section. The `--available` view (items
  discoverable from configured sources) is deferred for a future release.
- `doctor` — verify on-disk state matches the lock file. Five independent
  buckets: missing per-client outputs (reinstall fixes), SSOT hash drift on
  `local:` sources (`update` fixes), lockfile entries with no recoverable source
  (`remove` fixes), installed plugins missing from the client (`update`
  reinstalls), and skipped plugins (informational — CLI was absent at install
  time). Exit 0 when clean, 1 when any drift bucket is non-empty. Skipped
  plugins are warnings and do not affect the exit code.

### 2.5 `new` / `lint` / `fmt`

Author commands. Run inside a source-registry working tree.

- `new <kind> <name>` — scaffold a new SSOT item directory (`<kind>` is one of
  `rule`, `skill`, `agent`).
- `lint [paths...] [--strict]` — validate SSOT files. `--strict` is the CI mode
  (warnings become errors).
- `fmt [paths...]` — canonicalise YAML frontmatter only. Markdown body
  formatting is dprint's responsibility.

### 2.6 Bundles

A **bundle** is a `*.bundle.yaml` manifest that names a curated set of items and
(optionally) other bundles to install together. Bundles contain no content of
their own; they reference items by `name`. Frontmatter shape is defined in
[format-spec §3.7](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md#37-bundle-schema).

`upskill add <source>:path/to/foo.bundle.yaml` installs a bundle. Implementation
behaviour:

- **Items.** Every entry in `items.rules`, `items.skills`, `items.agents` is
  resolved against the same source repository and installed. Unresolved names
  error out before any write.
- **Transitive `requires:`.** When the bundle declares `requires:`, every bundle
  in the transitive closure is installed. A bare `{ name }` entry is resolved
  against the same source; a `{ name, source }` entry is resolved cross-source
  (`source` reuses the `upskill add` DSL), and the closure MAY span sources.
  Each bundle and item is recorded in the lockfile under its own source.
  Circular `requires:` is rejected; the same bundle name resolving to two
  different sources within one closure is an error.
- **Lockfile.** Each installed bundle is recorded as a top-level `bundles[]`
  entry in `.upskill-lock.json` alongside its resolved items. The same item may
  appear in multiple bundle `items[]` lists; it is installed once.
- **Removal.** `upskill remove --source <label>` removes every item that came
  from that source, including those introduced via bundles. A
  bundle-name-targeted removal (`remove <bundle-name>` removing only items
  exclusive to that bundle) is not yet implemented.

### 2.7 Aliases

`add` does **not** alias to `install`. `remove` does **not** alias to
`uninstall`. The unified verb names are canonical.

## 3. Generation pipeline

SSOT → per-client output runs on the developer's machine, on every install or
update. No CI generation step. No committed `dist/` artifacts.

### 3.1 Per-client output paths

| Item kind | Claude Code                             | GitHub Copilot                                                | opencode                                           |
| --------- | --------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | `.agents/rules/<name>/RULE.md` (canonical-store)   |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (canonical-store) |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                       |

Per-client frontmatter mapping is defined in
[format-spec §7](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md#7-generation-client-specific-output)
and Appendix B.

### 3.2 Copy, not symlink

All installation is file copy or generated output. No symlinks anywhere. One
code path; Windows portability without Developer Mode or `core.symlinks=true`.

### 3.3 Markdown formatting

Generated output passes through embedded `dprint-plugin-markdown` (exact-pinned
at `=0.21.1`) before writing. This guarantees idempotence under the same
formatter in `dprint.json`. Authors who run `dprint` locally see no diffs on
generated files.

### 3.4 Ancillary file handling

Three files outside the per-item path tree are managed idempotently:

- **`CLAUDE.md`** at repo root — created once with `@AGENTS.md` content if
  absent. Never overwritten.
- **`.vscode/settings.json`** — `chat.instructionsFilesLocations` is set; other
  keys preserved.
- **`opencode.json`**: checked on rule-bearing installs. The entry
  `".agents/rules/**/RULE.md"` is added to `instructions[]` when absent,
  including when a later install finds that it was removed. An existing entry
  leaves the file unchanged; unrelated entries and keys are preserved
  ([implementation](https://github.com/driftsys/upskill/blob/main/src/ancillary.rs),
  [call site](https://github.com/driftsys/upskill/blob/main/src/pipeline/mod.rs)).
  The installer relies on client glob expansion; this is a shipped write
  contract, not qualification of OpenCode 2.1.4 runtime behavior
  ([retained retry](https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-opencode-explicit-paths-retry.md)).

Planned ancillary transition (2026-10-24 decision, 2026-10-25 reconciliation):
the behavior above remains shipped behavior. The separate step-8 code PR removes
bridge creation and the OpenCode glob write only after Q5 explicit-path PASS.
Existing generated entries are to be reported by doctor, never automatically
deleted; that report is required future behavior, not a current capability. VS
Code's write remains pending its deferred profile decision. No docs-only
adoption changes these commands
([AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md),
[Q5 deferred retry](https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-opencode-explicit-paths-retry.md),
[handoff step 8](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-10-23-customization-delivery-implementation-spec.md#slice-1-qualification-of-the-gating-routes)).

## 4. State files

`.upskill-lock.json` records what was installed, from where, at what resolved
git ref, with what content hashes. Same schema (`schema: 1`), two possible
locations:

| Scope   | Location                   | Committed? |
| ------- | -------------------------- | ---------- |
| Project | `<cwd>/.upskill-lock.json` | Yes        |
| Global  | `$HOME/.upskill-lock.json` | No         |

The lockfile plays the same role as `package-lock.json`: deterministic
regeneration on another developer's machine and in CI for project scope, and
across-repo continuity for global scope.

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
  ],
  "plugins": []
}
```

### 4.1 Pure legacy-state projection

See
[Pure legacy-state projection](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#41-pure-legacy-state-projection)
for the canonical requirements and architecture.

### 4.2 Pure owned instruction planning

See
[Pure owned instruction planning](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#42-pure-owned-instruction-planning)
for the canonical requirements and architecture.

### 4.3 Strict captured source manifests

See
[Strict captured source manifests](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#43-strict-captured-source-manifests)
for the canonical requirements and architecture.

### 4.4 Owned whole-file operation plans

See
[Owned whole-file operation plans](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#44-owned-whole-file-operation-plans)
for the canonical requirements and architecture.

### 4.5 Captured item compilation

See
[Captured item compilation](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#45-captured-item-compilation)
for the canonical requirements and architecture.

### 4.6 Mixed owned-artifact operation plans

See
[Mixed owned-artifact operation plans](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
for the canonical requirements and architecture.

### 4.7 Qualified scope roots and advisory writer locking

See
[Qualified scope roots and advisory writer locking](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#47-qualified-scope-roots-and-advisory-writer-locking)
for the canonical requirements and architecture.

### 4.8 Qualified destination observations

See
[Qualified destination observations](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#48-qualified-destination-observations)
for the canonical requirements and architecture.

### 4.9 Qualified source tree observations

See
[Qualified source tree observations](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#49-qualified-source-tree-observations)
for the canonical requirements and architecture.

### 4.10 Explicit file permission selection

See
[Explicit file permission selection](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#410-explicit-file-permission-selection)
for the canonical requirements and architecture.

### 4.11 Ordered recovery checkpoint model

See
[Ordered recovery checkpoint model](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#411-ordered-recovery-checkpoint-model)
for the canonical requirements and architecture.

### 4.12 Qualified file execution and recovery

See
[Qualified file execution and recovery](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery)
for the canonical requirements and architecture.

### 4.13 Combined lifecycle state foundation

See
[Combined lifecycle state foundation](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#413-combined-lifecycle-state-foundation)
for the canonical requirements and architecture.

### 4.14 Captured rule desired sets

See
[Captured rule desired sets](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#414-captured-rule-desired-sets)
for the canonical requirements and architecture.

### 4.15 Authoritative lifecycle reference reconciliation

See
[Authoritative lifecycle reference reconciliation](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#415-authoritative-lifecycle-reference-reconciliation)
for the canonical requirements and architecture.

### 4.16 Reference-driven shared-owner removal planning

See
[Reference-driven shared-owner removal planning](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#416-reference-driven-shared-owner-removal-planning)
for the canonical requirements and architecture.

### 4.17 Captured lifecycle selection preflight

See
[Captured lifecycle selection preflight](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#417-captured-lifecycle-selection-preflight)
for the canonical requirements and architecture.

### 4.18 Complete single-owner lifecycle upsert planning

See
[Complete single-owner lifecycle upsert planning](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#418-complete-single-owner-lifecycle-upsert-planning)
for the canonical requirements and architecture.

### 4.19 Per-owner recorded contract evidence

See
[Per-owner recorded contract evidence](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#419-per-owner-recorded-contract-evidence)
for the canonical requirements and architecture.

### 4.20 Explicit shared-output reconciliation

See
[Explicit shared-output reconciliation](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#420-explicit-shared-output-reconciliation)
for the canonical requirements and architecture.

### 4.21 Complete selected-set preflight reporting

See
[Complete selected-set preflight reporting](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#421-complete-selected-set-preflight-reporting)
for the opt-in library diagnostic contract and its production activation limits.

## 5. Source format and authentication

Source format: see [§2.1](#21-add).

upskill never injects credentials into clone URLs. Clones use the bare
`https://<host>/...` URL and rely entirely on git's own configuration —
credential helpers, `url.<base>.insteadOf` rewrites, and SSH. A private
repository works whenever a manual `git clone <url>` would; configure git the
same way. upskill resolves no tokens and reads no token env vars.

Any https host is treated as an opaque git remote
(`https://git.mycompany.com/team/repo`), including projects nested under groups
to any depth (`https://git.mycompany.com/group/subgroup/team/repo`).

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
- Global `-q` / `--quiet` suppresses informational stdout. Errors on stderr and
  exit codes are unaffected.
- Single static binary, no runtime dependencies.

### 6.3 Environment variables

| Variable         | Purpose                                        |
| ---------------- | ---------------------------------------------- |
| `NO_COLOR`       | Disable colored output.                        |
| `HTTPS_PROXY`    | HTTP proxy for network requests.               |
| `XDG_CACHE_HOME` | Override cache directory for registry indexes. |

## 7. Out of scope

- Runtime invocation of installed content (each AI client's responsibility).
- Hosting a registry server or central marketplace.
- Content quality review (delegated to AI workflows outside upskill).
- Telemetry, analytics, or usage tracking.
- Publishing to non-Git destinations (npm, crates.io, PyPI).

## 8. References

- Format spec:
  [`docs/specification/format-spec.md`](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md)
- User guide:
  [Getting started](https://driftsys.github.io/upskill/getting-started.html),
  [Commands](https://driftsys.github.io/upskill/commands.html),
  [Recipes](https://driftsys.github.io/upskill/recipes.html)
- Agent Skills open standard: <https://agentskills.io>
- skills.sh ecosystem: <https://skills.sh>
