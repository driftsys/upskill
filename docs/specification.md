# upskill — Specification

> Upskill your coding agents.

## 1. Overview

`upskill` is a cross-agent skills package manager for the
[Agent Skills][agent-skills-spec] ecosystem. It installs,
lists, searches, removes, and scaffolds `SKILL.md` packages across all major
coding agents from a single static Rust binary.

### 1.1 Design principles

- **Canonical target**: `.agents/skills/` is always the install destination.
  Agent-specific directories receive symlinks.
- **Zero runtime**: single static binary, no Node.js, no Python, no npm, no npx.
- **Any repo is a registry**: any GitHub repository containing `SKILL.md` files
  is a valid source. No central registry required.
- **Human-first CLI**: compliant with [clig.dev][clig] guidelines,
  POSIX conventions, and GNU flag standards.
- **Minimal footprint**: ~2-3 MB binary, instant startup, no background
  processes, no telemetry.

### 1.2 Features

- [x] **Install from GitHub/GitLab** — fetch via `git clone --depth 1`; supports
      GitHub shorthand, GitLab prefix, full URLs, self-hosted GitLab
- [x] **Canonical `.agents/skills/` target** — always install to the cross-agent
      standard directory
- [x] **Agent symlinks** — auto-detect or explicitly target Claude Code (`--claude`)
      and Copilot (`--copilot`); `--all` targets all 7 supported agents
- [x] **Selective install** — pick specific skills by name (`--skill`) or
      interactive multi-select in a TTY
- [x] **Install from local path** — use a local directory as source for testing
      or monorepo workflows
- [x] **List installed skills** — show name, source, and symlink status (`list`)
- [x] **Remove skills** — clean removal of canonical copy and all agent symlinks
      (`remove`)
- [x] **Global install** — install to `~/.agents/skills/` for cross-project
      availability (`-g`)
- [x] **CI-friendly mode** — no prompts in non-TTY, TTY detection for color
- [x] **Lockfile** — track installed skills with source, pinned ref, and content hash
- [x] **Branch/tag/commit pinning** — install a specific ref with `owner/repo@v1.0`
- [x] **Private repos** — authenticate via `GITHUB_TOKEN` / `GITLAB_TOKEN` env
      vars, or reuse existing `gh` / `glab` CLI login
- [x] **Update** — re-fetch skills from their recorded source; hash-based local
      modification detection skips modified skills unless `--force` is passed
- [x] **Registry search** — `upskill search <query>` hits the skills.sh public API
- [ ] **Custom registries** — configure private or internal skill sources via
      `.upskill/registries.toml` (v0.4)

## 2. Installation

```bash
# From crates.io
cargo install upskill

# From source
git clone https://github.com/TODO/upskill
cd upskill
cargo install --path .

# Pre-built binary (GitHub Releases)
curl -fsSL https://github.com/TODO/upskill/releases/latest/download/upskill-$(uname -m)-$(uname -s | tr A-Z a-z) -o ~/.local/bin/upskill
chmod +x ~/.local/bin/upskill
```

Uninstall: `cargo uninstall upskill` or delete the binary.

## 3. Commands

### 3.1 `upskill add` — install skills

```
upskill add <source> [options]
```

**Source formats:**

| Format                  | Example                                |
| ----------------------- | -------------------------------------- |
| GitHub repo             | `microsoft/skills`                     |
| GitHub repo + subfolder | `owner/repo:skills/my-tool`            |
| Local directory         | `./path/to/skills` or `/absolute/path` |

**Options:**

| Flag          | Short | Description                                    |
| ------------- | ----- | ---------------------------------------------- |
| `--skill <n>` | `-s`  | Specific skill name(s) to install. Repeatable. |
| `--all`       |       | Symlink to all 7 supported agent directories.  |
| `--global`    | `-g`  | Install to `~/.agents/skills/` (user-level).   |
| `--copy`      |       | Independent copies instead of symlinks.        |
| `--claude`    |       | Symlink to `.claude/skills/`.                  |
| `--copilot`   |       | Symlink to `.github/skills/`.                  |

No explicit flag: auto-detect from existing agent config directories in CWD.

**Behavior:**

1. Fetch the source (`git clone --depth 1` for GitHub/GitLab, direct read for
   local).
2. Resolve the subfolder if `:path` was specified.
3. Select skills: `--skill <n>`, interactive prompt in a TTY, or default to repo
   name in CI.
4. Copy to `.agents/skills/<name>/` (canonical target).
5. Create agent-specific symlinks:
   - If `--claude` or `--copilot` flags given → symlink only those.
   - If `--all` → symlink all 7 supported agent dirs.
   - If no flags → auto-detect: walk CWD for existing agent config dirs, symlink
     those.
6. Write/update `.upskill-lock.json` with source, pinned ref, and content hash.
7. For `--global`: install to `~/.agents/skills/`, no agent symlinks created.

**Examples:**

```bash
# Interactive selection, auto-detect agents
upskill add microsoft/skills

# Specific skill, specific agents
upskill add owner/repo --skill markspec --claude --cursor

# List available skills
upskill add microsoft/skills --list

# CI-friendly
upskill add owner/repo --skill my-skill --claude -y

# From local path
upskill add ./my-local-skills --skill my-tool

# Global install
upskill add owner/repo -s azure-cosmos-db-py -g --claude
```

### 3.2 `upskill list` — list installed skills

```
upskill list [options]
```

| Flag       | Short | Description              |
| ---------- | ----- | ------------------------ |
| `--global` | `-g`  | Show global skills only. |

Lists all skills found in `.agents/skills/` (project) or `~/.agents/skills/`
(global), with name, description, and symlink status.

### 3.3 `upskill search` — search for skills

```
upskill search <query> [options]
```

| Flag      | Default | Description                |
| --------- | ------- | -------------------------- |
| `--limit` | `10`    | Maximum number of results. |

Queries the [skills.sh](https://skills.sh) public search API (no auth required).

```
$ upskill search rust
rust-mcp-server-generator    7608 installs    upskill add awesome-copilot --skill rust-mcp-server-generator
rust-analyzer                3200 installs    upskill add anthropics/skills --skill rust-analyzer
```

Each result includes the install command to copy-paste directly.

**Custom registries** (v0.4): once `.upskill/registries.toml` is supported,
`upskill search` will also query configured private registries.

### 3.4 `upskill remove` — remove skills

```
upskill remove [name...] [options]
```

| Flag       | Description                  |
| ---------- | ---------------------------- |
| `--all`    | Remove all installed skills. |
| `--global` | Target global scope.         |

Removes the skill directory from `.agents/skills/` and any symlinks in
agent-specific directories. Without arguments: interactive multi-select.

### 3.5 `upskill check` — show lockfile state

```
upskill check [options]
```

| Flag       | Description               |
| ---------- | ------------------------- |
| `--global` | Check global skills only. |

Reads `.upskill-lock.json` and prints each skill's recorded source and pinned ref.

```
$ upskill check
my-skill    github:owner/repo    pinned: latest
other-skill github:owner/repo    pinned: v1.2
```

Does not make any network requests — use `upskill update` to re-fetch.

### 3.6 `upskill update` — re-install skills from recorded sources

```
upskill update [name...] [options]
```

| Flag        | Description                              |
| ----------- | ---------------------------------------- |
| `--global`  | Update global skills only.               |
| `--force`   | Overwrite locally modified skills.       |
| `--dry-run` | Show what would change without applying. |

**Behavior:**

1. Load `.upskill-lock.json`.
2. Filter to requested names (or all if none given).
3. If `--dry-run`: print what would be updated and return.
4. For each skill:
   - Compare SHA-256 content hash against the lockfile's stored hash.
   - If hash differs and `--force` is not set: warn and skip.
   - Otherwise: re-run `persist_installed_skills` from the recorded source.
5. Print updated skill names; warn about skipped skills.

```
$ upskill update
updated: my-skill
warning: other-skill has local modifications, skipping (use --force to overwrite)
1 skill(s) skipped due to local modifications
```

**`--dry-run` output:**

```
$ upskill update --dry-run
dry-run: would update my-skill from github:owner/repo (latest)
```

## 4. Agent directory conventions

### 4.1 Canonical install path

All skills are always installed to:

| Scope   | Path                       |
| ------- | -------------------------- |
| Project | `.agents/skills/<name>/`   |
| Global  | `~/.agents/skills/<name>/` |

### 4.2 Agent-specific symlinks

| Agent       | Project symlink target    | Global symlink target               | Reads `.agents/` natively |
| ----------- | ------------------------- | ----------------------------------- | ------------------------- |
| Claude Code | `.claude/skills/<name>`   | `~/.claude/skills/<name>`           | No (planned)              |
| Copilot     | `.github/skills/<name>`   | `~/.copilot/skills/<name>`          | Yes                       |
| Codex       | `.codex/skills/<name>`    | `~/.codex/skills/<name>`            | Yes                       |
| Cursor      | `.cursor/skills/<name>`   | `~/.cursor/skills/<name>`           | No                        |
| Kiro        | `.kiro/skills/<name>`     | `~/.kiro/skills/<name>`             | No                        |
| Windsurf    | `.windsurf/skills/<name>` | `~/.codeium/windsurf/skills/<name>` | Yes                       |
| OpenCode    | `.opencode/skills/<name>` | `~/.config/opencode/skills/<name>`  | Yes                       |

### 4.3 Symlink resolution order

1. Explicit flags (`--claude`, `--cursor`, ...) → create those symlinks only.
2. `--all` → create symlinks for all known agents.
3. No flags → auto-detect: walk the project root for existing agent directories
   (`.claude/`, `.github/`, `.cursor/`, etc.). Create symlinks only for those
   found.

### 4.4 Directory creation policy

- `upskill` creates `.agents/skills/` if it does not exist.
- `upskill` creates `<agent>/skills/` if `<agent>/` already exists.
- `upskill` never creates an agent config directory from scratch (e.g., will not
  create `.claude/` — only `.claude/skills/` if `.claude/` exists).

### 4.5 Symlink vs copy

Default: symlinks from agent dirs to `.agents/skills/`. One source of truth,
updates propagate.

`--copy`: independent copies in each agent dir. Use when symlinks are not
supported (Windows without developer mode, read-only container mounts).

## 5. Source fetch

### 5.1 Source format

The `<source>` argument determines where skills are fetched from. `upskill`
detects the source type from the format of the string:

| Format                          | Source type | Example                                         |
| ------------------------------- | ----------- | ----------------------------------------------- |
| `owner/repo`                    | GitHub      | `microsoft/skills`                              |
| `owner/repo:path`               | GitHub      | `owner/repo:skills/my-tool`                     |
| `owner/repo@ref`                | GitHub      | `owner/repo@v1.0` (v0.2)                        |
| `owner/repo@ref:path`           | GitHub      | `owner/repo@main:skills/my-tool` (v0.2)         |
| `https://github.com/owner/repo` | GitHub      | Full URL, same behavior as `owner/repo`         |
| `https://gitlab.com/owner/repo` | GitLab      | Full GitLab URL (v0.2)                          |
| `gitlab:owner/repo`             | GitLab      | Short prefix form (v0.2)                        |
| `./path`, `/path`, `~/path`     | Local       | `./my-skills`, `/opt/skills`, `~/dev/my-skills` |

**Detection rules** (evaluated in order):

1. Starts with `./`, `../`, `/`, or `~` → **local path**.
2. Starts with `https://github.com/` → **GitHub URL**, extract `owner/repo` from
   path.
3. Starts with `https://gitlab.com/` or `gitlab:` → **GitLab** (v0.2).
4. Starts with `https://` → **unsupported host**, error with message.
5. Matches `{owner}/{repo}` (with optional `@ref` and `:path`) → **GitHub
   shorthand**.
6. Otherwise → error.

### 5.2 GitHub fetch

**Mechanism:** `git clone --depth 1` into a temporary directory. Requires `git`
on PATH. The clone is cleaned up after install.

```
git clone --depth 1 [--branch <ref>] https://github.com/{owner}/{repo}.git <tmpdir>
```

**Ref support:** when `@ref` is specified, passed as `--branch <ref>`. Accepts
branch names, tag names, and full commit SHAs.

**Subfolder filtering:** when `:path` is specified, only the subtree at `path`
within the cloned repo is used for install. The full shallow clone is still
fetched.

**Authentication (resolution order):**

| Priority | Method                 | Mechanism                                                        |
| -------- | ---------------------- | ---------------------------------------------------------------- |
| 1        | `GITHUB_TOKEN` env var | Injected into clone URL as `https://token@github.com/...`.       |
| 2        | `GH_TOKEN` env var     | Same as above (matches `gh` CLI convention).                     |
| 3        | `gh auth token` output | Shell out to `gh auth token`, use result.                        |
| 4        | Unauthenticated        | Public repos only. Private repos fail with git credential error. |

The `gh` CLI fallback means: if the user already ran `gh auth login`, `upskill`
reuses that session without any extra configuration.

### 5.3 GitLab fetch (v0.2)

**Mechanism:** same principle, different URL format.

**URL construction:**

```
https://gitlab.com/{owner}/{repo}/-/archive/{branch}/{repo}-{branch}.tar.gz
```

**Detection:** source starts with `https://gitlab.com/` or `gitlab:` prefix.

**Self-hosted GitLab:** full URL form supports arbitrary hosts:

```bash
upskill add https://gitlab.mycompany.com/team/skills-repo
```

**Authentication (resolution order):**

| Priority | Method                   | Mechanism                                               |
| -------- | ------------------------ | ------------------------------------------------------- |
| 1        | `GITLAB_TOKEN` env var   | `PRIVATE-TOKEN` header (matches GitLab API convention). |
| 2        | `GL_TOKEN` env var       | Same as above.                                          |
| 3        | `glab auth token` output | Shell out to `glab auth token`, use result.             |
| 4        | Unauthenticated          | Public repos only.                                      |

### 5.4 Local path fetch

**Detection:** source starts with `.`, `/`, or `~`.

**Behavior:** no network request. The path is resolved to an absolute path and
scanned directly for skills. Supports:

- Relative paths: `./my-skills` (resolved from CWD).
- Absolute paths: `/opt/company/skills`.
- Home expansion: `~/dev/my-skills` (expanded via `dirs::home_dir()`).
- Symlinks: followed transparently.

**Use cases:**

- Testing a skill before publishing.
- Installing from a local git clone.
- Sharing skills on a network mount or monorepo.

```bash
# From a local clone
upskill add ~/dev/markspec

# From a monorepo subfolder (: syntax not needed, just use the path)
upskill add ./packages/markspec/skills

# From a network mount
upskill add /mnt/team-share/agent-skills
```

**Error handling:**

| Condition           | Behavior                                           |
| ------------------- | -------------------------------------------------- |
| Path does not exist | Error: `"Source path does not exist: {path}"`.     |
| Not a directory     | Error: `"Source path is not a directory: {path}"`. |
| No SKILL.md found   | Warning: `"No skills found in {path}"`.            |
| Permission denied   | Error: `"Cannot read {path}: permission denied"`.  |

### 5.5 Skill discovery

After the source is fetched (or resolved locally), `upskill` scans for skills:

1. Walk the directory tree recursively, max depth 4.
2. Skip `.git/` directories.
3. For each `SKILL.md` file found: a. Parse YAML frontmatter (content between
   first `---` pair). b. Deserialize into `SkillMeta` struct. c. Validate
   required fields (`name`, `description`). d. On validation failure: warn to
   stderr, skip this skill, continue.
4. Sort discovered skills by name.

**Why max depth 4:** covers all known layouts:

```
depth 1: skills/markspec/SKILL.md
depth 2: .github/skills/markspec/SKILL.md
depth 3: packages/foo/skills/markspec/SKILL.md
depth 4: src/packages/foo/skills/markspec/SKILL.md
```

### 5.6 Frontmatter validation

Per the [Agent Skills specification][agent-skills-spec]:

| Field           | Required | Constraints                                                        |
| --------------- | -------- | ------------------------------------------------------------------ |
| `name`          | Yes      | 1-64 chars, lowercase alphanum + hyphens, matches parent dir name. |
| `description`   | Yes      | 1-1024 chars, non-empty.                                           |
| `license`       | No       | Informational.                                                     |
| `compatibility` | No       | 1-500 chars.                                                       |
| `metadata`      | No       | Arbitrary key-value map.                                           |
| `allowed-tools` | No       | Experimental, passed through.                                      |

**Validation strictness:** lenient. An invalid skill in one subdirectory
produces a warning but does not abort the entire operation. Valid skills in the
same source are still offered for installation.

**Common validation failures:**

| Issue                    | Severity | Message                                                 |
| ------------------------ | -------- | ------------------------------------------------------- |
| Missing `name`           | Error    | `"SKILL.md missing required 'name' field"`              |
| Missing `description`    | Error    | `"SKILL.md missing required 'description' field"`       |
| `name` has uppercase     | Warning  | `"Skill name should be lowercase: {name}"`              |
| `name` doesn't match dir | Warning  | `"Skill name '{name}' doesn't match directory '{dir}'"` |
| No YAML frontmatter      | Error    | `"No YAML frontmatter found in SKILL.md"`               |
| YAML parse error         | Error    | `"Failed to parse frontmatter: {err}"`                  |

## 6. CLI compliance

### 6.1 clig.dev guidelines

`upskill` follows the [Command Line Interface Guidelines][clig]:

| Guideline                                | Implementation                                                   |
| ---------------------------------------- | ---------------------------------------------------------------- |
| Human-first design                       | Natural subcommand verbs, clear `--help`, interactive fallbacks. |
| Composability                            | Stdout for data, stderr for progress/errors. Pipe-friendly.      |
| `--help` and `--version` on all commands | Provided by `clap` derive.                                       |
| Standard flag names                      | `--yes`, `--all`, `--global`, `--version`, `--help`.             |
| Short flags only for common options      | `-s` (skill), `-g` (global), `-y` (yes), `-l` (list).            |
| Color and formatting                     | Honors `NO_COLOR` env var. Uses `console` crate for detection.   |
| Error messages to stderr                 | All warnings and errors go to stderr via `eprintln!`.            |
| Confirmation for destructive actions     | `remove` prompts unless `--yes` is passed.                       |
| Single binary distribution               | Static Rust binary, no runtime dependencies.                     |
| Easy uninstall                           | `cargo uninstall upskill` or delete binary.                      |

### 6.2 Exit codes

| Code  | Meaning                              |
| ----- | ------------------------------------ |
| `0`   | Success.                             |
| `1`   | General error (network, parse, I/O). |
| `2`   | Usage error (invalid args or flags). |
| `130` | Interrupted (Ctrl+C / SIGINT).       |

### 6.3 POSIX / GNU conventions

- Long flags: `--flag-name` (GNU double-dash).
- Short flags: `-f` (POSIX single-dash, single letter).
- `--` terminates flag parsing.
- Subcommand aliases: `ls` for `list`, `rm` for `remove`, `i` for `add`.
- No positional flag overloading.

### 6.4 Environment variables

| Variable               | Purpose                                                         |
| ---------------------- | --------------------------------------------------------------- |
| `NO_COLOR`             | Disable colored output.                                         |
| `GITHUB_TOKEN`         | Authenticate GitHub requests (private repos).                   |
| `GH_TOKEN`             | Fallback for `GITHUB_TOKEN` (matches `gh` CLI).                 |
| `GITLAB_TOKEN`         | Authenticate GitLab requests (private repos).                   |
| `GL_TOKEN`             | Fallback for `GITLAB_TOKEN` (matches `glab` CLI).               |
| `UPSKILL_REGISTRY_URL` | Override the skills.sh base URL (default: `https://skills.sh`). |
| `HTTPS_PROXY`          | HTTP proxy for network requests.                                |

If no token env var is set, `upskill` shells out to `gh auth token` or
`glab auth token` as a final fallback before falling back to unauthenticated
access.

### 6.5 Output modes

| Context         | Behavior                                                |
| --------------- | ------------------------------------------------------- |
| Interactive TTY | Colored output, progress spinners, interactive prompts. |
| Piped / CI      | No color, no spinners, no prompts. Machine-parseable.   |
| `--yes`         | Suppress all confirmation prompts.                      |

## 7. File layout

### 7.1 Project structure after install

```
project/
├── .agents/
│   └── skills/
│       └── markspec/             ← canonical copy (always)
│           ├── SKILL.md
│           ├── references/
│           └── scripts/
├── .claude/
│   └── skills/
│       └── markspec → ../../.agents/skills/markspec    ← symlink
├── .github/
│   └── skills/
│       └── markspec → ../../.agents/skills/markspec    ← symlink
└── ...
```

### 7.2 Global structure after install

```
~/
├── .agents/
│   └── skills/
│       └── markspec/             ← canonical copy
├── .claude/
│   └── skills/
│       └── markspec → ../../.agents/skills/markspec
├── .copilot/
│   └── skills/
│       └── markspec → ../../.agents/skills/markspec
└── ...
```

### 7.3 Lockfile

`.upskill-lock.json` in project root (or `~/.upskill-lock.json` for global):

```json
{
  "skills": [
    {
      "name": "my-skill",
      "source": "github:owner/repo@v1.2",
      "ref": "v1.2",
      "hash": "a3f8c2..."
    }
  ]
}
```

| Field    | Description                                                               |
| -------- | ------------------------------------------------------------------------- |
| `name`   | Skill directory name.                                                     |
| `source` | Full source label including prefix and ref.                               |
| `ref`    | Pinned git ref (omitted if tracking latest).                              |
| `hash`   | SHA-256 of all files in the skill directory (for modification detection). |

Skills are sorted by name (deterministic output). Commit this file to track
exact skill versions across environments.

### 7.4 Registries configuration

`upskill search` queries skill registries. Well-known public registries are
built-in. Custom registries are configured via a `registries.toml` file.

**File locations (merged, project overrides global):**

| Scope   | Path                         |
| ------- | ---------------------------- |
| Project | `.upskill/registries.toml`   |
| Global  | `~/.upskill/registries.toml` |

**Format:**

```toml
# Custom registries (added to the built-in well-known list)

[[registry]]
name = "internal"
source = "https://gitlab.mycompany.com/platform/agent-skills"
token_env = "GITLAB_TOKEN"

[[registry]]
name = "team-tools"
source = "myorg/team-skills"

[[registry]]
name = "vendor"
source = "vendor/sdk-skills"
branch = "stable"
```

**Fields:**

| Field       | Required | Description                                                                           |
| ----------- | -------- | ------------------------------------------------------------------------------------- |
| `name`      | Yes      | Short identifier, used in `upskill search` output and `upskill add <name> --skill`.   |
| `source`    | Yes      | GitHub shorthand (`owner/repo`), GitLab shorthand (`gitlab:owner/repo`), or full URL. |
| `branch`    | No       | Branch to query. Default: `main`.                                                     |
| `token_env` | No       | Environment variable holding the auth token for this registry.                        |

**Built-in well-known registries** (always queried, no config needed):

| Name            | Source                   |
| --------------- | ------------------------ |
| anthropic       | `anthropics/skills`      |
| microsoft       | `microsoft/skills`       |
| awesome-copilot | `github/awesome-copilot` |
| skills.sh       | skills.sh index          |

Custom registries are queried after the built-in ones. A custom registry with
the same `name` as a built-in one overrides it.

**Install from a named registry:**

```bash
# "internal" resolves to the source URL from registries.toml
upskill add internal --skill k8s-deploy
```

### Deferred / as-needed

| Feature                             | Trigger                                       |
| ----------------------------------- | --------------------------------------------- |
| New agent paths (Antigravity, etc.) | When agents ship SKILL.md support             |
| `upskill publish` (PR-based)        | If manual PR workflow proves insufficient     |
| Skill scaffolding (`init`)          | If `mkdir` + template copy proves too tedious |
| `UPSKILL_HOME` override             | User request                                  |

### Explicitly out of scope

- Runtime skill invocation (each agent's responsibility).
- Hosting a registry server or central marketplace.
- Skill validation beyond frontmatter (use `skills-ref validate`).
- GUI or TUI beyond interactive prompts.
- Telemetry, analytics, or usage tracking.
- Publishing to non-Git destinations (npm, crates.io, PyPI).
- Skill rating or ranking (no infrastructure for it).

## 8. References

### Standards

- [Agent Skills specification](https://agentskills.io/specification) — SKILL.md
  format, progressive disclosure, validation rules.
- [clig.dev](https://clig.dev) — Command Line Interface Guidelines.
- [GNU Coding Standards — Command-Line Interfaces][gnu-cli]
  — flag naming, `--help`, `--version`.
- [POSIX Utility Conventions][posix-utility]
  — argument syntax, exit codes.
- [NO_COLOR][no-color] — color output control convention.
- [XDG Base Directory Specification][xdg-basedir]
  — config/data directory conventions.

### Agent documentation

- [Claude Code — Agent Skills][claude-skills]
- [GitHub Copilot — Agent Skills][copilot-skills]
- [OpenAI Codex — Skills][codex-skills]
- [Kiro — Skills][kiro-skills]
- [Windsurf — Cascade Skills][windsurf-skills]
- [OpenCode — Skills][opencode-skills]
- [Microsoft Agent Framework — Skills][ms-agent-skills]

### Ecosystem

- [Anthropic skills repo][anthropic-repo]
- [Microsoft skills repo][microsoft-repo]
- [GitHub awesome-copilot][awesome-copilot-repo]
- [Vercel npx skills][vercel-skills]
- [skills.sh — Agent Skills directory][skills-sh]

<!-- Reference links -->

[agent-skills-spec]: https://agentskills.io/specification
[clig]: https://clig.dev
[gnu-cli]: https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html
[posix-utility]: https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html
[no-color]: https://no-color.org/
[xdg-basedir]: https://specifications.freedesktop.org/basedir-spec/latest/
[claude-skills]: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview
[copilot-skills]: https://docs.github.com/en/copilot/concepts/agents/about-agent-skills
[codex-skills]: https://developers.openai.com/codex/skills
[kiro-skills]: https://kiro.dev/docs/skills/
[windsurf-skills]: https://docs.windsurf.com/windsurf/cascade/skills
[opencode-skills]: https://opencode.ai/docs/skills/
[ms-agent-skills]: https://learn.microsoft.com/en-us/agent-framework/agents/skills
[anthropic-repo]: https://github.com/anthropics/skills
[microsoft-repo]: https://github.com/microsoft/skills
[awesome-copilot-repo]: https://github.com/github/awesome-copilot
[vercel-skills]: https://github.com/vercel-labs/skills
[skills-sh]: https://skills.sh/
