# Debt #570 and small debt #321, #348 — plan

Issue: <https://github.com/driftsys/upskill/issues/570> (epic #291). Also #321
and #348 (epic #283). #360 is stale and closed separately.

Spec: the issue bodies. No separate spec exists, so rulings made here are
provisional.

## Global Constraints

- Rust edition 2024, MSRV 1.85, zero warnings, `-D warnings` for clippy and
  rustdoc. No new dependencies.
- Only `main.rs` writes to stdout/stderr.
- Integration tests use `common::upskill_cmd(&fake_home)`.
- Write code comments, docs and commit messages in plain, literal English.
- Do not commit: the implementer leaves changes uncommitted in the working tree.
  The controller runs `just fmt`, `just verify` and commits.
- Pre-1.0: delete old shapes outright, no migration paths or fallback fields.
- A test that "pins" behaviour must fail when the named mutation is applied. The
  implementer applies the mutation, shows the test failing, and reverts it.
- Update docs that describe changed behaviour (specification, user guide, ADR
  text) in the same task.

## Rulings

- `update` recording `add --exclude` (issue bullet 4) is a feature with lockfile
  impact. It is out of scope and stays open in #570.
- `add --kind` must not drop `required_by` edges of requirers outside the kind
  restriction: a kind-restricted `add` refreshes provenance only for the entries
  it plans, the same rule as a restricted `update`.

## Task 1: Pin the six test gaps of #570

Read the section "Tests that do not pin the behaviour they name" of issue #570
(`gh issue view 570`). Add or strengthen tests so that each of the six mutations
or gaps listed fails the suite. Test-only change, except that a small refactor
to make a path reachable is allowed when no test can reach it.

Verify: for each gap, apply the named mutation, show a test failing, revert.

## Task 2: `upskill pack --force` stages and renames

Issue bullet 2. `pack --force` must write the new client directory to a staging
directory and replace the old directory by rename, so a failed write leaves the
old directory intact and a failure on the second client leaves no client
changed. `src/pack.rs` may already stage (see its tests
`force_failure_on_the_second_client_leaves_both_directories_unchanged`); read
the code first, and if the bullet is already satisfied, report that with
evidence and add only the missing tests.

## Task 3: Notice run lock replacement race

Issue bullet 3. After a stale lock file is replaced, a second run can delete the
new lock, and the first run then deletes the lock of the second run. Fix in
`src/notice.rs` so a run deletes only the lock file that it created (for example
by writing a unique token into the file and comparing before delete). Update the
ADR-0044 known-limit text and the AGENTS.md paragraph on the lock if they
describe the old behaviour. Add a unit test for the race.

## Task 4: Kind restriction provenance

Issue bullets 1 and 5.

- Bullet 1: apply the ruling above to `add --kind`; add a test, state the result
  in the docs.
- Bullet 5: a kept item of a kind outside a kind restriction is reported as up
  to date also when its source is skipped later in the same run. Find the cause.
  If the fix is local, fix it with a test. If it is not local, stop with
  NEEDS_CONTEXT and describe the options.

## Task 5: #321 and #348

- #321: `gh issue view 321`. In
  `scripts/tests/test_qualify_native_instructions.py`, pin the complete `GIT_`
  prefix policy (a test variable name outside the nine known sentinels must be
  removed), pin preservation of ordinary variables and synthetic authentication
  sentinels, and cover the fixture Git and native version/model invocations.
  Tests stay offline. Never use real credentials.
- #348: `gh issue view 348`. In
  `docs/technotes/gate6-authenticated-project-controls.md` narrow the statement
  about exclusive configuration backups to what the retained receipts prove
  (original and restored hash and mode equality). Keep the unqualified
  statements about interrupted restoration and native/cloud recovery.
