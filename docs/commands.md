# Commands

`upskill` ships ten commands. They split cleanly into **consumer**
(run inside a project that consumes AI-assistance content) and
**author** (run inside a source-registry repo).

| Command      | Role     | Purpose                                                     |
| ------------ | -------- | ----------------------------------------------------------- |
| `add`        | Consumer | Install content from any source.                            |
| `remove`     | Consumer | Remove installed content.                                   |
| `remove-mcp` | Consumer | Unregister an MCP server configured by a bundle.            |
| `update`     | Consumer | Pull latest, regenerate changed items.                      |
| `list`       | Consumer | Show installed content from the lock file.                  |
| `doctor`     | Consumer | Verify installation consistency.                            |
| `search`     | Consumer | Look up skills via the public registry.                     |
| `index`      | Consumer | Build or manage the local registry index cache.             |
| `new`        | Author   | Scaffold a new rule, skill, or agent.                       |
| `lint`       | Author   | Validate SSOT files against the format spec.                |
| `fmt`        | Author   | Canonicalise YAML frontmatter and format the markdown body. |

## Global flags

These work on every subcommand:

| Flag              | Effect                                                                                 |
| ----------------- | -------------------------------------------------------------------------------------- |
| `--no-color`      | Disable colored output. Honored alongside `NO_COLOR`, `UPSKILL_NO_COLOR`, `TERM=dumb`. |
| `-q`, `--quiet`   | Suppress informational stdout. Errors and exit codes are unchanged.                    |
| `-h`, `--help`    | Show help.                                                                             |
| `-V`, `--version` | Show version.                                                                          |

## Consumer commands

### `upskill add <source> [items...]`

Install content from any source.

```bash
upskill add owner/repo                              # GitHub shorthand
upskill add owner/repo:path/to/items                # subfolder
upskill add owner/repo@v1.2                         # pin to tag
upskill add owner/repo@main                         # pin to branch
upskill add owner/repo@abc123                       # pin to commit SHA
upskill add https://gitlab.com/owner/repo           # GitLab.com
upskill add https://git.example.com/owner/repo      # any https git host (self-hosted, Gitea, …)
upskill add https://gitlab.com/group/subgroup/repo  # nested groups (any depth)
upskill add ./path/to/local                         # local directory
upskill add owner/repo:platform.bundle.yaml         # bundle file (explicit path)
upskill add owner/repo platform-baseline            # bundle by name (resolves .bundle.yaml)
```

`upskill add <source>` installs **everything** the source contains.
Append item names to filter:

```bash
upskill add driftsys/skills code-review secret-scanner
```

Default scope is `--project` (writes into `.agents/...` of the current
repo), falling back to `--global` (`$HOME/.agents/...`) if you're not
inside a git repo. Pass either flag explicitly to override.

**Client selection.** By default `add` emits output for **every** client.
Restrict it to the clients you actually use with one or more of `--claude`,
`--copilot`, `--vscode`, `--opencode` (an allowlist — naming any restricts to
exactly those):

```bash
upskill add owner/repo --claude               # only .claude/**
upskill add owner/repo --copilot --vscode     # only .github/** (+ VS Code MCP)
```

`--vscode` writes the shared `.github/**` rules/skills/agents (VS Code reads
Copilot's tree) plus VS Code's own MCP config; `--copilot --vscode` writes
`.github/**` once and forks only the MCP config. To persist a selection across
runs, set `clients:` in config (`.upskill/config.yaml` for the project,
`~/.config/upskill/config.yaml` globally):

```yaml
clients: [claude, opencode]
```

Precedence is **flag > project config > global config > all clients**. A
per-invocation flag scopes that run only and never rewrites config. An item
whose author `audience:` excludes every selected client is warn-skipped.
See [ADR-0012](./adr/0012-consumer-client-filtering.md).

Supporting files in an item directory — anything besides the entrypoint and
per-client override files, e.g. `scripts/`, `references/`, `assets/` — are
copied into each client's output alongside the rendered item. For rules and
agents that render to a flat file (Claude Code, GitHub Copilot), resources go
into a sibling `<name>/` directory and the body's relative links are rewritten
to match. See [format-spec §2.4](./format-spec.md).

**Bundle plugins.** When a bundle declares a `plugins:` map
([format-spec §3.7](./format-spec.md#37-bundle-schema)), `add` installs
those client-native plugins by shelling out to each client's own CLI
(`claude plugin install`, `copilot plugin install`, `code
--install-extension`, `opencode plugin`) — they are **not** rendered
through the generation pipeline. If a client's CLI is not on `PATH`, the
plugin is skipped (not an error) and recorded as `skipped` in the
lockfile; `upskill doctor` reports skipped and missing plugins. Removing
the bundle cleans up its plugins.

**Bundle MCP servers.** When a bundle declares an `mcps:` map
([format-spec §3.7](./format-spec.md#37-bundle-schema)), `add` configures
those Model Context Protocol servers into each targeted client's MCP
configuration — CLI-first (`claude mcp add ...`), falling back to writing
the client config file (e.g. `.mcp.json`) when the CLI is absent. MCP
configuration is idempotent, and a single MCP failure or skip never fails
the rest of the bundle install. Configured and warn-skipped servers are
recorded in the lockfile; remove one with
[`upskill remove-mcp`](#upskill-remove-mcp-name). If an MCP server needs
an environment variable that is unset, `add` warns so you can export it.

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

`update` honours the same [client-selection](#upskill-add-source-items)
flags and `clients:` config as `add` (`--claude`, `--copilot`, `--vscode`,
`--opencode`). With no flag and no config, `update` **preserves each source's
recorded selection** — a bare `update` of a `--claude`-only install stays
Claude-only rather than re-expanding to all clients. Pass a flag (or set
`clients:`) to change the selection on update.

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

`--source` triggers a y/N confirmation prompt on a TTY (it removes
every item from that label at once). Pass `-y` / `--yes` to skip.
Non-interactive contexts (CI, pipes) skip the prompt automatically.

### `upskill remove-mcp <name>`

Unregister an MCP server that a bundle install configured (see the
`mcps:` note under [`upskill add`](#upskill-add-source-items)).

```bash
upskill remove-mcp drawio            # remove from the current project
upskill remove-mcp drawio --global   # remove from the $HOME scope
```

`<name>` is the upskill-level MCP name recorded in the lockfile.
`remove-mcp` unregisters the server from the client (`claude mcp remove
<name>`, falling back to deleting the entry from the client config file
when the CLI is absent) and drops the `LockedMcp` entry from the
lockfile. Scope follows the same `--project` / `--global` rules as
`add`, auto-detecting global when `cwd` is not inside a git repo.

### `upskill list`

Show installed content from `.upskill-lock.json`, grouped by kind.
Bundles, when present, are surfaced as a separate section.

The `--available` view (items discoverable from configured sources) is
deferred for a future release; v0.2.0 ships the installed-state view
only.

Pass `--json` for a stable machine-readable document:

```json
{
  "rules":   [{ "kind": "rule",  "name": "...", "source": "...", "git_ref": null }],
  "skills":  [{ "kind": "skill", "name": "...", "source": "...", "git_ref": null }],
  "agents":  [{ "kind": "agent", "name": "...", "source": "...", "git_ref": null }],
  "bundles": [{ "name": "...", "source": "...", "git_ref": null, "items": [] }]
}
```

`kind` is one of `"rule"`, `"skill"`, `"agent"`. `git_ref` is the pinned
ref/tag/branch when the source is a git URL, otherwise `null`. `source`
matches the lockfile label (`local:/path` or `github:owner/repo` etc.).

### `upskill doctor`

Verify on-disk state matches `.upskill-lock.json`. Reports drift in
six independent buckets:

- **Missing per-client output files** — reinstall fixes (`upskill add
  <source>`).
- **SSOT hash drift on `local:` sources** — `upskill update` fixes.
- **Lockfile entries with no recoverable source** (the local path went
  away, or the named item was removed in the source) — `upskill remove`
  to clear.
- **Missing plugins** — recorded as `installed` in the lockfile but no
  longer found in the client. Likely uninstalled out-of-band; `upskill
  update` reinstalls them. Causes exit 1.
- **Skipped plugins** (informational) — recorded as `skipped` because
  the client CLI was not on PATH at install time. Install the CLI then
  run `upskill update` to install them. Does **not** cause exit 1.
- **MCP servers** — each Claude-client MCP entry in the lockfile is
  checked against `claude mcp list`. A server in the lockfile but **not
  registered** in the client causes exit 1 (`upskill update`
  reconfigures it). A missing `claude` CLI or a failed query is advisory
  and does **not** cause exit 1.

Exits 0 when clean, 1 when any drift bucket is non-empty (missing
outputs, stale hashes, orphan entries, missing plugins, or unregistered
MCP servers). Skipped plugins and unverifiable MCP entries are
informational warnings and do not affect the exit code. `doctor` never
fetches; remote-source drift detection is `update --dry-run`.

Pass `--json` for a stable machine-readable document. Exit code is
unchanged.

```json
{
  "missing_outputs": [
    { "kind": "skill", "name": "...", "missing_files": ["..."] }
  ],
  "stale_hashes": [
    { "kind": "rule",  "name": "...", "source": "local:...",
      "stored_hash": "...", "current_hash": "..." }
  ],
  "orphan_entries": [
    { "kind": "agent", "name": "...", "source": "local:...",
      "reason": "local-path-gone" }
  ],
  "missing_plugins": [
    { "name": "superpowers", "client": "vscode",
      "identifier": "anthropic.superpowers", "bundle": "baseline" }
  ],
  "skipped_plugins": [
    { "name": "superpowers", "client": "claude",
      "identifier": "superpowers@anthropics/claude-plugins",
      "bundle": "baseline" }
  ],
  "mcp_entries": [
    { "name": "drawio", "client": "claude", "bundle": "baseline",
      "status": "not-registered" }
  ]
}
```

`reason` is `"local-path-gone"` or `"item-missing-in-source"`. Hashes
may be `null` when the SSOT can't be hashed (e.g. unreadable). MCP
`status` is one of `"ok"`, `"not-registered"`, `"cli-not-found"`, or
`{ "query-failed": { "stderr": "..." } }`. The plugin and item arrays
are always present (possibly empty); `mcp_entries` is omitted when no
MCP servers are recorded.

### `upskill search <query>`

Search the public skills registry and any configured registries.

```bash
upskill search code-review
upskill search code-review --limit 20
upskill search auth --registry corp
upskill search auth --kind rule
```

| Flag                | Description                                             |
| ------------------- | ------------------------------------------------------- |
| `--registry <name>` | Search only a specific configured registry.             |
| `--kind <kind>`     | Filter results by item kind (`skill`, `rule`, `agent`). |
| `--limit <n>`       | Maximum number of results (default 10).                 |

### `upskill index`

Build or manage the local registry index cache.

```text
upskill index [--registry <name>] [--clear]
```

| Flag                | Description                       |
| ------------------- | --------------------------------- |
| `--registry <name>` | Rebuild only a specific registry. |
| `--clear`           | Remove all cached indexes.        |

Without flags, rebuilds the index for all configured registries.

## Author commands

Run inside a **source-registry** working tree. Each refuses to run
inside a consumer project (detected by `.upskill-lock.json` at the
path's root) so you don't accidentally lint generated outputs or
scaffold into the wrong tree.

### `upskill new <kind> <name>`

Scaffold a new SSOT item directory. `<kind>` is one of `rule`, `skill`,
or `agent`; `<name>` is both the item name and the directory name
(lowercase letters, digits, hyphens; max 64 chars).

```bash
upskill new rule  no-direct-database-access
upskill new skill code-review
upskill new agent security-reviewer
```

Creates `<name>/<KIND>.md` **relative to the current directory** with the
minimum frontmatter the format spec requires — there is no `<kind>`
parent folder and no separate "destination" argument. The folder is the
`<name>` you pass, so to place an item under `skills/` you run the
command from inside `skills/` (see
[Conventions](./conventions.md#scaffolding-under-the-convention)). Agents
get `mode: subagent` and `model: sonnet` so the file is generation-ready
out of the box. The scaffold round-trips through `upskill fmt` as a no-op
and passes `upskill lint --strict`.

**Co-location.** Run `new` again with a different kind and the same
`<name>` to add that kind as a sibling entrypoint inside the existing
item directory ([format-spec §2.1](./format-spec.md#21-item-directory-structure)):

```bash
upskill new skill api-handler   # → api-handler/SKILL.md
upskill new rule  api-handler   # → api-handler/RULE.md (added alongside)
```

This expresses one capability across multiple kinds (for example a skill
paired with its enforcing rule). Co-location is refused only when an
existing **skill** entrypoint's `name:` diverges from `<name>`; rule and
agent siblings MAY carry a divergent name.

`upskill new` is an author command — it refuses to run inside a consumer
project (one with a `.upskill-lock.json`).

### `upskill lint [paths...]`

Validate SSOT files against the [format spec](./format-spec.md). Five
rules ship out of the box:

| Rule ID            | Severity | Source           |
| ------------------ | -------- | ---------------- |
| `frontmatter`      | error    | format-spec §3   |
| `name-matches-dir` | error    | format-spec §2.1 |
| `body-h1`          | warning  | format-spec §5.1 |
| `fence-lang`       | warning  | format-spec §5.2 |
| `body-format`      | warning  | format-spec §3.8 |
| `directive`        | error    | format-spec §6.3 |

```bash
upskill lint                # lint everything in the working tree
upskill lint my-skill/      # lint a single item directory
upskill lint --strict       # CI mode: warnings become errors
```

### `upskill fmt [paths...]`

Canonicalise YAML frontmatter (key order, indentation, alphabetised
unknown keys) and format the markdown body via dprint (the same formatter
the generation pipeline uses). The frontmatter↔body seam is normalised to
a single blank line; YAML comments and prose wrapping are preserved.

```bash
upskill fmt                  # format everything in the working tree
upskill fmt my-skill/        # format a single item directory
```

Files whose frontmatter and body are already canonical are left untouched
(no `mtime` thrash).

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
