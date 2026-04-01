# upskill

`upskill` is a package manager for [Agent Skills][agent-skills] — the shared
skill format supported by Claude Code, GitHub Copilot, Codex, Cursor, Kiro,
Windsurf, and OpenCode.

It installs skills from any GitHub or GitLab repository into your project in
one command. Skills land in `.agents/skills/` and are automatically linked into
whichever agent config directories already exist in your project. No Node.js.
No npm. A single static binary.

```bash
upskill search code-review          # find skills
upskill add owner/repo              # install from a GitHub repo
upskill list                        # see what's installed
upskill update                      # re-fetch from source
```

## Installation

```bash
cargo install upskill
```

Or download a pre-built binary from the [releases page][releases].

## Quick start

```bash
# Search the public registry
upskill search python

# Install a skill (auto-detects your agents)
upskill add anthropics/skills --skill python-best-practices

# See what's installed and which agents are linked
upskill list
```

Skills are installed to `.agents/skills/` in the current directory. Agent
symlinks (`.claude/skills`, `.github/skills`, etc.) are created automatically
when the corresponding agent config directory is detected.

---

## Commands

### `upskill add <source>`

Install skills from a source into the current project (or globally with `-g`).

```bash
upskill add owner/repo                   # GitHub shorthand
upskill add owner/repo:path/to/skills    # Subfolder only
upskill add owner/repo@v1.2              # Pin to tag
upskill add owner/repo@main              # Pin to branch
upskill add owner/repo@abc123            # Pin to commit SHA
upskill add gitlab:owner/repo            # GitLab.com
upskill add https://gitlab.example.com/owner/repo  # Self-hosted GitLab
upskill add ./path/to/local              # Local directory
```

**Skill selection:**

```bash
upskill add owner/repo --skill my-skill         # Install one skill
upskill add owner/repo -s foo -s bar            # Install multiple skills
# (no --skill): interactive prompt in a TTY, default name in CI
```

**Agent flags:**

```bash
upskill add owner/repo --claude     # Symlink to .claude/skills
upskill add owner/repo --copilot    # Symlink to .github/skills
upskill add owner/repo --all        # Symlink to all 7 supported agents
upskill add owner/repo --copy       # Copy instead of symlinking
# (no flag): auto-detect from existing agent config directories
```

**Other flags:**

```bash
upskill add owner/repo -g           # Global install (~/.agents/skills)
```

### `upskill list`

List installed skills and their agent symlinks.

```bash
upskill list         # Project skills
upskill list -g      # Global skills
```

Output format:

```
my-skill    source=github:owner/repo    symlinks=claude,copilot
other-skill source=local:/path/to/src  symlinks=none
```

### `upskill search <query>`

Search the public skills registry (skills.sh).

```bash
upskill search rust
upskill search code-review
upskill search --limit 20 python
```

Output:

```
rust-mcp-server-generator    7608 installs    upskill add awesome-copilot --skill rust-mcp-server-generator
rust-analyzer                3200 installs    upskill add anthropics/skills --skill rust-analyzer
```

Each result includes the install command to use directly.

**Flags:**

| Flag      | Default | Description                |
| --------- | ------- | -------------------------- |
| `--limit` | `10`    | Maximum number of results. |

### `upskill remove <skill>`

Remove an installed skill and clean up agent symlinks.

```bash
upskill remove my-skill         # Prompts for confirmation in a TTY
upskill remove my-skill --yes   # Skip confirmation
upskill remove my-skill -g      # Remove from global install
```

### `upskill check`

Show installed skills and their pinned refs.

```bash
upskill check     # Project lockfile
upskill check -g  # Global lockfile
```

Output:

```
my-skill    github:owner/repo    pinned: latest
other-skill github:owner/repo    pinned: v1.2
```

### `upskill update`

Re-install skills from their recorded sources.

```bash
upskill update                      # Update all skills
upskill update my-skill             # Update one skill
upskill update --dry-run            # Preview without applying
upskill update --force              # Overwrite locally modified skills
upskill update -g                   # Update global skills
```

Local modifications are detected via a SHA-256 content hash stored in the
lockfile. Modified skills are skipped with a warning unless `--force` is used.

---

## Recipes

### CI usage

```bash
# Install without prompts (auto-detects NO_COLOR, non-TTY)
upskill add owner/repo

# Explicit non-interactive
GITHUB_TOKEN=${{ secrets.GITHUB_TOKEN }} upskill add owner/repo
```

In a non-TTY environment (CI, pipes), `upskill` automatically:

- Skips interactive prompts and uses defaults.
- Disables colored output when `NO_COLOR` is set.

### Override the search registry URL

For testing or private deployments:

```bash
UPSKILL_REGISTRY_URL=https://my-registry.example.com upskill search query
```

Defaults to `https://skills.sh`.

### Private repositories

```bash
# GitHub — set one of:
export GITHUB_TOKEN=ghp_...
export GH_TOKEN=ghp_...
# or rely on `gh auth token` if the GitHub CLI is authenticated

# GitLab — set one of:
export GITLAB_TOKEN=glpat_...
export GL_TOKEN=glpat_...
# or rely on `glab auth token` if the GitLab CLI is authenticated
```

### Pin a skill to a specific version

```bash
upskill add owner/repo@v1.2 --skill my-skill
```

The pinned ref is recorded in `.upskill-lock.json`. `upskill update` will
re-fetch from the same ref.

### Install to all agents

```bash
upskill add owner/repo --all
```

Creates symlinks in all 7 supported agent directories:
`.claude/skills`, `.github/skills`, `.codex/skills`, `.cursor/skills`,
`.kiro/skills`, `.windsurf/skills`, `.opencode/skills`.

### Copy instead of symlinking

For environments where symlinks are not supported (e.g. some Windows
setups, Docker mounts):

```bash
upskill add owner/repo --copy
```

Copies skill files directly into each agent directory. Copied skills are
independent of the source after installation.

### Global install

```bash
upskill add owner/repo -g      # Install to ~/.agents/skills
upskill list -g                # List global skills
upskill remove my-skill -g     # Remove from global
```

Global skills are not tied to a project directory.

---

## Lockfile

Every `add` operation writes `.upskill-lock.json` in the project root (or
`~/.upskill-lock.json` for global installs). Commit this file to track exact
skill versions.

Example:

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

Fields:

| Field    | Description                                 |
| -------- | ------------------------------------------- |
| `name`   | Skill directory name                        |
| `source` | Full source label including prefix and ref  |
| `ref`    | Pinned git ref (omitted if tracking latest) |
| `hash`   | SHA-256 of all files in the skill directory |

The `hash` is used by `upskill update` to detect local modifications before
overwriting.

---

## Exit codes

| Code | Meaning                |
| ---- | ---------------------- |
| 0    | Success                |
| 1    | General error          |
| 2    | Usage error (bad args) |
| 130  | Interrupted (Ctrl+C)   |

---

## Supported agents

| Agent    | Skills directory   |
| -------- | ------------------ |
| Claude   | `.claude/skills`   |
| Copilot  | `.github/skills`   |
| Codex    | `.codex/skills`    |
| Cursor   | `.cursor/skills`   |
| Kiro     | `.kiro/skills`     |
| Windsurf | `.windsurf/skills` |
| OpenCode | `.opencode/skills` |

[releases]: https://github.com/driftsys/upskill/releases
[agent-skills]: https://agentskills.io/specification
