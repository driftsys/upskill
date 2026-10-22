# Gate 6 OpenCode path controls with unchanged policy

Task [#363](https://github.com/driftsys/upskill/issues/363) continues the
bounded [#337](https://github.com/driftsys/upskill/issues/337) native campaign
after the [earlier child controls](gate6-opencode-child-controls.md). That
earlier path positive had an extra broad edit allowance; its receipts and
limitation remain unchanged. This new pair uses a different disposable project
and one unchanged agent/configuration throughout both turns. No route is
adopted; all 160 delivery cells remain unqualified and production stays
schema 1.

## Declared control and native observations

OpenCode v2.0.32 ran on macOS arm64 in project scope with its normally
provisioned isolated profile. The explicit model was
`github-copilot/gpt-6.1-sol`; both actual child exports identify that provider
and model with variant `default`. No credentials or authentication contents were
copied or retained. Runs used `--standalone --format json`, without `--auto`.

One file-backed `.opencode/agents/*.md` child had an unchanged body and ordered
permissions: deny all, allow reading `*.txt`, allow editing `sentinel.txt`. The
project parent configuration declared default denial and one allowance to launch
that named child. Both agent and parent configuration snapshots remained
byte/mode identical before, during and after the two fresh turns. The only
declared parent run prompt difference was the target filename. Actual delegated
messages also vary: the denied turn repeats manual guidance about avoiding
shell, permission and configuration changes. The delegated prompts are not
identical apart from their target filenames.

| Control                 | Actual native behavior                                                                                      | Fixture effect                                                               |
| ----------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Matching `sentinel.txt` | One parent delegation; child read call completes and the sentinel patch succeeds                            | Exact declared marker bytes, mode 0640; inspected before guarded restoration |
| Nonmatching `other.txt` | Fresh parent/child; read succeeds, then one patch receives `permission.rejected`, `Permission denied: edit` | Protected file and restored sentinel retain original bytes and modes         |

The [summary](evidence/2026-10-22-gate6-opencode-policy-parity/summary.json)
records four distinct native sessions, two exact parent responses and six tool
calls: two completed delegations, three completed child calls and one rejected
child patch. Both children reach native idle outcome `succeeded`: the second
completes its instruction by reporting the expected rejection. That outcome does
not convert its rejected operation into success. This is an attempted native
denial, not a no-attempt refusal.

Selected patch-header receipts bind each actual patch call ID to the owned
fixture resource. Native sanitized exports redact inputs; full own-session
exports were inspected in memory only to select patch headers and hashes. Full
unsanitized exports and provider state were not saved. The captured header paths
resolve to exactly the declared sentinel or protected file within the fixture.

## Preservation and evidence limits

The
[fixture snapshots and assessment](evidence/2026-10-22-gate6-opencode-policy-parity/fixture-and-assessment.json)
retain the predeclared policy, commands, agent, parent configuration, manual
team guidance, sentinel and protected resource. The successful marker was
verified before restoring only the owned sentinel; final snapshots match the
original bytes and modes. This establishes bounded fixture preservation, not
installed upskill lifecycle, unknown native profile effects or crash recovery.

The
[source index](evidence/2026-10-22-gate6-opencode-policy-parity/private-source-index.json)
identifies the original private packet and manifest/helper hashes. Selected
parent JSONL and child exports omit provider state, accounting, snapshots and
child tool output bodies. Original and retained hashes intentionally differ; all
original manifest entries were verified before retention. A separate
[retained manifest](evidence/2026-10-22-gate6-opencode-policy-parity/receipt-hashes.json)
covers the seven other evidence files. Private capture/retention helpers are not
shipped or tested qualification runners, and this evidence is not product input.

The earlier pair fails a direct policy-equality check because its positive
appended an extra edit glob. This new pair addresses that comparability limit in
its own context; it does not retroactively change earlier declarations or
qualify a complete profile. Manual native permissions do not establish support
for the historical generated permission map. Shared catalogs, coexistence and
duplicate exclusion, scoped rules, MCP/delegation inheritance, user scope, Linux
x86_64, other clients/IDE surfaces and integrated ownership/recovery remain
unqualified. Task 2/CD-11 are incomplete and schema-2 activation remains
blocked.
