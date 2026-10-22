# Gate 6 authenticated project rule observations

Task [#345](https://github.com/driftsys/upskill/issues/345) retains a bounded
part of [#337](https://github.com/driftsys/upskill/issues/337), Task 2 of the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md).
These are native macOS arm64 project observations after normal authentication,
not qualification of a complete profile or an activation gate. All 160 matrix
cells remain unqualified. Production stays schema 1.

## Profiles and native completion

The user normally signed into the prepared Codex home through ChatGPT, signed
into the prepared OpenCode XDG root through GitHub Copilot OAuth, and reviewed
both Codex SessionStart definitions in `/hooks`. No credentials were copied or
exported. Hook trust was not bypassed. Both hooks displayed `Unnamed hook`; the
Source fields identified the two plugins. Display-label debt is
[#346](https://github.com/driftsys/upskill/issues/346). Current definitions were
left unchanged to preserve the reviewed trust decisions.

[Receipt metadata](evidence/2026-10-20-gate6-authenticated-project/summary.json)
retains commands, executable fingerprints, per-session callback records, team
snapshots, paired assessments and raw/retained receipt hashes. Codex is pinned
to 0.163.0 and OpenCode to 2.0.32. Controls explicitly select `gpt-6.1-sol` in
Codex and `github-copilot/gpt-6.1-sol` in OpenCode. The first Codex calibration
turn used its client default; the later reinstalled fresh positive explicitly
selects the model and calibrates the paired assessments.

Codex JSONL contains a completed turn and an agent response without a tool
event. OpenCode JSONL contains the response but no terminal completion event, so
native sanitized session exports supply the successful outcome, assistant
completion time, `finish: stop`, and final successful idle record. Session and
message identities match the current response; exported assistant content has no
tool part. Retained exports omit opaque provider state and unrelated transcript
content. A zero exit code alone does not establish completion.

## Observed controls

| Client   | Native condition                                | Response and callback observation                                               |
| -------- | ----------------------------------------------- | ------------------------------------------------------------------------------- |
| Codex    | Both plugins, fresh explicit-model turn         | Team marker and both rule markers; one startup event per rule                   |
| Codex    | Rule A disabled in persisted fixture config     | Team marker and rule B only; one B event                                        |
| Codex    | Both rules disabled in persisted fixture config | Team marker and empty rules; no rule events                                     |
| Codex    | Rule A removed through native CLI               | Team marker and rule B only; A cache absent                                     |
| Codex    | Both plugins removed through native CLI         | Team marker and empty rules; both caches absent                                 |
| Codex    | Both plugins reinstalled through native CLI     | Team marker and both rules; one event each, existing trust definitions retained |
| Codex    | Resume original session                         | Team marker and both rules; one resume event each                               |
| OpenCode | Both project plugin registrations, fresh        | Team marker and both rules; one context event each                              |
| OpenCode | Project registration A omitted                  | Team marker and rule B only; one B event                                        |
| OpenCode | Both project registrations omitted              | Team marker and empty rules; no rule events                                     |
| OpenCode | Resume original session                         | Team marker and both rules; one context event each                              |

Thirteen model turns include these cases and two unsuccessful Codex control
attempts. Six paired assessments returned `observed` through the existing strict
classifier; two command-line-override assessments failed. The classifier never
returns campaign qualification. Team bytes and mode `0640` match before and
after every turn. Exclusive configuration backups were used for temporary
fixture changes, and original bytes/modes were restored. Codex native removal
also removed each selected cache. The evidence does not demonstrate arbitrary
native/cloud recovery or an interrupted backup-restoration path.

Codex `-c plugins.<id>.enabled=false` invocation overrides did not disable
either hook. These are failed controls, not negative passes or an established
upstream bug. Persisted fixture configuration did control native enablement and
catalog state. Prospective adapters must not assume command-line overrides
provide isolation. OpenCode selections changed the owned project `plugins` array
and started a private server for each invocation; this does not qualify a native
hot-disable API, registration disposal, or live reload.

## Discovery and remaining work

After login, Codex's native catalog also listed account-level cloud plugins.
Retained catalog receipts include the fixture entries and an unrelated-entry
count, omitting unrelated identifiers. Those plugins were not removed or
changed. This is an observed discovery input, not proof that every account-level
plugin activated or that the profile has no other readers. Fixture user
registration/cache writes also do not qualify project-only write authority.

Raw stdout, invocation records and sanitized native session exports are outside
the repository at `/private/tmp/upskill-gate6-authenticated-20261020`. Retained
receipt hashes apply to their actual sanitized bytes. The reused capture helper
reports a combined stdout/stderr digest, separately from retained stdout hashes;
it did not retain raw stderr. Its existing process-group cleanup path was used,
but no independent live process-cleanup trace was retained. This evidence slice
introduces no automated model runner or native receipt parser. Those capture and
cleanup obligations remain before full qualification.

The later [Codex compaction note](gate6-codex-compaction.md) retains one actual
compaction record, subsequent rule messages and same-session callback counts. It
lacks independent launch/exit and immediate preservation receipts; it does not
qualify the whole compaction obligation.

Remaining compaction, update/trust-definition changes,
unintended/default/configured readers, nested/ancestor sources,
direct-plus-packaged duplicates, user-scope registration authority, Linux,
Claude Code, Copilot CLI and mixed-kind compatibility remain incomplete.
Authentication and trust are resolved only for the two stated project fixtures.
No production adapter is selected, no plugin is published, and schema-2
activation stays blocked under #337.
