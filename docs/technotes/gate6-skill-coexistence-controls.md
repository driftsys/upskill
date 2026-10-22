# Gate 6 skill coexistence and duplicate observations

Task [#356](https://github.com/driftsys/upskill/issues/356) retains the next
bounded [#337](https://github.com/driftsys/upskill/issues/337) native slice
after [Copilot skill controls](gate6-shared-skill-controls.md) and
[Claude route controls](gate6-claude-skill-controls.md). The
[shared-root evaluation](shared-content-root-evaluation.md) remains an unadopted
comparison. Production stays schema 1 and all 160 cells remain unqualified.
Recorded on 2026-10-22; retained Copilot events keep their original UTC
timestamps.

## Context and native accounting

A common disposable macOS arm64 project contained one portable `.agents/skills`
item, its self-contained installed Claude plugin export, and an unrelated manual
`.claude/skills` item. Each client used its own normally provisioned isolated
authentication/configuration root. Claude Code 2.1.303 resolved
`claude-sonnet-5-5`; Copilot CLI 1.0.98 used explicit `claude-sonnet-5.5`
through native provider `copilot`. The retained Claude events do not
independently identify the account provider route.

The [summary](evidence/2026-10-22-gate6-skill-coexistence/summary.json) accounts
for 18 distinct native sessions: 17 completed without native error and one
blocked by a revoked Claude OAuth token before any skill call. The user renewed
normal login before the subsequent successful Claude controls. Fifteen completed
sessions matched their declared response exactly; two did not. Sixteen native
skill calls have matching start/result identities: 14 successful calls and two
unsuccessful unavailable-skill calls. Native tool success is distinct from
matching the control's expected response.

Only native skill tools were exposed. Model runs used fresh sessions, empty
strict MCP configuration for Claude and disabled built-in MCP servers for
Copilot. No authentication contents are included. This is project evidence, not
user-scope qualification, despite using an isolated user authentication root.

## Common-project observations

| Control                                                     | Observation                                                               | Interpretation                                                                               |
| ----------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Initial Claude installed positive                           | Native error: OAuth token revoked; no tool call                           | Blocked calibration; never an absence pass                                                   |
| Claude installed positive after normal login                | One namespaced catalog entry and successful `Skill` call                  | Bounded installed-plugin positive                                                            |
| Copilot shared positive in the same project                 | One shared catalog entry and successful `skill` call                      | Bounded shared-route positive                                                                |
| Copilot request for Claude plugin namespace                 | Native skill call unsuccessful; exact absent fallback                     | Bounded unintended-namespace negative after working Copilot positive                         |
| Bare-name request to Claude with plugin enabled             | Model invoked the advertised namespaced plugin and returned its marker    | Failed declared absence expectation; not direct `.agents` discovery or a second native entry |
| Reinstalled Claude plugin positive                          | Namespaced skill works again                                              | Fresh installed-state calibration                                                            |
| Manual Claude-compatible item requested through each client | One successful call per client; correct manual marker                     | Intentional manual compatibility consumption survives                                        |
| Claude plugin disabled                                      | Catalog omits fixture plugin skill; no tool call; prose precedes fallback | Failed exact-response expectation; do not accept marker substring matching                   |
| Fresh reinstall before independent survival checks          | Namespaced skill works                                                    | Recalibrated positive for the remaining controls                                             |
| Copilot shared item with Claude plugin disabled             | Successful call and exact marker                                          | Selected shared route survives native Claude disable                                         |
| Claude plugin re-enabled                                    | Successful namespaced call and exact marker                               | Restored positive                                                                            |

Native project marketplace/install/disable/enable/uninstall commands and
configuration snapshots are retained in packet metadata. Each attempt removed
its owned plugin and marketplace in its finalization path. Native cache removal,
unknown registration recovery and preservation of unrelated registered plugins
are not established. As in the earlier packet, native removal leaves empty
`enabledPlugins` and `extraKnownMarketplaces` objects. No marketplace was
published and no production plugin was installed.

A bare-name natural-language request is not a path-specific discovery test. In
the failed Claude control, the native invocation named the installed plugin and
the loaded skill's base directory was inside that plugin. This supplies no new
positive for direct `.agents/skills` discovery. The initial working Claude
catalog advertised one plugin identity and no bare shared identity; this is a
bounded count at that state, not proof about every alias/import/scope.

## Copilot duplicate and alias fixtures

A separate disposable Copilot project declared a shared item, a byte-identical
same-name Claude-compatible copy and a renamed alias as one logical lineage. The
unrelated manual skill was a separate item. These were deliberate negative
fixtures, not a proposed installation layout.

| Catalog state                                      | Declared expected logical entries | Observed entries  | Result                                                                                    |
| -------------------------------------------------- | --------------------------------- | ----------------- | ----------------------------------------------------------------------------------------- |
| Shared baseline                                    | 1                                 | 1                 | Matches                                                                                   |
| Same-name shared and Claude-compatible copies      | 2                                 | 1, at shared path | Count expectation failed; precedence does not prove one physical installation             |
| Shared item and differently named compatible alias | 2                                 | 2                 | Both entries observed and separately invoked successfully                                 |
| Owned alias removed                                | 1                                 | 1                 | Shared item still works; removed alias invocation fails and returns exact absent fallback |

The manual compatible skill also remained invokable. Six completed sessions
covered the shared positive, alias positive, shared-with-alias, manual survivor,
restored shared positive and removed-alias negative. All six matched their
declared responses. The same-name count mismatch remains a failed diagnostic
expectation. The two alias entries show why planning must track captured logical
lineage, rather than treating a renamed compatible copy as another independent
item. Native precedence cannot replace complete selected-set duplicate checks.

## Preservation and capture limitations

Packet metadata retains before/after fixture contents, hashes and modes, plus
native command configuration snapshots. Common-project team files and source,
packaged and unrelated manual skill bytes/modes survived the native controls.
The unrelated project settings value and mode survived; original settings bytes
differ after the first native removal because empty native objects remain. Later
continuations began with those empty objects. They do not qualify exact
original-state recovery for the initial installation.

The private Copilot duplicate helper had a post-capture permissions error: its
receipt-mode loop also changed three owned fixture files from `0640` to `0600`
after native controls and snapshots. Its completion reported preservation false.
An independent audit verified unchanged bytes/hashes and restored only those
three owned fixture modes to `0640`; it also verified unchanged isolated user
settings.
[The final audit](evidence/2026-10-22-gate6-skill-coexistence/duplicate-helper-final-audit.json)
retains the error, guarded repair and declared/observed catalog counts. This is
not uninterrupted preservation or native lifecycle recovery evidence. No
personal file was affected and no native client is blamed for the helper error.

The capture helpers remain private experiment scripts, not shipped/tested
qualification runners. Selected original init/catalog, assistant, skill-result,
loaded-skill and final-result evidence is retained; requests, thinking, usage,
authentication stores and unrelated metadata are omitted. Paths are normalized
to project, packet and client-profile aliases. Complete private stream hashes
remain separate from retained-file hashes. The
[hash manifest](evidence/2026-10-22-gate6-skill-coexistence/receipt-hashes.json)
covers retained receipts and metadata; private manifest hashes are recorded
separately. The private packets are identified in the summary and metadata.

## Remaining gate

The two exact-response failures, count mismatch and authentication failure are
not converted to passing negatives. These observations do not select an adapter
or adopt a shared layout. All four clients, project/user scopes, macOS
arm64/Linux x86_64 and existing IDE obligations remain required. Codex/OpenCode
coexistence, configured/ancestor readers, complete aliases/scopes and plugin
exports, resources, automatic rule delivery, native rule matching and actual
subagent restrictions remain open. No timeout/interruption/descendant cleanup,
update, lock, backup or crash recovery was qualified.

Reuse the ownership, preflight and guarded recovery foundations at the relevant
implementation milestone. New owned-skill lifecycle remains after Gate 6 and
blocks adopting a new skill layout and releasing the combined MVP until its
integrated controls pass. Schema-2 activation remains blocked.
