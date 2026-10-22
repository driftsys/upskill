# Gate 6 Codex project compaction observations

Task [#368](https://github.com/driftsys/upskill/issues/368) retains an existing
private observation from the
[#337](https://github.com/driftsys/upskill/issues/337) campaign. It supplements
the [authenticated project controls](gate6-authenticated-project-controls.md),
without rerunning the native client or qualifying a whole profile. All 160
matrix cells remain unqualified; production stays schema 1.

## Native session and selected records

The preserved session identifies Codex `0.163.0`, provider `openai`, model
`gpt-6.1-sol`, the existing isolated project and a single session ID. The prior
private assessment describes the ordinary CLI TUI `/compact` action. The source
packet lacks a contemporaneous invocation command and process-exit receipt, so
this note does not independently establish those facts. The earlier fixture
campaign ran on macOS arm64; the session metadata does not itself identify the
operating system or architecture. No credentials, authentication files or trust
configuration were read or changed during retention.

The selected
[session records](evidence/2026-10-22-gate6-codex-compaction/session-selected.jsonl)
retain timestamps and original zero-based line indices. They show:

1. An initial completed response containing the team marker and both rule
   markers, following two developer messages containing the respective rules.
2. One native `compacted` record inside a second task. This task completes with
   no last agent message; it is not counted as a response turn.
3. The same two developer rule messages before a subsequent response. The user
   prompt and exact response match the first response turn; its distinct task
   completes successfully.

All twelve original `response_item` records have type `message`; none is a tool
call. The selection retains nineteen records, including the three turn contexts
and three completed tasks. The compaction record's history, retained context and
message content are omitted; the actual native record's presence and position
remain visible. The two short fixture rule messages, qualification prompts and
agent responses are retained. Account identifiers, base instructions, world
state and provider/accounting data are omitted.

## Callback and preservation observations

The same session's
[callback selection](evidence/2026-10-22-gate6-codex-compaction/callback-events.json)
contains one `startup` and one `compact` event for each of rules A and B. These
callback records contain no timestamps. The post-compaction placement of the
rule messages is established by native session order, not a callback wall-clock
trace. Matching responses alone would not establish fresh rule delivery, since
compaction may preserve prior content. The developer messages and callback
records are separate observations; they do not qualify every compaction mode or
activation-count boundary.

The
[fixture assessment](evidence/2026-10-22-gate6-codex-compaction/fixture-assessment.json)
compares the initial prepared team snapshot with the file at retention time.
Both SHA-256 and mode `0640` agree. These are not immediate before/after
compaction snapshots and do not establish uninterrupted preservation,
configuration/cache equality or recovery of native side effects. Retention made
no changes to the fixture, plugins or normally provisioned profile.

## Provenance and remaining acceptance

The
[private source index](evidence/2026-10-22-gate6-codex-compaction/private-source-index.json)
records exact source paths and original hashes, including the preserved session
hash recorded by the earlier assessment. It also fingerprints the private
retention helper. The full activation trace remains private because it contains
other sessions; only the selected session's four events are retained here. The
separate
[retained manifest](evidence/2026-10-22-gate6-codex-compaction/receipt-hashes.json)
binds the five selected files. All original and retained hashes were checked.
This is informative data, not an executable qualification runner or product
input.

There is no fresh reproduction, independent launch/exit evidence, immediate
preservation capture, repeated/automatic compaction control or disabled/removed
compaction pair in this packet. The remaining clients, user scopes, Linux, IDE
surfaces, unintended readers, duplicate activation, update/trust changes,
timeout cleanup and integrated ownership/recovery obligations remain open. Task
2 is incomplete. No route is adopted and schema-2 activation remains blocked.
