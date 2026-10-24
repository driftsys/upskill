# Gate 6 Copilot reader of Claude rules

Task [#382](https://github.com/driftsys/upskill/issues/382), Q1 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
records bounded project discovery on 2026-10-24. Production remains schema 1.

## Method and pins

The [campaign revision](gate6-plugin-profile-inventory.md) pins Copilot CLI
1.0.103, Claude Code 2.1.308, Codex CLI 0.164.0 and OpenCode 2.1.4 on macOS
arm64. Only Copilot ran model requests in this packet.

The existing `scripts/qualify-native-instructions.py` constructs Q1 fixtures and
classifies reader outcomes. Every fixture is a separate disposable Git
repository with a distinct empty root commit, hooks and signing disabled, and
inherited `GIT_*` selectors removed. The positive has only
`.github/copilot-instructions.md`. The candidates have only
`.claude/rules/probe.md`, separately without frontmatter and with
`paths: "**/*.rs"`. Each has independent unpredictable instruction and absent
markers. Prompts contain only fallback markers. The paths variant performs no
matching-file access; it cannot establish native scoping or ignored frontmatter.

Private capture helpers use the previously provisioned isolated Copilot profile,
an allowlisted environment, disabled tools and built-in MCPs, disabled automatic
updates and structured JSON output. A successful request requires native exit 0,
a completed result, an assistant response and no observed tool execution or
session error. The positive must return its exact marker before candidates run.
No team instructions, discovery settings or client plugin caches were written by
the helpers. No `CLAUDE.md` or `AGENTS.md` was created.

## Attempts and controls

The initial positive failed with HTTP 402 `quota_exceeded`. After the user
identified available provisioned routes, an explicitly selected route failed
with HTTP 400 and a deployment-availability error. Neither failed positive ran
candidates or yielded a reader conclusion. Both remain retained.

A different normally provisioned route completed the positive and both
candidates. Model selection and identifiers remain private. This is an
observation on that account route, not a provider-wide claim.

| Control                                                 | Pin             | Native result                                  | Interpretation                                                                        |
| ------------------------------------------------------- | --------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------- |
| Initial native positive                                 | Copilot 1.0.103 | FAIL: exit 1, HTTP 402                         | Quota failure; candidates not run                                                     |
| First selected-route retry positive                     | Copilot 1.0.103 | FAIL: exit 1, HTTP 400                         | Deployment unavailable; candidates not run                                            |
| Final native `.github/copilot-instructions.md` positive | Copilot 1.0.103 | PASS: exact marker, exit 0, no tools           | Calibrates the final candidate pair                                                   |
| `.claude/rules/probe.md` without frontmatter            | Copilot 1.0.103 | PASS: exact candidate marker, exit 0, no tools | **Reads** in this project request                                                     |
| `.claude/rules/probe.md` with `paths:`                  | Copilot 1.0.103 | PASS: exact fallback, exit 0, no tools         | **Does not read** in this tool-free request; matching-file activation remains unknown |

The [summary](evidence/2026-10-24-copilot-claude-rules-reader/summary.json)
records five requests: two failed positives followed by three successful
controls. The
[initial failure summary](evidence/2026-10-24-copilot-claude-rules-reader/initial-failure-summary.json)
preserves the earlier checkpoint. Native responses, distinct session IDs,
fixture hashes and root commits are retained. Failed positives are never
reclassified as absence observations.

The result informs the adopted native-first rules decision in the
[delivery proposal](../wip/specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24).
Unscoped `.claude/rules` is a demonstrated Copilot reader. In the selected
shared Claude/Copilot route it is an explicitly declared per-item consumer of
one native file, not a second `.github/instructions` copy. The scoped matching-
file question remains deferred as Q1b. This packet does not establish ignored
`paths:` frontmatter, a selective exclusion or production activation.

## Replay and pre-run pin evidence

The existing harness now exposes the structured Copilot event gate used to
verify the retained outcome selections. It requires native invocation success,
one assistant response, a terminal successful result, and no tool, error,
malformed or unknown event. Response whitespace is preserved. The ordinary
Copilot native command requests JSON; the offline Q1 replay reads only retained
selections and compares both candidates and failed positives with the summary.
The
[full-stream audit](evidence/2026-10-24-copilot-claude-rules-reader/raw-event-gate-audit.json)
also applies the gate to the complete private stdout, including nonselected
events, and matches all five retained outcomes. Replay performs no model
request:

```bash
python3 scripts/qualify-native-instructions.py --replay-copilot-reader \
  docs/technotes/evidence/2026-10-24-copilot-claude-rules-reader
```

The
[pre-run provenance](evidence/2026-10-24-copilot-claude-rules-reader/pre-run-pin-provenance.json)
and
[version output](evidence/2026-10-24-copilot-claude-rules-reader/copilot-pre-run-version.txt)
bind Copilot's version before all three attempts. Native request timestamps and
retained file times corroborate the helper's enforced ordering; file times are
not independently attested. The other three clients have only post-attempt
version confirmation in this packet. This disclosed gap does not relax the
all-four pre-run requirement for new campaigns. The all-four pre-run step is
therefore unproven, and the inventory no longer asserts it. Q1's
executing-client pin is retained; other clients' controls require their own
pre-run evidence.

## Preservation, provenance and residue

The
[initial preservation receipt](evidence/2026-10-24-copilot-claude-rules-reader/preservation.json)
and
[retry receipt](evidence/2026-10-24-copilot-claude-rules-reader/retry-preservation.json)
record unchanged isolated-profile `config.json` bytes and mode `0600`. The
absence of `settings.json` and `mcp-config.json` was not retained as a measured
value and is not claimed. No measured configuration required restoration.
Authentication stores were not read, copied or exported. Native session, log and
search state may remain; exact whole-profile restoration is unqualified. No real
user profile was selected.

Raw transcripts, fixtures and capture helpers remain outside the repository. The
[private source index](evidence/2026-10-24-copilot-claude-rules-reader/private-source-index.json)
identifies them and their hashes. Selected retained events omit model
identifiers, thinking, usage, unrelated metadata and error text. Public command
receipts explicitly redact the private model selection. Raw-output hashes refer
to complete private bytes, not sanitized retained events.

## Offline validation and limits

The public event-gate and replay review fixes pass 51 offline tests and full
verification. Five rejected mutations cover independently duplicated message and
result records, failed-native-event propagation, exact orchestration responses
and summary-response identity. The
[validation receipt](evidence/2026-10-24-copilot-claude-rules-reader/validation.json)
binds these logs and source snapshots through the private index. No new native
request was made. Remaining nonblocking packet-review comments are tracked in
[debt #399](https://github.com/driftsys/upskill/issues/399).

Tests were written first. `just lint` failed on the two missing Q1 helper
attributes before implementation, then passed all 42 offline tests. Follow-up
regressions cover complete fixture directives, frontmatter, fresh separated
markers, distinct empty root commits and inherited Git selector isolation.
Classification requires an exact successful positive before either candidate
marker or fallback can yield a conclusion. Mutation checks reject positive-guard
bypass, incomplete frontmatter and inverted candidate instructions.

These tests never call a client and cannot qualify discovery. Linux, user scope,
IDE and hosted surfaces remain unqualified. No plugin, subagent, update, resume,
compaction or production lifecycle route was tested. This bounded Q1 result
promotes no production adapter or complete Gate 6 profile.
