# Gate 6 Codex resume-only attribution

Task [#379](https://github.com/driftsys/upskill/issues/379) has a completed
resume-only attribution control on macOS arm64, Codex 0.164.0, dated 2026-10-25.
Startup records one exact developer body per logical rule. Resume adds one
further body per rule, giving two in cumulative recorded history, without any
compaction. Both model turns complete with the expected response
([summary](evidence/2026-10-25-codex-resume-attribution/summary.json),
[observation](evidence/2026-10-25-codex-resume-attribution/observation.json)).

## Method and observations

The [four-client campaign revision](gate6-plugin-profile-inventory.md) precedes
the native calls. Each executing-client receipt additionally verifies Codex
0.164.0. The isolated profile is authenticated through ordinary native login.
Previously trusted user hook registrations and their existing guarded project
are reused unchanged. This qualifies a continuity observation, not fresh
registration or a new project route
([positive receipt](evidence/2026-10-25-codex-resume-attribution/positive-receipt.json),
[resume receipt](evidence/2026-10-25-codex-resume-attribution/resume-receipt.json)).

Each invocation explicitly selects read-only policy, no daemon and an automatic
compaction threshold of 100000 tokens. One session ID and distinct native turn
IDs correlate the two completed turns. Neither turn uses tools. The resumed
native log retains the exact startup prefix, and no compacted record occurs. Two
hook trace callbacks identify resume, one for each registered rule
([observation](evidence/2026-10-25-codex-resume-attribution/observation.json),
[selected startup records](evidence/2026-10-25-codex-resume-attribution/positive-native-selection.json),
[selected resumed records](evidence/2026-10-25-codex-resume-attribution/resume-native-selection.json)).

| Control or record window        | Pin           | Result                                                     |
| ------------------------------- | ------------- | ---------------------------------------------------------- |
| Startup positive                | Codex 0.164.0 | PASS: completed expected response, one exact body per rule |
| Resume-only turn                | Codex 0.164.0 | PASS: completed expected response, zero compaction records |
| Cumulative history after resume | Codex 0.164.0 | Two exact bodies per rule; uniqueness FAIL                 |
| Newly appended resume records   | Codex 0.164.0 | One additional exact body per rule                         |

The extra recorded body is present on resume before any compaction in this
control. Cumulative history and newly appended records are separate counting
windows. These observations do not identify provider-effective input, prove its
uniqueness, or establish how long a duplicate remains active. The unchanged
classifier returns `recorded_counts_observed`, not complete route qualification
([classification](evidence/2026-10-25-codex-resume-attribution/classification.json),
[summary](evidence/2026-10-25-codex-resume-attribution/summary.json)).

## Preservation and provenance

Measured guidance, configuration and installed definitions retain exact hashes
and modes. No team guide, hook definition or native cache is directly written.
Native authentication, session, log and cache effects are outside the measured
preservation contract, so whole-profile equality is not claimed
([preservation](evidence/2026-10-25-codex-resume-attribution/preservation.json)).

Raw native sessions, traces, model selection, source helpers and test logs stay
outside the repository. Public events omit response text, usage, thinking and
model identifiers; selected native records retain only relevant session/turn
metadata and exact fixture rule bodies. Public command receipts redact the
private model selection and normalize fixture paths. Manifests hash retained
files and private sources separately
([source index](evidence/2026-10-25-codex-resume-attribution/private-source-index.json),
[receipt hashes](evidence/2026-10-25-codex-resume-attribution/receipt-hashes.json)).

Three offline tests fail before the existing harness gains
`codex_resume_body_windows`, then all 173 offline tests pass. They distinguish
cumulative and appended counts, require an unchanged nonempty startup prefix,
and reject compaction or malformed history. Tests make no native call
([validation](evidence/2026-10-25-codex-resume-attribution/validation.json)).

## Consequence

This completes the requested resume-only observation for the proposal's
[Codex always-on rules row](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules).
The [earlier combined resume/compaction FAIL](gate6-codex-rule-lifecycle.md) and
[HTTP 401 failed positive](gate6-codex-resume-only.md) remain retained
unchanged. The user owns step 7 adoption and the proposed #379 severity
amendment under the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md).
This packet changes no durable record or production route. It adds no evidence
about OpenCode, whose Q5 retry remains deferred
([summary](evidence/2026-10-25-codex-resume-attribution/summary.json)).
