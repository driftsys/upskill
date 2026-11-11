# AD-0042 — Re-steer customization delivery to reliable install, update and removal

Status: **accepted**. Date: 2026-11-11.
[MVP epic #283](https://github.com/driftsys/upskill/issues/283). The
[re-steer design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-11-11-customization-resteer-design.md)
holds the full design and delivery order. This record describes the decision;
the behaviour it selects is not implemented yet.

## Context

Between 2026-10-14 and 2026-11-11 the work on the customization MVP added about
25,000 lines of Rust and about 35,000 lines of tests. `src/main.rs`,
`src/cli.rs` and `src/pipeline/` did not change in that period. No user-visible
behaviour changed. The new code was not reachable from any command.

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
   SHA-256 of the bytes written. `update` and `remove` replace or delete only
   files whose current hash equals the recorded hash, unless the user passes
   `--force`.
2. **Repeatable operations.** An install or update records its intended files as
   pending in the lockfile before it writes, writes each file through a
   temporary file and a rename, and then promotes the pending record. Running
   the same command again after an interruption completes the operation. There
   is no journal, no checkpoint model and no writer lock.
3. **Support levels in place of activation gates.** Each client and kind has one
   published level: supported, experimental or unsupported. A smoke test against
   the real client sets the level. A failed or outdated cell changes only its
   own level. It does not block a release or another cell.
4. **Claude Code and Copilot first.** Codex and OpenCode follow, each with its
   own short design. OpenCode keeps its current generation and is published as
   experimental. Codex stays unsupported in the MVP.
5. **Unused code is parked.** The library modules that no command could reach
   (`lifecycle`, `filesystem`, `operations`, `files`, `compile`, `instructions`,
   `source_manifest`, `ownership`), their tests, the legacy projection and the
   `rustix` dependency were removed from `main`. They are kept on the branch
   `archive/lifecycle-2026-11`, which receives no further work.

[AD-0040](https://github.com/driftsys/upskill/blob/main/docs/adr/0040-separate-customization-delivery-from-discovery.md)
and
[AD-0041](https://github.com/driftsys/upskill/blob/main/docs/adr/0041-separate-client-representations-as-design-baseline.md)
stay accepted. AD-0041 is revisited only if trial T1 shows that Copilot lists or
applies an item installed for both Claude Code and Copilot twice.

## Consequences

- Every merged pull request changes behaviour that a command can reach, or tests
  such behaviour.
- The properties of the ownership record and the repeatable operations are
  verified by ordinary Rust tests in hosted CI and are required for every
  release.
- Client trials no longer gate a release. Their results set support levels.
- Two upskill processes that change the same scope at the same time are not
  supported.
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
