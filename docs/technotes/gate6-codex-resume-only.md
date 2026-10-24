# Gate 6 Codex resume-only retry: failed positive

Task [#379](https://github.com/driftsys/upskill/issues/379) requires a separate
resume turn without compaction under the
[current handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md).
The 2026-10-25 retry on macOS arm64, Codex 0.164.0, fails its native positive
with HTTP 401. No resume invocation follows
([summary](evidence/2026-10-25-codex-resume-only/summary.json),
[provider error](evidence/2026-10-25-codex-resume-only/provider-error.json)).

## Method and observations

The
[four-client campaign receipts](evidence/2026-10-25-codex-resume-only/campaign-pins.json)
precede this request. An additional executing-client receipt confirms Codex
0.164.0. The existing isolated, normally authenticated profile and two
unchanged, previously trusted user hook registrations are reused. Their existing
guard selects an existing project; this is a continuity diagnostic, not fresh
project registration. No credentials are copied and no hook trust is bypassed
([request receipt](evidence/2026-10-25-codex-resume-only/positive-receipt.json),
[private source](evidence/2026-10-25-codex-resume-only/private-source-index.json)).

The invocation explicitly selects read-only policy and an automatic-compaction
threshold of 100000 tokens. This differs from the earlier forced threshold of
1000 so that an ordinary next turn could isolate resume. No model or discovery
setting is changed in the profile
([request receipt](evidence/2026-10-25-codex-resume-only/positive-receipt.json),
[earlier combined control](gate6-codex-rule-lifecycle.md)).

| Control                   | Pin           | Result                                |
| ------------------------- | ------------- | ------------------------------------- |
| Fresh continuity positive | Codex 0.164.0 | FAIL: HTTP 401, no completed response |
| Resume-only turn          | Codex 0.164.0 | NOT RUN after failed positive         |

The native startup log contains one exact developer body per rule and read-only
turn context. Those records cannot replace a successful model positive. The
classifier returns `positive_control_failed`; no new attribution, lifecycle PASS
or provider-effective input qualification follows
([assessment](evidence/2026-10-25-codex-resume-only/assessment.json),
[native selection](evidence/2026-10-25-codex-resume-only/native-selection.json)).

## Preservation, checks and consequence

Measured existing guidance, configuration and installed definitions retain exact
bytes and modes. Existing guidance and client cache files are read for
preservation checks and are never directly written. Native session, log, cache
and authentication effects remain outside whole-profile equality
([preservation](evidence/2026-10-25-codex-resume-only/preservation.json)).

Four offline classification tests precede the harness extension: `just lint`
fails four missing-helper assertions, then passes 158 tests. They gate positive
completion, pin, read-only policy, tools, session identity, preservation,
absence of compaction, resume callback sources and strict recorded-body count
types. An observed count is reported independently of effective-input
qualification
([validation](evidence/2026-10-25-codex-resume-only/validation.json)).

The [earlier combined resume/compaction result](gate6-codex-rule-lifecycle.md)
remains unchanged. Resume-only attribution is still open. The proposed #379
severity amendment and durable route adoption remain at the user's step 7
review. No production route or durable record changes in this packet. Existing
Q8 and Q7 captures can be prepared independently without another model request
([handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
[summary](evidence/2026-10-25-codex-resume-only/summary.json)).
