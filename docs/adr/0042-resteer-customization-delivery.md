# AD-0042 — Re-steer customization delivery to reliable install, update and removal

Status: **accepted**. Date: 2026-11-11.
[MVP epic #283](https://github.com/driftsys/upskill/issues/283). The
[re-steer design](https://github.com/driftsys/upskill/blob/main/docs/wip/specs/2026-11-11-customization-resteer-design.md)
holds the full design and delivery order. The change that adds this record is
delivery step 0: it removes the unused code (Decision 5). Decision 7 describes
the layout that upskill already writes. The other decisions are not implemented
yet.

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

1. **Ownership record in the lockfile.** Each installed item will record the
   files written for it, with the client, the path relative to the scope root
   and the SHA-256 of the bytes written. `update` and `remove` will replace or
   delete only files whose current hash equals the recorded hash (or whose hash
   with every CRLF replaced by LF equals it), unless the user passes `--force`.
2. **Repeatable operations.** An install or update will record its intended
   files as pending in the lockfile before it writes, will write each file
   through a temporary file and a rename, and will then promote the pending
   record. Running the same command again after an interruption will complete
   the operation. There will be no journal, no checkpoint model and no writer
   lock.
3. **Support levels in place of activation gates.** Each client and kind will
   have one published level: supported, experimental or unsupported. A smoke
   test against the real client will set the level. A failed or outdated cell
   will change only its own level. It will not block a release or another cell.
4. **Claude Code and Copilot first.** Codex and OpenCode follow, each with its
   own short design. OpenCode keeps its current generation and is published as
   experimental. Codex stays unsupported in the MVP.
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
   last.
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
