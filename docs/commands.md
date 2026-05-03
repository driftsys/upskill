# Commands

`upskill` ships nine commands. They split cleanly into **consumer**
(run inside a project that consumes AI-assistance content) and
**author** (run inside a source-registry repo).

| Command  | Role     | Purpose                                             |
| -------- | -------- | --------------------------------------------------- |
| `add`    | Consumer | Install content from any source.                    |
| `remove` | Consumer | Remove installed content.                           |
| `update` | Consumer | Pull latest, regenerate changed items.              |
| `list`   | Consumer | Show installed content from the lock file.          |
| `doctor` | Consumer | Verify installation consistency.                    |
| `search` | Consumer | Look up skills via the public registry.             |
| `new`    | Author   | Scaffold a new rule, skill, or agent.               |
| `lint`   | Author   | Validate SSOT files against the format spec.        |
| `fmt`    | Author   | Canonicalise YAML frontmatter (key order, quoting). |

## Consumer commands

### `upskill add <source> [items...]`

Install content from any source.

```bash
upskill add owner/repo                              # GitHub shorthand
upskill add owner/repo:path/to/items                # subfolder
upskill add owner/repo@v1.2                         # pin to tag
upskill add owner/repo@main                         # pin to branch
upskill add owner/repo@abc123                       # pin to commit SHA
upskill add gitlab:owner/repo                       # GitLab.com
upskill add https://gitlab.example.com/owner/repo   # self-hosted GitLab
upskill add ./path/to/local                         # local directory
upskill add owner/repo:platform.bundle.md           # bundle file
```

`upskill add <source>` installs **everything** the source contains.
Append item names to filter:

```bash
upskill add driftsys/skills code-review secret-scanner
```

Default scope is `--project` (writes into `.agents/...` of the current
repo), falling back to `--global` (`$HOME/.agents/...`) if you're not
inside a git repo. Pass either flag explicitly to override.

### `upskill update [name...]`

Pull latest sources and regenerate changed items.

```bash
upskill update                       # update everything
upskill update code-review           # update one item
upskill update --dry-run             # preview changes without applying
```

`update` always fetches before regenerating; there is no separate
`sync`. It also re-runs the generation pipeline against the current
upskill version, so client-format updates land without a separate
command.

### `upskill remove [name...] [--source <label>]`

Remove installed items.

```bash
upskill remove code-review                         # remove one item
upskill remove code-review secret-scanner          # remove several
upskill remove --source github:driftsys/skills     # remove every item from a source
upskill remove --global code-review                # global scope
```

Bare `upskill remove` is rejected — be explicit. Ancillary files
(`CLAUDE.md`, `opencode.json`, `.vscode/settings.json`) are not
touched.

### `upskill list`

Show installed content from `.upskill-lock.json`, grouped by kind.
Bundles, when present, are surfaced as a separate section.

The `--available` view (items discoverable from configured sources) is
deferred for a future release; v0.2.0 ships the installed-state view
only.

### `upskill doctor`

Verify on-disk state matches `.upskill-lock.json`. Reports drift in
three independent buckets:

- **Missing per-client output files** — reinstall fixes (`upskill add
  <source>`).
- **SSOT hash drift on `local:` sources** — `upskill update` fixes.
- **Lockfile entries with no recoverable source** (the local path went
  away, or the named item was removed in the source) — `upskill remove`
  to clear.

Exits 0 when clean, 1 when any bucket is non-empty. `doctor` never
fetches; remote-source drift detection is `update --dry-run`.

### `upskill search <query>`

Search the public skills registry.

```bash
upskill search code-review
upskill search code-review --limit 20
```

## Author commands

Run inside a **source-registry** working tree. Each refuses to run
inside a consumer project (detected by `.upskill-lock.json` at the
path's root) so you don't accidentally lint generated outputs or
scaffold into the wrong tree.

### `upskill new <kind> <name>`

Scaffold a new SSOT item directory.

```bash
upskill new rule  no-direct-database-access
upskill new skill code-review
upskill new agent security-reviewer
```

Creates `<kind>s/<name>/<KIND>.md` with the minimum frontmatter the
format spec requires. Agents get `mode: subagent` and `model: sonnet`
so the file is generation-ready out of the box. The scaffold round-trips
through `upskill fmt` as a no-op and passes `upskill lint --strict`.

### `upskill lint [paths...]`

Validate SSOT files against the [format spec](./format-spec.md). Five
rules ship out of the box:

| Rule ID            | Severity | Source           |
| ------------------ | -------- | ---------------- |
| `frontmatter`      | error    | format-spec §3   |
| `name-matches-dir` | error    | format-spec §2.1 |
| `body-h1`          | warning  | format-spec §5.1 |
| `fence-lang`       | warning  | format-spec §5.2 |
| `directive`        | error    | format-spec §6.3 |

```bash
upskill lint                # lint everything in the working tree
upskill lint rules/         # lint a subtree
upskill lint --strict       # CI mode: warnings become errors
```

### `upskill fmt [paths...]`

Canonicalise YAML frontmatter (key order, indentation, alphabetised
unknown keys). Markdown body formatting is left to dprint — the two
tools don't overlap.

```bash
upskill fmt                  # format everything in the working tree
upskill fmt rules/           # format a subtree
```

Files whose frontmatter is already canonical are left untouched (no
`mtime` thrash).

## State files

`.upskill-lock.json` lives in one of two places depending on scope:

| Scope   | Location                   | Committed? | Purpose                                    |
| ------- | -------------------------- | ---------- | ------------------------------------------ |
| Project | `<cwd>/.upskill-lock.json` | Yes        | Deterministic regeneration in CI.          |
| Global  | `$HOME/.upskill-lock.json` | No         | Cross-repo continuity for global installs. |

The lockfile carries a top-level `schema: 1` field for forward
compatibility.

## Per-client output paths

| Item kind | Claude Code                | GitHub Copilot                                | opencode                       |
| --------- | -------------------------- | --------------------------------------------- | ------------------------------ |
| Rule      | `.claude/rules/<name>.md`  | `.github/instructions/<name>.instructions.md` | `.agents/rules/<name>/RULE.md` |
| Skill     | `.claude/skills/<name>/`   | `.github/skills/<name>/`                      | `.agents/skills/<name>/`       |
| Agent     | `.claude/agents/<name>.md` | `.github/agents/<name>.agent.md`              | `.opencode/agents/<name>.md`   |

All output is **copy** (not symlink) — Windows portability without
Developer Mode.

## Exit codes

| Code | Meaning                |
| ---- | ---------------------- |
| 0    | Success                |
| 1    | General error          |
| 2    | Usage error (bad args) |
| 130  | Interrupted (Ctrl+C)   |
