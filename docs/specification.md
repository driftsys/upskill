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

- [ ] **Install from GitHub/GitLab** — fetch via tarball with
      `git clone --depth 1` fallback for self-hosted or non-standard hosts
- [ ] **Canonical `.agents/skills/` target** — always install to the cross-agent
      standard directory
- [ ] **Agent symlinks** — auto-detect or explicitly target Claude Code,
      Copilot, Codex, Cursor, Kiro, Windsurf, OpenCode
- [ ] **Selective install** — pick specific skills by name (`--skill`) or
      interactive multi-select
- [ ] **Install from local path** — use a local directory as source for testing
      or monorepo workflows
- [ ] **List installed skills** — show name, description, source, and symlink
      status (`list`)
- [ ] **Remove skills** — clean removal of canonical copy and all agent symlinks
      (`remove`)
- [ ] **Global install** — install to `~/.agents/skills/` for cross-project
      availability (`-g`)
- [ ] **CI-friendly mode** — `--yes` flag, no prompts, TTY detection for color
      and spinners
- [ ] **Lockfile** — track installed skills with source, branch, and commit SHA
- [ ] **Branch/tag/commit pinning** — install a specific ref with
      `owner/repo@v1.0`
- [ ] **Private repos** — authenticate via `GITHUB_TOKEN` / `GITLAB_TOKEN` env
      vars, or reuse existing `gh` / `glab` CLI login
- [ ] **Check and update** — compare installed skills against upstream, re-fetch
      outdated ones
- [ ] **Registry search** — search skills across well-known registries
      (skills.sh, awesome-copilot, anthropics/skills, microsoft/skills)
- [ ] **Custom registries** — configure private or internal skill sources via
      `registries.toml`

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
| `--all`       |       | Install all skills from the source.            |
| `--list`      | `-l`  | List available skills without installing.      |
| `--global`    | `-g`  | Install to `~/.agents/skills/` (user-level).   |
| `--yes`       | `-y`  | Skip confirmation prompts.                     |
| `--copy`      |       | Independent copies instead of symlinks.        |
| `--claude`    |       | Symlink to `.claude/skills/`.                  |
| `--copilot`   |       | Symlink to `.github/skills/`.                  |
| `--codex`     |       | Symlink to `.codex/skills/`.                   |
| `--cursor`    |       | Symlink to `.cursor/skills/`.                  |
| `--kiro`      |       | Symlink to `.kiro/skills/`.                    |
| `--windsurf`  |       | Symlink to `.windsurf/skills/`.                |
| `--opencode`  |       | Symlink to `.opencode/skills/`.                |

**Behavior:**

1. Fetch the source (tarball for GitHub, direct read for local).
2. Scan for `SKILL.md` files (max depth 4).
3. Select skills: `--all`, `--skill <n>`, or interactive multi-select.
4. Copy to `.agents/skills/<name>/` (always).
5. Create agent-specific symlinks:
   - If agent flags (`--claude`, `--kiro`, etc.) are given → symlink only those.
   - If `--all` → symlink all known agent dirs.
   - If no flags → auto-detect existing agent dirs in project root, symlink
     those.
6. For `--global`: same logic but under `$HOME`.

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
upskill search [query] [options]
```

| Flag             | Description                                |
| ---------------- | ------------------------------------------ |
| `--registry <n>` | Search a specific registry only.           |
| `--local`        | Search installed skills only (no network). |

Searches for skills across installed skills and configured registries.

**Local search (no query or `--local`):**

```
$ upskill search --local pdf
  .agents/skills/pdf   Extract text, fill forms, merge PDFs   (anthropics/skills)
```

**Registry search (default when query is given):**

Queries all well-known registries plus any custom registries from
`registries.toml`:

```
$ upskill search deployment

  SOURCE                SKILL                DESCRIPTION
  internal              k8s-deploy           Deploy to production k8s clusters
  microsoft/skills      azure-deploy-py      Azure deployment workflow
  awesome-copilot       deploy-checklist     Pre-deployment safety checks
```

Install directly from a search result:

```
upskill add microsoft/skills --skill azure-deploy-py
```

**Well-known registries** (queried by default):

| Name            | Source                   |
| --------------- | ------------------------ |
| skills.sh       | skills.sh index API      |
| anthropic       | `anthropics/skills`      |
| microsoft       | `microsoft/skills`       |
| awesome-copilot | `github/awesome-copilot` |

**How registry search works:**

1. For each registry, fetch the skill index (tarball scan, cached locally for 1
   hour).
2. Match `query` against skill `name` and `description` fields (case-insensitive
   substring).
3. Deduplicate by skill name across registries (first match wins by registry
   order).
4. Display results sorted by registry priority.

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

### 3.5 `upskill check` — check for updates (v0.2)

```
upskill check [options]
```

| Flag       | Description               |
| ---------- | ------------------------- |
| `--global` | Check global skills only. |

Compares installed skills against their upstream source using the lockfile.

**Prerequisite:** `.upskill-lock.json` must exist in the project root (or
`~/.upskill-lock.json` for global). Skills installed before lockfile support are
not tracked — re-install with v0.2+ to enable tracking.

```
$ upskill check
No lockfile found. Re-install skills with `upskill add` to enable update tracking.
```

**Behavior:**

1. Read `.upskill-lock.json`.
2. For each installed skill, query the source repo's latest commit on the
   recorded branch:
   - GitHub: `HEAD https://api.github.com/repos/{owner}/{repo}/commits/{branch}`
     — returns SHA, lightweight.
   - GitLab:
     `GET https://gitlab.com/api/v4/projects/{id}/repository/branches/{branch}`.
   - Local source: compare file modification times against lockfile timestamp.
3. Compare remote HEAD SHA against lockfile's recorded `commit`.
4. Print results:

```
$ upskill check
  markspec         owner/markspec@main   ✓ up to date
  azure-cosmos-db  microsoft/skills@main ⬆ update available (abc123 → def456)
  my-local-tool    ./my-tools            — local source (no remote tracking)

1 update available. Run `upskill update` to apply.
```

**Pinned skills** (installed with a specific ref like `owner/repo@v1.0`) are
reported as pinned and not checked:

```
markspec  owner/markspec@v1.0  📌 pinned to v1.0
```

**Rate limits:** uses conditional requests (`If-None-Match` / ETag) to minimize
API calls. Checking 10 skills = 10 lightweight HEAD requests.

### 3.6 `upskill update` — update installed skills (v0.2)

```
upskill update [name...] [options]
```

| Flag        | Description                              |
| ----------- | ---------------------------------------- |
| `--global`  | Update global skills only.               |
| `--yes`     | Skip confirmation.                       |
| `--dry-run` | Show what would change without applying. |

**Behavior:**

1. Run `check` internally to identify outdated skills.
2. If no `name` arguments: update all outdated skills. If names given: update
   only those.
3. For each skill to update: a. Re-fetch the source tarball (same mechanism as
   `add`). b. Extract the specific skill directory. c. Replace the canonical
   copy in `.agents/skills/<name>/`. d. Existing symlinks remain intact — they
   point to the canonical dir, which was updated in-place. e. Update lockfile
   with the new commit SHA and timestamp.
4. Print summary:

```
$ upskill update
  ⬆ azure-cosmos-db  abc123 → def456
  ✓ 1 skill updated.
```

**Local modification detection:** if the user has edited an installed skill
(detected by comparing file content hashes against the lockfile), warn and
prompt:

```
⚠ markspec has local modifications. Overwrite? [y/N]
```

With `--yes`: overwrite without prompting. No backup — skills should be in
version control.

**Pinned skills are never auto-updated.** A skill installed with
`owner/repo@v1.0` stays at v1.0. To move to a new version, the user must
explicitly re-add:

```bash
upskill add owner/repo@v2.0 --skill markspec
```

**`--dry-run` output:**

```
$ upskill update --dry-run
  Would update azure-cosmos-db: abc123 → def456 (microsoft/skills@main)
  Skipping markspec: pinned to v1.0
  0 skills would be updated.
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

**Mechanism:** archive tarball via the GitHub archive endpoint. No `git` binary
required. Single HTTP request, streamed through gzip decompression and tar
extraction into a temporary directory.

**URL construction:**

```
https://github.com/{owner}/{repo}/archive/refs/heads/{branch}.tar.gz
```

For tag/commit refs (v0.2):

```
https://github.com/{owner}/{repo}/archive/refs/tags/{tag}.tar.gz
https://github.com/{owner}/{repo}/archive/{commit}.tar.gz
```

**Branch resolution (v0.1):**

1. Try `main`.
2. Fall back to `master`.
3. Fail with error:
   `"Could not fetch {owner}/{repo}: neither 'main' nor 'master' branch found."`

**Ref resolution (v0.2):** when `@ref` is specified, use it directly. No
fallback. Accepts branch names, tag names, and full commit SHAs.

**Subfolder filtering:** when `:path` is specified, after extraction, only the
subtree at `path` is scanned for skills. The full tarball is still downloaded
(GitHub does not support partial archive downloads), but only the relevant
subtree is searched.

**Authentication (resolution order):**

| Priority | Method                 | Mechanism                                                 |
| -------- | ---------------------- | --------------------------------------------------------- |
| 1        | `GITHUB_TOKEN` env var | `Authorization: Bearer $GITHUB_TOKEN` header.             |
| 2        | `GH_TOKEN` env var     | Same as above (matches `gh` CLI convention).              |
| 3        | `gh auth token` output | Shell out to `gh auth token`, use result as bearer token. |
| 4        | Unauthenticated        | Public repos only. Private repos fail with guidance.      |

The `gh` CLI fallback means: if the user already ran `gh auth login`, `upskill`
reuses that session. No extra configuration needed. If `gh` is not on PATH, this
step is silently skipped.

**Rate limits:** GitHub tarball downloads for public repos are not subject to
the REST API rate limit (60/hr unauthenticated). Authenticated requests get
higher limits.

**Error handling:**

| HTTP status | Behavior                                                   |
| ----------- | ---------------------------------------------------------- |
| 200         | Stream, decompress, extract.                               |
| 404         | Branch not found → try fallback → fail with clear error.   |
| 401 / 403   | Auth failure → suggest setting `GITHUB_TOKEN`.             |
| 429         | Rate limited → print retry-after and exit.                 |
| Network err | Connection refused / timeout → print error, suggest proxy. |

### 5.2.1 Git clone fallback

If the tarball download fails (404, non-standard host, custom auth middleware),
`upskill` falls back to `git clone`:

```
git clone --depth 1 --filter=blob:none <repo-url> <tmpdir>
```

**Fallback chain:**

```
1. Tarball endpoint          ← fast, no git binary needed
   ↓ (on failure)
2. git clone --depth 1       ← works with any host, any auth, any git config
   ↓ (git not found)
3. Error with guidance       ← "Install git or set GITHUB_TOKEN for tarball auth"
```

**When git fallback is used:**

- Self-hosted GitLab/GitHub Enterprise with non-standard archive URLs.
- Hosts behind corporate proxies that block direct downloads but allow git
  protocol.
- Any source URL that doesn't match known tarball URL patterns.

**Git inherits the user's auth configuration:** SSH keys, credential helpers,
`.netrc`, `~/.gitconfig` — all respected transparently. This means private repos
that work with `git clone` also work with `upskill add`.

**Output:**

```
$ upskill add gitlab.internal.com/team/skills --skill k8s-deploy
  ⚠ Tarball not available, falling back to git clone
  ✓ Fetched gitlab.internal.com/team/skills
  ✓ k8s-deploy → .agents/skills/k8s-deploy
```

The fallback is silent except for the warning line. No user action required.

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

| Variable       | Purpose                                           |
| -------------- | ------------------------------------------------- |
| `NO_COLOR`     | Disable colored output.                           |
| `GITHUB_TOKEN` | Authenticate GitHub requests (private repos).     |
| `GH_TOKEN`     | Fallback for `GITHUB_TOKEN` (matches `gh` CLI).   |
| `GITLAB_TOKEN` | Authenticate GitLab requests (private repos).     |
| `GL_TOKEN`     | Fallback for `GITLAB_TOKEN` (matches `glab` CLI). |
| `HTTPS_PROXY`  | HTTP proxy for network requests.                  |

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

### 7.3 Lockfile (v0.2)

`.upskill-lock.json` in project root:

```json
{
  "version": 1,
  "skills": [
    {
      "name": "markspec",
      "source": "owner/markspec",
      "subpath": "skills/markspec",
      "branch": "main",
      "commit": "abc123def456...",
      "installed_at": "2026-03-30T12:00:00Z"
    }
  ]
}
```

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
