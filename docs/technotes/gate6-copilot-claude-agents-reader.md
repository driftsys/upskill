# Gate 6 Copilot reader of restricted Claude agents

Task [#383](https://github.com/driftsys/upskill/issues/383), Q2 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
records catalog discovery and a real delegated child on 2026-10-24. These are
bounded project observations, not a general safe exception-route claim.
Production remains schema 1.

## Method and pins

Copilot CLI 1.0.103 on macOS arm64 uses the
[2026-10-24 campaign revision](gate6-plugin-profile-inventory.md). The other
campaign pins are Claude 2.1.308, Codex 0.164.0 and OpenCode 2.1.4; none ran a
control in this packet. The model route was explicitly selected from the
normally provisioned account catalog. Model identifiers remain private.

The existing `scripts/qualify-native-instructions.py` builds a fresh disposable
Git project with a distinct empty root commit. Inherited `GIT_*` selectors are
removed; fixture hooks and signing are disabled. The project contains only
`.claude/agents/probe.md` and `allowed.txt`. The Claude-format agent has an
unpredictable name, `tools: Read` and `permissionMode: plan`. Its instructions
request a read of `allowed.txt`, an attempt to create `denied.txt` with the read
marker through a native editing tool, no shell, and a final marker response. The
unpredictable marker is only in the resource, not the agent or parent prompt. No
team instruction files are created.

The private helper connects to the native Copilot ACP server and captures the
`session/new` agent catalog without a model request. ACP is a documented
[native Copilot interface](https://github.blog/changelog/2026-01-28-acp-support-in-copilot-cli-is-now-in-public-preview/).
The
[catalog receipt](evidence/2026-10-24-copilot-claude-agents-reader/catalog.json)
retains only the fixture entry and catalog counts, not the model catalog.

A separate native prompt requests exactly one delegation to that agent. The
parent exposes and permits `task`, `view` and `create`, disables built-in MCPs
and auto-update, and does not disable custom discovery. The request instructs
the parent to delegate without reading or editing files itself. Native parent
and child events are retained separately and correlated by task/tool-call IDs.
No user or team discovery settings were edited to manufacture exclusion.

## Controls

| Control                         | Pin             | Observation                                                     | Result                                             |
| ------------------------------- | --------------- | --------------------------------------------------------------- | -------------------------------------------------- |
| Native agent catalog            | Copilot 1.0.103 | Fixture appears under its generated name                        | PASS: compatibility discovery observed             |
| Real task delegation            | Copilot 1.0.103 | Matching native child start/completion and parent task result   | PASS: real delegation observed                     |
| Child tools selection           | Copilot 1.0.103 | `subagent.selected` reports `Read`                              | Read selection observed for this fixture           |
| Child resource access           | Copilot 1.0.103 | One successful child `view` call reads the unpredictable marker | PASS: resource read observed                       |
| Outside-tool use                | Copilot 1.0.103 | No child editing or shell call; no `denied.txt`                 | No violation observed; editing attempt unavailable |
| `permissionMode` interpretation | Copilot 1.0.103 | No editing tool call or native permission-denial event          | UNQUALIFIED                                        |

The child explicitly reported having only `view` and no native editing tool. Its
final message contains that explanation and the read marker. The parent returned
the exact marker. The request exited 0, completed successfully and created no
forbidden file. The
[summary](evidence/2026-10-24-copilot-claude-agents-reader/summary.json) records
`delegation_observed`, with one successful child tool and no outside tool. A
catalog entry or response text alone is not counted as delegation.

## Consequence for the proposal

The affected row is the
[proposal Subagents section's Claude native-file
exception](../wip/specs/2026-10-23-customization-delivery-proposal.md#subagents).
Copilot is a demonstrated reader of this restricted Claude-format file, so
placing it under `.claude/agents` does not isolate it from Copilot. The native
event selected `Read` and the only observed child invocation was `view`. The
child reported editing unavailable. The complete effective child tool catalog
was not independently captured, so these observations do not prove tool-filter
enforcement, unrestricted execution or a restriction violation.

Do not generalize that observation into preservation of `permissionMode`,
agent-local hooks, MCP settings or other Claude-only semantics. No actual
permission-boundary denial was exercised. The broad safe exception-route claim
remains unqualified; projects requiring those semantics cannot rely on this
packet to support it. No durable record is amended before step 7.

## Preservation and provenance

The
[preservation receipt](evidence/2026-10-24-copilot-claude-agents-reader/preservation.json)
records unchanged bytes and mode `0600` for the previously provisioned isolated
profile's `config.json` before catalog creation, afterward and after delegation.
No measured configuration required restoration. No authentication store was
read, copied or exported. Native session, log and search state may remain;
whole-profile restoration is unqualified. Helpers did not write client plugin
caches or team instruction files.

Raw protocol responses, transcripts, fixtures and capture helpers remain outside
the repository. The
[private source index](evidence/2026-10-24-copilot-claude-agents-reader/private-source-index.json)
and hash manifests identify them. Selected native events retain agent/tool IDs,
child result evidence and final responses, omitting model identifiers, thinking,
usage and unrelated metadata. Private paths are normalized. Full raw-output
hashes differ from sanitized retained-event hashes.

## Offline checks and remaining limits

Tests preceded the harness change: `just lint` failed on five missing Q2 helper
tests, then passed all 47 offline tests. Review follow-ups pin missing
delegation despite read evidence and isolate hostile host hooks/signing; the
subsequent full verification passes all 48 offline tests. Fixture tests cover
exact restrictions and instructions, resource-only marker placement, fresh
names/markers, independent empty root commits and inherited Git selector
isolation. The classifier rejects catalog-only evidence, missing child/tool
evidence, nonmatching responses and failed invocations. Any observed child call
outside the declared tool restriction wins over both response text and
invocation failure, including an outside call that fails. Final review
regressions cover unfamiliar outside tools, violation precedence over missing
catalog evidence and trailing-response rejection; the latest lint run passes all
49 offline tests.

The tests do not call clients or qualify native behavior. Broader tool lists,
permission inheritance, attempted native denials, other Claude fields, user
scope, Linux, IDE/hosted surfaces and integrated upskill lifecycle remain
unqualified. This packet promotes no production adapter or complete Gate 6
profile.
