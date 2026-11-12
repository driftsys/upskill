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
lockfile entry is not changed. The other items are written. `add` then prints
one line per conflict to stderr, `skipped <kind> <name>: <path> was modified` or
`skipped <kind> <name>: <path> exists and is not managed by upskill`, followed
by `Use --force to replace.`, and exits with code 1. `--force` replaces these
files, deletes modified files that are no longer produced, and also replaces
items installed from another source; it then lists these files after
`Replaced or deleted <n> modified or unmanaged file(s) (--force)`, unless
`-q/--quiet` is given. An installed name or an `--as` alias that contains `/` or
`\` is refused before any change, with an error that names the item.

### 2.2 `update`

```text
upskill update [name...] [--global|--project] [--dry-run] [--force]
```

Always fetches the latest source before regenerating. Absorbs the role of a
separate `sync` command — there is no `--offline` flag in v1; offline use is
covered by passing local paths to `add` / `update`.

`update` covers two distinct behaviours:

- **Absorb client drift.** When Copilot or Claude Code change a path or
  frontmatter field, `update` regenerates outputs from the same SSOT.
- **Track upstream evolution.** `update` git-pulls source repos and regenerates
  items whose source hash changed.

`update` applies the conflict rules of `add` (§2.1). An item with a modified or
unowned destination is not written and keeps its lockfile entry. When the source
hash of the item equals its recorded hash, the item has an empty `pending`
record, every such destination is modified (none is unowned), applying the item
would write or delete no other file, the item has no recorded file that the new
version no longer produces (present or absent), and its lockfile entry would not
change in any field other than `files`, `pending` and `required_by` (for example
`clients`), nothing is withheld: `update` reports `kept modified file: <path>`
for each modified destination (in a terminal and with `--dry-run`, in the block
of its source; with `--yes` or without a terminal, on stderr), and this note
does not change the exit code. Otherwise the item is reported as skipped (in a
terminal and with `--dry-run`, as a `skip` line of its block; with `--yes` or
without a terminal, as
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

`update` handles the sources one at a time, sorted by source label. For each
source it fetches the source once, runs the checks of §2.1, and computes a
preview from the fetched content, the files on disk and the lockfile, just
before the source is applied. A preview **changes something** when applying the
source writes or deletes a file, changes the lockfile record of an item (any
field except `pending` and `required_by`), completes an interrupted item (a
non-empty `pending` record, §4), removes an item, or creates or changes a
hand-shake file (§3.4). A write of a file that already holds the planned bytes
is not a change. A `skip` item and a `kept modified file` note are not changes.

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
  `<n> modified file(s) kept` and `<n> file(s) changed`, or `applied`. After the
  last source it prints `Updated <n> of <m> item(s) from <a> of <s> source(s).`,
  followed by `<k> source(s) not applied.` when a source with changes was not
  applied, and by `Stopped; <r> source(s) not examined.` or `Stopped.` after `q`
  or end of input; then `Changed <n> file(s) written or deleted` when no item
  was updated, completed or removed but files were written or deleted. It prints
  `everything is up to date — nothing to do` instead when no source was applied,
  no source with changes was declined, no file was kept with a note, no item was
  skipped and no lockfile was adopted. The exit code is 1 when an item is
  skipped in an applied source or in a source whose preview changes nothing, or
  when a step fails; answers `n` and `q` do not change it, and a `skip` line of
  a source that was not applied does not cause exit code 1. With `--quiet`, the
  blocks, the notes, the result lines and the closing summary are not printed;
  the question is `Apply changes from <source>? [y/N/q]`, and the `skipped`
  lines of the skipped items that cause exit code 1, `Use --force to replace.`
  and the adoption notice are printed on stderr. Without `--quiet`, a terminal
  run prints no `skipped` line.
- **`--yes`, or stdin is not a terminal.** Every source is applied without a
  question, also a source whose preview changes nothing, so its plugin and MCP
  steps run. `update` prints `Updated <n> of <m> item(s)` and one status line
  per item when an item changed or was skipped, files were written or deleted,
  or a lockfile was adopted (a run whose only output is `kept modified file`
  notes prints nothing on stdout), and reports a completed interrupted item as
  `completed — interrupted before`. When no item was updated, completed or
  removed but files were written or deleted, it prints
  `Changed <n> file(s) written or deleted`. It prints
  `everything is up to date — nothing to do` when nothing changed, no item was
  skipped, no file was kept with a note and no lockfile was adopted.
- **`--dry-run`.** `update` prints the same blocks, asks nothing, writes nothing
  and does not adopt a `schema: 1` lockfile (the previews use an adoption held
  in memory). It then prints `Dry-run: would update <n> of <m> item(s)` and
  `Each block is computed for the current state; a later block does not include the effect of an earlier one.`,
  and exits with code 0 when no step fails. `update --dry-run --quiet` prints
  nothing. Because nothing is applied, a later block does not include the effect
  of applying an earlier source.

A `schema: 1` lockfile is adopted once per run, without a question, for the
first source that passes its checks (§4); in a terminal the adoption notice is
printed before the block of that source. The adoption stays when the run fails
or is stopped.

Known limits: the plugin and MCP steps of a bundle source run when the source is
applied, are not part of the preview and are not a reason to ask, so a terminal
run does not repair a missing plugin or MCP registration of a source whose
preview changes nothing (`doctor` reports it, `update --yes` repairs it). A
change of only `required_by` in an item record is not shown and does not cause a
question; it is saved when the source is applied for another reason, or with
`--yes`.

Persisted schema-1 `gitlab:` and `gitlab+<host>:` source labels remain readable
for updates. Unambiguous historical labels decode to generic HTTPS sources,
preserving namespace, ref and subfolder. Applying `update` canonicalizes the
source labels of reinstalled items and bundles, even when their content is
unchanged. `update --dry-run`, `list` and `doctor` do not rewrite lockfiles or
generated output. Aliases, recorded selection, bundle membership and unrelated
item/plugin/MCP records remain intact. The label canonicalization itself does
not change the schema; the reinstall adopts a `schema: 1` lockfile as described
in §4 (State files).

The supported historical authority grammar and reconciliation limits are defined
in the
[ADR-0011 amendment](https://github.com/driftsys/upskill/blob/main/docs/adr/0011-generic-git-url-source.md#amendment-persisted-gitlab-source-labels-2026-10-12).
Unsupported or ambiguous labels fail before fetch with the recorded label and
guidance to reconcile a verified HTTPS URL, ref and subfolder. Modern `add`
still rejects the removed GitLab shorthands with exit 2; use a full HTTPS URL.

### 2.3 `remove`

```text
upskill remove [name...] [--source <label>] [--global] [--force]
```

Removes installed items from the matching scope. Either name one or more items,
or pass `--source <label>` to remove every item that came from a single source.
Bare `upskill remove` is rejected — be explicit. Ancillary files (`CLAUDE.md`,
`opencode.json`, `.vscode/settings.json`) are not touched.

`remove` deletes only files listed in the item's `files` or `pending` record
(§4). A recorded file is **owned** when it is a regular file with the recorded
SHA-256: `remove` deletes it. It is **modified** when its content differs,
compared as it is and again with every CRLF replaced by LF (§2.1), or it is no
longer a regular file: `remove` keeps it and prints `kept modified file: <path>`
to stderr; `--force` deletes a modified regular file, but never a path that is
not a regular file. A recorded regular file that cannot be read is handled as
modified. An absent recorded file is ignored. A file that the record does not
list is **unowned** and is never deleted. Empty item directories are then
deleted; client roots and kind directories are kept. A recorded path outside the
client roots (`.claude`, `.github`, `.agents`, `.opencode`) is never deleted,
and a symbolic link in the item part of a recorded path's parent directories
makes `remove` fail before any deletion. An item whose `files` and `pending`
records are both empty has no file deleted. The lockfile entry of each removed
item is dropped. `remove` adopts a `schema: 1` lockfile and prints the adoption
notice, except when a name is not in the lockfile, which is an error.

### 2.4 `list` / `doctor`

- `list` — show installed content from `.upskill-lock.json`, grouped by kind.
  Bundles are surfaced in a separate section. The `--available` view (items
  discoverable from configured sources) is deferred for a future release.
- `doctor` — verify on-disk state matches the lock file. Ten independent
  buckets: missing per-client outputs (reinstall fixes), modified recorded files
  (content differs from the recorded SHA-256, compared as in §2.1), recorded
  files that could not be inspected (`doctor` continues with the others), SSOT
  hash drift on `local:` sources (`update` fixes), lockfile entries with no
  recoverable source (`remove` fixes), installed plugins missing from the client
  (`update --yes` reinstalls: a run in a terminal applies a source only when its
  preview changes something, and the plugin and MCP steps are not part of the
  preview), skipped plugins (informational — CLI was absent at install time),
  orphaned dependencies (informational), MCP server reconciliation, and
  interrupted items (an item with a non-empty `pending` record: run the same
  command again to complete it). Exit 0 when clean, 1 when any drift bucket is
  non-empty. Skipped plugins are warnings and do not affect the exit code.
  Missing, modified and unreadable files are checked against the `files` record;
  a recorded path that `remove` would never delete (outside the client roots,
  fewer than three components, or with a `..` component) is skipped without a
  report. For an item with an empty or missing `files` record (a `schema: 1`
  lockfile, or a `schema: 2` item with an empty list), `doctor` checks each
  per-client entry file instead.

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
git ref, with what content hashes. Same schema in both locations. A new lockfile
is written with `schema: 2`; a `schema: 1` lockfile is still read. Each item
entry may carry `files` and `pending` lists of records with `client`, `path` and
`sha256` fields. `add` and `update` fill `files` with every file they write for
the item and delete the recorded files that a reinstall no longer produces and
that are unchanged, unless another item installed by the same command writes
that path; a modified recorded file is kept and stays in `files`, unless
`--force` deletes it (§2.1, §2.2). `remove` deletes only recorded files (§2.3).

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
of them as described in §2.1, is not written; only the mode of a resource is set
on it again. (4) For each written item it sets `files` to the planned files and
the records of its kept stale files, clears `pending` (older `pending` hashes of
a planned path are dropped), and saves the lockfile. If the command fails or is
stopped between (2) and (4), the lockfile keeps the `pending` records: a file
that holds the bytes of a `pending` or `files` record is owned, so running the
same command again, without `--force`, completes the operation, also after a
second command that failed; `doctor` reports the item as interrupted, and
`remove` deletes the files of both records. `update` completes an interrupted
item also when its source did not change: the block of its source has a
`complete` line for it, and in a terminal without `--yes` that source is asked
about as for any other change (§2.2). A path of a `pending` record that the
command that completes the operation does not plan is a stale recorded path:
deleted when it matches its `pending` or `files` hash, kept and still recorded
when modified. A stage file left by a process that was killed is not deleted by
upskill. Two upskill processes that change one scope at the same time are not
supported. `add`, `update` and `remove` (not `update --dry-run`, not `doctor`,
not a `remove` that fails on an unknown name, not an `add`, or an `update` for
its first source, that is refused for a source item that overlaps its own output
paths, for a conflict with another source, for two items with one installed
name, or for an installed name with a path separator, and not an `update` whose
first source resolves to no items while the lockfile records items from it)
adopt a `schema: 1` lockfile: they keep the old bytes as `.upskill-lock.json.v1`
(an existing `.upskill-lock.json.v1` is kept and not replaced), record each
existing file at the items' output paths with its current hash, save
`schema: 2`, and print the adopted paths to stderr.

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
  "schema": 2,
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
