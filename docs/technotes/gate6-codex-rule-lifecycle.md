# Gate 6 Codex rule lifecycle: repeated native bodies

Task [#379](https://github.com/driftsys/upskill/issues/379), Q9 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains a 2026-10-24 continuity experiment on Codex CLI 0.164.0, macOS arm64.
Recorded developer-body counts are one per rule at startup, two in the combined
reopened-session and first automatic-compaction window, then one after the
second compaction. This is a retained counterexample to the former
zero-duplicate criterion, not a separate resume-only result
([summary](evidence/2026-10-24-codex-rule-lifecycle/summary.json),
[assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json)).

## Current direction and historical criterion

The user's
[2026-10-24 rules decision](../wip/specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24)
accepts the transient duplicate as a declared limitation. Step 7 records Codex
rules as supported with that limitation once a resume-only attribution control
is retained, and lists the amendment of #379 from a K0 blocker to K1 debt. This
packet does not implement that amendment or rewrite its historical FAIL
([handoff Q9](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
[packet decisions](evidence/2026-10-24-codex-rule-lifecycle/decisions.json)).

The offline classifier retains the stricter diagnostic criterion: a calibrated
startup and complete resume/compaction observations with one exact body per
rule. Clear and fork are outside that classifier. A separate resume-only turn
was not run, so the retained assessment explicitly marks it incomplete and the
classifier returns `lifecycle_control_failed`. Known native counts remain
independently checkable; an incomplete diagnostic matrix does not erase the
observed duplicate or decide current route adoption
([assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json),
[window audit](evidence/2026-10-24-codex-rule-lifecycle/native-window-audit.json)).

No runtime transcript parsing, native history deletion or session-level sent
flag is adopted as a workaround. Diagnostic parsing runs only in the offline
harness and retention helper. The unchanged candidate hook reads its static rule
body; it does not call upskill
([decisions](evidence/2026-10-24-codex-rule-lifecycle/decisions.json),
[earlier lifecycle diagnosis](gate6-codex-user-session-controls.md)).

## Method and pins

This row names Codex CLI 0.164.0 from the
[2026-10-24 campaign](gate6-plugin-profile-inventory.md). The other campaign
pins, Claude Code 2.1.308, Copilot CLI 1.0.103 and OpenCode 2.1.4, do not run in
this packet. Model identifiers remain private
([positive receipt](evidence/2026-10-24-codex-rule-lifecycle/positive-receipt.json),
[source index](evidence/2026-10-24-codex-rule-lifecycle/private-source-index.json)).

The fresh offline fixture has two independent rule packages and an empty Git
project. It creates no guidance files, trust configuration or registration. Its
ordinary hook-review requirement is explicit. Q6 and Q9 now share the neutral
root guard; tests reject configured roots, discovery roots and ancestor guidance
without creating guide files
([fresh fixture](evidence/2026-10-24-codex-rule-lifecycle/fresh-fixture.json),
[validation](evidence/2026-10-24-codex-rule-lifecycle/validation.json)).

The native continuity run instead reuses the isolated provisioned profile and
two unchanged, previously reviewed user registrations. Their callback guard
selects an existing project. This preserves the existing definitions and user
review, but does not qualify a fresh project registration. The newly prepared
project supplies only the outside-guard negative
([positive receipt](evidence/2026-10-24-codex-rule-lifecycle/positive-receipt.json),
[resume command](evidence/2026-10-24-codex-rule-lifecycle/resume-command.json),
[negative receipt](evidence/2026-10-24-codex-rule-lifecycle/negative-receipt.json)).

A fresh noninteractive positive completes before the same session is reopened in
the native UI. Every invocation explicitly selects read-only policy. The UI uses
an automatic-compaction threshold of 1000 tokens; two ordinary prompts produce
two compactions. No `/compact` input is sent. The native child terminal is
resized after its initial zero dimensions, and exits normally with code 0
([native selection](evidence/2026-10-24-codex-rule-lifecycle/native-selection.json),
[resume command](evidence/2026-10-24-codex-rule-lifecycle/resume-command.json),
[input audit](evidence/2026-10-24-codex-rule-lifecycle/input-audit.json),
[exit receipt](evidence/2026-10-24-codex-rule-lifecycle/exit-receipt.json)).

## Observations and attribution

| Control                           | Pin           | Observation                                                     |
| --------------------------------- | ------------- | --------------------------------------------------------------- |
| Fresh continuity positive         | Codex 0.164.0 | PASS: expected JSON object; one recorded body per rule          |
| Separate resume-only turn         | Codex 0.164.0 | NOT RUN; attribution remains incomplete                         |
| Reopen and automatic compaction 1 | Codex 0.164.0 | Historical FAIL: two recorded bodies per rule                   |
| Automatic compaction 2            | Codex 0.164.0 | One recorded body per rule; provider-effective input unobserved |
| Outside guarded project           | Codex 0.164.0 | PASS: empty rules array                                         |
| Fresh project-scoped registration | Codex 0.164.0 | NOT RUN; prepared fixture not registered                        |

The [assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json)
retains one startup, one resume and two compact callbacks per rule, all in the
same session. Callback counts are separate from body counts. Three native turns
complete with read-only context. The
[window audit](evidence/2026-10-24-codex-rule-lifecycle/native-window-audit.json)
reproduces the same `1/1`, `2/2`, `1/1` body counts from the full private native
stream and selected public records.

The trace records both resume and compaction callbacks for the combined window.
It is consistent with queued outputs being delivered together, as the
[earlier diagnosis](gate6-codex-user-session-controls.md) records. Internal
queue state is not inspected, and this run cannot attribute the duplicate to
compaction alone. The remaining resume-only control separates those causes
([assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json),
[input audit](evidence/2026-10-24-codex-rule-lifecycle/input-audit.json)).

Replacement histories contain no exact plaintext fixture body in the retained
diagnostic. Encrypted content and provider-effective input are not inspected;
correct final responses do not prove unique effective delivery. The independent
negative returns a JSON object with an empty `rules` array. It is a guard
negative, not a disable/uninstall matrix or proof of arbitrary-workspace
delivery
([assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json),
[negative events](evidence/2026-10-24-codex-rule-lifecycle/negative-events.json)).

## Offline replay and checks

```sh
python3 scripts/qualify-plugin-rules.py --replay-codex-rule-lifecycle docs/technotes/evidence/2026-10-24-codex-rule-lifecycle
```

The replay binds package-body hashes, startup response/session, native turn
identities, read-only policy, compaction windows and observed counts to the
assessment and summary. It explicitly represents the unrun resume-only control.
A semantic fingerprint allows JSON layout changes while preserving recorded
values and types; original byte hashes remain separate evidence. Altering a
recorded count, status, session or qualification flag fails replay
([rule bodies](evidence/2026-10-24-codex-rule-lifecycle/rule-bodies.json),
[window audit](evidence/2026-10-24-codex-rule-lifecycle/native-window-audit.json),
[validation](evidence/2026-10-24-codex-rule-lifecycle/validation.json)).

Tests precede the harness changes. They require structured startup evidence,
positive-first precedence, complete lifecycle checks before count checks, count
failures before effective-input uncertainty, every boundary kind, all text
blocks and strict JSON types. Fixture tests use isolated Git environments for
both creation and verification and pin the absence of trust/registration writes.
The validation record retains failures before these changes and the subsequent
passing checks. These are offline tests, not additional native runs
([validation](evidence/2026-10-24-codex-rule-lifecycle/validation.json)).

## Preservation and remaining limits

Measured team content, configuration and both installed definitions retain their
original hashes and modes. No registration, guide or cache was directly modified
by the control. Native session, log and database effects remain outside
whole-profile equality
([positive receipt](evidence/2026-10-24-codex-rule-lifecycle/positive-receipt.json),
[assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json)).

Raw sessions, terminal data and capture helpers remain outside the repository.
Selected fixture paths are normalized; command receipts retain executable and
profile locations, and the private index retains local source paths. Model
identifiers are omitted from committed evidence
([source index](evidence/2026-10-24-codex-rule-lifecycle/private-source-index.json)).

Remaining nonblocking maintenance is tracked in
[debt #402](https://github.com/driftsys/upskill/issues/402).

Resume-only attribution, clear/fork, fresh registration, any changed-definition
review, Linux and IDE/hosted surfaces remain unqualified. No production adapter,
durable record or schema-1 route is changed by this packet
([summary](evidence/2026-10-24-codex-rule-lifecycle/summary.json),
[step 7](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md)).
