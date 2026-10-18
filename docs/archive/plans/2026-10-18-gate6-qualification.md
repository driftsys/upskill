# Gate 6 native qualification

> Execute the approved M1 design in bounded delivery slices. Routine decisions,
> reviews and delivery are delegated by the user.

**Goal:** Establish reproducible native instruction discovery evidence, resolve
the staging exposure prerequisite in #316, then deliver Codex selection and
qualified schema-2 activation through subsequent reviewed slices.

**Specification:** `docs/wip/specs/2026-10-18-gate6-native-qualification.md`,
derived from `docs/wip/specs/2026-10-12-m1-owned-lifecycle-design.md` and M1-B
in `docs/wip/plans/2026-10-11-instructions-rules-m1.md`.

## Global constraints

- This first slice qualifies discovery; it does not activate schema 2.
- Preserve the four existing default surfaces. Codex remains explicit opt-in.
- Preserve uncertain local material and authentication. Never record secrets.
- Native probes use disposable fixtures and fresh sessions, disable tools where
  supported, and retain versions, exact commands, outputs and limitations.
- A successful renderer or unit test is not proof of native discovery.
- Never remove a staging file based only on its filename prefix.
- A negative probe counts only when the matching positive control succeeds.
- Separate default discovery from configured globs, explicit imports and file
  reads. Record which configuration was actually exercised.
- Every delivery slice includes appropriate tests, docs, full review and hosted
  CI. Follow-up activation must aggregate conflicts and preserve all supported
  legacy item kinds before changing production command routing.

### Task 1: Native discovery qualification harness

Own `scripts/qualify-native-instructions.py`, its focused Python tests and
`docs/technotes/m1-gate6-native-qualification.md`. Read the native client help
and current official instruction discovery documentation before choosing flags.

Implement an opt-in harness that creates disposable fixture trees and records
sanitized, reproducible qualification evidence for installed Codex, Claude,
Copilot and OpenCode clients. Never copy or print authentication files. Use
timeouts and explicit result states for unavailable clients, authentication
failures, positive-control failures and successful qualification. No automatic
installation or configuration changes. Keep raw evidence outside tracked docs.

Begin with focused tests for fixture construction and evidence classification:
canonical instruction markers must exist only in canonical positive fixtures;
staging markers only in `.upskill-stage-<nonce>` siblings in negative fixtures;
failure, timeout or missing positive marker must never qualify a negative. Use
distinct unpredictable markers and require exact observable responses. Cover
project and user roots where safe and supported; explicitly record any
unqualified scope. Include default and configured discovery separately where the
client supports instruction globs. Do not treat incidental AGENTS readers as
additional owners.

Run native project probes on the installed versions. Record exact versions,
platform, configuration, commands, positive and negative outcomes and evidence
hashes. Report real blockers honestly; continue independent probes. Update the
technote with the #316 conclusion justified by these observations, including
limitations and remaining Gate 6 requirements. Keep production Rust unchanged.

### Task 2: Review qualification and prepare the next bounded slice

Review Task 1 against the M1 contract and #316. Correct confirmed findings and
run repository formatting, verification, all-target lint and MSRV checks. Garden
the completed plan into durable records before opening the PR. Publish, run full
review and hosted CI, merge when exact-head gates pass, and verify main.

Use the resulting evidence to define the next Codex adapter and selection slice.
If staging exposure is observed, resolve its cause before schema-2 activation;
do not weaken the qualification or infer ownership from names. Keep Gate 6 open
until native scopes, unsupported overrides, reader compatibility, all-kind
preservation and complete preflight conflict aggregation are verified.
