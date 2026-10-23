# Gate 6 Codex user-registration rule observations

Task [#374](https://github.com/driftsys/upskill/issues/374) continues
[#337](https://github.com/driftsys/upskill/issues/337), Task 2/CD-02 of the
[corrected plan](../wip/plans/2026-10-19-customization-delivery-reconciliation.md).
These are bounded macOS arm64 observations of Codex `0.163.0` user-profile
plugin registration beside manual team instructions. All 160 qualification cells
remain unqualified and production stays schema 1.

## Delivery, scope and ordinary trust

Two fresh local marketplace plugin identifiers are registered through normal
`codex plugin marketplace add` and `codex plugin add` commands in the already
provisioned isolated user profile. Each package has an independent rule body and
a `SessionStart` callback. The callback supplies its rule as
`additionalContext`; it does not edit team instructions or run model tools.

Registration is user-scoped, but the fixture callbacks deliberately accept only
two declared disposable project paths. Both projects have distinct unpredictable
manual `AGENTS.md` markers. This is not arbitrary-workspace user delivery,
project-scoped native registration or an adopted production adapter. No shared
catalog, import/glob, compatibility directory, other client or alternate direct
route is qualified here. Client-specific fixture packages stay outside shared
catalogs. The old trusted project probes remain unchanged.

Native registration succeeds before trust, but the first model positive returns
an empty rules array with no new callback. That positive remains failed. Native
UI inspection reports four installed `SessionStart` hooks, two active and two
requiring review. The first UI inspection saved trust for the disposable first
project when buffered input reached its folder prompt; this configuration change
is retained. A second recorded inspection confirms pending hook trust. The agent
did not approve the new hooks. After the user reviewed those exact definitions
through ordinary native trust, fresh model turns were run. Display labels remain
unnamed; source identities distinguish the definitions, with usability debt
[#346](https://github.com/driftsys/upskill/issues/346) still separate.

No authentication contents were inspected or copied. Native session metadata
binds provider `openai`, model `gpt-6.1-sol` and read-only sandbox policy.
Successful reinstall controls use the unchanged reviewed definitions without an
agent trust override. This fixture does not implement general hook, tool or
permission management.

## Controls and original failures

The first post-trust packet completes successful positives in both projects,
then its configuration guard stops before the first disable edit. The matcher
included adjacent TOML tables and therefore found two `enabled` values. It did
not perform that disable edit. The partial packet's final configuration equals
its pre-run bytes/mode. A synthetic preservation regression reproduces the
matcher failure and passes with a matcher bounded to one table. The partial
packet remains incomplete; its skipped controls are not passes.

A separate corrected run uses the same reviewed packages, projects, model and
policy. Disablement uses a guarded edit of only each fixture plugin's persisted
`enabled` value; this is not a native disable CLI or installed upskill
lifecycle. Native plugin inventory confirms every declared enabled/removed
state.

| Corrected native condition          | Project | Expected rules | Observed callbacks |
| ----------------------------------- | ------- | -------------- | ------------------ |
| Both enabled                        | A       | A and B        | One per rule       |
| Both enabled                        | B       | A and B        | One per rule       |
| Rule A disabled                     | A       | B              | One for B          |
| Both disabled                       | B       | None           | None               |
| Both re-enabled                     | B       | A and B        | One per rule       |
| Rule A removed through native CLI   | A       | B              | One for B          |
| Both removed through native CLI     | B       | None           | None               |
| Both reinstalled through native CLI | A       | A and B        | One per rule       |
| Both reinstalled through native CLI | B       | A and B        | One per rule       |

Every corrected turn completes with exactly the declared JSON object, allowing
JSON formatting and rule-array order. Manual team markers always survive. Native
session records contain each expected rule body as a separate developer message,
matching the package body exactly. Callback records bind session, startup
source, project path and rule identity. These once-per-rule observations are not
a full duplicate/discovery proof. No model tool calls occur.

The [summary](evidence/2026-10-23-gate6-codex-user-rules/summary.json) retains
twelve completed turns across all three phases: eleven matched responses and the
failed pre-trust positive. The partial runner guard failure is separate from
native response failures. Sixteen native rule messages bind sixteen fixture
callbacks. The corrected nine-turn run has twelve of those messages/callbacks.

## Preservation and evidence limits

The
[fixture assessment](evidence/2026-10-23-gate6-codex-user-rules/fixture-and-assessment.json)
records unchanged manual team bytes/modes, equal prior installed-item metadata
for thirteen registrations during installation, and ten cached package files
matching their source. Initial registration and trust change configuration;
there is no claim of equality to the pre-install configuration. The corrected
post-trust run ends with exact equality to its own configuration bytes/mode and
all fifteen starting installed-item records. The old project activation trace
also remains unchanged. This is selected preservation evidence, not a complete
native cache/cloud inventory or ownership/recovery qualification. New fixture
plugins remain installed and enabled; removal controls do not mean the final
profile returned to its pre-install state.

[Native session selections](evidence/2026-10-23-gate6-codex-user-rules/native-session-selections.json)
and per-turn CLI receipts retain the actual team/rule messages, final responses,
model and completion evidence. The
[command receipts](evidence/2026-10-23-gate6-codex-user-rules/commands.json)
retain native arguments, exit status and raw hashes. The
[source index](evidence/2026-10-23-gate6-codex-user-rules/private-source-index.json)
binds original manifests, frozen capture helpers, twelve owned native sessions,
the UI transcript through its original packet, trace snapshots and the synthetic
regression. All original and
[selected hashes](evidence/2026-10-23-gate6-codex-user-rules/receipt-hashes.json)
were checked. Account/provider state, reasoning records, unrelated catalogs and
accounting are omitted. Private helpers are not qualified runners; selected
records are informative data, not product inputs.

Remaining all-client/platform/scope/IDE, fresh/resume/compaction, complete
reader and compatibility inventory, aliases/imports/globs, unintended and
duplicate activation, subagent/MCP restrictions, timeout cleanup and integrated
ownership/update/removal/recovery obligations stay open. No root or rule adapter
is adopted, and schema-2 activation remains blocked.
