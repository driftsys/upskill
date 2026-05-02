# upskill

`upskill` lets you author and distribute AI-assistance content — **rules**,
**skills**, and **agents** — across multiple AI coding clients (Claude Code,
GitHub Copilot, opencode) from a single source of truth. One CLI, one set of
files, per-client output generated on demand.

```bash
upskill add owner/repo               # install everything from a source repo
upskill update                       # pull latest, regenerate per-client files
upskill list                         # see what's installed
upskill new skill code-review        # scaffold a new SSOT skill (in a registry)
```

For the full behavioural spec, see [`specification.md`](./specification.md).
For the on-disk contract, see [`format-spec.md`](./format-spec.md).

## Installation

```bash
cargo install upskill
```

Or download a pre-built binary from the [releases page][releases].

## Quick start

### As a consumer

Inside a project where you want AI clients to pick up rules / skills /
agents:

```bash
# Install everything from a source repo (auto-detects clients)
upskill add driftsys/skills

# Install only specific items
upskill add driftsys/skills code-review secret-scanner

# Pin to a tag, branch, or commit
upskill add driftsys/skills@v1.2.0
```

Generated files land in `.claude/`, `.github/`, and `.agents/` per the
[generation pipeline](./specification.md#3-generation-pipeline). The
`.upskill-lock.json` file in your repo records what was installed and at which
ref — commit it.

### As an author

Inside a source-registry repo (where SSOT items live):

```bash
# Scaffold a new item
upskill new skill code-review

# Validate the SSOT before publishing
upskill lint

# Canonicalise frontmatter (key order, quoting)
upskill fmt
```

## Commands

### Consumer commands

#### `upskill add <source> [items...]`

Install content from any source.

```bash
upskill add owner/repo                       # GitHub shorthand
upskill add owner/repo:path/to/items         # subfolder
upskill add owner/repo@v1.2                  # pin to tag
upskill add owner/repo@main                  # pin to branch
upskill add owner/repo@abc123                # pin to commit SHA
upskill add gitlab:owner/repo                # GitLab.com
upskill add https://gitlab.example.com/owner/repo  # self-hosted GitLab
upskill add ./path/to/local                  # local directory
upskill add platform-defaults                # bundle from a configured registry
```

`upskill add <source>` installs **everything** the source contains. Append
item names to filter:

```bash
upskill add driftsys/skills code-review secret-scanner
```

Default scope is `--project` (writes into `.agents/...` of the current repo),
falling back to `--global` (`$HOME/.agents/...`) if you're not inside a git
repo. Pass either flag explicitly to override.

#### `upskill update [name...]`

Pull latest sources and regenerate changed items.

```bash
upskill update                       # update everything
upskill update code-review           # update one item
upskill update --dry-run             # preview changes without applying
```

`update` always fetches before regenerating; there is no separate `sync`. It
also re-runs the generation pipeline against the current upskill version, so
client-format updates land without a separate command.

#### `upskill remove [name...]`

Remove installed items.

```bash
upskill remove code-review
upskill remove --global code-review
```

#### `upskill list`

Show installed content from `.upskill-lock.json`, grouped by kind. Bundles,
when present, are surfaced as a separate section. (`--available` to dump
items discoverable from configured sources is deferred for a future release;
v0.2.0 ships the installed-state view only.)

#### `upskill doctor`

Verify on-disk state matches `.upskill-lock.json`. Reports drift in three
independent buckets: missing per-client output files (reinstall fixes),
SSOT hash drift on `local:` sources (`update` fixes), and lockfile
entries with no recoverable source (`remove` fixes). Exits non-zero
when any bucket is non-empty.

### Author commands

Run inside a **source-registry** working tree.

#### `upskill new <kind> <name>`

Scaffold a new SSOT item directory.

```bash
upskill new rule  no-direct-database-access
upskill new skill code-review
upskill new agent security-reviewer
```

Creates the kind-appropriate directory and entrypoint file (`RULE.md`,
`SKILL.md`, `AGENT.md`) with starter frontmatter.

#### `upskill lint [paths...]`

Validate SSOT files against the [format spec](./format-spec.md).

```bash
upskill lint                # lint everything in the working tree
upskill lint rules/         # lint a subtree
upskill lint --strict       # CI mode: warnings become errors
```

#### `upskill fmt [paths...]`

Canonicalise YAML frontmatter (key order, version quoting, indentation).
Markdown body formatting is left to dprint — the two tools don't overlap.

## Recipes

### CI usage

```bash
# Install without prompts (auto-detects NO_COLOR, non-TTY)
upskill add owner/repo

# Lint in CI (fail on warnings)
upskill lint --strict
```

In a non-TTY environment, `upskill` skips interactive prompts and disables
colored output when `NO_COLOR` is set.

### Private repositories

Token resolution per host:

```bash
# GitHub
export GITHUB_TOKEN=ghp_...      # or GH_TOKEN, or rely on `gh auth token`

# GitLab
export GITLAB_TOKEN=glpat_...    # or GL_TOKEN, or rely on `glab auth token`
```

### Pin a source to a specific version

```bash
upskill add owner/repo@v1.2.0
```

The pinned ref is recorded in `.upskill-lock.json`. `upskill update` re-fetches
from the same ref unless you bump it.

## State files

Per [specification §4](./specification.md#4-state-files):

| File                        | Scope       | Committed? | Purpose                              |
| --------------------------- | ----------- | ---------- | ------------------------------------ |
| `.upskill-lock.json`        | Per-project | Yes        | Deterministic regeneration in CI.    |
| `~/.upskill/installed.json` | Per-user    | No         | Global install state, source caches. |

v0.1 users keep the same `.upskill-lock.json` filename. v0.2 stamps a
`schema: 2` field on first run and rewrites the file in place with the new
entry shape — see
[ADR-0003](./adr/0003-generation-pipeline.md). No manual migration step.

## Per-client output paths

Per [ADR-0003](./adr/0003-generation-pipeline.md):

| Item kind | Claude Code                | GitHub Copilot                                | opencode                       |
| --------- | -------------------------- | --------------------------------------------- | ------------------------------ |
| Rule      | `.claude/rules/<name>.md`  | `.github/instructions/<name>.instructions.md` | `.agents/rules/<name>/RULE.md` |
| Skill     | `.claude/skills/<name>/`   | `.github/skills/<name>/`                      | `.agents/skills/<name>/`       |
| Agent     | `.claude/agents/<name>.md` | `.github/agents/<name>.agent.md`              | `.opencode/agents/<name>.md`   |

All output is **copy** (not symlink) — Windows portability without Developer
Mode. The deliberate divergence from skills.sh is documented in
[ADR-0005](./adr/0005-skills-sh-ecosystem-interop.md).

## Exit codes

| Code | Meaning                |
| ---- | ---------------------- |
| 0    | Success                |
| 1    | General error          |
| 2    | Usage error (bad args) |
| 130  | Interrupted (Ctrl+C)   |

[releases]: https://github.com/driftsys/upskill/releases
