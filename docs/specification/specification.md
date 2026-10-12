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
- `owner/repo:path/to/name.bundle.yaml` — bundle file (resolves transitively, see §2.6)
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `https://<host>/<path>[...]` — any other https git host: GitLab
  (including self-hosted instances and subgroups at any nesting depth),
  Bitbucket, Gitea, or a plain git server
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

Persisted schema-1 `gitlab:` and `gitlab+<host>:` source labels remain
readable for updates. Unambiguous historical labels decode to generic HTTPS
sources, preserving namespace, ref and subfolder. Applying `update` canonicalizes
the source labels of reinstalled items and bundles, even when their content is
unchanged. `update --dry-run`, `list` and `doctor` do not rewrite lockfiles or
generated output. Aliases, recorded selection, bundle membership and unrelated
item/plugin/MCP records remain intact. This does not introduce schema 2 or infer
artifact ownership.

The supported historical authority grammar and reconciliation limits are defined
in the [ADR-0011 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12).
Unsupported or ambiguous labels fail before fetch with the recorded label and
guidance to reconcile a verified HTTPS URL, ref and subfolder. Modern `add` still
rejects the removed GitLab shorthands with exit 2; use a full HTTPS URL.

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
- `doctor` — verify on-disk state matches the lock file. Five
  independent buckets: missing per-client outputs (reinstall fixes),
  SSOT hash drift on `local:` sources (`update` fixes), lockfile
  entries with no recoverable source (`remove` fixes), installed
  plugins missing from the client (`update` reinstalls), and skipped
  plugins (informational — CLI was absent at install time). Exit 0
  when clean, 1 when any drift bucket is non-empty. Skipped plugins
  are warnings and do not affect the exit code.

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
  bundle in the transitive closure is installed. A bare `{ name }` entry is
  resolved against the same source; a `{ name, source }` entry is resolved
  cross-source (`source` reuses the `upskill add` DSL), and the closure MAY
  span sources. Each bundle and item is recorded in the lockfile under its own
  source. Circular `requires:` is rejected; the same bundle name resolving to
  two different sources within one closure is an error.
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
  ],
  "plugins": []
}
```

### 4.1 Pure legacy-state projection

The internal library boundary `legacy::project_legacy(&Lockfile)` describes typed
schema-1 state without I/O or mutation. It retains an owned snapshot and indexed
item and bundle rows, including order, duplicates, installed aliases, source names,
recorded refs and SSOT hashes, dependency/group links, and raw client strings.
Explicitly tagged plugin and MCP rows preserve their original targets, scopes,
statuses and bundle references. Typed preservation does not recover unknown JSON
fields or the original serialization bytes.

Persisted source labels are decoded with the existing compatibility parser. A
parsed source or recorded ref/hash does not prove an immutable source revision or
ownership of generated bytes. Requested selection remains explicitly unresolved
for every item, including singleton effective client lists. Empty client lists
expand to the fixed historical Claude, Copilot and OpenCode clients; unknown
spellings remain in the snapshot and produce structured diagnostics.

Candidates contain only a historical client, a relative entrypoint and a relative
resource search base. The frozen nine kind/client layouts are shared with the
current pipeline and use the installed alias. They do not inspect roots, files or
resources, establish ownership, or authorize adoption or deletion. Bundle names
and ancillary files do not produce candidates. Invalid installed names suppress
candidates: empty names, dot, dot-dot, slash, backslash, colon and control
characters are invalid; spaces and Unicode remain representable. Invalid sources
retain otherwise known layout facts. Duplicate
identities and identical entrypoints identify all affected item indices without
selecting a winner. Diagnostics retain malformed rows for inspection. Unsupported
schema values retain all typed records but produce no interpreted sources, clients
or destinations.

This prerequisite leaves commands and schema-1 persistence unchanged. Verified
sources, reproducible historical rendering, resource enumeration, adoption,
requested-selection reconciliation, schema-2 lifecycle preservation, exact backup,
containment, preflight, writer locking and recovery remain later work.

See the [projection design](https://github.com/driftsys/upskill/blob/main/docs/design/generation-pipeline.md#pure-legacy-state-projection)
and [AD-0015](https://github.com/driftsys/upskill/blob/main/docs/adr/0015-pure-legacy-state-projection.md) for implementation
boundaries and rationale.

### 4.2 Pure owned instruction planning

The library function `instructions::plan::plan_instructions` joins validated
owned-artifact records with the Markdown aggregate editor. Callers supply complete
installation contribution replacements or removals and exact observations for
every affected file. The result contains the complete resulting artifact vector
and file plans with exact optional before bytes and after bytes, including content
no-ops. Absent files and existing empty files have distinct preconditions. Removing
the last region retains manual bytes and keeps the aggregate file, which is empty
only when no content remains. The plan has no deletion action.

Set replaces one installation's complete contribution set. Existing installation
IDs retain their source, item kind, source name and installed alias; recorded
revision may change. Untouched selectors retain their bodies, owners and consumers
and cannot be taken over, even with identical content. Affected regions require
one owner; affected whole files or shared regions conflict. Unaffected records,
including whole files and shared regions, survive exactly. At affected locations,
the editor rebuilds separator metadata for retained regions after other removals.

`instructions::aggregate::inspect_owned` reads exact verified Markdown bodies in
document order through the same private marker/span validator as reconciliation.
Edited, missing, duplicate, overlapping, unknown or malformed regions conflict.
Bytes outside managed regions remain unchanged. Locations use scope, raw lexical
root spelling and portable relative path; file plans sort by that key. Surviving
artifact rows retain prior order and new selectors append in location/ID order.

Planning performs no I/O and validates prior and final ownership plus all desired
data. Observations must match affected locations exactly; missing, duplicate or
extraneous observations conflict. Supplied bytes do not establish source or
filesystem evidence. Callers must resolve direct/bundle references and native
compatibility first. Complete reference-graph authority, per-owner consumer
association, whole-file/resource plans, state/lifecycle integration and
transactional execution remain pending. This prerequisite changes no CLI behavior,
source format or lockfile format and enables no native client support.

See the [planner design](https://github.com/driftsys/upskill/blob/main/docs/design/owned-instruction-planning.md)
and its decision records for implementation boundaries and rationale.

### 4.3 Strict captured source manifests

The separate library module `source_manifest` provides
`capture_source(&Path) -> anyhow::Result<SourceManifest>` and pure
`validate_manifest(&SourceManifest) -> anyhow::Result<()>`. It does not change
commands, legacy source hashing/discovery, source formats or lockfile writes.
This is a prerequisite for future whole-file and resource planning.

`SourceManifest` contains sorted `CapturedSourceFile` records and a lowercase
`content_sha256`. Each file contains a UTF-8 relative `path` joined with `/`,
owned `bytes`, their lowercase `content_sha256`, and optional `mode`. Capture
includes every observed regular file without ignore rules or entrypoint
exclusions, including nested entrypoints, overrides, empty and binary files.
Empty directories have no record. Each file is read once; on Unix its observed
mode is `PermissionsExt::mode() & 0o7777`, and on other hosts mode is absent.
Mode evidence does not authorize applying permissions or executing content.

Capture reconstructs the root from `Path::components()` before metadata lookup,
so trailing separators and dot suffixes do not hide an observed root symlink.
It rejects observed root symlinks and non-directory roots, every observed entry
symlink and special entry, and enumeration, metadata or file-read errors. Errors
return no partial manifest. Relative paths reject empty, dot and parent components,
backslashes, colons, control characters and non-UTF-8 names. Hard-linked regular
files are represented by each path without a topology claim.

The manifest hash is SHA-256 over these exact bytes, in order:

1. ASCII `upskill:source-manifest:v1` followed by one NUL byte.
2. File count as an unsigned 64-bit big-endian integer.
3. For each file in Rust string path order: path byte length as unsigned 64-bit
   big-endian, UTF-8 path bytes, content byte length in the same encoding, exact
   content bytes, then mode-presence byte (`0` absent or `1` present). A present
   mode follows as an unsigned 32-bit big-endian integer.

Length conversion is checked. Per-file hash fields are verified but are not
additional digest inputs. Root spelling, timestamps and enumeration order are
excluded. Pure validation borrows the public values without mutation or sorting
and checks lexical paths, strict ascending uniqueness, optional mode range
`0..=0o7777`, exact file hashes and the aggregate hash. Public records and valid
hashes do not establish trusted provenance or source authenticity.

Standard synchronous filesystem observation is not an atomic snapshot,
descriptor-relative containment or protection against concurrent replacement.
Root ancestors may be symlinks. Lexical validation does not qualify destination
case collisions, reserved names, aliases or filesystem limits. Immutable Git
evidence, source stabilization, qualified filesystem observation, destination
qualification, whole-file/resource plans, lifecycle state and execution, and
Codex activation remain pending.

## 5. Source format and authentication

Source format: see [§2.1](#21-add).

upskill never injects credentials into clone URLs. Clones use the bare
`https://<host>/...` URL and rely entirely on git's own configuration —
credential helpers, `url.<base>.insteadOf` rewrites, and SSH. A private
repository works whenever a manual `git clone <url>` would; configure git
the same way. upskill resolves no tokens and reads no token env vars.

Any https host is treated as an opaque git remote
(`https://git.mycompany.com/team/repo`), including projects
nested under groups to any depth
(`https://git.mycompany.com/group/subgroup/team/repo`).

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

- Format spec: [`docs/specification/format-spec.md`](./format-spec.md)
- User guide: [Getting started](https://driftsys.github.io/upskill/getting-started.html), [Commands](https://driftsys.github.io/upskill/commands.html), [Recipes](https://driftsys.github.io/upskill/recipes.html)
- Agent Skills open standard: <https://agentskills.io>
- skills.sh ecosystem: <https://skills.sh>
