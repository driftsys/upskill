# upskill — Specification

> Upskill your coding agents.

This specification describes the released command behaviour. Sections 4.1 to
4.21 keep their headings for link compatibility; the library behaviour they
described was removed and withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md).

## 1. Overview

`upskill` is a single-binary CLI that lets authors **create, manage, and
prolong** AI-assistance content — rules, skills, agents — across multiple AI
coding clients (Claude Code, GitHub Copilot, opencode) from a single source of
truth.

The central abstraction is **generation**: the SSOT (Single Source of Truth) on
disk is portable; per-client output is produced on demand by a generation
pipeline.

<a id="11-three-item-kinds"></a>

### 1.1 Item kinds

| Kind    | Purpose                                  | Entrypoint file |
| ------- | ---------------------------------------- | --------------- |
| Rule    | Always-on behavioural guidance.          | `RULE.md`       |
| Skill   | On-demand procedural content.            | `SKILL.md`      |
| Agent   | Persona definition with tool scope.      | `AGENT.md`      |
| Command | A procedure that the user calls by name. | `COMMAND.md`    |

The on-disk contract is defined by the
[portable format spec](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md).
This document describes upskill's behaviour against that contract.

### 1.2 Two scopes

| Scope   | Root    | Use                                            |
| ------- | ------- | ---------------------------------------------- |
| Project | `<cwd>` | Content shared across the team via git.        |
| Global  | `$HOME` | Content available to a single user, all repos. |

The root of a scope holds its lockfile (§4) and the per-client output of its
items. The output paths relative to the root are in §3.1. `$HOME` is the value
of the `HOME` environment variable, or of `USERPROFILE` when `HOME` is not set.

### 1.3 Two roles

- **Source registry** — repository where SSOT items are authored. Holds the
  canonical `RULE.md` / `SKILL.md` / `AGENT.md` / `COMMAND.md` files and
  `*.bundle.yaml` manifests. Author commands (`new`, `lint`, `fmt`, `pack`)
  operate here.
- **Consumer project** — repository where generated outputs are installed for
  use by AI clients. Holds only generated per-client files. Consumer commands
  (`add`, `remove`, `update`, `list`, `doctor`, `search`) operate here.

The same repository can be both, but SSOT and generated outputs do not mix in
the same tree.

## 2. CLI surface

No overlap between verbs.

| Command  | Role     | Purpose                                               |
| -------- | -------- | ----------------------------------------------------- |
| `add`    | Consumer | Install items or bundles from any source.             |
| `remove` | Consumer | Remove installed content.                             |
| `update` | Consumer | Pull latest, regenerate changed items.                |
| `list`   | Consumer | Show installed content from the lock file.            |
| `doctor` | Consumer | Verify installation consistency.                      |
| `search` | Consumer | Look up skills via the public registry.               |
| `index`  | Consumer | Build or manage the local registry index cache.       |
| `new`    | Author   | Scaffold a rule, skill, agent, or command.            |
| `lint`   | Author   | Validate SSOT files against the format spec.          |
| `pack`   | Author   | Build Claude Code and Copilot CLI plugin directories. |
| `fmt`    | Author   | Canonicalise YAML frontmatter (key order, quoting).   |

### 2.1 `add`

```text
upskill add <source> [items...] [--global|--project]
```

`add --kind <kind>` (short `-k`, repeatable) restricts items to `rule`, `skill`
or `agent`. A required item of another kind is not installed. It also applies
without item names. An unknown kind is a usage error (exit 2); filtering out
every named item is an error (exit 1). With any kind restriction, the plugin and
MCP steps are skipped: no plugin or MCP registration or lockfile record is
created by those steps. For example:
`upskill add owner/repo my-bundle --kind rule --project --claude`.

`<source>` accepts (parity with `npx skills add`):

- `owner/repo` — GitHub shorthand
- `owner/repo@ref` — pinned ref (branch, tag, or commit SHA)
- `owner/repo:path/to/items` — subfolder: the directory that contains the item
  directories (a subfolder that is itself one item directory is refused, see
  below)
- `owner/repo:path/to/name.bundle.yaml` — bundle file (resolves transitively,
  see §2.6)
- `owner/repo@ref:path` — combined
- `https://github.com/owner/repo[...]` — full HTTPS URL
- `https://<host>/<path>[...]` — any other https git host: GitLab (including
  self-hosted instances and subgroups at any nesting depth), Bitbucket, Gitea,
  or a plain git server
- `.`, `./path`, `../path`, `/abs/path`, `~/path` — local paths

`upskill add <source>` installs everything the source contains. Optional
`items...` after the source filter to a subset. There is no interactive picker;
install-all-by-default fits bundle curation.

`add` of a local path that does not exist fails with exit code 1 and writes
nothing. With no item name, `add` of a local path that exists and holds no item
(other than the refused item directory described below) installs no item, still
writes the lockfile and `CLAUDE.md`, and exits with code 0.

A local path whose directory is itself an item directory (it holds `SKILL.md`,
`RULE.md` or `AGENT.md` at its root) is refused when it has no item in its child
or grandchild directories, holds no `*.bundle.yaml` file, and no item name given
to `add` matches a bundle in it. `add` then writes nothing, exits with code 1,
and prints an error that names the path. When the path is inside an output path
of the install, the error says that the path is an installed output of upskill,
not a source, and that the item must be added from its source registry.
Otherwise the error names the command to use instead,
`upskill add <parent> <item-name>`, with the parent path and the item name each
in single quotes when it holds a shell-special character (on Windows, the path
separator `\` is not one); for the file system root, which has no parent, the
error says so. A source is the directory that contains the item directories. A
git source is checked the same way after the fetch (`owner/repo:path`,
`owner/repo@ref:path`, a full URL with `:path`, and also a git source without a
subfolder), on the fetched directory, with the same exemptions; the subfolder is
first normalised: empty components (from a doubled or trailing `/`) and `.`
components are removed, and each `..` component removes the component before it.
When the subfolder is an item directory, the error names the source in the form
that `add` accepts (a github.com URL is shown as `owner/repo`), and the command
to use instead keeps the source form and the ref, with the parent subfolder in
place of the subfolder, for example `upskill add owner/repo@v1:skills foo` for
`owner/repo@v1:skills/foo`; when the parent is the repository root, the command
has no subfolder (`upskill add owner/repo foo`). When the repository root is one
item directory (no subfolder, or a subfolder that normalises to the root, such
as `:.`), the error says that the repository root is one item directory and that
upskill installs items from the directory that contains the item directories,
and names no command. The output-path case does not apply to a git source. With
item names given that match nothing in it, a git source that is one item
directory fails before this check, with exit code 1, nothing written, and the
error that says there are no matching items or bundles, not with the command to
use instead. This check applies to `add` only: `update` and the sources of
`requires` entries are not checked. `.` alone is a local path, the same as `./`.

`--project` writes into the current directory; `--global` writes under `$HOME`.
Inside a git repository, `add` without a scope flag uses the project scope.
Outside a git repository, `add` without a scope flag writes nothing and stops
with exit code 2 and a message that asks for `--project` or `--global`. When the
current directory is the home directory, which is the root of the global scope,
the project scope is refused: `add` writes nothing and stops with exit code 2
and a message that asks for `--global`, with `--project` and with no flag, in or
outside a git repository. `update`, `remove` and `remove-mcp` do the same with
`--project`, and with no flag in a home directory that is a git repository; with
no flag in a home directory that is not a git repository, they use the global
scope. `update --check` and `update --notice` with no flag in a home directory
report the global scope, in a git repository or not; `update --check --project`
is refused there.

`add`, `update`, `remove` and `remove-mcp` print one line to stderr before they
change anything, `scope: <scope> (<root>)`. `--quiet` hides it. `update --check`
prints no such line; the block of each scope names the scope and its root.
`update --notice` prints no such line either.

**Source overlap.** Before any write, `add` refuses the install, with exit code
1 and without any change to a file or the lockfile, when the directory of an
item in a source that it reads is the same as, is inside, or contains a path
that it deletes or writes for an item that it installs: the entrypoint or the
resource directory of that item for any client in the target scope, and in the
global scope also the paths that released versions wrote. An item that is not
selected or that `--exclude` names has no such paths, but its directory is
compared too. The sources that `add` reads are the given source and every
dependency source of a `requires` closure. An `--as` alias is used as the
installed name of an item of the given source. The `audience` field and the
client selection are not applied. The error names the item, its directory, the
output path and the item of that output path. In the same way, also with
`--force`, `add` refuses a source whose item directory contains a file that the
lockfile of the scope records (`files` or `pending`, §4) for an item from
another source (for this check, two local paths that resolve to the same
directory are one source); the error names the file and that item. With a
`schema: 1` lockfile, the recorded files are those that the adoption (§4) would
record; the check saves nothing.

Before any write, `add` classifies every destination of every item against the
item's `files` and `pending` records (§4). A destination is a conflict when it
is **modified** (a recorded regular file with other content) or **unowned** (a
regular file that is not recorded), unless its content already equals the
planned bytes. A file is compared with its record, and a recorded file with the
planned bytes, as it is and again with every CRLF (`\r\n`) replaced by LF
(`\n`); it is equal when one comparison matches. So a git checkout with
`core.autocrlf=true` does not make a file modified; where the recorded content
has a LF, a carriage return added by hand before it is also read as unchanged.
An owned file is not rewritten when it equals the planned bytes exactly, or when
it is a CRLF variant of the planned bytes and the hash of the planned bytes is
already in its `files` record (a `pending` hash does not count). So a CRLF file
is left as it is until its generated content changes, and a change of the line
endings of the generated content is written. A destination that exists and is
not a regular file (for example a directory or a symbolic link) is not a
conflict: it fails the command before any file is written. Two exceptions
concern recorded files of the same item that the plan no longer contains: a
directory at the path of a planned file that holds at least one file and whose
files, at any depth, are all such files (an empty directory, or one that holds
an empty subdirectory, fails), and such a regular file where a planned file
needs a directory. These files are handled as stale files (§2.2). For `update`
over several sources, the sources updated before are already applied; with a
`schema: 1` lockfile, the adoption is already saved; when the deletion of a file
of an item that its source no longer contains fails, the install of that source
is already written and saved. A removal that is refused when it is planned (a
symbolic link above a recorded path, or a recorded path that cannot be
inspected) fails before the install of that source is written. For `update`, a
source that resolves to no items while the lockfile records items from it, and a
hand-shake file that cannot be updated (for example an `opencode.json` whose
`instructions` is not an array), also fail the source before any file of that
source is written, in every mode. Without `--force`, an item with at least one
conflict is skipped as a whole: none of its files is written or deleted and its
lockfile entry is not changed, except its `required_by` (§2.2). The other items
are written. `add` then prints one line per conflict to stderr,
`skipped <kind> <name>: <path> was modified` or
`skipped <kind> <name>: <path> exists and is not managed by upskill`, followed
by `Use --force to replace.`, and exits with code 1. `--force` replaces these
files, deletes modified files that are no longer produced, and also replaces
items installed from another source; it then lists these files after
`Replaced or deleted <n> modified or unmanaged file(s) (--force)`, unless
`-q/--quiet` is given. An installed name or an `--as` alias that is empty, `.`
or `..`, or that contains `/` or `\`, is refused before any change, with an
error that names the item.

### 2.2 `update`

```text
upskill update [name...] [--global|--project] [--dry-run] [--force]
upskill update --check [name...] [--global|--project]
upskill update --notice [--repeat]
```

Without a scope flag, `update` uses the project scope inside a git repository
and the global scope outside one; `update --check` and `update --notice` report
both scopes instead. `--project` uses the current directory. When the current
directory is the home directory, the project scope is refused with exit code 2
and a message that asks for `--global`; `update --check` and `update --notice`
with no flag report the global scope there.

Without `--check` and `--notice`, always fetches the latest source before
regenerating; `--check` and `--notice` fetch nothing. Absorbs the role of a
separate `sync` command — there is no `--offline` flag in v1; offline use is
covered by passing local paths to `add` / `update`.

`update` covers two distinct behaviours:

- **Absorb client drift.** When Copilot or Claude Code change a path or
  frontmatter field, `update` regenerates outputs from the same SSOT.
- **Track upstream evolution.** `update` git-pulls source repos and regenerates
  items whose source hash changed.

`update --global` also moves an install that a released version wrote at the
earlier global Copilot paths (`.github/skills`, `.github/instructions` and
`.github/agents` under `$HOME`): it writes the item at the path of §3.1 and
deletes the file at the earlier path when it is unchanged. An edited file at the
earlier path is kept and named in the output. The item of such a run is reported
as up to date when its source did not change, and the last line counts the moved
files. For one skill, the output is:

```text
Updated 0 of 1 item(s)
  skill foo                              up to date
Changed 2 file(s) written or deleted
```

The empty client root and kind directories that remain (for example
`$HOME/.github/skills`) are kept, as the pruning rule of §2.3 states.

`update` applies the conflict rules of `add` (§2.1). An item with a modified or
unowned destination is not written and keeps its lockfile entry (with its
recorded commit, except under the rule that follows), except its `required_by`.
When a source hash is recorded for the item and equals the source hash of the
item (an item with no recorded hash is skipped), the item has an empty `pending`
record, every such destination is modified (none is unowned), applying the item
would write or delete no other file, the item has no recorded file that the new
version no longer produces (present or absent), and its lockfile entry would not
change in any field other than `files`, `pending`, `required_by` and `commit`
(for example `clients`), nothing is withheld: `update` reports
`kept modified file: <path>` for each modified destination (in a terminal and
with `--dry-run`, in the block of its source; with `--yes` or without a
terminal, on stderr), and this note does not change the exit code. Otherwise the
item is reported as skipped (in a terminal and with `--dry-run`, as a `skip`
line of its block; with `--yes` or without a terminal, as
`skipped <kind> <name>: <path> was modified or is not managed by upskill` on
stderr). A skipped item of an applied source, or of a source whose preview
changes nothing, makes `update` print `Use --force to replace.` on stderr and
exit with code 1 after the other items are written; a `skip` line of
`--dry-run`, or of a source that the user declined, does neither. Known limit:
when a new version of upskill renders an unchanged source to different bytes in
the modified file only, the new rendering is not written and only the note is
printed, until `update --force`; `doctor` lists the modified files. A recorded
file that the new version of an item no longer produces, for example a file of a
client that a narrower selection no longer targets, is deleted when it is owned;
a modified one is kept, stays in the record, and is reported
(`keeps modified <path>` on the item line of a block when the item has another
change, otherwise a `kept modified file: <path>` note of the block;
`kept modified file: <path>` on stderr with `--yes` or without a terminal). An
item that its source no longer contains is removed with the rules of `remove`
(§2.3). `update --force` replaces modified and unowned destinations and deletes
modified regular files that are no longer produced. A block names each of these
files as `replaces <path> (--force)` or `deletes <path> (--force)`; a run with
`--yes` or without a terminal lists them after
`Replaced or deleted <n> modified or unmanaged file(s) (--force)`.

In the **prepare step**, `update` fetches every source once (a source of a
cross-source dependency is fetched again when the source that requires it is
reinstalled), runs the checks of §2.1 and plans it, without any change. The
source overlap check of §2.1 is then applied to the items of all these sources
together, and also protects the item directories of every local-path source
recorded in the lockfile that the run does not otherwise read. The same item
directories are also compared with every recorded file (`files` and `pending`)
of an item that the run removes because its source no longer contains it, also a
modified file that the removal keeps: a directory that contains such a file
refuses the run with exit code 1 and an error that names both items, also with
`--force` and in a dry run. With a `schema: 1` lockfile, the recorded files are
those that the adoption (§4) would record; the check saves nothing. A recorded
directory source that no longer exists is skipped. For a recorded bundle file,
also one that no longer exists, the items of the source directory that `add`
resolves for it (§2.6) are protected; when that directory does not exist,
nothing is protected for it. A recorded source that exists and cannot be read as
a directory fails the step with an error that names its source label; the error
suggests `upskill remove --source <label>` only when a lockfile item has that
source. A git source is not fetched for this check; a skipped source (below) is
protected by this rule as a recorded source that the run does not read. A source
whose content cannot be fetched (a `git clone` that fails, for example because
the host does not answer, the repository was deleted, the ref does not exist or
the authentication is refused, or a local-path source whose path does not exist)
is skipped, also when the source of a cross-source dependency that it requires
cannot be fetched: none of its files is written, and its lockfile entries do not
change, except the `files` record that the adoption of a `schema: 1` lockfile
writes for every entry (see below); `update` prints on stderr, in every mode and
also with `--quiet`, one line that names the source label, says that the source
could not be fetched and was skipped, and gives the cause, and the other sources
are handled. A prepared source whose plan would write an item that the lockfile
records from a skipped source is skipped too, with a line that names its label,
says that it was skipped (not that it could not be fetched), and names the item
and the skipped source that the item comes from; this repeats until no further
source is skipped. The run then exits with code 1, also when every other source
was applied and in a dry run. After an interrupt, the prepare step fetches no
further source and records no further skip, and the run exits with code 130; the
lines of the sources skipped before the interrupt are still printed. Any other
error in the prepare step stops the run before any source is applied. Then
`update` handles the sources one at a time, sorted by source label. For each
source it computes a preview from the fetched content, the files on disk and the
lockfile, just before the source is applied. A preview **changes something**
when applying the source writes or deletes a file, changes the lockfile record
of an item (any field except `pending`, `required_by` and `commit`), completes
an interrupted item (a non-empty `pending` record, §4), removes an item, or
creates or changes a hand-shake file (§3.4). A write of a file that already
holds the planned bytes is not a change, unless the executable bits (`0o111`) of
the file differ from the target executable bits (Unix only): each executable bit
of the source resource for which the file has the matching read bit (owner,
group, other). The apply sets the executable bits to the target, so the file is
a change. A difference in the other permission bits is not a change. A `skip`
item and a `kept modified file` note are not changes.

- **Terminal, no `--yes`.** For each source, `update` prints a block: the source
  label; one line per item that the apply changes or skips, with the action
  `update`, `complete`, `remove` or `skip`; one `write <path>` line per
  hand-shake file; `up to date` when there is neither; then one
  `kept modified file: <path>` line per note. An item line counts the files that
  are written or deleted (`1 file`, `<n> files`) and names each kept modified
  file (`keeps modified <path>`), each file that `--force` replaces or deletes,
  and for `skip` each file that causes the skip. An `update` line whose item
  changes only in its lockfile record ends with `record only`. A `remove` line
  says `no longer in source`. A cross-source dependency that the reinstall
  writes has a line in the block of the source that requires it. When the
  preview changes something, `update` asks `Apply? [y/N/q]` on stderr: `y` or
  `yes` (any letter case) applies the source; `n`, `no`, an empty line or any
  other text does not apply it and continues; `q` or `quit` (any letter case),
  end of input or a read error does not apply it and stops the run, and earlier
  sources stay applied. Ctrl-C at the question exits with code 130 at once, and
  earlier sources stay applied; Ctrl-C during the apply of a source stops the
  run before the next source, with code 130. A source whose preview changes
  nothing is not applied and asks nothing. The apply decides again from the
  disk. After an applied source, `update` prints a result line with the non-zero
  counts of `<n> updated`, `<n> completed`, `<n> removed`, `<n> skipped`,
  `<n> modified file(s) kept` and `<n> file(s) changed`, or `applied`; the
  counts include a dependency from another source that the apply updated,
  completed or skipped. After the last source it prints
  `Updated <n> of <m> item(s) from <a> of <s> source(s).`: `<n>` is the number
  of items that the run updated, completed or removed, also as a dependency of
  another source, and `<m>` is the number of items that the run covers (the
  lockfile entries that the run selects, which are all entries or with names
  only the named entries, also of skipped sources and of sources not examined
  after `q`, and the items that the plans of the sources that were fetched and
  not skipped install). Each item, by kind and installed name, is counted once
  in `<n>` and in `<m>`, and `<m>` does not depend on where the run stops. The
  line is followed by `<k> source(s) not applied.` when a source with changes
  was not applied, by `<u> source(s) skipped.` when sources were skipped, for
  either cause, and by `Stopped; <r> source(s) not examined.` or `Stopped.`
  after `q` or end of input; then `Changed <n> file(s) written or deleted` when
  no item was updated, completed or removed but files were written or deleted.
  It prints `everything is up to date — nothing to do` instead when no source
  was applied, no source with changes was declined, no file was kept with a
  note, no item was skipped, no source was skipped and no lockfile was adopted.
  The exit code is 1 when an item is skipped in an applied source or in a source
  whose preview changes nothing, when a source was skipped, or when a step
  fails; answers `n` and `q` do not change it, and a `skip` line of a source
  that was not applied does not cause exit code 1. With `--quiet`, the blocks,
  the result lines and the closing summary are not printed, and each
  `kept modified file` note is printed on stderr, as with `--yes`; the question
  is `Apply changes from <source>? [y/N/q]`, and the `skipped` lines of the
  skipped items that cause exit code 1, `Use --force to replace.` and the
  adoption notice are printed on stderr. Without `--quiet`, a terminal run
  prints no `skipped` line.
- **`--yes`, or stdin is not a terminal.** Every source is applied without a
  question, also a source whose preview changes nothing, so its plugin and MCP
  steps run unless the source has a kind restriction, which skips both steps.
  `update` prints `Updated <n> of <m> item(s)` and one status line per item when
  an item changed or was skipped, files were written or deleted, or a lockfile
  was adopted (a run whose only output is `kept modified file` notes prints
  nothing on stdout), and reports a completed interrupted item as
  `completed — interrupted before`. `<n>` and `<m>` have the meaning of the
  terminal summary above; `<m>` can be larger than the number of status lines,
  because a dependency that the plan of a source installs unchanged has no
  status line. When no item was updated, completed or removed but files were
  written or deleted, it prints `Changed <n> file(s) written or deleted`. It
  prints `everything is up to date — nothing to do` when nothing changed, no
  item was skipped, no source was skipped, no file was kept with a note and no
  lockfile was adopted. An item of a skipped source has the status line
  `skipped — source not reachable` when its source could not be fetched, and
  `skipped — requires a source that was skipped` when its source requires an
  item of a skipped source.
- **Recorded commit.** For a git source, `update` sets the fetched commit on the
  lockfile entries of the source that the run selected (the named items, or
  every item when no name is given): with the item records when the source is
  applied (also on an item that keeps its modified files), and alone when the
  source has no changes and is not applied; an entry of an item whose update is
  skipped keeps its recorded commit. An entry that the names did not select
  keeps its recorded commit when the source is not applied. When the source is
  applied, the reinstall covers the named items, the items that a named item
  requires (from the same source or from another one). For a directory source
  with recorded bundles, it reinstalls the members of those bundles and the
  items of that source installed directly, including their `requires` closure.
  It also reinstalls each dependency of that source that no installed item
  requires, which `doctor` reports as an orphaned dependency, unless a recorded
  bundle of the source lists it as a member: a bundle member follows its bundle.
  It installs no unrelated item of the source. It applies a kind restriction
  only when every recorded bundle of the source has recorded kinds, using their
  union. With one unrestricted bundle of the source, no kind restriction
  applies. Every lockfile entry of that source whose kind is outside the
  restriction is left untouched and reported as up to date, regardless of bundle
  membership or `required_by`, unless its source is skipped. In that case it is
  reported as skipped with the cause of its source, also when the source is
  skipped later because it requires an item of a skipped source. To update such
  an item, run `upskill add <source> <item>` again. A `.bundle.yaml` source is
  reinstalled as a whole. Recorded bundle names resolve only as bundles. A
  missing bundle causes `bundle '<name>' is no longer in source <label>` before
  any write, even if an item has that name. Each reinstalled entry gets the
  commit of its own source (an entry from another source gets that source's
  commit), unless its update is skipped, and any other entry keeps its recorded
  commit. The lockfile is saved only when a value differs. This write is not a
  change of the source (see the preview above) and does not count in the output.
  A dry run, a declined source and a stopped run record no commit.
- **`--dry-run`.** `update` prints the same blocks, asks nothing, writes nothing
  and does not adopt a `schema: 1` lockfile (the previews use an adoption held
  in memory). It then prints `Dry-run: would update <n> of <m> item(s)` (`<n>`
  is the number of selected lockfile entries that the block of their own source
  lists with an action other than `skip`; a dependency from another source and
  an item that is not in the lockfile are not counted; `<m>` has the meaning of
  the terminal summary) and
  `Each block is computed for the current state; a later block does not include the effect of an earlier one.`,
  and exits with code 0 when no step fails and no source was skipped, for either
  cause (code 1 otherwise). `update --dry-run --quiet` prints nothing on stdout,
  also no `kept modified file` note; to see in CI what an update would change,
  run `update --dry-run` without `--quiet`, or `update --check`. Because nothing
  is applied, a later block does not include the effect of applying an earlier
  source.

A `schema: 1` lockfile is adopted once per run, without a question, after every
source that was not skipped (because it could not be fetched, or because it
requires an item of a skipped source) passed the checks of the prepare step, for
the first such source that also resolves to items (§4); in a terminal the
adoption notice is printed before the block of that source. The adoption stays
when the run fails or is stopped. The adoption covers the whole lockfile: an
entry of a skipped source gets the `files` record of its files on disk; no other
field of that entry changes and none of its files is written. When every source
is skipped, nothing is adopted. When a later step fails after a source was
skipped, the line of the skipped source is still printed, before the error.

Known limits: the plugin and MCP steps of a bundle source run when the source is
applied without a kind restriction. `update` skips both steps for a source under
a kind restriction. The steps are not part of the preview and are not a reason
to ask, so a terminal run does not repair a missing MCP registration of a source
whose preview changes nothing (`doctor` reports it, `update --yes` repairs it
without a kind restriction). A plugin that the lockfile records for a bundle of
a source of the run as installed, when the check of `doctor` reports it missing,
or as skipped, is installed again when the source is applied and also when its
preview changes nothing, in a terminal run as well; not for a source under a
kind restriction, not for a source that the answer did not apply, not with
`--dry-run` and not with `--check` (ADR-0008). A change of only `required_by` in
an item record is not shown and does not cause a question; it is saved when the
source is applied, and also when the preview of the source changes nothing, but
not with `--dry-run` or for a declined source. `required_by` is refreshed when
`add` writes its install, when `update` applies a source, and when `update` does
not apply a source whose preview changes nothing; a dry run, a declined source
and a source at which the run stops refresh nothing. A refresh computes
`required_by` of every lockfile entry again, and rewrites an entry only where
the new value differs. An install by item names, or of a whole directory source,
evaluates every item in its item closure as a requirer, including required
items. A bundle install, and a request that also names a bundle (resolved by
name), evaluate no requirer. An update of a directory source with recorded
bundles evaluates every item in the merged bundle and direct-item closure,
including bundle members even when there are no direct items. The bundle closure
itself contributes no item-level `requires` edges. A kind restriction retains
only items of the selected kinds in the closure; a kind-restricted `add` or
`update` leaves `required_by` unchanged on entries it does not plan, also in
other sources. Requirers outside the kind restriction keep their edges on
planned entries as well, because they are not evaluated. Within the entries
being refreshed, an evaluated requirer is removed and added back where it still
requires the item; an excluded dependency gets no link (see below). Every other
requirer is kept, also when it is not installed. The result does not depend on
the order of the sources or of two `add` commands. Each lockfile records one set
of excluded item names, for its install scope (§4). `add` and `update` skip
every item of the set, also an excluded bundle member and an excluded
`requires:` dependency from another source, before they link a `required_by`
entry to it; an excluded dependency does not pull its own `requires:` items, and
a whole-source `add` skips excluded items too. An `add` records its `--exclude`
names in the set. The excludes of an `add` are the recorded names, minus the
item names that it installs by name, plus its own `--exclude` names; an `update`
uses the same rule, with the item names that are installed by name in place of
the item names of the command line, so an exclude never removes an item that was
installed by name. For this rule, `update` counts every item that the lockfile
records for the source as installed by name, also a bundle member. An `update`
never removes a name from the set. An item name given on the command line of an
`add` is removed from the set, unless the same `add` also excludes it, so
`add <source> <name>` installs an item that an earlier `add` excluded. An `add`
without item names and without `--exclude` keeps the set and keeps the recorded
items out. When `add <source> --exclude <name>` names an item that is installed
from the same source, and the same `add` does not also name the item, `add`
removes that item only, within the kinds that `--kind` selects: its files and
its lock entry, no other item of a co-located group, no bundle record and no
plugin entry. It keeps each `required_by` link that names the item, as `remove`
does, so `doctor` reports a dependency that no installed item requires as an
orphaned dependency; it prints `removed — excluded <kind> <name>` on stdout.
When a recorded file of the item was modified, `add` removes nothing of the
item: its files and its lock entry stay, the exclude is recorded, and stderr
gets `kept — excluded <kind> <name>: <path> was modified` for each modified
file. An `--exclude` name that matches no item of the entry source and no item
that the resolver skipped is a usage error (exit code 2) that writes nothing, so
the lockfile is unchanged. `add` and `update` print `excluded <kind> <name>` on
stderr, once for each item that an exclude kept out of the run; `--quiet` hides
the line, and `update` prints it with `--dry-run` too. `doctor` reports nothing
about excludes.

`update --check` reports, for each selected scope, the sources that have a newer
version. It changes no file, does not fetch and conflicts with `--dry-run`,
`--notice`, `--yes`, `--force` and the client flags (exit code 2). Without a
scope flag it reports both scopes, in the order project, global; the global
scope alone when the current directory is the home directory. A scope prints one
block, headed `<scope> (<root>):`, with one line for each source that is not up
to date, then `up to date` when it has sources and no such line, or
`nothing installed` when it has no source. A line starts with the status label
(`update available`, `no recorded commit`, `pinned to a commit`,
`not reachable`, `not checked`) and names the source label. The last line is
`No updates available.` or `<n> update(s) available. Run ...` with the command
of each scope that has updates. The rules for one source, tested in this order:

| Condition                                                                         | Status                                                    |
| --------------------------------------------------------------------------------- | --------------------------------------------------------- |
| The source label cannot be parsed                                                 | `not reachable` (the parse error)                         |
| Local path source that does not exist, or cannot be read                          | `not reachable` (`path not found`, or the read error)     |
| Local path source: an item directory has another hash than recorded, or is absent | `update available`, those items listed; else `up to date` |
| The ref is a full commit hash                                                     | `pinned to a commit`, not looked up                       |
| The time limit (10 seconds for all sources) passed before the lookup              | `not checked`                                             |
| `git ls-remote` timed out                                                         | `not reachable` (`time limit reached`)                    |
| `git ls-remote` failed                                                            | `not reachable` (the message of git)                      |
| The remote has no such ref, or no HEAD                                            | `not reachable` (`ref <ref> not found`, `HEAD not found`) |
| An item has no recorded commit                                                    | `no recorded commit`                                      |
| Every recorded commit equals the remote commit, without regard to case            | `up to date`                                              |
| Otherwise                                                                         | `update available`, every selected item listed            |

A git source is looked up at most once for all scopes, and a source that shares
a lookup with an earlier source gets its result also after the time limit. A
newer tag is not looked for. With names, only the entries with those names are
grouped by source, so the status and the item list of a source cover only those
items. An item of a git source that `update` skipped keeps its older commit, so
its source is `update available` until the item is resolved, except that an
entry without a recorded commit gives `no recorded commit`; a local path source
has no recorded commit and is compared by content. Exit code 0 when the check
ran, also when a source is not reachable; 1 when a lockfile cannot be read or
parsed. `--quiet` suppresses the output.

`update --notice` prints a session notice for a session-start hook. It accepts
no name and no other flag except `--repeat` (exit code 2). `--repeat` without
`--notice` is a usage error (exit code 2). Its roots are the scopes that
`update --check` reports without a flag; a root without a lockfile is left out.
It changes no lockfile and no installed file. It keeps the cache file
`notice.json` in the upskill cache directory (it writes the file through a
temporary file `notice.json.<pid>.tmp` and a rename): `$XDG_CACHE_HOME/upskill`
when `XDG_CACHE_HOME` is an absolute path (an empty or relative value is
ignored), else `~/.cache/upskill`. The cache file is a JSON object
`{"scopes": {<root path>: <state>}}`; the key is the path of the scope root, and
a state has these fields, each defaulting to zero or empty when absent:

| Field        | Type            | Meaning                                                                                                     |
| ------------ | --------------- | ----------------------------------------------------------------------------------------------------------- |
| `checked_at` | integer         | Unix time in seconds of the last check. A check that reached no source also sets it.                        |
| `updates`    | array of string | The updates found by the checks that reached a source, sorted: `<kind>:<name>@<commit>` or `<kind>:<name>`. |
| `reported`   | array of string | The set that the last printed notice reported, sorted.                                                      |

Before reading `notice.json`, the command creates `notice.lock` in the same
cache directory with create-only-if-absent semantics. Each run writes a unique
ownership token to its lock. A run that finds a lock younger than 60 seconds
exits with code 0 without output, network requests or cache changes, also with
`--repeat`. A lock at least 60 seconds old, or dated in the future, is removed
only if the token read from it is unchanged, then creation is retried once. If
the token has changed, the run exits silently as for a busy lock. Other lock
errors let the run continue without a lock. A completed run removes the lock
only if the file still holds its own token; an interrupted or killed run can
leave it until it expires. Install, update and removal have no writer lock.

Three limits remain: the token check and removal are separate operations, so a
replacement between them can still be deleted and concurrent runs reclaiming an
expired lock can both print; a run that lasts 60 seconds or more can overlap its
replacement; sessions in different repositories sharing this cache also share
the lock, so one can print no notice until its next session start.

A file that is absent, unreadable or not valid JSON is an empty cache. The
rules, in order:

1. A root is checked when it has no state, when `checked_at` is 86 400 seconds
   or more before the current time, or when `checked_at` is after the current
   time. The check is `update --check` for the selected roots with a time limit
   of 5 seconds for all sources, and a source looked up for two roots once. The
   check sets `checked_at` to the current time. It sets `updates` as follows:
   when every source of the root answered (a root without a source counts as
   answered), `updates` becomes the set that the check found; when at least one
   source answered and at least one was `not reachable` or `not checked`, the
   check adds its entries and keeps every earlier entry whose `<kind>:<name>`
   part is not in the found set (the check cannot tell which source an earlier
   entry came from); when every source was `not reachable` or `not checked`,
   `updates` stays.
2. For each root, project then global, the command compares `updates` with
   `reported`. When `updates` is empty, it empties `reported` and prints
   nothing. When they are equal and `--repeat` is not given, it prints nothing.
   Otherwise it sets `reported` to `updates` and prints
   `upskill: <n> update(s) available (<scope>). Run "<command>".`, where `<n>`
   is the number of distinct entries of `updates` and `<command>` is
   `upskill update` for the project scope and `upskill update --global` for the
   global scope. `update --check` counts the items as it lists them.
3. The state of a root that has no lockfile is removed, and so is every state
   whose root directory holds no `.upskill-lock.json`. The command reads the
   cache file again, applies the states of the roots of this run to that copy,
   and writes the copy, through a temporary file and a rename, only when its
   content changed. For a root that this run did not check and that the copy
   holds, it keeps `checked_at` and `updates` of the copy and sets `reported` to
   the entries of those `updates` that the `reported` of the copy or of this run
   holds. An entry that is no longer in `updates` is dropped, so a removal by an
   `update` that saved during this run stays removed, a set that another run
   printed stays recorded as printed, and an entry of an older set does not keep
   `reported` different from `updates`. For a root that the copy does not hold,
   the state of this run is added. For a root that this run checked, its own
   state replaces the state of the copy: when an `update` of that root saved
   during the lookups, the entries that the `update` removed come back and stay
   until the next check in which every source of the scope answers. A failed
   write is ignored. An applying `update`, or another notice run that proceeds
   without a lock, loses its save when it saves between the second read and the
   rename of this run. This also applies to an `update` that removes entries:
   its removal is lost when a notice run renames after it, and a notice run is
   lost when the `update` saves between its own load and rename. A lost check of
   a notice run is made again at the next session start, because the root is
   still due. A lost removal is not: the root is not due, so the removed entries
   stay until the next check in which every source of the scope answers. The
   notice run lock does not cover applying `update` or notice runs that proceed
   without a lock.

Every failure after argument parsing prints nothing and gives exit code 0;
nothing is written to stderr. An interrupt (SIGINT) ends the command at once
with exit code 0. During the lookups it prints nothing and writes no cache file.
After the line was written and before the cache file was saved, the same line
prints again in the next session. A temporary file `notice.json.<pid>.tmp` can
remain in the cache directory. A failed installation of the signal handler is
ignored. The failures are: no cache directory is known (neither an absolute
`XDG_CACHE_HOME` nor the home directory), the scope roots cannot be determined,
and a lockfile that cannot be parsed. Only a root that is due has its lockfile
read; when that lockfile cannot be parsed, the command prints nothing for any
root, also not the cached line of a root that is not due, and writes no cache
file. A source that is not reachable adds no update, but the entries that
earlier checks stored stay in `updates` (rule 1) and rule 2 counts them; a
failed read or write of the cache file is ignored. When the cache directory
cannot be written, nothing is stored, so the check runs and the line is printed
at every session start. With `--quiet`, rule 1 and rule 3 apply, and rule 2
prints nothing and leaves `reported` unchanged.

An `update` that applies (not `--dry-run`, `--check` or `--notice`) and returns
its report, with exit code 0, 1 or 130 (an interrupt), removes from `updates`
and `reported` of its scope root every entry whose `<kind>:<name>` part is an
item that the run reports as updated, up to date, completed after an
interruption, or removed. It loads the file, applies the change and saves the
file at once, without a lookup between the load and the save. Without this step,
`--repeat` would print updates that are already installed until the next check.
An `update` that stops with an error removes nothing: the items that it had
updated stay in `updates` until the next check in which every source of the
scope answers. A failed read or write of the cache file does not change the exit
code of `update` and prints nothing.

Persisted schema-1 `gitlab:` and `gitlab+<host>:` source labels remain readable
for updates. Unambiguous historical labels decode to generic HTTPS sources,
preserving namespace, ref and subfolder. Applying `update` canonicalizes the
source labels of reinstalled items and bundles, even when their content is
unchanged. `update --dry-run`, `update --check`, `list` and `doctor` do not
rewrite lockfiles or generated output. Aliases, recorded selection, bundle
membership and unrelated item/plugin/MCP records remain intact. The label
canonicalization itself does not change the schema; the reinstall adopts a
`schema: 1` lockfile as described in §4 (State files).

The supported historical authority grammar and reconciliation limits are defined
in the
[ADR-0011 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12).
Unsupported or ambiguous labels fail before fetch with the recorded label and
guidance to reconcile a verified HTTPS URL, ref and subfolder. Modern `add`
still rejects the removed GitLab shorthands with exit 2; use a full HTTPS URL.

### 2.3 `remove`

```text
upskill remove [name...] [--source <label>] [--global|--project] [--force]
```

Removes installed items from the matching scope. Without a scope flag, `remove`
uses the project scope inside a git repository and the global scope outside one.
When the current directory is the home directory, the project scope is refused
with exit code 2 and a message that asks for `--global`. When a named item is
not in the lockfile of that scope but the other scope records it, `remove` fails
as for any unknown name and prints
`hint: '<name>' is installed in the <scope> scope (<root>); pass --<scope>` to
stderr, one line for each such name. A name that the current scope also holds
gets no such line. When no name qualifies, `remove` prints the `upskill doctor`
hint instead; `update <name>` does the same. Either name one or more items, or
pass `--source <label>` to remove every item that came from a single source.
Bare `upskill remove` is rejected — be explicit. Ancillary files (`CLAUDE.md`,
`opencode.json`, `.vscode/settings.json`) are not touched, except that removing
an item of a bundle, or the source of a bundle with `--source`, deletes the
installed opencode plugin entry of that bundle from the `plugin` and `plugins`
arrays of the `opencode.json` of the scope. In the global scope, removing a rule
that is installed for opencode also deletes its `instructions` entry from the
global opencode config.

`remove` deletes only files listed in the item's `files` or `pending` record
(§4). A recorded file is **owned** when it is a regular file with the recorded
SHA-256: `remove` deletes it. It is **modified** when its content differs,
compared as it is and again with every CRLF replaced by LF (§2.1), or it is no
longer a regular file: `remove` keeps it and prints `kept modified file: <path>`
to stderr; `--force` deletes a modified regular file, but never a path that is
not a regular file. A recorded regular file that cannot be read is handled as
modified. An absent recorded file is ignored. A file that the record does not
list is **unowned** and is never deleted. Empty item directories are then
deleted; client roots and kind directories are kept. Before any deletion,
`remove` checks the `files` and `pending` records of every item in the lockfile
of the scope, also of items that it does not remove. A recorded path outside the
layout that upskill writes for its item (the entry file or the resource
directory of the item, for any client and scope, including the paths of released
versions) makes `remove` fail with exit code 1; the error names the lockfile,
the list, the item and the path, and nothing is deleted. Absolute paths and
paths with a `..` component are outside that layout. A symbolic link in the item
part of a recorded path's parent directories makes `remove` fail before any
deletion. An item whose `files` and `pending` records are both empty has no file
deleted. Before any deletion and before the adoption, also with `--force`,
`remove` fails with exit code 1 when the source directory of another installed
item recorded from a local-path source contains a recorded file (`files` or
`pending`, also a modified file that `remove` keeps) of an item that it removes;
the error names both items and asks to remove the other item first, no file is
deleted and the lockfile is not changed. A git source is not fetched for this
check, only the recorded local sources of the items that stay installed are
read, such a source that does not exist is skipped, and such a source that
exists and cannot be read fails `remove` with exit code 1 and an error that
names its source label, before any deletion. With a `schema: 1` lockfile, the
recorded files are those that the adoption (§4) would record; the check runs
before the adoption and saves nothing. The lockfile entry of each removed item
is dropped. `remove` adopts a `schema: 1` lockfile and prints the adoption
notice, except when a name is not in the lockfile, which is an error, and when
the check above refuses it.

### 2.4 `list` / `doctor`

- `list` — show installed content from `.upskill-lock.json`, grouped by kind.
  Bundles are surfaced in a separate section. The `--available` view (items
  discoverable from configured sources) is deferred for a future release.
  Without a scope flag, `list` and `doctor` report both scopes: the project
  scope (the current directory, in or outside a git repository) first, then the
  global scope (`$HOME`). `--global` or `--project` limits the report to that
  scope. When the current directory is the home directory, only the global scope
  is reported, and `--project` is refused with exit code 2 and a message that
  asks for `--global`. When neither `HOME` nor `USERPROFILE` is set, only the
  project scope is reported and stderr gets
  `note: the global scope is not shown: HOME is not set`. With `--json`, `list`
  prints `{"scopes": [{"scope", "root", <the buckets>}]}` and `doctor` prints
  `{"scopes": [...], "in_both_scopes": [...], "support": [...]}`; both also when
  one scope is selected. `list` prints `excluded: <names>` at the end of the
  block of a scope that has excluded item names, also when the scope has no
  items, and the `excludes` key of the scope in `--json` (omitted when empty).
- `doctor` — verify on-disk state matches the lock file. Per scope, thirteen
  independent buckets: missing per-client outputs (reinstall fixes), modified
  recorded files (content differs from the recorded SHA-256, compared as in
  §2.1), recorded files that could not be inspected (`doctor` continues with the
  others), SSOT hash drift on `local:` sources (`update` fixes; for a recorded
  `.bundle.yaml` source, the items are looked up in the registry root that `add`
  resolves for the bundle file, see §2.6; `doctor` finds each item in the source
  the way `add` discovers items, in a child or grandchild directory with the
  entry's kind and recorded folder name; among several, the one with the
  recorded item name, else the entry is reported as missing in the source),
  lockfile entries with no recoverable source (`remove` fixes), installed
  plugins missing from the client (`update --yes` reinstalls them, also when no
  item changed, for a source without a kind restriction; a source under a kind
  restriction skips the plugin and MCP steps and this repair; an opencode plugin
  recorded with `config_write` is checked in the `opencode.json` of the scope
  only, and no opencode command is run for it; for another opencode plugin, an
  identifier in that file's `plugin` or `plugins` array means installed, and
  otherwise, with an opencode version below 2, no `opencode plugin` command is
  run and the plugin is not reported missing; for a plugin whose bundle's
  recorded source is the source of no recorded item, which `update` does not
  reach, and for a plugin of a source under a kind restriction, `doctor` names
  the `upskill add` command instead), skipped plugins (informational — CLI was
  absent, or opencode was below version 2, at install time; `update --yes` tries
  them again and records `installed` when the install succeeds; a plugin that
  needs a manual step, status `instructions`, is listed with them and keeps its
  instructions: `update` runs no install for it and prints its instructions
  again when it applies the source), orphaned dependencies (informational), MCP
  server reconciliation, and interrupted items (an item with a non-empty
  `pending` record: run the same command again to complete it), and, for the
  global scope only, files at the earlier global Copilot paths
  (`earlier_layout`; `upskill update --global` moves them) and rules recorded
  for opencode whose absolute path is missing from the `instructions` array of
  the global opencode config (`missing_rule_entries`; `upskill update --global`
  writes them again, also when the file has no `instructions` array; it cannot
  repair a file that is not strict JSON or whose `instructions` is not an
  array), and entries of the form `<home>/.agents/rules/<name>/RULE.md` in that
  array for a name that the lockfile does not record as an opencode rule
  (`stale_rule_entries`; for a rule that the lockfile records for other clients,
  `recorded` is `true` and `upskill update --global` and `upskill add --global`
  delete the entry; for a name that the lockfile does not record, `recorded` is
  `false` and the user deletes the entry by hand; entries in another form are
  not reported). `doctor` also prints a note for each item that the lockfiles of
  both scopes record; the note does not change the exit code. After the scope
  reports, `doctor` prints the support level of each client and kind
  (`Support levels:`; the compiled table in `src/support.rs`, stated in the user
  guide page "Support levels"); the section does not change the exit code and is
  not based on a detection of client versions (the plugin check runs
  `opencode --version` before any `opencode plugin list`, ADR-0008). Exit 0 when
  clean, 1 when any drift bucket is non-empty, including `earlier_layout`.
  Skipped plugins are warnings and do not affect the exit code. Missing,
  modified and unreadable files are checked against the `files` record; a
  recorded path that `remove` would never delete (outside the client roots,
  fewer than three components, or with a `..` component) is skipped without a
  report. For an item with an empty or missing `files` record (a `schema: 1`
  lockfile, or a `schema: 2` or `schema: 3` item with an empty list), `doctor`
  checks each per-client entry file instead.

### 2.5 `new` / `lint` / `fmt`

Author commands. Run inside a source-registry working tree.

- `new <kind> <name>` — scaffold a new SSOT item directory (`<kind>` is one of
  `rule`, `skill`, `agent`, `command`).
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
  error out before any write. The source of a bundle file is the registry root
  found upward from it: the first directory, starting at the directory of the
  bundle file, that holds an item in a child or grandchild directory, or the
  directory of the bundle file when no such directory exists. When bundle
  discovery from that root cannot reach the file (a directory between them
  starts with a dot), the source is the directory that holds its `bundles`
  directory, or the directory of the bundle file when that directory is not
  named `bundles`. A bundle that selects no item and only declares `requires:`
  is valid also when that source holds no item.
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

### 2.8 `pack`

Build plugin directories from a named bundle in the current source registry.
`--source` is required and is written unchanged into init; it does not select or
fetch the registry to pack. The entry bundle is resolved in the current
directory; a `requires` entry with a `source` is fetched from that source.
`--out` is required. With no client flag, both outputs are written; `--claude`
or `--copilot` selects one, and both flags select both. `--quiet` suppresses the
result lines.

```bash
upskill pack my-bundle --source owner/repo@v1 --out dist
```

| Content                                    | Claude Code output                     | GitHub Copilot CLI output              |
| ------------------------------------------ | -------------------------------------- | -------------------------------------- |
| Directory                                  | `<out>/<bundle>-claude/`               | `<out>/<bundle>-copilot/`              |
| Manifest                                   | `.claude-plugin/plugin.json`           | `plugin.json`                          |
| Skills and resources                       | `skills/<name>/SKILL.md` and resources | `skills/<name>/SKILL.md` and resources |
| Agents                                     | `agents/<name>.md`                     | `agents/<name>.agent.md`               |
| MCP servers, when declared                 | `.mcp.json`                            | `.mcp.json`                            |
| LSP servers, when declared                 | `.lsp.json`                            | `lsp.json`                             |
| Init, when rules target this client        | `commands/init.md`                     | `commands/init.md`                     |
| Notice hook, when rules target this client | `hooks/hooks.json`                     | `hooks.json`                           |

Skills and agents include the bundle's `requires` closure and use the existing
client generators. The manifest takes `name`, `description`, optional `license`
and optional `metadata.version` (as `version`) from the bundle. No README and no
LICENSE file is written, so `claude plugin validate` reports both as missing.

`pack` writes the MCP servers of the bundle closure under `mcpServers` in
`.mcp.json` at the root of each client directory. A local server contains
`command`, `args` and `env`; a remote server contains `type`, `url` and
`headers`. Empty `args`, `env` and `headers` are omitted. `requires-env` is not
written. An output without MCP servers has no `.mcp.json`. The client loads the
servers from the plugin. No trial has confirmed that a client removes them when
a plugin from a hosted marketplace is uninstalled. Equal server objects declared
under one name in several bundles produce one entry. If two bundles of the
closure declare one server name with different written content, `pack` fails
with exit code 1 before writing output. A difference only in `requires-env` does
not cause a conflict.

`pack` also writes the
[`lsps` map](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md#lsps-map)
of the bundle closure. Each entry requires a non-empty `command` and a non-empty
`extensions` map from file extensions (a dot followed by at least one character)
to non-empty language identifiers; `args` and `env` are optional. The bundle
parser checks these rules, so `add`, `update` and `lint` also refuse a bundle
with an invalid entry. Claude Code uses `.lsp.json` at the plugin root, with one
top-level key per server and `extensionToLanguage` for the extensions map.
Copilot CLI uses `lsp.json` at the plugin root, with the servers below
`lspServers` and `fileExtensions` for the same map. Both formats contain
`command`, `args` and `env`; empty `args` and `env` are omitted. An output
without LSP servers has no LSP file. Equal declarations of one name produce one
entry. Different `command`, `args`, `env` or `extensions` values fail with exit
code 1 before writing output:

```text
LSP server `<name>` is declared with different content by the bundles `<first>` and `<second>`
```

The plugin does not install the server binary. `upskill add` and
`upskill update` do not configure LSP servers or record them in the lockfile.

All selected output directories are checked before writing. A non-empty client
directory is refused with exit code 1 unless `--force` is set. `--force`
replaces that directory, including removal of files from an earlier run. A file
or symbolic link at the client directory path is refused even with `--force`. An
unknown bundle fails with exit code 1. Each client directory is first written in
full to a staging directory next to it, `.<bundle>-<client>.upskill-stage`. When
all are written, each existing client directory is renamed to
`.<bundle>-<client>.upskill-old`, its staging directory is renamed into place,
and `pack` then tries to remove the `.upskill-old` directories; one that cannot
be removed is left in place. A failure while writing or renaming tries to undo
the renames already made, so every client directory keeps its earlier content,
and tries to remove the staging directories. If an undo rename also fails, the
earlier content is at the `.upskill-old` path; the next `pack` run of that
bundle deletes it, so copy it away first. A file, directory or link left at a
staging or `.upskill-old` path by an earlier run is removed. An existing empty
client directory is replaced by a new one, also without `--force`.

`--source` must parse as an install source, must not start with a hyphen and
must contain only ASCII letters, digits and `/ @ : . _ - ~ +`. Whitespace and
shell metacharacters are refused with exit code 2. The bundle name must be
nonempty, must not start with a hyphen and may contain only ASCII letters,
digits and `- _ .`.

`pack` reads no lockfile and writes only below `--out`. It does not install or
register the plugin, write a marketplace manifest, or publish it. The bundle
schema adds the optional `lsps` map without changing its version. Copy the
outputs into a marketplace repository and install through the client. Plugin
output is experimental until a smoke test with a hosted git marketplace passes
in both clients (the local-marketplace smoke test of 2026-11-19 passed);
`doctor` gains no row.

No rule file is included. Init instructs the model to run
`upskill add <source> <bundle> --kind rule --project --claude`, or the same
command with `--copilot`, in the repository root and report the result. On
failure it reports the error and runs no substitute command. It lists declared
plugins for that client (with its `install_url` or manual `instructions_url`
when present, otherwise by name) from the closure for the user, without
installing them. An output with no rules for its client has neither init nor the
notice hook. The hook is copied unchanged from the corresponding upskill notice
plugin and runs `upskill update --notice`; it is silent when the upskill binary
is absent.

Known limits: init and the hook need the upskill binary on the host; installing
the same skills with both a plugin and `add` exposes duplicates; references by
short skill name are not verified to resolve to `<plugin>:<skill>`. When both
outputs are published together, check that Copilot does not also load the Claude
directory. Codex, OpenCode, VS Code, general hook authoring, third-party plugin
packaging, and duplicate detection are outside this command's scope.

## 3. Generation pipeline

SSOT → per-client output runs on the developer's machine, on every install or
update. No CI generation step. No committed `dist/` artifacts.

### 3.1 Per-client output paths

Paths relative to the root of the scope, in the project scope and in the global
scope, except where the global scope is stated.

| Item kind | Claude Code                             | GitHub Copilot                                                | opencode                                           |
| --------- | --------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- |
| Rule      | `.claude/rules/<name>.md` with `paths:` | `.github/instructions/<name>.instructions.md` with `applyTo:` | `.agents/rules/<name>/RULE.md` (canonical-store)   |
| Skill     | `.claude/skills/<name>/SKILL.md`        | `.github/skills/<name>/SKILL.md`                              | `.agents/skills/<name>/SKILL.md` (canonical-store) |
| Agent     | `.claude/agents/<name>.md`              | `.github/agents/<name>.agent.md`                              | `.opencode/agents/<name>.md`                       |

In the global scope, Copilot content is written to the personal directories of
Copilot CLI under `$HOME`:

| Item kind | GitHub Copilot, global scope                   |
| --------- | ---------------------------------------------- |
| Rule      | `.copilot/instructions/<name>.instructions.md` |
| Skill     | `.copilot/skills/<name>/SKILL.md`              |
| Agent     | `.copilot/agents/<name>.agent.md`              |

Sources:
[Copilot CLI configuration directory](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)
(`skills/` and `agents/` under `~/.copilot`) and
[custom instructions](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions)
(`$HOME/.copilot/instructions/**/*.instructions.md`). The Claude Code paths and
the opencode paths are the same in both scopes. upskill does not read
`COPILOT_HOME` or `CLAUDE_CONFIG_DIR`.

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

Three files outside the per-item path tree are managed idempotently. In the
global scope `CLAUDE.md` and `.vscode/settings.json` are written under `$HOME`
as in a project; the `.vscode/settings.json` entry points at
`.github/instructions`, which a global install no longer writes:

- **`CLAUDE.md`** at repo root — created once with `@AGENTS.md` content if
  absent. Never overwritten.
- **`.vscode/settings.json`** — `chat.instructionsFilesLocations` is set; other
  keys preserved.
- **`opencode.json`**: in the project scope, the glob `.agents/rules/**/RULE.md`
  is checked on rule-bearing installs. In the global scope upskill writes, for
  each rule installed for opencode, the absolute path
  `<home>/.agents/rules/<name>/RULE.md` to the `instructions` array of the
  global opencode config, and no glob, because opencode expands no `**` there
  (ADR-0008, issue #581). `remove` deletes the entry of a removed rule, `update`
  writes the entries of the rules it installs and deletes those of the rules
  that left the source, an install that records a rule for fewer clients deletes
  the entry of that rule, an `add` or `update` deletes a stale entry (an entry
  for a rule that the lockfile records for other clients than opencode, which
  `remove` keeps), and `doctor` reports a missing entry and a stale entry.
  `doctor` also reports an entry of the same form for a name that the lockfile
  does not record; upskill keeps that entry, and keeps every entry that it does
  not delete as described above. The rules step reads and parses the file
  whenever the run installs an opencode rule or deletes an entry, so a file that
  is not strict JSON fails such an `add` or `update` (`remove` ignores that
  error and leaves the entry), and `add` and `update` (not `remove`) read it
  also to find a stale entry when the lockfile records a rule for other clients
  than opencode (a file that cannot be read as JSON adds no deletion and no
  error there); the plugin and MCP steps read it when the run writes an entry of
  theirs. The plugin and MCP writers of both scopes use the opencode config file
  of the scope: the project root in the project scope, and
  `$XDG_CONFIG_HOME/opencode/` (when set and absolute) or `~/.config/opencode/`
  in the global scope. `doctor`, `update` and `remove` use the file of the scope
  whose lockfile they work on, not the scope that an entry of that lockfile
  records. A trial with opencode 2.1.4 confirmed that opencode reads
  `~/.config/opencode/opencode.json` and ignores `~/opencode.json`. The
  `XDG_CONFIG_HOME` location is used when set and absolute. In the trials of
  issue #581, opencode loaded the `instructions` of that location; no trial
  tested the plugin or MCP entries there. A `plugin_uri` plugin is written to
  the `plugins` array. upskill neither reads nor changes `$HOME/opencode.json`,
  which earlier versions wrote. In the project scope, the entry
  `".agents/rules/**/RULE.md"` is added to `instructions[]` when absent,
  including when a later install finds that it was removed. An existing entry
  leaves the file unchanged; unrelated entries and keys are preserved
  ([implementation](https://github.com/driftsys/upskill/blob/main/src/ancillary.rs),
  [call site](https://github.com/driftsys/upskill/blob/main/src/pipeline/prepared.rs)).
  In the project scope the installer relies on client glob expansion; this is a
  shipped write contract, not qualification of OpenCode 2.1.4 runtime behavior
  ([retained retry](https://github.com/driftsys/upskill/blob/main/docs/technotes/gate6-opencode-explicit-paths-retry.md)).

## 4. State files

Bundle entries in `.upskill-lock.json` have an optional `kinds` array, for
example `"kinds": ["rule"]`, recording the restriction from `add --kind`. Absent
or empty means unrestricted; an empty list is omitted on write. Recorded kinds
are sorted without duplicates. Bundle entries are sorted by name. This field
does not change the bundle source schema.

`.upskill-lock.json` records what was installed, from where, at what resolved
git ref, with what content hashes. Same schema in both locations. A new lockfile
is written with `schema: 3`; a `schema: 1` or `schema: 2` lockfile is still
read. A `schema: 2` lockfile has the same shape as `schema: 3` and is written
back as `schema: 3` without other change. The number changed with the top-level
`excludes` list: an upskill version that reads at most `schema: 2` refuses the
file instead of rewriting it without that list. Each item entry may carry
`files` and `pending` lists of records with `client`, `path` and `sha256`
fields. `add` and `update` fill `files` with every file they write for the item
and delete the recorded files that a reinstall no longer produces and that are
unchanged, unless another item installed by the same command writes that path; a
modified recorded file is kept and stays in `files`, unless `--force` deletes it
(§2.1, §2.2). `remove` deletes only recorded files (§2.3).

An `add` or `update` runs in this order. (1) It checks every destination and
decides which items are written; nothing is changed yet. (2) For each item that
is written and that writes at least one file or deletes at least one stale file,
it adds the planned files to `pending` (an item new to the lockfile is added
with an empty `files` list); the records already in `pending` stay. It saves the
lockfile when at least one item got a `pending` record. (3) For each item, it
first deletes the stale recorded files, then writes each file to a new file
`.upskill-stage-<16 hex digits>` in the destination directory, sets the mode of
the source resource on it (a generated file gets the default mode of a new
file), and renames it to the destination; on an error it deletes that stage file
and stops. An owned file that already holds the planned bytes, or a CRLF variant
of them as described in §2.1, is not written; when its executable bits differ
from the target executable bits (§2.2), they are set to the target, and the
other permission bits of the file are left as they are. (4) For each written
item it sets `files` to the planned files and the records of its kept stale
files, clears `pending` (older `pending` hashes of a planned path are dropped),
and saves the lockfile. If the command fails or is stopped between (2) and (4),
the lockfile keeps the `pending` records: a file that holds the bytes of a
`pending` or `files` record is owned, so running the same command again, without
`--force`, completes the operation, also after a second command that failed;
`doctor` reports the item as interrupted, and `remove` deletes the files of both
records. `update` completes an interrupted item also when its source did not
change: the block of its source has a `complete` line for it, and in a terminal
without `--yes` that source is asked about as for any other change (§2.2). A
path of a `pending` record that the command that completes the operation does
not plan is a stale recorded path: deleted when it matches its `pending` or
`files` hash, kept and still recorded when modified. Before the first write of a
run, `add` and `update` (also `update --dry-run`) check the `files` and
`pending` records of every item in the lockfile of the scope, also of items that
the run does not select. A recorded path outside the layout of its item (§2.3)
fails the command with exit code 1, and no file and no lockfile is written or
deleted. `doctor` does not apply this check. A stage file left by a process that
was killed is not deleted by upskill. Two upskill processes that change one
scope at the same time are not supported. `add`, `update` and `remove` (not
`update --dry-run`, not `doctor`, not a `remove` that fails on an unknown name,
not a `remove` that is refused because a removed file overlaps the source
directory of another installed item or because the recorded local source of an
item that stays installed exists and cannot be read, not an `add` or an `update`
that is refused for a source item that overlaps an output path of the run or
that contains a recorded file that the check of §2.1 or §2.2 compares, for a
conflict with another source, for two items with one installed name, or for an
installed name that is empty, `.` or `..` or that has a path separator, and not
an `update` whose first source resolves to no items while the lockfile records
items from it) adopt a `schema: 1` lockfile: they keep the old bytes as
`.upskill-lock.json.v1` (an existing `.upskill-lock.json.v1` is kept and not
replaced), record each existing file at the items' output paths with its current
hash, save `schema: 3`, and print the adopted paths to stderr.

Two possible locations:

| Scope   | Location                   | Committed? |
| ------- | -------------------------- | ---------- |
| Project | `<cwd>/.upskill-lock.json` | Yes        |
| Global  | `$HOME/.upskill-lock.json` | No         |

The lockfile plays the same role as `package-lock.json`: deterministic
regeneration on another developer's machine and in CI for project scope, and
across-repo continuity for global scope.

```json
{
  "schema": 3,
  "items": [
    {
      "kind": "skill",
      "name": "code-review",
      "source": "github:driftsys/skills",
      "ref": "v1.2.0",
      "commit": "<40 hex digits>",
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

The optional top-level `excludes` array holds the excluded item names of the
lockfile, so of one install scope: strings, sorted and without duplicates, with
no source key. The array is omitted when empty and does not change the schema
number. The rules for the set are in §2.2.

Each entry of `plugins` records one plugin for one client: `name`, `client`,
`identifier`, `scope`, `bundle` and `status` (`installed`, `skipped` or
`instructions`), and the optional `config_write`. `config_write` is `true` for
an opencode plugin installed by a write of `identifier` to the `plugins` array
of the `opencode.json` of the scope (a `plugin_uri` descriptor); it is omitted
for every other install form, and a record without it reads as `false`. For a
record with `config_write`, `doctor` and the repair of `update` check that file
only and run no opencode command (ADR-0008). `update` sets the field when it
applies or repairs a `plugin_uri` plugin whose record lacks it.

### 4.1 Pure legacy-state projection

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#41-pure-legacy-state-projection)
is retained as history.

### 4.2 Pure owned instruction planning

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#42-pure-owned-instruction-planning)
is retained as history.

### 4.3 Strict captured source manifests

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#43-strict-captured-source-manifests)
is retained as history.

### 4.4 Owned whole-file operation plans

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#44-owned-whole-file-operation-plans)
is retained as history.

### 4.5 Captured item compilation

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#45-captured-item-compilation)
is retained as history.

### 4.6 Mixed owned-artifact operation plans

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
is retained as history.

### 4.7 Qualified scope roots and advisory writer locking

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#47-qualified-scope-roots-and-advisory-writer-locking)
is retained as history.

### 4.8 Qualified destination observations

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#48-qualified-destination-observations)
is retained as history.

### 4.9 Qualified source tree observations

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#49-qualified-source-tree-observations)
is retained as history.

### 4.10 Explicit file permission selection

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#410-explicit-file-permission-selection)
is retained as history.

### 4.11 Ordered recovery checkpoint model

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#411-ordered-recovery-checkpoint-model)
is retained as history.

### 4.12 Qualified file execution and recovery

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery)
is retained as history.

### 4.13 Combined lifecycle state foundation

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#413-combined-lifecycle-state-foundation)
is retained as history.

### 4.14 Captured rule desired sets

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#414-captured-rule-desired-sets)
is retained as history.

### 4.15 Authoritative lifecycle reference reconciliation

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#415-authoritative-lifecycle-reference-reconciliation)
is retained as history.

### 4.16 Reference-driven shared-owner removal planning

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#416-reference-driven-shared-owner-removal-planning)
is retained as history.

### 4.17 Captured lifecycle selection preflight

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#417-captured-lifecycle-selection-preflight)
is retained as history.

### 4.18 Complete single-owner lifecycle upsert planning

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#418-complete-single-owner-lifecycle-upsert-planning)
is retained as history.

### 4.19 Per-owner recorded contract evidence

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0004-state-and-ownership.md#419-per-owner-recorded-contract-evidence)
is retained as history.

### 4.20 Explicit shared-output reconciliation

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#420-explicit-shared-output-reconciliation)
is retained as history.

### 4.21 Complete selected-set preflight reporting

This library behaviour was withdrawn by
[AD-0042](https://github.com/driftsys/upskill/blob/main/docs/adr/0042-resteer-customization-delivery.md);
the
[historical record](https://github.com/driftsys/upskill/blob/main/docs/topics/0005-lifecycle-planning.md#421-complete-selected-set-preflight-reporting)
is retained as history.

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

| Code  | Meaning                                                                                                                |
| ----- | ---------------------------------------------------------------------------------------------------------------------- |
| `0`   | Success.                                                                                                               |
| `1`   | General error (network, parse, I/O), or `add` / `update` skipped an item because of a modified or unowned file (§2.1). |
| `2`   | Usage error (invalid args or flags).                                                                                   |
| `130` | Interrupted (Ctrl+C / SIGINT).                                                                                         |

### 6.2 Conventions

- POSIX/GNU long flags (`--flag-name`) and short flags (`-f`).
- Stdout for data, stderr for progress and errors.
- Honors `NO_COLOR` and TTY detection (no color in pipes/CI).
- Global `-q` / `--quiet` suppresses informational stdout. Errors on stderr and
  exit codes are unaffected.
- Single static binary, no runtime dependencies.

### 6.3 Environment variables

| Variable         | Purpose                                                                                                                                                                |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `NO_COLOR`       | Disable colored output.                                                                                                                                                |
| `HTTPS_PROXY`    | HTTP proxy for network requests.                                                                                                                                       |
| `XDG_CACHE_HOME` | Override cache directory (an absolute path) for registry indexes and the notice cache file, which `update --notice` reads and writes and an applying `update` updates. |

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
