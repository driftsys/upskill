# Gate 6 Copilot plugin components: failed rules and bounded delegation

Task [#388](https://github.com/driftsys/upskill/issues/388), Q3 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains Copilot CLI 1.0.103 controls on macOS arm64, 2026-10-24. Native
instruction positives pass. Plugin rule positives fail. A guarded retry observes
one real namespaced user-scope custom child and its resource read, while its
leading response whitespace fails the exact marker contract. The full Q3 route
is unqualified. Production remains schema 1.

## Fixture and pins

Copilot 1.0.103 is the
[2026-10-24 campaign pin](gate6-plugin-profile-inventory.md). Claude 2.1.308,
Codex 0.164.0 and OpenCode 2.1.4 do not run controls in this packet. Native
session-start records establish the pin and parent selection for all 14 initial
and 16 completed-retry parent sessions. Later requests additionally check the
version before invocation. Automatic client updates are disabled.

The existing `scripts/qualify-native-instructions.py` creates neutral managed
packages with Agent Plugins 1.0 `plugin.json`, rule files under
`com.github.copilot/rules/`, and `probe.agent.md` under
`com.github.copilot/agents/`. Two independent disposable Git projects have
native `.github/copilot-instructions.md` positives, distinct markers and
ordinary resources. No team-guide file, symlink or client cache is authored.

The initial rule filenames are `.md`; a separate `.instructions.md` inventory
variant is retained. The completed retry uses `.instructions.md` and complete
native `name`, `description` and `applyTo` headers: `"**"` for the unscoped rule
and `"matching/**"` for the scoped rule. Each variant has fresh markers. Old
source and failed responses remain unchanged. The
[documented component layout](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference)
is a fixture basis, not proof of installed-version support.

A separate native `.github/agents/catalog-positive.agent.md` calibrates later
catalog checks. The child response marker exists only in `agent-resource.txt`.
The normally authenticated isolated profile and stripped child environment are
reused without copying credentials. No existing user/team discovery setting is
changed to manufacture a negative.

## Installation and scope

Native direct local-directory install and uninstall succeed. Local marketplace
installation reports live loading from the managed source, with nothing copied.
That native behavior does not authorize direct writes into a client cache.

The project attempt disables only the newly created user binding through the
native CLI and creates its own project `enabledPlugins` entry. The user attempt
uses ordinary native install, disable and uninstall. Project-overlay activation
is not established: the completed retry's ACP catalog contains the native agent
positive but no plugin candidate. The user-enabled catalog contains the exact
namespaced candidate. Prompt-mode workspace admission and ACP catalog behavior
are separate observations.

## Initial failures and catalog correction

Both original instruction positives pass. Installed component rule responses
fail the required marker/JSON contract; some include refusals, prose or fences.
Matching and nonmatching responses do not establish scoped activation. Empty
arrays after disable/uninstall do not qualify negatives after failed installed
positives. The initial user uninstall request was interrupted and its reinstall
request was not run; native management cleanup completed.

The original project delegation used the display name instead of the native
selection ID. It selected generic delegates, reached its timeout and produced
four nonrequested delegate-model selection events. Starts do not prove inference
completed on each selected model. This is a retained model-policy failure, not a
successful custom-child control. Further calls were paused.

Two subsequent ACP catalog checks made no model requests. The candidate was
present at both projects under the direct user-enabled registration. The first
parser incorrectly searched selection values for the friendly name. The
[parser correction](evidence/2026-10-24-copilot-plugin-components/catalog-parser-correction.json)
retains the original raw result and identifies the native `plugin:probe` value.
The candidate's friendly name and selection value are distinct.

The original empty `--available-tools` argument also failed to provide a
reliable tool-free boundary. It is not reused as enforcement. Observed shell
commands inspected files and Git history; no model file-write tool was observed.

## Guarded retry

The retry requires the exact native namespaced agent ID and an explicit task
model matching the user's private selection. Missing model arguments, generic
agents, resumed/background tasks, unrelated reads and other tools are denied. A
private project `preToolUse` guard allows one exact task and its declared
resource read. It is test instrumentation, not a shipped permission adapter.

Before delegation, a real native `view` request is denied by this hook, with a
native tool error and matching private trace. Prompt sessions use normal
`--trust-folder` one-session admission. ACP rejects that flag; the rejected
request and cleanup are retained. An earlier unadmitted catalog attempt is also
retained. No saved trust setting or existing discovery preference is changed.

| Completed retry control     | Pin             | Project attempt                                                        | User attempt                                                                   |
| --------------------------- | --------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Native instruction positive | Copilot 1.0.103 | PASS: exact JSON object, empty rules                                   | PASS: same                                                                     |
| Installed rules             | Copilot 1.0.103 | FAIL: empty rules array                                                | FAIL: empty rules array                                                        |
| Matching/nonmatching access | Copilot 1.0.103 | Resource reads observed; required rule markers absent                  | Same; one fenced response also fails JSON contract                             |
| Candidate catalog           | Copilot 1.0.103 | Native positive present; plugin candidate absent under project overlay | Native positive and namespaced candidate present                               |
| Custom child                | Copilot 1.0.103 | NOT RUN: no catalog candidate; no fallback                             | Real namespaced child and resource read; final response has leading whitespace |
| Disabled/uninstalled        | Copilot 1.0.103 | Empty arrays; unqualified negatives                                    | Same                                                                           |
| Reinstalled rules           | Copilot 1.0.103 | FAIL: empty rules array                                                | FAIL: empty rules array                                                        |

The
[retry audit](evidence/2026-10-24-copilot-plugin-components/guarded-retry-audit.json)
records 16 parent sessions, one custom child, explicit model override and zero
alternate delegate selections. The native task and child `view` are correlated
in the private session transcript. The retained final response includes its
leading whitespace; it is not silently trimmed into an exact-marker PASS. This
establishes bounded delegation, not all child restriction semantics or a working
project-overlay route.

## Preservation and provenance

Native uninstall leaves an owned empty `installedPlugins` key. Original
auth-bearing configuration bytes stay only in memory and are restored in place
after checking unrelated values. The completed retry's measured configuration
hash and mode exactly match the original. Interrupted attempts also record the
removal of their new project-only settings entry after native uninstall.

The
[preservation record](evidence/2026-10-24-copilot-plugin-components/preservation.json)
and
[retry restoration](evidence/2026-10-24-copilot-plugin-components/guarded-retry-preservation.json)
retain measured results. Session, log, native cache and trust bookkeeping are
not whole-profile equality. No team guide or client cache is directly rewritten.

The
[source index](evidence/2026-10-24-copilot-plugin-components/private-source-index.json)
binds six private roots, including inventory variants and failed attempts.
Transcripts, model identifiers and private guard source stay outside the
repository. Public event selections omit thinking and auth-bearing values and
represent model agreement as Booleans. The initial four selection failures
remain visible beside the clean guarded retry.

## Offline checks and consequence

Final-review fixes pass all 65 offline tests and reject three further mutants
for resolved native homes, writes before rejection and tilde expansion. Native
user homes are resolved independently, including symlinked profiles; rejected
roots leave no new paths.

Tests precede harness changes. Initial missing helpers fail before the 54-test
suite passes; catalog and pin assertions then reach 55 tests. Selector/guard,
filename and metadata assertions extend this to 62 passing offline tests.
Requiring an explicit child model fails first, then passes. Nine adverse
mutations are rejected by the strengthened tests. No offline test calls a
client. Full-review assertions raise the passing suite to 63 tests and reject
seven additional root, exact-response, catalog and fixture mutations. Fixture
roots under `.copilot` or a configured `COPILOT_HOME` are rejected before
writes. Validation logs and their hashes are retained in the packet.

The affected proposal rows are
[Rules, Copilot](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules)
and
[Subagents, Copilot](../wip/specs/2026-10-23-customization-delivery-proposal.md#subagents).
The rule route remains a failed candidate. A bounded user custom-child route is
observed, with exact response and project activation limits. Failed positives
cannot establish a universal absence reader result. The task stays open; Linux,
IDE/hosted surfaces, complete preservation and integrated ownership remain
unqualified. Durable adoption waits for step 7.

After integration with merged Q1 and Q2, the
[main integration audit](evidence/2026-10-24-copilot-plugin-components/main-integration-audit.json)
records unchanged Q3 function syntax trees and exact syntax-tree equality with
main for the rest of the harness. Full verification passes all 79 offline tests
([validation](evidence/2026-10-24-copilot-plugin-components/validation.json)).
Native observations were not rerun or widened.
