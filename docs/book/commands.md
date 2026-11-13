# Commands

`upskill` ships eleven commands. They split cleanly into **consumer** (run
inside a project that consumes AI-assistance content) and **author** (run inside
a source-registry repo).

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

`upskill add <source>` installs **everything** the source contains. Append item
names to filter:

```bash
upskill add driftsys/skills code-review secret-scanner
```

`--project` writes into the current directory; `--global` writes under `$HOME`.
Inside a git repository, `add` without a flag uses the project scope. Outside a
git repository, `add` without a flag installs nothing and stops with exit code 2
and a message that asks for `--project` or `--global`. When the current
directory is the home directory, which is the root of the global scope, the
project scope is refused: `add` installs nothing and stops with exit code 2 and
a message that asks for `--global`, with `--project` and with no flag, in or
outside a git repository. `update`, `remove` and `remove-mcp` do the same with
`--project`, and with no flag in a home directory that is a git repository; with
no flag in a home directory that is not a git repository, they use the global
scope.

`add`, `update`, `remove` and `remove-mcp` print one line to stderr that names
the scope and its root, for example `scope: global (/home/alice)`. `--quiet`
hides it.

**Client selection.** By default `add` emits output for **every** client.
Restrict it to the clients you actually use with one or more of `--claude`,
`--copilot`, `--vscode`, `--opencode` (an allowlist — naming any restricts to
exactly those):

```bash
upskill add owner/repo --claude               # only .claude/**
upskill add owner/repo --copilot --vscode     # only .github/** (+ VS Code MCP)
```

`--vscode` writes the same Copilot rules/skills/agents as `--copilot` (VS Code
reads Copilot's tree: `.github/**` in the project scope, `.copilot/**` in the
global scope) plus VS Code's own MCP config; `--copilot --vscode` writes the
Copilot tree once and forks only the MCP config. To persist a selection across
runs, set `clients:` in config (`.upskill/config.yaml` for the project,
`~/.config/upskill/config.yaml` globally):

```yaml
clients: [claude, opencode]
```

Precedence is **flag > project config > global config > all clients**. A
per-invocation flag scopes that run only and never rewrites config. An item
whose author `audience:` excludes every selected client is warn-skipped. See
[ADR-0012](https://github.com/driftsys/upskill/blob/main/docs/adr/0012-consumer-client-filtering.md).

Supporting files in an item directory — anything besides the entrypoint and
per-client override files, e.g. `scripts/`, `references/`, `assets/` — are
copied into each client's output alongside the rendered item. For rules and
agents that render to a flat file (Claude Code, GitHub Copilot), resources go
into a sibling `<name>/` directory and the body's relative links are rewritten
to match. See [format-spec §2.4](format-spec.md).

`add` refuses a source item that it installs when the item's directory is the
same as, is inside, or contains one of that item's output paths for any client
in the target scope (for example `upskill add ./.claude/skills`). With `--as`,
the output paths of the installed (alias) name are checked as well. The check
covers the items that the run selects from the source, the positional names, the
bundles and their `requires` dependencies; other items of the source are not
checked. The `audience` field is not applied to the check: an item whose
`audience` shares no client with the selected clients is still checked and can
cause a refusal, although nothing would be installed for the item. The refusal
names the item, the source directory and the output path. `add` fails before any
file is changed. `upskill update` is refused in the same way, including for
recorded aliases, but it checks each source just before it updates that source:
the refusal comes before any change for that source, and an `update` over
several sources may have updated earlier sources before it stops.

**Modified and unmanaged files.** Before `add` writes anything, it checks every
destination of every item against the item's `files` and `pending` records (see
[State files](#state-files)). A destination is a conflict when it is a file that
upskill wrote and that was changed afterwards (modified), or a file that upskill
has no record of (not managed by upskill, for example a file you wrote by hand).
A file is compared with its record as it is and again with every CRLF (`\r\n`)
replaced by LF (`\n`); it is unchanged when one of the two comparisons matches.
So a git checkout with `core.autocrlf=true`, which writes LF files with CRLF,
does not make a file modified. The limit of this rule: where the recorded
content has a LF, a carriage return that you add before it is read as unchanged.
An unchanged file with CRLF line endings is left as it is until its generated
content changes; a change of the line endings of the generated content is such a
change, and the file is then written. A destination whose content already equals
the generated bytes is never a conflict; for a recorded file, the generated
bytes are compared in the same two ways. An item with at least one conflict is
skipped as a whole: none of its files is written and its lockfile entry is not
changed. The other items are written. For each conflict, `add` prints one line
to stderr, then a hint:

```text
skipped rule api-conventions: .claude/rules/api-conventions.md was modified
skipped skill review: .claude/skills/review/SKILL.md exists and is not managed by upskill
Use --force to replace.
```

and exits with code 1. A printed path is relative to the install root and uses
`/` as separator. On Windows, the path of a resource file can also contain `\`
after the kind directory, so one printed path can contain both separators.
`add --force` replaces these files, deletes a modified file that the source no
longer contains, and also replaces an item installed from another source. It
then lists the replaced or deleted files after
`Replaced or deleted <n> modified or unmanaged file(s) (--force)`; `-q/--quiet`
hides this list.

An installed name, or an `--as` alias, must not contain a path separator (`/` or
`\`). `add` refuses such a name before it changes any file and names the item in
the error.

A source rooted in a client directory (for example `~/.claude`) or one that
contains a client output directory is not supported. The first `add` succeeds,
but its output is inside the source, so the next `add` and every `update` of
that source are refused with a message that names the source root and the client
directory. Move the source items to a directory outside the client directories
and add them from there.

**Bundle plugins.** When a bundle declares a `plugins:` map
([format-spec §3.7](format-spec.md#37-bundle-schema)), `add` installs those
client-native plugins by shelling out to each client's own CLI
(`claude plugin install`, `copilot plugin install`, `code
--install-extension`,
`opencode plugin`) — they are **not** rendered through the generation pipeline.
If a client's CLI is not on `PATH`, the plugin is skipped (not an error) and
recorded as `skipped` in the lockfile; `upskill doctor` reports skipped and
missing plugins. Removing the bundle cleans up its plugins.

**Bundle MCP servers.** When a bundle declares an `mcps:` map
([format-spec §3.7](format-spec.md#37-bundle-schema)), `add` configures those
Model Context Protocol servers into each targeted client's MCP configuration —
CLI-first (`claude mcp add ...`), falling back to writing the client config file
(e.g. `.mcp.json`) when the CLI is absent. MCP configuration is idempotent, and
a single MCP failure or skip never fails the rest of the bundle install.
Configured and warn-skipped servers are recorded in the lockfile; remove one
with [`upskill remove-mcp`](#upskill-remove-mcp-name). If an MCP server needs an
environment variable that is unset, `add` warns so you can export it.

### `upskill update [name...]`

Pull latest sources and regenerate changed items.

```bash
upskill update                       # update everything
upskill update code-review           # update one item
upskill update --dry-run             # preview changes without applying
```

Without a scope flag, `update` uses the project scope inside a git repository
and the global scope outside one. `--project` uses the current directory and
`--global` uses `$HOME`. When the current directory is the home directory, the
project scope is refused with exit code 2 and a message that asks for
`--global`.

`update` always fetches before regenerating; there is no separate `sync`. It
also re-runs the generation pipeline against the current upskill version, so
client-format updates land without a separate command.

```bash
upskill update --force               # also replace modified and unmanaged files
```

**Earlier global Copilot paths.** A released version wrote global Copilot
content to `.github/skills`, `.github/instructions` and `.github/agents` under
`$HOME`. `upskill update --global` moves such an install to the
[global paths](#per-client-output-paths): it writes the new file and deletes the
file at the earlier path when that file is unchanged. An edited file at the
earlier path is kept and named in the output. The items of such a run are
reported as up to date when their source did not change; the line
`Changed N file(s) written or deleted` counts the moved files. The empty client
root and kind directories that remain, for example `$HOME/.github/skills`, are
kept.

`update` honours the same [client-selection](#upskill-add-source-items) flags
and `clients:` config as `add` (`--claude`, `--copilot`, `--vscode`,
`--opencode`). With no flag and no config, `update` **preserves each source's
recorded selection** — a bare `update` of a `--claude`-only install stays
Claude-only rather than re-expanding to all clients. Pass a flag (or set
`clients:`) to change the selection on update. The files that a narrower
selection no longer writes are deleted when they are unchanged; a modified one
is kept, stays in the record, and is reported as described below (when the item
is skipped, nothing of it changes).

`update` protects files in the same way as `add`. An item with a destination
that was modified, or that upskill does not manage, is not written and keeps its
lockfile entry. When all of these hold, nothing is withheld from the item: a
source hash is recorded for the item and the source hash of the item equals it
(an item with no recorded hash is skipped), the item has no interrupted
operation (no `pending` record), every such destination is a recorded file that
was modified (not a file that upskill does not manage), applying the item would
write or delete no other file, the item has no recorded file that the new
version no longer contains (present or absent), and its lockfile entry would not
change in any field other than its file records and `required_by` (for example
`clients`). `update` then reports `kept modified file: <path>` for each modified
destination: in a terminal and with `--dry-run`, in the block of its source (see
below); with `--yes` or without a terminal, on stderr. This note does not change
the exit code. In every other case (for example the source changed, or the flags
no longer select a client of the item), `update` reports the item as skipped. In
a terminal and with `--dry-run`, the block of the source has a `skip` line for
the item that names each such file. With `--yes` or without a terminal, `update`
prints `skipped <kind> <name>: <path> was modified or is not managed by upskill`
on stderr for each such file. A skipped item of a source that is applied, or of
a source whose preview changes nothing, makes `update` print
`Use --force to replace.` on stderr and exit with code 1 after it has written
the other items. A `skip` line of `--dry-run`, or of a source that the user
declined, does neither. The limit of this rule: when a new version of upskill
renders an unchanged source to different bytes in the edited file only, the new
rendering is not written and only the note is printed, until `update --force`;
`upskill doctor` lists the modified files. A recorded file that the new version
of an item no longer contains is deleted when it is unchanged; a modified one is
kept and stays in the record. When the item has another change, a block names
the file on the item line as `keeps modified <path> (no longer produced)`; when
it has none, the block has a `kept modified file: <path>` note. A run with
`--yes` or without a terminal reports it as `kept modified file: <path>` on
stderr. An item that its source no longer contains is removed with the rules of
[`upskill remove`](#upskill-remove-name---source-label---force): unchanged
recorded files are deleted, modified ones are kept and reported, and the
lockfile entry is dropped. `update --force` replaces modified and unmanaged
files and deletes modified regular files that are no longer produced. A terminal
run names each of these files in the block, as `replaces <path> (--force)` or
`deletes <path> (--force)`. A run with `--yes` or without a terminal lists them
after `Replaced or deleted <n> modified or unmanaged file(s) (--force)`.

`update` handles the sources one at a time, sorted by source label. For each
source it fetches the source once (a source of a cross-source dependency is
fetched again when the source that requires it is reinstalled), checks it, and
computes a preview: what applying this source now does to the files on disk and
to the lockfile. The preview is computed just before the source is applied, so
it includes the effect of the sources applied before it. The modes differ in
what they do with the preview:

| Mode                                | Behaviour                                                                                                                                    |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Terminal, no `--yes`                | Prints one block per source. Asks `Apply? [y/N/q]` for each source whose preview changes something. Applies the source only on `y` or `yes`. |
| `--yes`, or stdin is not a terminal | Prints no block and asks nothing. Applies every source, also a source whose preview changes nothing.                                         |
| `--dry-run`                         | Prints one block per source. Asks nothing, applies nothing, and does not adopt a `schema: 1` lockfile. Exits with code 0 when no step fails. |

A terminal run looks like this:

```text
$ upskill update
github:driftsys/skills
  update   skill tech-diagramming   4 files
  remove   skill old-skill   no longer in source, 1 file
Apply? [y/N/q] y
  1 updated, 1 removed, 5 file(s) changed

local:/home/me/local-rules
  up to date
  kept modified file: .claude/rules/style.md

Updated 2 of 12 item(s) from 1 of 2 source(s).
```

A block starts with the source label. It has one line per item that applying the
source changes or skips, with one of these actions:

| Action     | Meaning                                                                                                                                           |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `update`   | Files of the item are written or deleted, or only its lockfile record changes (the line then ends with `record only`).                            |
| `complete` | The item has an interrupted `add` or `update` (a non-empty `pending` record, see [State files](#state-files)) and its source hash did not change. |
| `remove`   | The source no longer contains the item (`no longer in source`).                                                                                   |
| `skip`     | The item is not written: `modified or not managed by upskill: <path>, ...` names the files that cause the skip.                                   |

An item line counts the files that are written or deleted (`1 file`,
`<n> files`). It names each modified file that is kept (`keeps modified <path>`)
and, with `--force`, each modified or unmanaged file that is replaced or
deleted. A cross-source dependency that the reinstall of the source writes has a
line in the block of that source. After the item lines, the block has one
`write <path>` line for each hand-shake file (`CLAUDE.md`, `opencode.json`,
`.vscode/settings.json`) that applying the source creates or changes. When the
block has no item line and no `write` line, it says `up to date`. The block ends
with one `kept modified file: <path>` line for each note.

A preview changes something when applying the source writes or deletes a file
(also a file with the same bytes whose executable bits differ from the target
executable bits, on Unix: each executable bit of its source resource for which
the file has the matching read bit; a difference in the other permission bits is
not a change), changes the lockfile record of an item, completes an interrupted
item, removes an item, or creates or changes a hand-shake file. A `skip` line
and a `kept modified file` note are not changes. A source whose preview changes
nothing is printed without a question and is not applied.

The answers to `Apply? [y/N/q]`:

| Answer                                   | Effect                                                                                                                                            |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `y` or `yes`, any letter case            | Apply this source, then continue with the next source.                                                                                            |
| `n`, `no`, an empty line, any other text | Do not apply this source. Continue with the next source.                                                                                          |
| `q` or `quit`, any letter case           | Do not apply this source. Stop the run. Earlier sources stay applied.                                                                             |
| End of input, or a read error            | Same as `q`.                                                                                                                                      |
| Ctrl-C                                   | At the question, stop at once with exit code 130. During an apply, stop before the next source, with exit code 130. Earlier sources stay applied. |

The apply checks the files on disk again; it does not rely on the preview. After
an applied source, `update` prints one result line with the non-zero counts of
`<n> updated`, `<n> completed`, `<n> removed`, `<n> skipped`,
`<n> modified file(s) kept` and `<n> file(s) changed`, or `applied` when every
count is 0. After the last source, it prints one closing summary:
`Updated <n> of <m> item(s) from <a> of <s> source(s).`, followed by
`<k> source(s) not applied.` when a source with changes was not applied, and by
`Stopped; <r> source(s) not examined.` (or `Stopped.` when no source remained)
after `q` or end of input. When no item was updated, completed or removed but
files were written or deleted (for example the files of a client that the flags
no longer select), the summary is followed by
`Changed <n> file(s) written or deleted`. The line
`everything is up to date — nothing to do` replaces the summary when no source
was applied, no source with changes was declined, no file was kept with a note,
no item was skipped and no `schema: 1` lockfile was adopted.

A terminal run exits with code 1 when an item is skipped in a source that was
applied or in a source whose preview changes nothing, or when a step fails.
Answers `n` and `q` do not change the exit code, and a `skip` line in a source
that was not applied does not cause exit code 1. Sources applied before a
failure stay applied. With `--quiet`, a terminal run does not print the blocks,
the `kept modified file` notes, the result lines or the closing summary. The
question names the source (`Apply changes from <source>? [y/N/q]`). The
`skipped` lines of the skipped items that cause exit code 1,
`Use --force to replace.` and the adoption notice of a `schema: 1` lockfile are
printed on stderr; without `--quiet`, a terminal run prints no `skipped` line,
because the blocks name these files. `update --dry-run --quiet` prints nothing.
A terminal run prints the adoption notice before the block of the source that
adopted the lockfile.

A run with `--yes`, or without a terminal, prints `Updated <n> of <m> item(s)`
and one status line per item when an item changed or was skipped, files were
written or deleted, or a lockfile was adopted. A run whose only output is
`kept modified file` notes prints nothing on stdout. It reports a completed
interrupted item as `completed — interrupted before` and counts it. When no item
was updated, completed or removed, but files were written or deleted, it prints
`Changed <n> file(s) written or deleted`. It prints
`everything is up to date — nothing to do` when nothing changed, no item was
skipped, no file was kept with a note and no lockfile was adopted.

`update --dry-run` prints the blocks, then
`Dry-run: would update <n> of <m> item(s)` and the line
`Each block is computed for the current state; a later block does not include the effect of an earlier one.`
Each block is exact for the files on disk, but because nothing is applied, a
later block does not include the effect of applying an earlier source. With a
`schema: 1` lockfile, the previews are computed from an adoption held in memory,
and nothing is written.

The plugin and MCP steps of a bundle source run when the source is applied. They
are not part of the preview and are not a reason to ask. A terminal run does not
apply a source whose preview changes nothing, so it does not repair a missing
plugin or MCP registration of that source; `upskill doctor` reports it and
`update --yes` repairs it. A change of only the `required_by` field of an item
record is not shown as a change and does not cause a question; it is saved when
the source is applied for another reason, or with `--yes`.

A destination that exists and is not a regular file (for example a directory or
a symbolic link) fails the command before any file of that source is written.
Two exceptions concern recorded files of the same item that the new version no
longer contains. A directory at the path of a planned file is accepted when it
holds at least one file and every file in it, at any depth, is such a recorded
file; an empty directory, or one that holds an empty subdirectory, fails. A
regular file where the new version needs a directory is accepted when it is such
a recorded file. These files are handled as described above: an unchanged one is
deleted, and a modified one is kept and the item is skipped (with `--force`, it
is deleted). Three cases are not "before any change": an `update` over several
sources has already applied the sources it updated before, a `schema: 1`
lockfile has already been adopted and saved, and when the deletion of a file of
an item that its source no longer contains fails, the install of that source has
already been written and saved. A removal that is refused when it is planned (a
symbolic link above a recorded path, or a recorded path that cannot be
inspected) fails before the install of that source is written. This applies also
with `--force`. For `update`, two other cases also fail a source before any file
of that source is written, in every mode: a source that resolves to no items
while the lockfile records items from it, and a hand-shake file that cannot be
updated (for example an `opencode.json` whose `instructions` is not an array).

Older installations may record `gitlab:org/repo` or
`gitlab+gitlab.example.com:org/team/repo:skills/team.bundle.yaml` source labels.
`update` reads unambiguous historical labels and writes the corresponding HTTPS
labels after a successful reinstall. Installed aliases, pinned refs and bundle
membership survive. `update --dry-run`, `list` and `doctor` leave recorded
labels and generated bytes unchanged. New `add` commands still require modern
syntax.

For an explicit port, historical state must separate the port from the
namespace: `gitlab+gitlab.example.com:8443:org/team/repo`. Malformed or
ambiguous authorities receive reconciliation guidance before fetch. See the
[historical grammar and limits](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12)
before correcting a label; verify the intended HTTPS remote, ref and subfolder.
This label compatibility step does not itself change the schema; the reinstall
adopts a `schema: 1` lockfile as described in [State files](#state-files).

### `upskill remove [name...] [--source <label>] [--force]`

Remove installed items.

```bash
upskill remove code-review                         # remove one item
upskill remove code-review secret-scanner          # remove several
upskill remove --source github:driftsys/skills     # remove every item from a source
upskill remove --global code-review                # global scope
upskill remove --project code-review               # current directory
upskill remove --force code-review                 # also delete modified files
```

Bare `upskill remove` is rejected — be explicit. Ancillary files (`CLAUDE.md`,
`opencode.json`, `.vscode/settings.json`) are not touched.

Without a scope flag, `remove` uses the project scope inside a git repository
and the global scope outside one. When the current directory is the home
directory, the project scope is refused with exit code 2 and a message that asks
for `--global`. When a named item is not in the lockfile of that scope but the
other scope records it, `remove` fails with `not in lockfile` and prints one
line for each such name,
`hint: '<name>' is installed in the <scope> scope (<root>); pass
--<scope>`, to
stderr. A name that the current scope also holds gets no such line. When no name
qualifies, `remove` prints the `upskill doctor` hint instead. `update <name>`
does the same.

`remove` deletes only files listed in the item's `files` or `pending` record in
the lockfile (see [State files](#state-files)). Each recorded file is in one of
three states:

- **Owned** — a regular file whose content matches the recorded SHA-256.
  `remove` deletes it.
- **Modified** — the content differs from the recorded SHA-256, or the path is
  no longer a regular file (for example a symbolic link or a directory). The
  content is compared as it is and again with every CRLF replaced by LF (see
  [Modified and unmanaged files](#upskill-add-source-items)). `remove` keeps it
  and prints `kept modified file: <path>` to stderr. With `--force`, `remove`
  deletes a modified regular file; a path that is not a regular file is kept
  even with `--force`. A recorded regular file that `remove` cannot read (for
  example because of its permissions) is handled as modified.
- **Absent** — the path does not exist. `remove` ignores it.

A file that the record does not list (unowned), for example a file you added
inside an item directory, is never deleted. After the files of an item are
handled, `remove` deletes each item directory left empty; it never deletes a
client root (such as `.claude`) or a kind directory (such as `.claude/skills`).
A recorded path outside the client roots `.claude`, `.github`, `.copilot`,
`.agents` and `.opencode` is never deleted. If a directory inside the item part
of a recorded path (below the kind directory) is a symbolic link, `remove` fails
before it deletes anything. An item whose `files` and `pending` records are both
empty has nothing deleted. The lockfile entry of every removed item is dropped,
including when files were kept.

`remove` adopts a `schema: 1` lockfile and prints the adoption notice, as
described in [State files](#state-files). A name that is not in the lockfile is
an error, and in that case `remove` does not adopt.

`--source` triggers a y/N confirmation prompt on a TTY (it removes every item
from that label at once). Pass `-y` / `--yes` to skip. Non-interactive contexts
(CI, pipes) skip the prompt automatically.

### `upskill remove-mcp <name>`

Unregister an MCP server that a bundle install configured (see the `mcps:` note
under [`upskill add`](#upskill-add-source-items)).

```bash
upskill remove-mcp drawio            # remove from the current project
upskill remove-mcp drawio --global   # remove from the $HOME scope
```

`<name>` is the upskill-level MCP name recorded in the lockfile. `remove-mcp`
unregisters the server from the client (`claude mcp remove
<name>`, falling back
to deleting the entry from the client config file when the CLI is absent) and
drops the `LockedMcp` entry from the lockfile. Without a scope flag,
`remove-mcp` uses the project scope inside a git repository and the global scope
outside one. When the current directory is the home directory, the project scope
is refused with exit code 2 and a message that asks for `--global`.

### `upskill list`

Show installed content from `.upskill-lock.json`, grouped by kind. Bundles, when
present, are surfaced as a separate section.

Without a scope flag, `list` reports both scopes: the project scope (the current
directory, in or outside a git repository), then the global scope (`$HOME`).
When at least one reported scope has items, each scope is a block that starts
with `<scope> (<root>)`, and a scope without items shows `No items installed.`
in its block. When every reported scope is empty, there is no block header: the
output is `No items installed.` followed by two hint lines. `--global` or
`--project` limits the report to that scope. When the current directory is the
home directory, only the global scope is reported, and `--project` is refused
with exit code 2 and a message that asks for `--global`. When neither `HOME` nor
`USERPROFILE` is set, only the project scope is reported and stderr gets
`note: the global scope is not shown: HOME is not set`.

The `--available` view (items discoverable from configured sources) is deferred
for a future release; v0.2.0 ships the installed-state view only.

Pass `--json` for a stable machine-readable document:

```json
{
  "scopes": [
    {
      "scope": "project",
      "root": "/path/to/project",
      "rules":   [{ "kind": "rule",  "name": "...", "source": "...", "git_ref": null }],
      "skills":  [{ "kind": "skill", "name": "...", "source": "...", "git_ref": null }],
      "agents":  [{ "kind": "agent", "name": "...", "source": "...", "git_ref": null }],
      "bundles": [{ "name": "...", "source": "...", "git_ref": null, "items": [] }]
    }
  ]
}
```

`scopes` has one element for each reported scope, in the order project, global,
also when `--global` or `--project` selects one scope. `scope` is `"project"` or
`"global"`.

`kind` is one of `"rule"`, `"skill"`, `"agent"`. `git_ref` is the pinned
ref/tag/branch when the source is a git URL, otherwise `null`. `source` matches
the lockfile label (`local:/path` or `github:owner/repo` etc.).

### `upskill doctor`

Verify on-disk state matches `.upskill-lock.json`. Without a scope flag,
`doctor` checks both scopes, in the order and with the same rules as
[`list`](#upskill-list); `--global` or `--project` limits the check to that
scope. For each scope it reports eleven independent buckets:

- **Missing per-client output files** — reinstall fixes
  (`upskill add
  <source>`). For an item with a non-empty `files` record, every
  recorded path that does not exist; for an item with an empty or missing record
  (a `schema: 1` lockfile, or a `schema: 2` item with an empty `files` list),
  each per-client entry file that does not exist. A recorded path that `remove`
  would never delete (outside the client roots, fewer than three components, or
  with a `..` component) is skipped without a report, in this bucket and in the
  modified and unreadable buckets.
- **Modified files** — recorded files whose content, compared as it is and again
  with every CRLF replaced by LF, differs from the recorded SHA-256, or that are
  no longer regular files. Causes exit 1.
- **Unreadable files** — recorded files that could not be inspected (for example
  a permission error). `doctor` continues with the other files and items. Causes
  exit 1.
- **SSOT hash drift on `local:` sources** — `upskill update` fixes.
- **Lockfile entries with no recoverable source** (the local path went away, or
  the named item was removed in the source) — `upskill remove` to clear.
- **Orphaned dependencies** (informational) — items installed only as a
  dependency of items that are no longer installed. Does **not** cause exit 1.
- **Missing plugins** — recorded as `installed` in the lockfile but no longer
  found in the client. Likely uninstalled out-of-band; `upskill update --yes`
  reinstalls them. A run in a terminal applies a source only when its preview
  changes something, and the plugin and MCP steps are not part of the preview.
  Causes exit 1.
- **Skipped plugins** (informational) — recorded as `skipped` because the client
  CLI was not on PATH at install time. Install the CLI then run
  `upskill update --yes` to install them. A run in a terminal applies a source
  only when its preview changes something, and the plugin and MCP steps are not
  part of the preview. Does **not** cause exit 1.
- **MCP servers** — each Claude-client MCP entry in the lockfile is checked
  against `claude mcp list`. A server in the lockfile but **not registered** in
  the client causes exit 1 (`upskill update --yes` reconfigures it; a run in a
  terminal applies a source only when its preview changes something, and the
  plugin and MCP steps are not part of the preview). A missing `claude` CLI or a
  failed query is advisory and does **not** cause exit 1.
- **Interrupted** — items with a non-empty `pending` record: an `add` or
  `update` failed or was stopped before it completed. Run the same command again
  to complete it. Causes exit 1.
- **Earlier layout** (global scope only) — files of a global install at the
  earlier global Copilot paths (`.github/skills`, `.github/instructions` and
  `.github/agents` under `$HOME`). `upskill update --global` moves them. Causes
  exit 1. The files are not also reported as missing outputs.

`doctor` also prints a note for each item that the lockfiles of both scopes
record, `note: <kind> <name> is installed in both scopes`. The note does not
change the exit code.

Exits 0 when clean, 1 when any drift bucket is non-empty (missing outputs,
modified files, unreadable files, stale hashes, orphan entries, missing plugins,
unregistered MCP servers, interrupted items, or files at earlier global paths).
Skipped plugins and unverifiable MCP entries are informational warnings and do
not affect the exit code. `doctor` never fetches; remote-source drift detection
is `update --dry-run`.

Pass `--json` for a stable machine-readable document. Exit code is unchanged.
The document has one element in `scopes` for each checked scope (also when
`--global` or `--project` selects one scope) and the list `in_both_scopes`. The
example shows one element of `scopes`; the keys `scope` and `root` come first
and `earlier_layout` is always present, empty for the project scope.

```json
{
  "scopes": [ { "scope": "project", "root": "/path/to/project", "...": "..." } ],
  "in_both_scopes": [ { "kind": "skill", "name": "..." } ]
}
```

Each element of `scopes` has these keys besides `scope` and `root`:

```json
{
  "missing_outputs": [
    { "kind": "skill", "name": "...", "missing_files": ["..."] }
  ],
  "modified_files": [
    { "kind": "rule", "name": "...", "files": ["..."] }
  ],
  "unreadable_files": [
    { "kind": "skill", "name": "...", "files": ["..."],
      "error": "..." }
  ],
  "stale_hashes": [
    { "kind": "rule",  "name": "...", "source": "local:...",
      "stored_hash": "...", "current_hash": "..." }
  ],
  "orphan_entries": [
    { "kind": "agent", "name": "...", "source": "local:...",
      "reason": "local-path-gone" }
  ],
  "orphaned_dependencies": [
    { "kind": "skill", "name": "...", "former_requirers": ["skill:..."] }
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
  ],
  "interrupted": [
    { "kind": "skill", "name": "..." }
  ],
  "earlier_layout": [
    { "kind": "skill", "name": "...", "files": [".github/skills/<name>/SKILL.md"] }
  ]
}
```

`reason` is `"local-path-gone"` or `"item-missing-in-source"`. Hashes may be
`null` when the SSOT can't be hashed (e.g. unreadable). MCP `status` is one of
`"ok"`, `"not-registered"`, `"cli-not-found"`, or
`{ "query-failed": { "stderr": "..." } }`. The plugin and item arrays are always
present (possibly empty); `mcp_entries` is omitted when no MCP servers are
recorded.

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

Run inside a **source-registry** working tree. Each refuses to run inside a
consumer project (detected by `.upskill-lock.json` at the path's root) so you
don't accidentally lint generated outputs or scaffold into the wrong tree.

### `upskill new <kind> <name>`

Scaffold a new SSOT item directory. `<kind>` is one of `rule`, `skill`, or
`agent`; `<name>` is both the item name and the directory name (lowercase
letters, digits, hyphens; max 64 chars).

```bash
upskill new rule  no-direct-database-access
upskill new skill code-review
upskill new agent security-reviewer
```

Creates `<name>/<KIND>.md` **relative to the current directory** with the
minimum frontmatter the format spec requires — there is no `<kind>` parent
folder and no separate "destination" argument. The folder is the `<name>` you
pass, so to place an item under `skills/` you run the command from inside
`skills/` (see [Conventions](conventions.md#scaffolding-under-the-convention)).
Agents get `mode: subagent` and `model: sonnet` so the file is generation-ready
out of the box. The scaffold round-trips through `upskill fmt` as a no-op and
passes `upskill lint --strict`.

**Co-location.** Run `new` again with a different kind and the same `<name>` to
add that kind as a sibling entrypoint inside the existing item directory
([format-spec §2.1](format-spec.md#21-item-directory-structure)):

```bash
upskill new skill api-handler   # → api-handler/SKILL.md
upskill new rule  api-handler   # → api-handler/RULE.md (added alongside)
```

This expresses one capability across multiple kinds (for example a skill paired
with its enforcing rule). Co-location is refused only when an existing **skill**
entrypoint's `name:` diverges from `<name>`; rule and agent siblings MAY carry a
divergent name.

`upskill new` is an author command — it refuses to run inside a consumer project
(one with a `.upskill-lock.json`).

### `upskill lint [paths...]`

Validate SSOT files against the [format spec](format-spec.md). Six rules ship
out of the box:

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

Canonicalise YAML frontmatter (key order, indentation, alphabetised unknown
keys) and format the markdown body via dprint (the same formatter the generation
pipeline uses). The frontmatter↔body seam is normalised to a single blank line;
YAML comments and prose wrapping are preserved.

```bash
upskill fmt                  # format everything in the working tree
upskill fmt my-skill/        # format a single item directory
```

Files whose frontmatter and body are already canonical are left untouched (no
`mtime` thrash).

## State files

`.upskill-lock.json` lives in one of two places depending on scope:

| Scope   | Location                   | Committed? | Purpose                                    |
| ------- | -------------------------- | ---------- | ------------------------------------------ |
| Project | `<cwd>/.upskill-lock.json` | Yes        | Deterministic regeneration in CI.          |
| Global  | `$HOME/.upskill-lock.json` | No         | Cross-repo continuity for global installs. |

The lockfile carries a top-level `schema` field for forward compatibility. A new
lockfile is written with `schema: 2`. Each item entry may carry `files` and
`pending` lists. `add` and `update` record in `files` every file they write for
the item, with its client, its path and the SHA-256 of its bytes. When they
reinstall an item, they delete the recorded files that the new version no longer
produces, unless another item installed by the same command writes that path,
and only when the file is unchanged: a modified recorded file is kept and stays
in the record. A file that the record does not list is never deleted. A file at
a path that the new version writes is replaced only when it is unchanged, absent
or already equal to the new bytes; otherwise the item is not written (see
[`upskill add`](#upskill-add-source-items) and
[`upskill update`](#upskill-update-name)) unless `--force` is given. An item
that its source no longer contains is removed by `update` with the rules of
`remove`. `remove` deletes only recorded files, as described in
[`upskill remove`](#upskill-remove-name---source-label---force).

`add` and `update` run in this order:

1. Check every destination and decide which items are written. Nothing is
   changed yet.
2. For each item that is written and that changes something on disk (it writes
   at least one file or deletes at least one stale file), add the planned files
   to `pending` (an item new to the lockfile is added with an empty `files`
   list), and save the lockfile. The records already in `pending` stay, so a
   file written by an earlier command that failed stays recorded. An item that
   writes and deletes nothing gets no `pending` record; when no item gets one,
   the lockfile is not saved in this step.
3. For each item, delete the stale recorded files, then write each file to a new
   file `.upskill-stage-<16 hex digits>` in the destination directory, set the
   mode of the source resource on it (a generated file, which has no such mode,
   gets the default mode of a new file), and rename it to the destination. A
   recorded file that is unchanged and already holds the generated bytes, or a
   CRLF variant of them as described in
   [`upskill add`](#upskill-add-source-items), is not written; when its
   executable bits differ from the target executable bits (each executable bit
   of the source resource for which the file has the matching read bit), they
   are set to the target, and the other permission bits of the file are left as
   they are. On an error, delete that stage file and stop.
4. For each written item, set `files` to the planned files and the records of
   its kept stale files, clear `pending` (so the older `pending` hashes of a
   planned path are dropped), and save the lockfile.

If the command fails or is stopped between steps 2 and 4, the lockfile keeps the
`pending` records. A file that holds the bytes of its `pending` record or of its
`files` record counts as unchanged, so running the same command again, without
`--force`, completes the operation. If that command fails too, the `pending`
records of the earlier command stay next to its own, so a file written by either
command still counts as unchanged. `doctor` reports such an item as interrupted,
and `remove` deletes the files of both records. `update` also completes an
interrupted item when its source did not change: the block of its source has a
`complete` line for it, so a terminal run without `--yes` asks `Apply? [y/N/q]`
for that source, and an applied update reports it as `completed`. When the
command that completes the operation no longer plans a file that a failed
command wrote (for example a narrower client selection, or a source that dropped
a resource), that file is handled as a stale recorded file: deleted when
unchanged, kept and still recorded when modified. A stage file left by a process
that was killed is not deleted by upskill. Two upskill processes that change one
scope at the same time are not supported.

A `schema: 1` lockfile is still read. The first `add`, `update` or `remove`
adopts it before it changes any file: it keeps a copy of the old lockfile as
`.upskill-lock.json.v1` (when that file already exists, from an earlier
adoption, it is kept unchanged and not replaced), records each file that exists
at the output paths of every installed item with its current hash, saves the
lockfile as `schema: 2`, and prints the adopted paths to stderr. Adoption cannot
tell an edited file from one an earlier version wrote. It records two kinds of
files as unchanged: generated files that were edited before the upgrade, and
files that were added inside an item directory. The command that adopts them, or
a later `update`, may replace or delete them, and a later `remove` may delete
them. Adoption records only the files that exist. For an item that still has at
least one file, `doctor` therefore no longer reports the other outputs of that
item that were already missing before adoption. The adopted paths are printed
also when the command fails after the adoption was saved. `update --dry-run` and
`doctor` do not adopt, and neither does a `remove` that fails because a name is
not in the lockfile. An `add` does not adopt when it is refused because a source
item overlaps its own output paths, because an item conflicts with an item
installed from another source, because two items would install under one name,
or because an installed name contains a path separator. An `update` refused for
one of these reasons does not adopt when the refused source is the first source
it updates, and neither does an `update` whose first source resolves to no items
while the lockfile records items from it.

## Per-client output paths

| Item kind | Claude Code                | GitHub Copilot                                | opencode                       |
| --------- | -------------------------- | --------------------------------------------- | ------------------------------ |
| Rule      | `.claude/rules/<name>.md`  | `.github/instructions/<name>.instructions.md` | `.agents/rules/<name>/RULE.md` |
| Skill     | `.claude/skills/<name>/`   | `.github/skills/<name>/`                      | `.agents/skills/<name>/`       |
| Agent     | `.claude/agents/<name>.md` | `.github/agents/<name>.agent.md`              | `.opencode/agents/<name>.md`   |

These paths are relative to the root of the scope: the current directory for the
project scope and `$HOME` for the global scope. In the global scope, the Claude
Code and opencode paths are the same, and Copilot content is written to the
personal directories of Copilot CLI:

| Item kind | GitHub Copilot, global scope                   |
| --------- | ---------------------------------------------- |
| Rule      | `.copilot/instructions/<name>.instructions.md` |
| Skill     | `.copilot/skills/<name>/SKILL.md`              |
| Agent     | `.copilot/agents/<name>.agent.md`              |

The locations are those of the
[Copilot CLI configuration directory reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
and of the
[custom instructions guide](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions).

Limits of the global scope:

- upskill does not read `COPILOT_HOME` or `CLAUDE_CONFIG_DIR`. Global paths are
  relative to `$HOME`.
- opencode output in the global scope uses the same paths as in the project
  scope. Of these paths, only `.agents/skills` is a location that opencode
  documents for the whole machine.
- In the global scope, the first-time files `CLAUDE.md`, `.vscode/settings.json`
  and `opencode.json` are written under `$HOME` as in a project. The entry in
  `.vscode/settings.json` points at `.github/instructions`, which a global
  install no longer writes.

All output is **copy** (not symlink) — Windows portability without Developer
Mode.

## Exit codes

| Code | Meaning                                                                                    |
| ---- | ------------------------------------------------------------------------------------------ |
| 0    | Success                                                                                    |
| 1    | General error; or `add` / `update` skipped an item because of a modified or unmanaged file |
| 2    | Usage error (bad args)                                                                     |
| 130  | Interrupted (Ctrl+C)                                                                       |
