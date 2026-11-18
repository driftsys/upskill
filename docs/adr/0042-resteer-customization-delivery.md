# AD-0042 — Re-steer customization delivery to reliable install, update and removal

Status: **accepted**. Date: 2026-11-11. Amended on 2026-11-15 to state the
delivered behaviour and to add Decision 8.
[MVP epic #283](https://github.com/driftsys/upskill/issues/283). The archived
[re-steer design](https://github.com/driftsys/upskill/blob/main/docs/archive/specs/2026-11-11-customization-resteer-design.md)
holds the original design and delivery order. This record states the decisions
as delivered on `main`: delivery steps 0 to 8 are merged. The user guide
([commands](https://github.com/driftsys/upskill/blob/main/docs/book/commands.md),
[support levels](https://github.com/driftsys/upskill/blob/main/docs/book/support-levels.md))
and the
[specification](https://github.com/driftsys/upskill/blob/main/docs/specification/specification.md)
describe the behaviour in detail. No release holds this work yet; the last
release is v0.8.1.

## Context

Between 2026-10-14 and 2026-11-11 the work on the customization MVP added about
11,000 lines under `src/` and about 19,000 lines under `tests/`. This change
removes about 16,600 lines under `src/` and about 28,500 lines under `tests/`;
part of the removed code predates 2026-10-14. `src/main.rs`, `src/cli.rs` and
`src/pipeline/` did not change in that period. No user-visible behaviour
changed. The new code was not reachable from any command.

That approach required every client to pass every native trial before any new
behaviour could be activated. Client versions changed during the trials, which
made earlier results out of date. One client (OpenCode) never produced a
successful positive control and blocked the other three.

The user decided on 2026-11-11 that this approach was a mistake and asked for a
new design and development plan. The user's requirements are: support Claude
Code and Copilot first; an update or removal must fully clean what an earlier
install wrote; an interrupted install or update must not leave a broken state;
the scope (project or global) must be clear; available updates must be reported
and installed through a command.

## Decision

1. **Ownership record in the lockfile.** Each installed item records the files
   written for it, with the client, the path relative to the scope root and the
   SHA-256 of the bytes written, in the `files` list of the item. The lockfile
   `schema` is `2`; `schema: 1` is still read. `update` and `remove` replace or
   delete only files whose current hash equals the recorded hash (or whose hash
   with every CRLF replaced by LF equals it), unless the user passes `--force`.
   A modified recorded file is kept. A file at a destination path that no record
   covers is never overwritten without `--force`. When an item cannot be written
   because a destination file is modified or unowned, `update` skips the item
   and reports the files. When the source of an item did not change and the user
   edited one of its files, `update` keeps that file and reports it. `remove`
   deletes the recorded files one by one and then the item directories that are
   empty; it never calls `remove_dir_all`. The first `add`, `update` or `remove`
   that changes a `schema: 1` lockfile adopts it: it records each existing file
   at the output paths of the items with its current hash, keeps the old bytes
   as `.upskill-lock.json.v1`, and saves `schema: 2`. Delivered differently from
   the design: the only earlier layout that upskill knows is the global Copilot
   layout (`.github/skills`, `.github/instructions` and `.github/agents` under
   `$HOME`). `doctor` reports files at those paths, and `update --global` moves
   them and deletes an earlier file only when it is unchanged. There is no
   confirmation prompt for files from earlier layouts.
2. **Repeatable operations.** An install or update records its intended files as
   pending in the `pending` list of the item in the lockfile before it writes,
   writes each file through a stage file `.upskill-stage-<16 hex
   digits>` in
   the destination directory and a rename, and then promotes the pending record
   to `files`. A file that holds the bytes of a `files` or `pending` record is
   owned, so running the same command again after an interruption completes the
   operation without `--force`. `doctor` reports an item with a pending record
   as interrupted. There is no journal, no checkpoint model and no writer lock.
   A stage file that a killed process left is not deleted.
3. **Support levels in place of activation gates.** Each client and kind has one
   published level: supported, experimental or unsupported. The levels are a
   table compiled into the binary (`src/support.rs`); `doctor` prints it and
   does not detect client versions. A maintainer runs the smoke test
   (`scripts/qualify-native-instructions.py`, listed by `just smoke`) against
   the real client, which needs authentication and makes paid model calls, and
   then updates the table. A failed or outdated cell changes only its own level.
   It does not block a release or another cell. As built on 2026-11-15, the
   Claude Code and Copilot CLI cells for rules, skills and agents are
   `supported`, with the client version and the date of the smoke run recorded
   in the table.
4. **Claude Code and Copilot first.** Codex and OpenCode follow, each with its
   own short design. OpenCode keeps its current generation, receives the shared
   ownership behaviour and is published as experimental. Codex stays unsupported
   in the MVP; it has no client flag.
5. **Unused code is parked.** The library modules that no command could reach
   (`lifecycle`, `filesystem`, `operations`, `files`, `compile`, `instructions`,
   `source_manifest`, `ownership`), their tests, the legacy projection and the
   direct `rustix` dependency were removed from `main`. `rustix` stays in
   `Cargo.lock` as a dependency of `tempfile`. The removed modules, tests and
   legacy projection are kept on the branch `archive/lifecycle-2026-11`, which
   receives no further work.
6. **Delivery order by capability.** The work follows the delivery order of the
   re-steer design: each step delivers one capability (ownership record,
   `update`, repeatable operations, scope, update check, support levels) for all
   content kinds at the same time. There is no order between skills and rules.
   The update check stays after the scope step, and the updater plugins stay
   last. All eight steps are merged; the release named in step 8 is a later,
   separate step.
7. **Both copies of a rule for Claude Code and Copilot.** Decided by the
   maintainer on 2026-11-11, after trial T1. When a rule is installed for both
   Claude Code and Copilot, upskill writes both the `.claude/rules` file and the
   `.github/instructions` file. Each client can then be removed without an
   effect on the other client's file. Known limit: in trial T1 (Copilot CLI
   1.0.109), with both copies present, `copilot instruction list` listed the
   rule twice and the model applied its marker twice. A skill installed for both
   clients was listed once. The trial could not determine the result for a
   subagent, because Copilot CLI has no command that lists agents. This decision
   replaces the two outcomes that section 5 of the re-steer design gives for
   trial T1. The change for
   [#465](https://github.com/driftsys/upskill/issues/465) adds the trial record
   as the technote `t1-copilot-duplicate-visibility.md`.
8. **Scope, update check and session notice.** The two scopes are named project
   and global. `add`, `update`, `remove` and `remove-mcp` print one line that
   states the scope and its root. Outside a git repository and without a scope
   flag, `add` stops and asks for `--project` or `--global`; `update`, `remove`
   and `remove-mcp` use the global scope there. A global install writes Copilot
   content to the personal directories of Copilot CLI under `$HOME/.copilot`.
   `list` and `doctor` report both scopes. The lockfile records the commit that
   each git source resolved to. `update --check` changes no file, compares the
   recorded commit with `git ls-remote` for the recorded ref, skips a source
   pinned to a full commit hash, does not look for newer tags, compares the
   content hash of a local path source, and stops its lookups after 10 seconds.
   `update
   --notice` is built for a session-start hook: it prints one line
   for each scope that has updates, prints a given set of updates once (for
   Copilot CLI, see the decision of 2026-11-19 at the end of this paragraph),
   contacts the sources at most once in 24 hours for each scope with a time
   limit of 5 seconds, keeps its state in `notice.json` in the cache directory,
   and exits with code 0 in every case after the arguments are parsed. upskill
   does not apply updates without the user's command. The hook is delivered as a
   plugin for each of Claude Code and Copilot CLI, both named `upskill`, with
   sources in `plugins/upskill-claude` and `plugins/upskill-copilot` of this
   repository and a published copy in `driftsys/metapowers`. The user installs
   the plugin with the client's own command; upskill does not install or own it.
   Trial T2 (technote `t2-session-notice-display.md`) records how each client
   shows the hook output. Decision of 2026-11-19 (issue #530): for Copilot CLI
   the notice repeats at every session start until `upskill update` runs. The
   Copilot plugin calls `upskill update --notice --repeat`, which prints the
   cached set at every run. Reason: Copilot CLI gives the hook output to the
   model and does not show it to the user, and in a trial in its interactive
   terminal the model passed the notice on in 0 of 3 runs, so a single print can
   be used up without the user seeing it. The Claude Code plugin keeps the
   single print, because Claude Code shows the line to the user.

This record replaces the following clauses of
[AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md):

- the activation and release-gate clauses, which block all-client release
  qualification, schema-2 activation and owned implementation until every
  applicable native requirement passes in all four clients, and the four-client
  release scope that follows from them (Decisions 3 and 4);
- the clause that reuses source capture, per-owner contracts, complete
  preflight, qualified locking, exact backup and recovery foundations (Decisions
  2 and 5);
- the selected Codex policy, "supported with declared limitation" (Decision 4);
- the implementation order of skills and resources first, then rules, then
  automatic maintenance (Decision 6);
- the one-copy policy for a rule read by both Claude Code and Copilot, which
  rejects a second `.github/instructions` copy (Decision 7).

It also replaces the following clauses of
[AD-0041](https://github.com/driftsys/upskill/blob/main/docs/adr/0041-separate-client-representations-as-design-baseline.md):

- the requirement of Q10/Q11 and CD-01 through CD-11 evidence before owned
  implementation (Decision 3);
- the implementation order of skills and resources first, then rules, then
  automatic maintenance (Decision 6);
- the one-copy policy for a rule read by both Claude Code and Copilot, which
  rejects a second Copilot copy (Decision 7);
- the clause that reuses capture, owner/reference contracts, complete preflight,
  qualified locking, backups and recovery (Decisions 2 and 5).

The separation of delivery from discovery and activation (AD-0040) and separate
client representations as the design baseline (AD-0041) stay accepted.

AD-0041 leaves a reduced four-client release scope to a separate decision. This
record is that decision: the MVP release covers Claude Code and Copilot, with
OpenCode experimental and Codex unsupported (Decisions 3 and 4).

## Consequences

- Every merged pull request changes behaviour that a command can reach, or tests
  such behaviour.
- The properties of the ownership record and the repeatable operations are
  verified by ordinary Rust tests in hosted CI and are required for every
  release.
- Client trials no longer gate a release. Their results set support levels.
- Two upskill processes that change the same scope at the same time are not
  supported.
- Shared files that upskill creates or edits (`CLAUDE.md`,
  `.vscode/settings.json`, `opencode.json`) are outside the ownership record.
- The decision records and design notes for the removed code are kept as history
  and marked withdrawn. The trial results stay in `docs/technotes/` as research.
- Work on Codex instructions inside a team-owned `AGENTS.md` is reviewed again
  when Codex rules are designed. It is not assumed to return.

## Alternatives considered

- **Continue the gated lifecycle and activate schema 2 for all clients.**
  Rejected: four weeks without a reachable change, results that go out of date
  with each client release, and one client blocking all others.
- **Activate the existing lifecycle library for Claude Code and Copilot only.**
  Rejected: the library models crash checkpoints, writer locks and qualified
  roots that the user's problems do not need.
- **Drive each client's native plugin system for all content.** Rejected: the
  retained trials showed that Claude's plugin install rewrites user settings,
  and that Copilot activated a Claude registration.

## Withdrawn records

The following records describe code that was removed from `main`. Each is kept
as history and is marked withdrawn by this record.

- [AD-0015](https://github.com/driftsys/upskill/blob/main/docs/adr/0015-pure-legacy-state-projection.md)
  to
  [AD-0039](https://github.com/driftsys/upskill/blob/main/docs/adr/0039-compile-captured-codex-instructions.md),
  all 25 records.
