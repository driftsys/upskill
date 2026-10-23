# Gate 6 Codex user rule session lifecycle observations

Task [#377](https://github.com/driftsys/upskill/issues/377) continues
[#337](https://github.com/driftsys/upskill/issues/337), Task 2/CD-02 of the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md).
The unchanged, user-reviewed experimental
[user-registration packages](gate6-codex-user-rule-controls.md) were observed
through fresh sessions, native UUID resume and native UI compaction on macOS
arm64 with Codex `0.163.0`. These observations include a failed policy control
and failed single-body delivery control. No adapter is adopted; all 160 cells
remain unqualified and production stays schema 1.

## Delivery and controls

The two independent plugin rules still use `SessionStart` additional context,
alongside manual project `AGENTS.md`. Registration is user-scoped, but the
callbacks intentionally guard only two disposable project paths. This slice uses
the already trusted first project, source packages and cached definitions. It
changes no hook definition and overrides no trust decision. It does not qualify
arbitrary-workspace user delivery, project registration, shared catalogs,
compatibility readers, imports/globs or direct-plus-plugin coexistence. Unnamed
native labels remain separate debt
[#346](https://github.com/driftsys/upskill/issues/346).

Both session pairs start new owned sessions; none of the twelve sessions frozen
by the earlier user-rule evidence is resumed. Each pair resumes its own exact
UUID. Native metadata binds provider `openai` and model `gpt-6.1-sol`. No model
tools run. Authentication contents are not inspected or copied.

| Observation                                | Response | Native policy   | New exact rule bodies A/B | Result                              |
| ------------------------------------------ | -------- | --------------- | ------------------------- | ----------------------------------- |
| Initial fresh session                      | Matches  | Read-only       | 1 / 1                     | Fresh positive                      |
| Initial native resume                      | Matches  | Workspace-write | 1 / 1                     | Failed read-only control            |
| New explicit-policy fresh session          | Matches  | Read-only       | 1 / 1                     | Fresh positive                      |
| Same explicit-policy session resumed       | Matches  | Read-only       | 1 / 1                     | Resume positive; history has 2 / 2  |
| UI reopen, native compact, next model turn | Matches  | Read-only       | 2 / 2                     | Failed single-body delivery control |

The first runner supplied `--sandbox read-only` only to the fresh `exec`
invocation. Its subsequent `exec resume` completed with workspace-write policy.
The runner's native-policy guard stopped that control. The failed resume's child
exit code was not retained before the guard raised; a post-run diagnostic
retains its actual completion, policy and callbacks without inventing an exit
receipt. This is not a provider failure or evidence that ephemeral flags persist
between native invocations.

A separate pair supplies the ordinary configuration override
`-c sandbox_mode="read-only"` on both invocations. Both native contexts are
read-only and both declared responses complete. Each hook point calls each rule
once. The append-only session history contains one body per rule after fresh
startup and two after resume. Historical message count alone does not establish
the number of copies in provider effective input.

## Native compaction and duplicate delivery

The ordinary UI reopens only the second pair's new session. Its two resume
callbacks are followed by native `/compact`, two compact callbacks and a
successful next model response. The recorded terminal reports
`Context
compacted`; the session contains a native `compacted` record. The
recorder retains the actual native child exit code of zero, input/output hashes
and the next turn's read-only policy. Initial inner terminal dimensions were
zero; a receipt records resizing only this recorder's child terminal to 40 by
120 before input. No trust prompt was approved by the agent.

The compacted replacement history contains no exact plaintext fixture rule body
or fixture rule marker in its developer messages. Its encrypted content is not
inspected. On the next model turn, the native session records **two exact
separate developer bodies for each rule**, following the reopen and compact
callbacks. This is consistent with both outputs remaining pending until that
turn; internal queue state is not inspected. Each source callback ran once per
rule, yet delivery was duplicated on that turn. The correct, unique JSON array
is not evidence that the rule payloads were unique. Blocking task
[#379](https://github.com/driftsys/upskill/issues/379) requires an evidenced
strategy for this lifecycle before selection or activation; this slice
introduces no hook management or speculative deduplication implementation.

## Preservation and evidence limits

Manual `AGENTS.md`, the post-user-review profile configuration and ten cached
package files retain their bytes and modes. The twelve earlier frozen native
sessions remain unchanged. The newly owned session files grow append-only;
earlier phase receipts bind prefix lengths and hashes rather than claiming that
a final whole-file hash stays constant. Original manifests and frozen trace
snapshots are preserved. New callback appends are intentional fixture state, not
evidence of a completely immutable profile or complete side-effect recovery.

The [summary](evidence/2026-10-23-gate6-codex-user-sessions/summary.json)
retains five completed, matching model responses and one native compaction,
while preserving both failed controls. The
[compaction receipt](evidence/2026-10-23-gate6-codex-user-sessions/compaction.json)
binds native terminal evidence, callbacks and duplicate counts. The
[failed pair](evidence/2026-10-23-gate6-codex-user-sessions/failed-policy-pair.json)
and
[explicit-policy pair](evidence/2026-10-23-gate6-codex-user-sessions/explicit-readonly-pair.json)
retain their different outcomes. Native selections and CLI receipts preserve
only experimental rule bodies, answers, policy and completion; full terminal,
account/provider state, accounting, world state and encrypted context stay out
of the selected packet. Original private sources, capture helpers and prefix
boundaries are bound by the
[source index](evidence/2026-10-23-gate6-codex-user-sessions/private-source-index.json)
and
[selected hashes](evidence/2026-10-23-gate6-codex-user-sessions/receipt-hashes.json).
The capture and selection helpers are not qualified runners.

This does not complete native timeout, active-context deduplication, update or
integrated ownership/recovery controls. The four-client, project/user,
macOS/Linux, IDE, unintended-reader and other customization obligations remain.
General hooks, tools and permissions stay planned. Schema-2 activation remains
blocked.
