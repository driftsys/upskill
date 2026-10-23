# Gate 6 OpenCode shared-skill and runtime-resource observations

Task [#366](https://github.com/driftsys/upskill/issues/366) continues the
bounded [#337](https://github.com/driftsys/upskill/issues/337) campaign and the
[shared-core authoring criteria](../wip/plans/2026-10-19-customization-delivery-reconciliation.md#shared-core-authoring-clarification-324).
It tests actual skill/resource behavior, not override execution, embedded
composition or a new owned-skill lifecycle. All 160 delivery cells remain
unqualified; no shared root or adapter is adopted and production stays schema 1.

## Context and declared sequence

The current
[OpenCode v2 skills documentation](https://dev.opencode.ai/v2/docs/skills/)
describes native and compatibility skill directories, relative supporting files
and separate skill/read permissions. That rolling documentation supplies a
candidate mechanism, not qualification for the pinned binary.

The actual experiment used OpenCode v2.0.32 on macOS arm64 in a fresh disposable
project, through its normally provisioned isolated profile. Every session export
identifies model `gpt-6.1-sol`, provider `github-copilot` and variant `default`.
Commands used `--standalone --format json`, without `--auto`. No credentials or
authentication contents were copied or retained. The isolated profile does not
make this user-scope delivery evidence.

One portable, consumption-ready `SKILL.md` instructed the agent to read
`references/review-procedure.md` relative to its skill directory. The resource
contained an unpredictable response marker. No client directive, override,
import/composition syntax, script or client-specific variant was placed in the
shared catalog. Client-specific invocation instructions remained in the manual
fixture guidance outside that catalog.

The same entry/resource directory moved from `.opencode/skills/<id>` to
`.agents/skills/<id>`; it was never exposed at both fixture paths
simultaneously. Project configuration remained unchanged: default deny, allow
that exact skill ID, and allow reads of the two exact candidate resource paths.
It added no project `skills` imports. Wider profile/configuration source
inventory remains unqualified; these observations do not establish exclusive
default discovery or absence of configured/ancestor/compatibility readers.

## Native results

| Control                               | Native tool evidence                                                           | Result                           |
| ------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------- |
| Native `.opencode/skills` calibration | One completed `skill`, then one completed `read` of that directory's resource  | Exact resource marker            |
| Shared `.agents/skills` candidate     | One completed `skill`, then one completed `read` bound to the shared directory | Same exact marker                |
| Supporting file temporarily absent    | Skill loads; actual resource `read` fails with `File not found`                | Exact missing-resource fallback  |
| Entrypoint temporarily absent         | Actual `skill` call fails with `Unable to load skill`; no resource read        | Exact unavailable-skill fallback |
| Shared entry and resource restored    | One completed `skill` and one completed shared-resource `read`                 | Exact resource marker again      |

The [summary](evidence/2026-10-22-gate6-opencode-shared-resources/summary.json)
accounts for five distinct successful native sessions and nine tool calls: seven
completed calls and the two expected native errors above. Each session reaches
idle outcome `succeeded`; expected tool errors remain errors. Every response
matches its declaration exactly. The failed reads/loads follow working
positives, and the final restored positive recalibrates the route. No negative
is inferred from provider failure or a no-attempt model refusal.

Selected native input paths bind each read, including the failed read, to its
actual fixture resource. Successful read results contain the resource marker.
This establishes bounded runtime file loading. It does not demonstrate an
embedded include feature, generated resource-copy behavior or the full review
procedure. These skill controls launch no review subagents; earlier child
controls remain separate evidence.

## Preservation, receipts and remaining limits

The
[fixture assessment](evidence/2026-10-22-gate6-opencode-shared-resources/fixture-and-assessment.json)
records exact entry/resource content and mode equality after restoration, plus
unchanged manual team guidance and project configuration bytes/modes. The final
entry location is intentionally the shared candidate; it is not a claim of
restoring the initial native layout or an upskill installation migration. Only
owned fixture files were moved temporarily. Unrelated global discovery and
configuration were not rewritten to force a result.

The
[private source index](evidence/2026-10-22-gate6-opencode-shared-resources/private-source-index.json)
retains the exact packet reference and original manifest/helper hashes. Selected
native CLI JSONL, session identities/models/idle outcomes, declarations and
snapshots are retained. Provider state, accounting and export message bodies are
omitted. Raw, selected and normalized hashes differ intentionally. All original
hashes and the separate
[retained manifest](evidence/2026-10-22-gate6-opencode-shared-resources/receipt-hashes.json)
were checked; private capture/retention helpers are not shipped or tested
qualification runners. The packet is informative data, not product input.

There are no native catalog counts, direct-plus-plugin duplicate tests or
four-client coexistence controls here. One controlled fixture entry at a time
does not prove absence of other physical or logical exposures. Other clients,
Linux x86_64, user scope, IDE surfaces, rules, agents/MCP inheritance, native
profile/cache effects, timeout cleanup and integrated ownership/update/removal/
recovery remain unqualified. Task 2/CD-11 and schema-2 activation remain
blocked.
