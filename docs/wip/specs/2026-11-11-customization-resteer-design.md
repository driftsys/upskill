# Customization MVP re-steer: reliable install, update and removal

Date: 2026-11-11. Status: **approved by the user on 2026-11-11; not
implemented**. Code baseline: `origin/main` at `8cc8f04`. This document replaces
the delivery approach of the customization MVP. It does not describe released
behaviour.

Related records:
[MVP epic #283](https://github.com/driftsys/upskill/issues/283),
[roadmap](../../roadmap.md),
[ADR-0013](../../adr/0013-agent-customization-targets.md),
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md),
[AD-0041](../../adr/0041-separate-client-representations-as-design-baseline.md).

## Why this re-steer exists

Between 2026-10-14 and 2026-11-11 the work on the customization MVP added about
11,000 lines under `src/` and about 19,000 lines under `tests/`. This change
removes about 16,600 lines under `src/` and about 28,500 lines under `tests/`;
part of the removed code predates 2026-10-14. `src/main.rs`, `src/cli.rs` and
`src/pipeline/` did not change in that period. No user-visible behaviour
changed. The new code is not reachable from any command.

The approach required every client to pass every native trial before any new
behaviour could be activated. Client versions changed during the trials, which
made earlier results out of date. One client (OpenCode) never produced a
successful positive control and blocked the other three.

The user decided on 2026-11-11 that this approach was a mistake and asked for a
new design and development plan.

## What the user needs

These statements come from the user on 2026-11-11.

1. Support Claude Code and Copilot first. Codex and OpenCode come next.
2. Support these customization kinds: rules and instructions, skills, subagents,
   plugins, bundles.
3. An update or removal must fully clean what an earlier install wrote. Today it
   sometimes cannot.
4. An install or update that stops partway must not leave a broken state. Today
   it sometimes does.
5. It must be clear whether content is installed in the local project or
   globally on the machine. Today this causes confusion, and content is then not
   taken into account by the client.
6. When the source of an installed skill or bundle publishes a new version,
   upskill installs the new version. This happens through a command. When a
   client session starts, upskill reports that an update is available and asks.
   The report must not repeat needlessly.
7. Commands, hooks, tool mapping and permission translation are not part of this
   MVP.
8. Code that this design does not use can be parked on a protected branch that
   receives no further work.

## Current behaviour that causes the problems

Verified on `origin/main`.

| Problem                            | Cause in the code                                                                                                                                                                                                                     |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cannot clean a previous install    | The lockfile records client names only, not the files written (`src/lockfile.rs`, `LockedItem`). `remove` recomputes paths from the current layout (`src/pipeline/lifecycle.rs`). A file written under another layout is never found. |
| Removal deletes too much           | `remove_item_outputs` calls `remove_dir_all` on the item directory (`src/pipeline/output.rs`). Manual files in that directory are deleted.                                                                                            |
| Update overwrites manual edits     | `update` reinstalls with `force: true` and does not compare the installed file with what upskill wrote.                                                                                                                               |
| Broken state after an interruption | Old outputs are deleted first, new outputs are written directly with `fs::write`, and the lockfile is saved last (`src/pipeline/install.rs`, `src/pipeline/mod.rs`).                                                                  |
| Scope confusion                    | Without a flag, the scope is the current directory inside a git repository and `$HOME` otherwise (`install_target` in `src/main.rs`). Only `add` prints the scope. `list`, `doctor`, `update` and `remove` read one scope only.       |
| Global content is not loaded       | The output paths do not depend on the scope (`src/legacy/layout.rs`). A global install writes project-shaped paths under `$HOME`, for example `$HOME/.github/skills/`, which Copilot does not read as a global location.              |
| No update report                   | `update --dry-run` exists but fetches every source. No fast check exists, and nothing runs at session start.                                                                                                                          |

## Goals

- G1. `remove` and `update` delete exactly the files that upskill wrote and that
  are unchanged since upskill wrote them.
- G2. upskill never overwrites or deletes a file that the user changed, or a
  file that upskill did not write, unless the user passes `--force`.
- G3. After an interruption at any point, running the same command again
  completes the operation without `--force`.
- G4. Every command that reads or changes installed content states the scope it
  used. `list` and `doctor` report both scopes.
- G5. `upskill update --check` reports available updates without changing any
  file, and completes within a bounded time.
- G6. A session-start notice reports available updates at most once for each
  distinct set of available updates.
- G7. Each client and kind has a published support level, backed by a test that
  a maintainer can run again against the real client.
- G8. Every merged pull request changes behaviour that is reachable from a
  command, or tests such behaviour.

## Non-goals

- Commands, hooks as a managed content kind, tool mapping, permission
  translation, and new MCP management. Existing MCP behaviour stays unchanged.
- Translating a subagent's tool and permission fields for another client. The
  MVP writes them as authored.
- Updating the upskill binary itself.
- Applying updates without the user's action.
- Codex and OpenCode changes. The live pipeline has no Codex generation; Codex
  stays unsupported in this MVP. OpenCode generation stays unchanged, receives
  the same ownership behaviour through the shared pipeline, and is published as
  experimental.
- Changes to the authoring format. An item stays a directory with a `RULE.md`,
  `SKILL.md` or `AGENT.md` entry file, and a bundle stays a `.bundle.yaml` file.
- Managing the internal files of a client's native plugin system. Plugins stay
  as they are today: upskill calls the client's own plugin command and records
  the result.
- Generating native plugin directories and catalog files from a bundle. The user
  decided on 2026-11-11 that this comes after the MVP release.
- Recovery from power loss or filesystem corruption.

## Results that would make this a failure

- A check that one client fails prevents a release for another client.
- Code is merged to `main` that no command can reach.
- Evidence records become a deliverable with more volume than the code they
  support.
- A session start changes files or prints the same notice in every session.
- An update silently replaces a file the user edited.

## Constraints

- Single static binary, no async runtime, no `git2`. The `rustix` dependency is
  used only by `src/filesystem/`; parking that module removes the dependency.
- Pre-1.0: no compatibility layer is required for the lockfile format, except
  the one adoption step described below, which exists because requirement 3 is
  about earlier installs.
- `main.rs` performs I/O orchestration only. Only `main.rs` writes to stdout and
  stderr.
- Acceptance tests come first and go through the CLI with `common::upskill_cmd`.

## Design

### 1. Ownership record in the lockfile

Each `LockedItem` gains the list of files written for it. Each entry holds the
client, the path relative to the scope root, and the SHA-256 of the bytes
written. Supporting resource files are entries too. The lockfile `schema`
becomes `2`.

Adoption of a schema-1 lockfile: on the first command that changes content,
upskill computes each item's paths with the existing layout function
(`legacy::layout`), records every file that exists there with its current hash,
and saves schema 2. This is a one-time step. The lockfile from before the step
is kept as `.upskill-lock.json.v1` beside the new one.

A file is in one of three states for upskill:

| State    | Definition                                             | `update` and `remove` behaviour                             |
| -------- | ------------------------------------------------------ | ----------------------------------------------------------- |
| Owned    | Recorded, and the current hash equals the recorded one | May replace or delete                                       |
| Modified | Recorded, and the current hash differs                 | Keep the file, skip the item, report it; `--force` replaces |
| Unowned  | Present at a destination path, and not recorded        | Refuse the item, report the conflict; `--force` replaces    |

`remove` deletes owned files one by one. It then removes directories that are
empty. It never calls `remove_dir_all`. A modified file stays on disk, and its
record is dropped with a message that names the file.

Files from earlier layouts: the builder lists the output paths that released
versions of upskill used. `doctor` reports files found at those paths that no
record covers. upskill has no hash for such a file, so it deletes one only when
the user confirms or passes `--force`.

Shared files that upskill edits in place (`CLAUDE.md`, `.vscode/settings.json`,
`opencode.json`, handled by `src/ancillary.rs`) are outside the ownership
record. Their behaviour stays as it is today. Their edits must stay safe to
repeat.

`doctor` reports modified files, missing files, and items installed in both
scopes.

### 2. Operations that can be repeated

One install or update of an item follows this order:

1. Compute all new file contents in memory. Stop here on any generation error.
2. Check every destination against the table above. Stop here on a conflict.
3. Save the lockfile with the item's intended files and hashes recorded as
   pending, next to the current record. The lockfile save is already atomic.
4. Write each file to a temporary file, then rename it to its destination.
5. Delete owned files that the new version no longer contains.
6. Save the lockfile with the pending record promoted to current.

When a command starts and finds a pending record, a file whose hash equals
either the current or the pending hash counts as owned. The command then
continues from step 4. This is what makes a repeated run complete without
`--force`.

A temporary file must not be visible to a client. It is either placed in a
directory that no client scans, or named `.upskill-stage-<random>` with no
extension. The retained trials of 2026-10-18 showed that Claude, Copilot and
Codex do not load that name from their default locations.

This design has no journal, no checkpoints and no writer lock. Two upskill
processes that change the same scope at the same time are not supported. The
builder may add a simple lock file if a test shows a real problem.

### 3. Scope

The two scopes are named **project** and **global** in all output and
documentation.

- `add`, `update` and `remove` print one line that states the scope and its root
  directory.
- Outside a git repository and without a flag, `add` stops and asks for
  `--project` or `--global`. It no longer chooses global silently. This is the
  author's decision; see "Decisions for the user to check".
- `list` and `doctor` show both scopes, with the scope of each item.
- Global output paths become the locations that each client documents for
  content of the whole machine, for example `~/.claude/skills/` and
  `~/.copilot/skills/`. The builder confirms each path against the client's
  documentation, and the smoke test of section 6 verifies it. Files at the
  earlier global paths are handled as files from an earlier layout (section 1).
- `update` and `remove` act on one scope. When the named item exists only in the
  other scope, the message says so.

### 4. Update check and session notice

The lockfile records the resolved commit for each git source at install time.

`upskill update --check`:

- changes no file;
- for each distinct git source, compares the recorded commit with the result of
  `git ls-remote` for the recorded ref;
- skips a source pinned to a full commit hash;
- for a source pinned to a tag, reports an update only when that tag now
  resolves to a different commit, and does not look for newer tags;
- compares the source hash for a local path source;
- applies a total time limit, and reports a source it could not reach without
  failing;
- lists the items that have an update, for both scopes.

`upskill update` applies updates for one scope, using the rules of sections 1
and 2.

Session notice: a mode of the check that is built for a session-start hook.

- It prints one line when updates exist, for example
  `upskill: 3 updates available (project). Run "upskill update".`
- It prints nothing when no update exists, when the check fails, or when the
  network is unavailable.
- It always exits with code 0.
- It stores the last reported set of updates in the cache directory. It prints
  again only when that set changes.
- It contacts the network at most once in a fixed period (author's default: 24
  hours) and uses the cached result otherwise.

The hook is delivered as a small native plugin for Claude Code and one for
Copilot CLI, published through `driftsys/metapowers`. The user installs the
plugin with the client's own command. upskill does not install or own this
plugin. The plugin contains only the hook that calls the notice mode. How each
client shows the line to the user, and whether the client's agent then asks the
user, must be confirmed by a trial per client before the plugin is published.

### 5. Layout for Claude and Copilot

The MVP keeps the current output paths:

| Kind     | Claude Code                      | Copilot                                       |
| -------- | -------------------------------- | --------------------------------------------- |
| Skill    | `.claude/skills/<name>/SKILL.md` | `.github/skills/<name>/SKILL.md`              |
| Rule     | `.claude/rules/<name>.md`        | `.github/instructions/<name>.instructions.md` |
| Subagent | `.claude/agents/<name>.md`       | `.github/agents/<name>.agent.md`              |

Copilot also reads some Claude locations. A trial on 2026-10-25 showed that
Copilot CLI 1.0.103 reads an unscoped `.claude/rules` file and Claude-format
agents. GitHub documents that Copilot reads `.claude/skills`. An item installed
for both clients may therefore be visible to Copilot twice.

Trial T1 runs before any layout change: install one skill, one rule and one
subagent for both clients, and record whether Copilot CLI lists or applies each
one once or twice.

- If Copilot handles the duplicate without a visible effect, the layout stays
  unchanged.
- If Copilot shows or applies an item twice, upskill stops writing the Copilot
  copy for that kind when Claude is also selected, and records Copilot as a
  reader of the Claude file. This changes AD-0041 for that kind and needs the
  user's approval at that time.

Result (2026-11-11): trial T1 ran with Copilot CLI 1.0.109. Copilot listed and
applied the rule twice, listed the skill once, and gave no result for the
subagent. The user decided to keep both copies of a rule, so that one client can
be removed without an effect on the other. The second outcome above is therefore
not applied. AD-0042 Decision 7 records this.

### 6. Support levels in place of activation gates

Each client and kind has one level, published in the user guide and printed by
`doctor`:

| Level        | Meaning                                                                  |
| ------------ | ------------------------------------------------------------------------ |
| Supported    | The smoke test passed on the recorded client version                     |
| Experimental | Implemented; the smoke test is missing, failing or older than the client |
| Unsupported  | Not implemented, or known not to work; the command refuses with a reason |

The smoke test extends `scripts/qualify-native-instructions.py`. For each cell
it installs a fixture with a random marker through the real `upskill` binary,
starts a new client session, and checks that the client returns the marker. It
covers skills, rules and subagents, for Claude Code and Copilot CLI, at project
and global scope. A maintainer runs it with one `just` recipe. It needs
authenticated clients and makes paid model calls, so hosted CI does not run it.

The levels are a table compiled into the binary. A maintainer updates the table
after a smoke run. `doctor` does not detect client versions. Initial levels:
Claude Code and Copilot CLI are experimental until their cells pass, OpenCode is
experimental, and Codex is unsupported.

A failed or outdated cell changes that cell's level. It does not block a
release, and it does not affect another cell.

The properties in sections 1 to 4 belong to upskill's own code. Ordinary Rust
tests verify them in hosted CI, and they are required for every release.

### 7. Parking unused code

Before any feature work:

1. Create `archive/lifecycle-2026-11` at the current `origin/main` commit.
2. Push `feat/native-delivery`, `feat/native-config-contributions`,
   `docs/q7-codex-agent-registration` and `docs/q8-codex-skill-updates` as
   `archive/<name>` branches.
3. Protect `archive/**` with a ruleset that rejects pushes and deletion.
4. In one pull request, remove from `main`:
   - the modules with no caller from `main.rs`: `lifecycle`, `filesystem`,
     `operations`, `files`, `compile`, `instructions`, `source_manifest`,
     `ownership`;
   - their tests;
   - the `rustix` dependency. `legacy::layout`, `conflict`, `target`, `select`
     and `index` stay, because live code uses them.
5. In the same pull request, make the documentation describe the code that
   remains. Decision records and design notes for the removed code are marked as
   withdrawn and point to the archive branch. The trial results stay in
   `docs/technotes/` as research. Working documents for the removed approach
   move to `docs/archive/`.
6. Close the open debt issues that concern only removed code, with a link to the
   archive branch.

The Codex instruction work (managed contributions inside a team-owned
`AGENTS.md`) is the part of the archive most likely to be needed again. It is
reviewed when Codex rules are designed, and is not assumed to return.

## Delivery order

Each step is one pull request, is releasable, and starts with a failing CLI
acceptance test.

| Step | Content                                                                            |
| ---- | ---------------------------------------------------------------------------------- |
| 0    | Park unused code (section 7). Record this re-steer as one decision record.         |
| 1    | Fix [#465](https://github.com/driftsys/upskill/issues/465). Run trial T1.          |
| 2    | Ownership record, schema 2, adoption step; `remove` by recorded files (section 1). |
| 3    | `update` with modified and unowned file protection (section 1).                    |
| 4    | Repeatable operations with pending records and interruption tests (section 2).     |
| 5    | Scope reporting, both-scope `list` and `doctor`, global output paths (section 3).  |
| 6    | Recorded commits and `update --check` (section 4).                                 |
| 7    | Smoke test and support levels for Claude Code and Copilot CLI (section 6).         |
| 8    | Notice mode; Claude Code plugin; Copilot CLI plugin (section 4). Release.          |
| Next | Codex, then OpenCode, each with its own short design.                              |
| Next | Plugin and catalog generation from a bundle, with its own design.                  |

Bundles and plugins need no separate step. Bundle items are ordinary items and
receive the ownership behaviour. `update --check` covers the items of a bundle.
Plugin behaviour is unchanged.

## Decisions for the user to check

These are the author's decisions, not the user's statements.

1. Outside a git repository, `add` without a scope flag stops and asks for one.
2. A modified file causes the item to be skipped, not merged.
3. The session notice uses a 24-hour limit for network checks.
4. The updater plugin is installed by the user with the client's own command.
5. The current Copilot output paths stay. Trial T1 showed a duplicate for a
   rule, and the user decided on 2026-11-11 to keep both copies (section 5).
6. OpenCode receives the shared ownership behaviour and is published as
   experimental. Codex stays unsupported.
7. Simultaneous upskill processes on one scope are not supported.
8. Files from earlier layouts are reported by `doctor` and deleted only on
   confirmation.
9. Support levels are a fixed table in the binary, not detected at run time.
10. The repository administration actions of section 7 (archive branches,
    ruleset, closing issues) are performed after approval of this design, with
    the user's confirmation at that time.
11. A source pinned to a tag stays on that tag. `update --check` does not report
    newer tags.

## Alternatives considered

- **Continue the gated lifecycle and activate schema 2 for all clients.**
  Rejected: four weeks without a reachable change, results that go out of date
  with each client release, and one client blocking all others.
- **Activate the existing lifecycle library for Claude and Copilot only.**
  Rejected: the library models crash checkpoints, writer locks and qualified
  roots that the user's problems do not need, and its instruction model targets
  shared `AGENTS.md` files that Claude and Copilot do not need.
- **Drive each client's native plugin system for all content.** Rejected: the
  retained trials showed that Claude's plugin install rewrites user settings,
  and that Copilot activated a Claude registration. upskill cannot make another
  program's command safe to repeat.
- **One shared `.agents/skills` copy for all clients.** Deferred: Claude Code
  reading that directory is not verified, and it is not needed for Claude and
  Copilot.
- **Apply updates automatically at session start.** Rejected by the user: the
  session start reports and asks.

## Left to the builder

- Exact lockfile field names, and the format of the pending record.
- Location and naming of temporary files, within the constraint of section 2.
- Flag names for the notice mode, the time limit of the check, and the cache
  file format.
- Module boundaries inside `src/pipeline/`.
- Fixture design for the smoke test.
- The decision record number, and which existing records are marked withdrawn.
