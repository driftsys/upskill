# Gate 6 Codex rule lifecycle: repeated native bodies

Task [#379](https://github.com/driftsys/upskill/issues/379), Q9 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains a 2026-10-24 continuity experiment on Codex CLI 0.164.0, macOS arm64.
The unchanged adapter again records two exact developer rule bodies per logical
rule after reopening a session and its first automatic compaction. The second
automatic compaction records one per rule. Correct final responses do not repair
the first failure. Q9 remains unresolved and #379 stays open.

## Contract and decision

The intended rule contract is one active body per logical rule: fresh startup
loads one; resume/reopen retains or supplies one; compaction leaves one for the
next model request. Clear would establish a new context with one copy. Fork must
preserve the same boundary without duplicating it. Clear and fork have not been
qualified. The proposed all-sources `SessionStart` adapter emits at startup,
resume, clear, fork and compact; this packet does not change that adapter.

The [official hook documentation](https://learn.chatgpt.com/docs/hooks)
documents additional developer context, including immediate continuation after
automatic compaction. It does not promise replacement of an earlier rule body.
It also states that transcript format is not a stable hook interface. Therefore
neither private transcript parsing nor a once-per-session sent flag is adopted
as a workaround: the former has no supported interface contract, and the latter
may omit needed reinjection after compaction. No native history is deleted.

The [decision record](evidence/2026-10-24-codex-rule-lifecycle/decisions.json)
records these rulings and their costs. Other client controls continue after this
retained failure. No production adapter or durable record is amended here. The
affected proposal row is
[Rules, Codex](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules).
That row cannot become supported while #379 is open.

## Method and pins

This row uses Codex CLI 0.164.0 from the
[2026-10-24 campaign](gate6-plugin-profile-inventory.md). The other campaign
pins are Claude Code 2.1.308, Copilot CLI 1.0.103 and OpenCode 2.1.4; they do
not run in this packet. Model selection stays private.

The offline fixture extension in `scripts/qualify-plugin-rules.py` creates a
neutral managed source with two independent rule packages and a new empty Git
project. It writes no team guidance files and no symlinks. Existing roots,
dangling symlinks, repository roots and documented discovery roots are rejected.
Each rule and fallback has an unpredictable marker. This fresh fixture is
prepared only: it is not registered or assumed trusted.

The native continuity experiment instead reuses the isolated normally
authenticated profile and two unchanged, previously reviewed user-registered
packages from the earlier campaign. Their callback guard selects an existing
project. This is a deliberate diagnostic limit: it preserves the user's hook
review and the prior source/cache definitions, but does not qualify a fresh
project-scoped registration or new Git identity. The newly prepared project is
used only for the negative outside the existing callback guard.

A fresh noninteractive invocation establishes the positive before any candidate.
The same session is then reopened in the native UI with explicit read-only
policy and a command-line automatic-compaction threshold of 1000 tokens. Two
ordinary prompts trigger two automatic compactions in that one UI process; no
`/compact` command is sent. The threshold override is documented in the
[configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
and does not edit discovery settings. Every invocation has its own explicit
read-only override. Selected native turn contexts also report read-only policy.

The environment uses the existing allowlisted isolated-profile builder,
stripping inherited Git and client overrides. Ordinary trust is preserved; no
hook trust bypass flag or trust-store edit is used. The native child PTY
initially has zero window dimensions; its size is corrected and the correction
is retained in the private packet. Native UI exit completes normally with
code 0.

## Observations

| Control                              | Pin           | Outcome                                                          |
| ------------------------------------ | ------------- | ---------------------------------------------------------------- |
| Fresh continuity positive            | Codex 0.164.0 | PASS: exact JSON, one recorded developer body per rule           |
| Reopen then automatic compaction 1   | Codex 0.164.0 | FAIL: two recorded developer bodies per rule                     |
| Automatic compaction 2, same session | Codex 0.164.0 | One recorded developer body per rule; effective input unobserved |
| Outside guarded project              | Codex 0.164.0 | PASS: exact empty rules array                                    |
| Fresh project-scoped registration    | Codex 0.164.0 | NOT RUN: no safe replacement strategy selected                   |

The [summary](evidence/2026-10-24-codex-rule-lifecycle/summary.json),
[assessment](evidence/2026-10-24-codex-rule-lifecycle/assessment.json) and
[selected native records](evidence/2026-10-24-codex-rule-lifecycle/native-selection.json)
retain session and turn identities, source line indices, compaction positions,
exact rule bodies, completed tasks and read-only context. One startup, one
resume and two compact callbacks per rule belong to the same session. Callback
counts remain separate from developer-body counts. Three response tasks complete
in that session; the independent negative completes in another session. No model
tool call is observed; the native `additional_tools` catalog record is not a
tool invocation.

The first compaction's subsequent native records contain two bodies per rule;
the second's contain one. Replacement histories contain no exact plaintext rule
body. Encrypted compaction content and provider-effective input are not
inspected. Neither the transcript nor the exact response proves uniqueness at
all effective-input boundaries. Resume without a separate noncompacting turn was
not separately observed in the reopened UI. The result is a counterexample for
the unchanged adapter, not a complete Q9 qualification.

The negative uses a new empty Git project outside the existing callback guard.
Its exact response is `{"rules":[]}`. No existing discovery setting is disabled
or edited to produce it. It is a guard-negative observation, not the missing
project-scope disable/uninstall matrix.

## Preservation and provenance

Measured team content, configuration and both installed package definitions
retain identical hashes and modes after the positive, UI lifecycle and negative.
There are no direct writes to client plugin caches or team guides. Native
session, log and database state changes are expected native effects;
whole-profile byte restoration is not claimed. No registration was changed, so
there is no new registration to remove. The prepared source remains outside the
repository.

Raw native sessions, terminal input/output, source files and capture helpers
remain private. The
[source index](evidence/2026-10-24-codex-rule-lifecycle/private-source-index.json)
binds each private manifest. The retained receipt manifest binds sanitized
selections. Private paths are normalized and model identifiers omitted. Native
record parsing is used only by the private retention diagnostic, never by the
candidate hook.

## Offline checks and remaining work

Tests preceded implementation: `just lint` failed with ten missing-helper errors
across 57 tests, then passed all 57 tests. The new tests pin copy-only fixtures
without guidance writes, two distinct rule bodies, role-specific exact payload
occurrence counts, positive-first classification, stable session identity,
completion, explicit read-only policy, two automatic compactions, strict integer
body counts and refusal to qualify unobserved effective input. They never run a
client.

A bounded replacement candidate, native effective-copy observability, fresh
project registration, ordinary review of any changed definitions, separate
resume/clear/fork controls and the remaining platform/scope/reader matrix remain
open. Linux and IDE/hosted surfaces are unqualified. Production stays schema 1;
step 7 cannot promote the Codex rule route from this packet.
