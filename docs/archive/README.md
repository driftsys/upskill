# Documentation archive

This directory retains completed or superseded working documents for provenance.
An archive entry records its source paths, delivered or superseding commit or
PR, durable successor records, and any unfinished scope retained in WIP.

No historical feature plans have been classified as complete by the
documentation refactor. The [WIP inventory](../wip/README.md) tracks their
reconciliation. Archive each feature only after its completion evidence has been
checked; dates, approval labels and checked boxes are insufficient on their own.

## Legacy-state projection

Source paths: `docs/wip/specs/2026-10-12-legacy-state-projection-design.md` and
`docs/wip/plans/2026-10-12-legacy-state-projection.md`. The raw
[spec](specs/2026-10-12-legacy-state-projection-design.md) and
[plan](plans/2026-10-12-legacy-state-projection.md) are retained unchanged.

Implementation commit: `30cdabd7146b45b457756b38b024a55ec3a00de2`. Final
implementation and regression-test commit:
`52f72bcae142e302587f493dd0c418169e22478c`. Completion evidence: the task's full
verification and Rust 1.85 all-targets check passed; the final test-only fix
passed full verification and scoped task review. These are local implementation
commits, not a claim of a PR merge.

Durable successors:
[specification §4.1](../topics/0004-state-and-ownership.md#41-pure-legacy-state-projection),
[generation design](../topics/0004-state-and-ownership.md#architecture-generation-pipeline-pure-legacy-state-projection)
and [AD-0015](../adr/0015-pure-legacy-state-projection.md). The
[command guide](../book/commands.md) describes the unchanged command and
persistence boundary.

The
[foundation](../wip/specs/2026-10-11-agent-customization-foundation-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active:
source verification, historical rendering and resources, adoption,
requested-selection reconciliation, schema-2 lifecycle preservation, exact
backup and transactional execution are unfinished. Other WIP material retains
its existing disposition; this archive entry concerns only the named pair.

## Owned instruction operation planning

Source paths: `docs/wip/specs/2026-10-12-owned-instruction-planning-design.md`
and `docs/wip/plans/2026-10-12-owned-instruction-planning.md`. The archived
[spec](specs/2026-10-12-owned-instruction-planning-design.md) and
[plan](plans/2026-10-12-owned-instruction-planning.md) retain the original
material; parent-context links follow their active WIP destination and the
completed plan explicitly qualifies its historical RED sequencing deviation.

Implementation commit: `0a0fb93dd83589cb554df8d2d302d7bfdba40105`. Final
acceptance-test and documentation commit:
`7b8b52d3a09b66645a72b0a02647c58b9fa80c4d`. Local completion evidence: ordered
formatting and full verification passed with 725 Rust tests and 11
documentation-checker tests; Rust 1.85 all-targets checking passed. Fresh task
review approved functional quality without functional blockers. These commits
and checks do not claim a PR merge. Compile RED preceded code; valid behavioral
assertion RED followed an implementation draft. Subsequent stub assertions and
semantic mutation checks validate current behavior, while the historical
test-first process exception remains explicit in the plan.

Durable successors:
[specification §4.2](../topics/0006-artifact-planning.md#42-pure-owned-instruction-planning),
[planner design](../topics/0006-artifact-planning.md#architecture-owned-instruction-planning),
[AD-0016](../adr/0016-pure-instruction-planning.md),
[AD-0017](../adr/0017-shared-instruction-owner-evidence.md) and
[AD-0018](../adr/0018-retain-empty-instruction-aggregates.md).

The
[operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active.
Closeout status recorded before the whole-file planner:

> Source/filesystem evidence, complete reference authority, per-owner consumer
> association, whole-file/resource planning, mixed-state lifecycle integration
> and transactional execution remain pending. This entry resolves only the named
> pair; other WIP material retains its existing disposition.

Current successor: the
[pure whole-file planner](../topics/0006-artifact-planning.md#44-owned-whole-file-operation-plans)
is implemented for explicit complete file sets. The
[captured item compiler](../topics/0002-generation.md#45-captured-item-compilation)
derives complete per-client entrypoint/resource outputs, and the
[mixed coordinator](../topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../topics/0002-generation.md#414-captured-rule-desired-sets)
producer binds rule outputs to explicit roots and consumers. Integration with
lifecycle reconciliation and planning, qualified source/destination evidence,
durable state and transactional execution remain pending. The operational
lifecycle design and M1 plan remain active; the whole-file planning pair is
archived in its own entry below.

## Strict captured source manifests

Source paths: `docs/wip/specs/2026-10-12-strict-source-manifests-design.md` and
`docs/wip/plans/2026-10-12-strict-source-manifests.md`. The archived
[spec](specs/2026-10-12-strict-source-manifests-design.md) and
[plan](plans/2026-10-12-strict-source-manifests.md) retain the original
material; the spec's parent-context link follows its active WIP destination.

Implementation commit: `daee42f176d8f0a54e6c810f63f454e45083de96`. Final task
correction: `2247e818edf9a4900f4e64c7c9469cf1b21823c7`. Local completion
evidence: fresh task review approved without findings; ordered formatting and
full verification passed with 763 Rust tests and 11 documentation tests, and
Rust 1.85 all-targets checking passed. These local commits and checks do not
claim a PR merge. Compile and valid behavioral assertion RED preceded
implementation; semantic mutations exposed skipped entries/errors and digest
framing, mode, directory-path and inclusion regressions, with exact restoration.

Durable successors:
[specification §4.3](../topics/0003-sources.md#43-strict-captured-source-manifests),
[source manifest design](../topics/0003-sources.md#architecture-strict-source-manifests),
[AD-0019](../adr/0019-separate-strict-source-capture.md) and
[AD-0020](../adr/0020-framed-source-manifest-evidence.md).

The
[operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active.
Closeout status recorded before the whole-file planner:

> Whole-file/resource planning, immutable Git evidence, source stabilization,
> qualified observation, destination qualification, state/execution and Codex
> activation remain pending. Actual non-UTF-8 filesystem-name creation was
> unsupported on the development macOS filesystem; pure conversion rejection was
> tested there, with the real fixture gated to other Unix hosts. This archive
> entry changes no other WIP disposition.

Current successor: the
[pure whole-file planner](../topics/0006-artifact-planning.md#44-owned-whole-file-operation-plans)
is implemented for explicit complete file sets. The
[captured item compiler](../topics/0002-generation.md#45-captured-item-compilation)
derives complete per-client entrypoint/resource outputs, and the
[mixed coordinator](../topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../topics/0002-generation.md#414-captured-rule-desired-sets)
producer is implemented. Immutable Git evidence, listed as pending at that
historical stage, is now supplied by the
[selected lifecycle source boundary](../topics/0003-sources.md#applying-selected-lifecycle-rule-sources).
Lifecycle reconciliation and planning, source stabilization, qualified
observation, destination qualification, state/execution and Codex activation
remain pending. The source-manifest platform evidence above is retained as
recorded; the whole-file planning pair is archived in its own entry below.

## Owned whole-file operation planning

Source paths: `docs/wip/specs/2026-10-12-owned-file-planning-design.md` and
`docs/wip/plans/2026-10-12-owned-file-planning.md`. The archived
[spec](specs/2026-10-12-owned-file-planning-design.md) and
[plan](plans/2026-10-12-owned-file-planning.md) retain the working material; the
spec's parent-context links follow their active WIP destinations.

Implementation commit: `5a69568dde1d9a8f3857237ef7fea93e647e2afb`. Final
acceptance-test and documentation correction:
`d9853787ad3c73d0cfde463702d7e6e336cb803d`. Local completion evidence: ordered
formatting and full verification passed with 799 Rust tests and 11
documentation-checker tests, warning-free; fresh task review approved
specification compliance and quality without findings. Production code passed a
locked Rust 1.85 all-targets check at the implementation commit and was
unchanged by the test/documentation correction. Compile RED and two valid stub
assertion failures preceded implementation; seven semantic mutation controls
failed their intended assertions with exact production restoration. These local
commits and checks do not claim a PR merge. Linux and Windows runtime execution
remain for hosted CI.

Durable successors:
[specification §4.4](../topics/0006-artifact-planning.md#44-owned-whole-file-operation-plans),
[whole-file planner design](../topics/0006-artifact-planning.md#architecture-owned-file-planning)
and [AD-0021 through AD-0023](../adr/README.md#owned-whole-file-planning). The
[command guide](../book/commands.md) describes the unchanged command boundary.

The
[operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active.
The
[captured item compiler](../topics/0002-generation.md#45-captured-item-compilation)
derives complete entrypoint/resource sets, and the
[mixed coordinator](../topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../topics/0002-generation.md#414-captured-rule-desired-sets)
producer binds rule outputs to explicit roots and consumers. Integration with
lifecycle reconciliation and planning, qualified source/destination observation,
filesystem equivalence/containment, writer locking, permission policy, durable
state, apply/recovery and Codex activation remain pending. This prerequisite
does not complete M1-A2, M1-A3 or M1. Other WIP retains its prior disposition;
this entry resolves only the exact named pair.

## Captured item compilation

Source paths: `docs/wip/specs/2026-10-12-captured-item-compilation-design.md`
and `docs/wip/plans/2026-10-12-captured-item-compilation.md`. The archived
[spec](specs/2026-10-12-captured-item-compilation-design.md) and
[plan](plans/2026-10-12-captured-item-compilation.md) retain the working
material; the spec's parent-context link follows its active WIP destination.

Implementation commit: `03a0f5d51f183e0389316199611faeece605e52c`. Final
acceptance-test correction: `0282aa16c5bcb0dd144a8cc9da6820b669435f7b`. Local
completion evidence: ordered formatting and full verification passed with 822
Rust tests and 11 documentation-checker tests, warning-free; fresh task review
approved compliance and quality without findings. Production code passed a
locked Rust 1.85 all-targets check before the test-only correction, which left
production unchanged. Genuine compile RED and two valid stub assertion failures
preceded implementation. Eight actual semantic controls failed their intended
assertions, with exact production restoration and focused GREEN. These local
commits and checks do not claim a PR merge. Hosted Linux and Windows runtime
execution remains pending.

Durable successors:
[specification §4.5](../topics/0002-generation.md#45-captured-item-compilation),
[compiler design](../topics/0002-generation.md#architecture-captured-item-compilation)
and [AD-0024 through AD-0026](../adr/README.md#captured-item-compilation). The
[command guide](../book/commands.md) describes the unchanged command boundary.

The pure producer now derives complete generation entrypoint/resource sets from
captured values. The
[mixed coordinator](../topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../topics/0002-generation.md#414-captured-rule-desired-sets)
producer binds complete rule outputs. Integration with ownership and lifecycle
planning, qualified source/destination observation, writer locking, destination
permission policy, durable state, apply/recovery and native Codex activation
remain pending. Active matching body overrides are explicitly rejected pending
precedence implementation. The
[operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active;
this prerequisite completes neither M1-A2 nor M1-A3. Other WIP retains its prior
disposition; this entry resolves only the exact named pair.

## Mixed owned-artifact planning

The archived [spec](specs/2026-10-12-mixed-owned-planning-design.md) and
[plan](plans/2026-10-12-mixed-owned-planning.md) preserve the exact working
material. Their parent-context links are historical relative references; the
operational lifecycle design and M1 delivery plan remain active in WIP.

Implementation and reviewed corrections: `7d00798` through `73fb891`. Local
completion evidence: 851 Rust tests and 11 documentation-checker tests, ordered
formatting and full verification, locked Rust 1.85 all-targets check and
whitespace check passed; fresh independent task review approved compliance and
quality. Genuine missing-API RED and valid stub assertion failures preceded
implementation. Six initial semantic controls and three review-fix controls
failed intended assertions with exact production restoration and focused GREEN.
The cross-form collision control bypassed both the initial membership check and
redundant duplicate-action guard. These checks establish local supplied-value
behavior; hosted runtime execution, final branch/PR review and merge remain
separate gates.

Durable successors:
[specification §4.6](../topics/0006-artifact-planning.md#46-mixed-owned-artifact-operation-plans),
[mixed planning design](../topics/0006-artifact-planning.md#architecture-mixed-owned-artifact-planning)
and [AD-0027](../adr/0027-compose-complete-owned-artifact-plans.md). The
coordinator composes complete explicit file/contribution sets; connecting
captured compiler outputs to lifecycle ownership and evidence remains pending.
Source/target qualification, complete reference authority, per-owner consumers,
filesystem safety, permission policy, writer locking, durable state and
apply/recovery remain deferred. No native, CLI or schema-2 activation follows;
M1-A2, M1-A3 and M1 remain incomplete. Other WIP retains its prior disposition;
this entry resolves only the exact named pair.

## Qualified scope roots and writer locking

Source paths: `docs/wip/specs/2026-10-13-qualified-scope-lock-design.md` and
`docs/wip/plans/2026-10-13-qualified-scope-lock.md`. The archived
[spec](specs/2026-10-13-qualified-scope-lock-design.md) and
[plan](plans/2026-10-13-qualified-scope-lock.md) preserve the exact working
bytes committed at `c25d3a8`. Before final archival, that source commit added
two blank lines required by Markdown formatting to the plan at its original WIP
path; the spec was unchanged. Parent-context and working-pair references remain
historical references.

Implementation and reviewed correction: `eeabc81` and `4a80c9d`. Local
completion evidence: the initial complete suite passed 862 main-harness tests;
the reviewed correction passed 12 real-filesystem integration tests, two scope
unit tests, locked Rust 1.85 all-targets compilation, warning-denying clippy and
documentation checks. Fresh independent task review and rereview approved the
corrected slice. Missing-API RED and independent semantic controls cover
locking, independent descriptions, binding validation, close-on-exec,
acquisition ordering, canonical relocation and final-symlink refusal. A
deliberate descriptor-leak control failed the bounded cleanup check.
Root-controller full-suite verification after the correction, hosted
Linux/Windows checks, branch/PR review and merge remain separate gates; these
local commits do not claim a PR merge.

Durable successors:
[specification §4.7](../topics/0007-filesystem-access.md#47-qualified-scope-roots-and-advisory-writer-locking),
[scope-lock design](../topics/0007-filesystem-access.md#architecture-qualified-scope-locking)
and [AD-0028](../adr/0028-lock-qualified-scope-directories.md). The working spec
introduced no requirements beyond that early durable contract.

The API pins explicitly selected ambient roots and excludes cooperating writers
on Linux/macOS without persistent files. It activates no CLI lifecycle or schema
change and promises no descendant containment, fork-safe lease or network
filesystem behavior. Native non-UTF-8 support remains conditional on filesystem
acceptance; local APFS refused the fixture with EILSEQ. Qualified observation,
destination equivalence/containment, source stabilization, permission policy,
state, execution, recovery and broader platform qualification remain unfinished.
The 34 other WIP files retain their existing disposition unchanged; this entry
resolves only the named pair and does not complete the M1 filesystem stage.

## Destination evidence regression controls

Source paths:
`docs/wip/specs/2026-10-13-destination-evidence-controls-design.md` and
`docs/wip/plans/2026-10-13-destination-evidence-controls.md`. The archived
[spec](specs/2026-10-13-destination-evidence-controls-design.md) and
[plan](plans/2026-10-13-destination-evidence-controls.md) preserve the exact raw
working bytes. Their working-path references are historical provenance. The
provisional supplier name `Mount::query` in spec line 23 and plan line 106 was a
sketch. The implemented private native-query supplier is `Mount::queries`,
passed by production callers to `Mount::of`. This as-built clarification
resolves the PR review naming finding without rewriting the raw archives.

Implementation: `621f19c`; reviewed PR head: `2a50ca6`.
[PR #273](https://github.com/driftsys/upskill/pull/273) is open, pending final
merge. Local APFS evidence passed 28 public and 22 private destination cases,
with 14 independent native omission controls producing assertion RED and exact
restoration before focused GREEN. On 2026-10-13,
[Actions run 38334882688](https://github.com/driftsys/upskill/actions/runs/38334882688)
passed at full head `2a50ca6a5163e11912cbcee7987593c68e0a4197`. Ubuntu job
`115092610337` passed 28 public destination cases, the native missing returned
mount-ID mask case and four separate profile/mask check and call-wiring
assertion omissions. Each omission restored `qualification.rs` to SHA-256
`1c5b288ed575ae1ccdd99a10170c13aa9054f92004a0927a64fb1587bac40bd5`, followed by
22 private cases GREEN. All 13 head checks succeeded, including Windows
unsupported-boundary job `115092610428`. Final-head CI after temporary harness
removal and final review remain merge gates; this entry makes no merge claim.

Durable successors:
[specification §4.8](../topics/0007-filesystem-access.md#48-qualified-destination-observations),
[qualified destination design](../topics/0007-filesystem-access.md#architecture-qualified-destination-observation)
and [AD-0029](../adr/0029-capture-qualified-destination-evidence.md). These
current records establish the implemented inactive qualified destination and
writer-lock boundaries. The separate
[qualified source design](../topics/0003-sources.md#architecture-qualified-source-observation)
and [AD-0030](../adr/0030-capture-qualified-source-evidence.md) now establish
inactive source observation; final-head hosted source qualification remains
required. Earlier archive entries describe their dated slices; statements there
that these boundaries remain pending are historical, not current successor
status. Integrating qualified source evidence into the lifecycle, permission
policy, durable state and apply/recovery remain pending. No qualified Windows
backend, real privileged mount or fault behavior is established. The 34 other
WIP files retain their existing disposition unchanged; this entry resolves only
the named pair and does not complete M1.

## Qualified source tree observation

The archived [spec](specs/2026-10-13-qualified-sources-design.md) and
[plan](plans/2026-10-13-qualified-sources.md) retain the exact final working
bytes from their respective `docs/wip/specs/` and `docs/wip/plans/` paths. Their
raw relative references remain unchanged. The plan includes controller-approved
completion marks captured before archival; the raw spec's pending status is
historical.

Durable successors:
[specification §4.9](../topics/0003-sources.md#49-qualified-source-tree-observations),
[source observation design](../topics/0003-sources.md#architecture-qualified-source-observation)
and [AD-0030](../adr/0030-capture-qualified-source-evidence.md). These
authorities existed before implementation; archival does not redirect their
citations.

Earlier implementation and task-reviewed corrections ended at
`a4e84d9334f7717bfb170d59afcbaf7f829d1e82`. Local formatting, full verification,
locked warning-denying all-target Clippy and Rust 1.85 compilation passed: 964
outer Rust tests and 11 documentation-checker tests, zero failures. Independent
controls record 43 valid assertion failures with exact restoration and focused
success; two historical discarded experiments are excluded. Fresh task re-review
approved the scoped diagnostic correction. This records local delivery only;
final-head hosted Linux/Windows, branch/PR review and merge remain separate
gates.

The opaque read-only snapshot captures and non-refreshingly revalidates complete
source evidence using qualified roots and shared mount policy. Legacy capture,
CLI, dependencies and schema 1 remain unchanged. It establishes no atomic tree
snapshot, authenticity, external-writer exclusion or qualified Windows backend.
The final whole-branch correction resolves initial public source-binding context
and independently pins retained-descendant full-path diagnostics, while keeping
the original scope cause and immutable manifest. Current focused source coverage
at `a393263` was 12 public and 31 private tests; combined independent omission
evidence had 46 valid controls and the same two historical discarded attempts.
The later PR closeout independently pins the final revalidation binding check
and adds positive qualified empty-root/directory-only fixtures plus a real
private unsupported-host source-method assertion.
[CI run 38346202172](https://github.com/driftsys/upskill/actions/runs/38346202172)
and
[CodeQL run 38346200154](https://github.com/driftsys/upskill/actions/runs/38346200154)
succeeded for `a393263` on 2026-10-13. Its Linux source mask/non-UTF-8 cases
executed natively; Windows checked scope rejection and compiled the source API
without calling its source helper. Scoped PR review and final-head hosted
checks, including native Windows execution of the new source assertion, remain
required. Source lifecycle integration, permission policy, durable state,
effects and recovery remain pending; M1-A2, M1-A3 and M1 are not complete. All
34 foreign WIP files and 18 earlier raw archive files retain their original
bytes and disposition; this entry resolves only the assigned pair.

## Explicit file permission selection

The archived [spec](specs/2026-10-13-file-permissions-design.md) and
[plan](plans/2026-10-13-file-application.md) retain the exact final working
bytes from their respective `docs/wip/specs/` and `docs/wip/plans/` paths. The
plan includes controller completion marks; its original nine-control list and
the spec's unimplemented status remain historical. The appended task brief
clarified special-bit-only source examples and the tenth independently valid
control.

Durable successors:
[specification §4.10](../topics/0007-filesystem-access.md#410-explicit-file-permission-selection),
[permission design](../topics/0007-filesystem-access.md#architecture-file-permission-policy)
and [AD-0031](../adr/0031-select-explicit-file-permissions.md). These records
existed before implementation and remain the current authorities.

Implementation commit: `b3a8acde76a81817fd39b662591d1e1a5421d11b`. Independent
Task 1 review approved the bounded pure API. Local full verification passed with
978 outer Rust tests and 11 documentation-checker tests; fresh locked Rust 1.85
all-target checking and warning-denying Clippy passed. Ten compiled semantic
assertion controls recorded exact restoration and focused success; one invalid
attempt is excluded. These are local checks, not hosted or numbered PR review.

The API selects supplied permission bits without filesystem or permission
effects. Owner/group, ACLs, extended metadata, containment, atomic replacement,
application, journal/recovery, actual resulting state and lifecycle activation
remain pending. No binary-size, performance or native-effect claim follows;
M1-A2, M1-A3 and M1 remain incomplete. All 54 inherited WIP/raw archive files,
including 34 foreign WIP files, retain their original bytes and disposition. The
broader application draft remains separate and unchanged. This entry resolves
only the assigned pair.

## Gate 5 combined lifecycle state

Source path: `docs/wip/plans/2026-10-15-m1-gate5-combined-state.md`. The
[archived plan](plans/2026-10-15-m1-gate5-combined-state.md) retains its
original bytes, including historical unchecked task boxes. Only this completed
pure state slice is archived. Its broader
[Gate 5 specification](specs/2026-10-15-m1-gate5-lifecycle-integration.md)
remains active, as explicitly required by the approved slice-completion scope.
All other WIP retains its prior disposition.

Durable successors:
[specification §4.13](../topics/0004-state-and-ownership.md#413-combined-lifecycle-state-foundation)
and
[combined lifecycle state design](../topics/0004-state-and-ownership.md#architecture-combined-lifecycle-state).
The [validation record](../technotes/m1-gate5-state-validation.md) records local
checks, independent review and omission-control evidence. Implementation through
`490b8e29a27040dac8172dfc5b4f582991029f56` passed full verification and locked
Rust 1.85 checking; subsequent changes strengthened tests. Final native/hosted
CI, PR review and merge remain separate gates. These are local slice records and
make no merge claim.

Compiler-to-desired-set reconciliation and shared-owner lifecycle planning,
exact original-JSON first-write backup, guarded executor/recovery integration,
and observational list/doctor/preview adapters remain unfinished. Commands still
use schema 1; this slice does not complete Gate 5 or activate M1.

## Gate 5 captured rule desired sets

Source path: `docs/wip/plans/2026-10-15-m1-gate5-rule-desired-sets.md`. The
[archived plan](plans/2026-10-15-m1-gate5-rule-desired-sets.md) preserves its
exact original bytes, including historical unchecked boxes and its historical
relative spec link. The active umbrella remains at
`docs/wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md`. The approved
completion scope archives only this completed task 2a plan; all other WIP
retains its prior inventory and disposition.

Durable successors:
[specification §4.14](../topics/0002-generation.md#414-captured-rule-desired-sets),
[desired-set design](../topics/0002-generation.md#architecture-captured-rule-desired-sets)
and the [validation note](../technotes/m1-gate5-desired-validation.md).
Implementation through `9228cc1` passed full local verification and locked Rust
1.85 all-target checking. Subsequent test-only quick-review fixes passed the
18-test focused suite with exact production restoration. Sixteen assertion
controls and the independent review of `ff4671f..9228cc1` are documented in the
validation note. Final checks after gardening and hosted CI remain delivery
gates; this entry makes no merge claim.

Task 2b still owns authoritative reference and cross-request identity
reconciliation, shared owners and per-owner consumers, selected legacy
conflicts, instruction contribution adapters and complete add/update/remove
planning. Guarded backup/application/recovery and observational adapters follow.
Commands remain on schema 1; Gate 5, M1 and native activation remain incomplete.

## Gate 5 authoritative reference reconciliation

Source path: `docs/wip/plans/2026-10-16-m1-gate5-reference-reconciliation.md`.
The [archived plan](plans/2026-10-16-m1-gate5-reference-reconciliation.md)
retains its historical relative scope link and unchecked boxes. Its current
scope is the active
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md). Only this
completed graph slice is archived; shared-owner lifecycle planning, guarded
backup/recovery and observational adapters remain unfinished.

Durable successors are
[specification §4.15](../topics/0005-lifecycle-planning.md#415-authoritative-lifecycle-reference-reconciliation),
the
[reference design](../topics/0005-lifecycle-planning.md#architecture-lifecycle-reference-reconciliation)
and [validation note](../technotes/m1-gate5-reference-validation.md).
Implementation through `4bfc357` passed local full verification, locked Rust
1.85 checking and hosted CI. These checks do not establish native activation,
Gate 5 completion or a PR merge. Commands remain on schema 1.

## Gate 5 shared-owner reference removal

Source path: `docs/wip/plans/2026-10-16-m1-gate5-retirement.md`. The
[archived plan](plans/2026-10-16-m1-gate5-retirement.md) preserves its exact raw
bytes, including historical unchecked task boxes and relative spec links. The
current umbrella remains at
[Gate 5 lifecycle integration](specs/2026-10-15-m1-gate5-lifecycle-integration.md);
the
[M1 owned lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
also remains active. The approved scope archives only this completed plan. All
other WIP retains its prior disposition.

Durable successors:
[specification §4.16](../topics/0005-lifecycle-planning.md#416-reference-driven-shared-owner-removal-planning),
[removal design](../topics/0005-lifecycle-planning.md#architecture-lifecycle-reference-removal)
and [validation note](../technotes/m1-gate5-retirement-validation.md). Original
local implementation `b5c9537`, rebased as `911915d`, passed full local
verification and locked Rust 1.85 checking with 11 acceptance examples.
Independent implementation review found no production defects. Subsequent review
corrections strengthened the focused suite to 15 passing tests; 16
assertion-failing controls established independent coverage with exact
production restoration. One symmetrical legacy fixture attempt is excluded.
Final checks after gardening, hosted delivery and PR review remain separate
gates; this entry makes no merge claim.

This pure rule-only removal API preserves surviving shared owners and their
consumer union, validates affected supplied observations and returns complete
state plus individual file actions. Shared add/update, new sharing contracts,
first-write backup, guarded execution/recovery and observational adapters remain
unfinished. Commands remain on schema 1; Gate 5, M1 and native activation remain
incomplete.

## Gate 5 captured selection preflight

Source path: `docs/wip/plans/2026-10-16-m1-gate5-selection.md`. The
[archived plan](plans/2026-10-16-m1-gate5-selection.md) preserves its exact raw
bytes, including historical unchecked boxes and relative spec links. Current
scope remains in the active
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md) and the
[M1 owned lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md).
The approved completion scope archives only this selection plan; all other WIP
retains its prior disposition.

Durable successors:
[specification §4.17](../topics/0005-lifecycle-planning.md#417-captured-lifecycle-selection-preflight),
[selection design](../topics/0005-lifecycle-planning.md#architecture-lifecycle-selection)
and [validation note](../technotes/m1-gate5-selection-validation.md). The 20
public acceptance examples passed locally, and 23 controls failed their intended
assertions with exact source restoration. Full local verification and locked
Rust 1.85 checking passed. Independent review and hosted delivery remain
separate gates; this entry makes no merge claim.

This pure library boundary resolves all/literal-name requests and reports every
selected legacy, unsupported, ambiguous or unknown-name blocker before later
planning. Source/scoped selection, CLI precedence, complete add/update planning,
new sharing compatibility, guarded backup/application/recovery and observational
adapters remain unfinished. Commands remain on schema 1; Gate 5, M1 and native
activation remain incomplete.

## Gate 5 complete single-owner upsert planning

Source path: `docs/wip/plans/2026-10-16-m1-gate5-upsert.md`. The
[archived plan](plans/2026-10-16-m1-gate5-upsert.md) preserves its exact
original bytes, including unchecked task boxes and historical relative spec
links. The active scope remains in the
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md), the
[M1 owned lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md),
[captured-rule desired-set design](../topics/0002-generation.md#architecture-captured-rule-desired-sets)
and
[selection design](../topics/0005-lifecycle-planning.md#architecture-lifecycle-selection).
Only this bounded plan is archived; the umbrella and all other WIP retain their
existing disposition.

Durable successors:
[specification §4.18](../topics/0005-lifecycle-planning.md#418-complete-single-owner-lifecycle-upsert-planning),
[upsert design](../topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert)
and [validation record](../technotes/m1-gate5-upsert-validation.md). The
supplied local evidence records 33 passing public acceptance examples and
twenty-three assertion/panic controls with exact source restoration. The
implementation is based on `ed4ab8dc018989d9f5e4485ce469aeb8baa89f1d`.
Formatting, all-target sanity, full verification and Rust 1.85 checking passed;
independent review and hosted checks remain separate delivery gates. This is a
local slice record and makes no merge or delivery claim. The raw plan declares
an unboxed `Ready(UpsertPlan)`; the implementation uses `Ready(Box<UpsertPlan>)`
to keep the outcome variants within the project's lint requirement.

The coordinator joins complete rule desired sets, explicit selection,
authoritative reference reconciliation and mixed-artifact planning. Empty sets
retire outputs without losing referenced history. Affected shared artifacts and
new sharing remain rejected. Shared add/update contracts, full diagnostic
aggregation, source/scoped selectors, CLI precedence, instruction adapters,
guarded first-write backup, execution/recovery and observational adapters remain
unfinished. Production commands and schema 1 remain unchanged; Gate 5, M1 and
native activation remain incomplete.

## Per-owner recorded contracts

Source paths: `docs/wip/specs/2026-10-16-m1-owner-contracts.md` and
`docs/wip/plans/2026-10-16-m1-owner-contracts.md`. PR #301 implements the
bounded inactive contract boundary; the parent implementation commit is
`d809fe2f10d221874fdf0af8c7cc5636ad40adf1`. This entry does not claim a merge.

The archived [spec](specs/2026-10-16-m1-owner-contracts.md) retains its original
bytes. The [plan](plans/2026-10-16-m1-owner-contracts.md) corrects its umbrella
reference after relocation. Its
[original raw bytes](../technotes/evidence/2026-10-16-m1-owner-contracts/original-archived-plan.md.gz)
remain available as compressed provenance; the evidence manifest records their
uncompressed digest. Other active and future WIP retains its existing
disposition.

Durable successors:
[specification §4.19](../topics/0004-state-and-ownership.md#419-per-owner-recorded-contract-evidence),
[design](../topics/0004-state-and-ownership.md#architecture-lifecycle-owner-contracts),
[AD-0034](../adr/0034-record-explicit-owner-contracts.md), and
[validation evidence](../technotes/m1-owner-contracts-validation.md). Shared
reconciliation, production activation and native qualification remain pending.

## Explicit shared-output reconciliation

Source paths: `docs/wip/specs/2026-10-17-m1-shared-reconciliation.md` and
`docs/wip/plans/2026-10-17-m1-shared-reconciliation.md`. The raw
[spec](specs/2026-10-17-m1-shared-reconciliation.md) and
[plan](plans/2026-10-17-m1-shared-reconciliation.md) are retained unchanged.

The archived spec uses `operations::plan_owned_artifacts` as a shortened symbol.
The actual function is `operations::plan::plan_owned_artifacts`; the raw archive
bytes are retained for provenance.

The contracted producer starts at `c9f453042c954ab2698942c4cbe4e2ee4878ba13`;
shared planning and acceptance examples were reconciled from the stable working
implementation supplied by the dispatcher. The dispatcher reports 1,242 passing
Rust tests, thirty-six focused examples, Rust 1.85 and sanity exit 0; see the
[execution evidence](../technotes/evidence/2026-10-17-m1-shared-reconciliation/manifest.json).
This is local completion evidence, not a claim of merge or hosted qualification.

Durable successors:
[specification §4.20](../topics/0005-lifecycle-planning.md#420-explicit-shared-output-reconciliation),
[captured desired sets](../topics/0002-generation.md#architecture-captured-rule-desired-sets),
[shared upsert](../topics/0005-lifecycle-planning.md#architecture-lifecycle-upsert-explicit-shared-upsert),
[owner contracts](../topics/0004-state-and-ownership.md#architecture-lifecycle-owner-contracts),
[AD-0034](../adr/0034-record-explicit-owner-contracts.md),
[AD-0035](../adr/0035-project-shared-artifacts-for-physical-planning.md),
[AD-0036](../adr/0036-separate-explicit-sharing-entrypoints.md) and
[validation limits](../technotes/m1-shared-reconciliation-validation.md). Other
WIP remains at its original paths with unresolved disposition. Qualified
integration, instruction adapters, guarded application/recovery, production
activation and native qualification remain unfinished.

## Guarded original-state backup prerequisite

Source paths: `docs/wip/specs/2026-10-17-m1-guarded-backup.md` and
`docs/wip/plans/2026-10-17-m1-guarded-backup.md`. The
[spec reading copy](specs/2026-10-17-m1-guarded-backup.md) uses canonical
Markdown formatting. Its exact original bytes are retained as
[compressed evidence](../technotes/evidence/2026-10-17-m1-guarded-backup/original-archived-spec.md.gz),
with hashes in the evidence manifest. The
[plan](plans/2026-10-17-m1-guarded-backup.md) retains its original bytes. Both
retain historical relative WIP references and unchecked delivery tasks. The
archived spec's relative umbrella link is historical: resolve it as
`../specs/2026-10-15-m1-gate5-lifecycle-integration.md` from the archived spec
directory. The bounded local implementation is reconciled against the working
code based on `f54c457a30473511b66349ab4d95f30ae62d654b`; this entry makes no
merge or hosted qualification claim.

Durable successors:
[specification §4.12](../topics/0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery),
[qualified execution design](../topics/0008-execution-and-recovery.md#architecture-qualified-file-execution),
[AD-0037](../adr/0037-require-create-only-backup-journals.md) and the
[validation record](../technotes/m1-guarded-backup-validation.md). The
dispatcher supplies local full Rust-suite, Rust 1.85, Clippy, independent review
and three restored assertion-failing publication/policy/sync controls; the
validation record distinguishes final verification and delivery gates.

This opt-in library primitive preserves exact original bytes with create-only
publication and forward recovery. It does not validate JSON or activate
production lifecycle writes. The
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md) remains
active: combined guarded application, observational adapters and native
qualification remain separate work. All other WIP remains at its original paths
with unchanged bytes and unresolved disposition; only this named pair is
archived.

## Documentation concern consolidation

Source paths: `docs/wip/specs/2026-10-17-documentation-consolidation.md` and
`docs/wip/plans/2026-10-17-documentation-consolidation.md`. The raw
[spec](specs/2026-10-17-documentation-consolidation.md) and
[plan](plans/2026-10-17-documentation-consolidation.md) retain their original
bytes, including historical WIP references and unchecked task boxes.

Durable successors: the eight numbered
[concern references](../topics/README.md), the retained
[CLI specification](../specification/specification.md), and the
[technical-note index](../technotes/README.md). Existing design and format paths
remain compatibility entrypoints. Technical-note filenames no longer contain
capture dates; internal observation dates, revisions and evidence provenance
remain in their records. ADR identifiers remain independent.

Local verification of the consolidation based on
`faae29892caa4ad0e79dec66f530324f1435c92c` passed `just docs-check`, `just fmt`
and `just verify` before this archive step. The documentation checker covers
repository links, rendered book anchors, publication isolation and schema
snapshots. The dispatcher also verified 235 captured assets against the parent
revision byte for byte before formatting. These checks describe local
validation, not merged or hosted publication evidence; final verification and
review remain delivery gates.

Only this consolidation pair is archived. All other WIP retains its current
paths and unresolved disposition; this documentation migration does not mark
product implementation slices complete or activate library capabilities.

The commit hook exposed a separate preservation boundary: Prim stdin formatting
does not apply `.primignore`. The staged formatter now excludes the captured
schema pack explicitly. A regression example covers root and nested schema
bytes, license whitespace, and continued formatting of neighboring research
prose; all three evidence examples failed before the correction.

The [compatibility inventory](../compatibility.json) records 223 historical
repository anchors and 235 captured asset hashes from the parent revision.
Documentation regression checks reject unreferenced anchor removal and changed
license or provenance bytes; the inventory is enforced by `just docs-check`.

## Read-only lifecycle inspection

Source path: `docs/wip/plans/2026-10-17-lifecycle-inspection.md`. The bounded
[inspection plan](plans/2026-10-17-lifecycle-inspection.md) retains its exact
original bytes, including unchecked task boxes. Its historical relative spec
link resolves to the active Gate 5 umbrella at
`../specs/2026-10-15-m1-gate5-lifecycle-integration.md`.

Durable successors: the
[inspection contract and design](../topics/0005-lifecycle-planning.md#read-only-lifecycle-inspection)
and [validation record](../technotes/m1-lifecycle-inspection.md). The local
implementation is based on `f61afbf`; ten integration examples, the read-only
journal unit test, `just msrv-check`, and full `just verify` passed. The full
verification rerun outside the sandbox passed after initial sandbox process
tests were denied permissions. This is local completion evidence, not a merge or
hosted qualification claim.

Only this completed plan is archived. The Gate 5 umbrella remains active:
Source-backed command orchestration and complete lifecycle acceptance remain
pending. Captured preview and applying adapters are implemented. Production
commands and schema 1 stay unchanged. All other WIP retains its original paths
and unresolved disposition.

## Read-only lifecycle preview

Source path: `docs/wip/plans/2026-10-17-lifecycle-preview.md`. The bounded
[preview plan](plans/2026-10-17-lifecycle-preview.md) retains its exact original
bytes, including unchecked task boxes. Its historical relative spec link
resolves to the active Gate 5 umbrella at
`../specs/2026-10-15-m1-gate5-lifecycle-integration.md`.

Durable successors: the
[preview contract and design](../topics/0005-lifecycle-planning.md#read-only-lifecycle-preview)
and [validation record](../technotes/m1-lifecycle-preview.md). The local slice
is based on `a74350e90b5b577cc4babb41e30dc1753be3547f`. Ten preview acceptance
examples and the 33 upsert and 18 retirement regressions passed. Full
`just verify` passed with 1,283 tests passing, no failures and one existing
ignored test; Rust 1.85 all-targets checking and independent review passed.
These are local completion checks, not merged or hosted qualification evidence.

Only this completed bounded plan is archived. The
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md) is now
archived after local combined acceptance. Captured adapters and source-backed
orchestration are implemented; final delivery review, hosted CI and merge remain
required. Production commands stay on schema 1 pending Gate 6. All other WIP
retains its original paths and unresolved disposition.

## Selected lifecycle source resolution

The bounded [source plan](plans/2026-10-18-lifecycle-sources.md) is archived
after local implementation, full verification, Rust 1.85 checking and
independent review. The
[source contract](../topics/0003-sources.md#selected-lifecycle-rule-sources) and
[validation note](../technotes/m1-gate5-sources-validation.md) describe selected
local/Git rule roots, private retained captures and immutable revisions. An
inherited Git index confinement defect was reproduced and corrected before
delivery. Hosted final-head checks remain separate evidence. Command
orchestration and whole-lane acceptance remain under the active Gate 5 umbrella;
production commands remain on schema 1.

## Lifecycle origin and captured command adapters

The [origin plan](plans/2026-10-18-lifecycle-state-origin.md) is retained after
its bounded delivery in PR #314, merged as
`77181b90222473345dab6b46a3665828649361b9`. Its durable records are
[AD-0038](../adr/0038-bind-lifecycle-origin-to-original-backup.md),
[origin state](../topics/0004-state-and-ownership.md#durable-lifecycle-origin),
[bound application](../topics/0008-execution-and-recovery.md#origin-bound-lifecycle-application)
and the [validation note](../technotes/m1-lifecycle-state-origin.md).

The [captured adapter plan](plans/2026-10-18-lifecycle-commands.md) is retained
after local implementation and independent review. Its durable successors are
the
[adapter design](../topics/0005-lifecycle-planning.md#captured-lifecycle-command-adapters)
and [validation note](../technotes/m1-captured-lifecycle-commands.md). The
shared Gate 5 umbrella and completion lane remain active for source-backed
command orchestration and full lifecycle acceptance. Production writes remain on
schema 1 pending Gate 6 qualification.

## Source-backed lifecycle commands and final acceptance

The bounded [orchestration plan](plans/2026-10-18-lifecycle-orchestration.md)
retains the source-backed command implementation record. Its durable successors
are the lifecycle planning concern and
[orchestration validation note](../technotes/m1-gate5-orchestration-validation.md).
The [final acceptance plan](plans/2026-10-18-gate5-final-acceptance.md) records
whole-lifecycle process testing; the
[acceptance matrix](../technotes/m1-gate5-final-acceptance.md) preserves its
results and limits, including staging-residue debt #316. The shared umbrella and
completion lane remain active until final integration and delivery
qualification. Production activation remains Gate 6 work.

## Combined lifecycle delivery lane

The completed raw
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md) and
[completion plan](plans/2026-10-18-gate5-completion.md) retain their original
bytes and historical status lines. Current contracts and evidence are in the
[combined lifecycle topic](../topics/0005-lifecycle-planning.md#combined-owned-content-lifecycle-contract)
and [final acceptance note](../technotes/m1-gate5-final-acceptance.md). Local
full verification, Rust 1.85 and all ten acceptance criteria passed. Final
delivery review, hosted CI and merge remain separate requirements. Production
activation and native Codex qualification remain Gate 6.

Historical links from archived records to the former WIP umbrella now resolve
through this archive entry. Older per-slice pending-work prose records its
original checkpoint; the final acceptance note is the current delivery
authority. The archived spec SHA-256 is
`916606a7078be19fca4cafab69b0ae9b118d476bcdc9fd88a2441a5c08bef5ec`; the archived
completion plan SHA-256 is
`816ac4d108884b6ccc8d76d5713363f92757c7211db49969ec44d17a556d248f`.

## Bounded Gate 6 native instruction qualification

Source paths: `docs/wip/specs/2026-10-18-gate6-native-qualification.md` and
`docs/wip/plans/2026-10-18-gate6-qualification.md`. The raw
[spec](specs/2026-10-18-gate6-native-qualification.md) and
[plan](plans/2026-10-18-gate6-qualification.md) retain their original bytes.

Implementation base: local commit `26add38`; the final review-fix revision and
delivery checks are pending. The durable successor is the informative
[native qualification note](../technotes/m1-gate6-native-qualification.md),
which preserves sanitized native observations, offline harness coverage and
qualification limits. Final verification, review, hosted CI and merge remain
separate delivery gates.

Only this bounded harness and evidence pair is archived. Later delivery and
activation tasks in the raw plan remain unfinished. OpenCode discovery, user
scopes and broader native coverage remain unqualified; these observations do not
complete Gate 6 or activate schema 2. Production Rust and schema-1 command
routing are unchanged. All other WIP retains its original paths and disposition.

## Captured Codex instruction boundary

Source paths: `docs/wip/specs/2026-10-18-codex-instruction-boundary.md` and
`docs/wip/plans/2026-10-18-codex-instruction-boundary.md`. The raw
[spec](specs/2026-10-18-codex-instruction-boundary.md) and
[plan](plans/2026-10-18-codex-instruction-boundary.md) retain their original
bytes. Their SHA-256 digests are, respectively,
`1db4b78d3222e0ca2565dbf36a0b22cae7f01aabb3bc2df8f6c21f98e00041f4` and
`658cd7817435af9653f660895bd0b5a0833f545245e1ba152166c78ebda6c975`. Historical
WIP links and task checkboxes inside the raw documents are preserved.

Local implementation commits: `e55ba39` (family audience/directives), `446c14b`
(captured producer and acceptance), and `22bdaf8` (boundary hardening). Task
#322 completion evidence includes 14 dedicated acceptance tests, eight semantic
mutation controls, clean independent task reviews, and passing full
verification, sanity and Rust 1.85 checks. These local checks do not claim a PR
merge or hosted CI completion.

Durable successors are the [content format](../topics/0001-content-format.md),
[generation contract and detailed design](../topics/0002-generation.md#captured-codex-instruction-boundary),
[AD-0039](../adr/0039-compile-captured-codex-instructions.md), and
[command boundary](../book/commands.md). The implementation and acceptance
contracts are in `src/lifecycle/desired/codex.rs` and
`tests/codex_instruction_boundary.rs`.

Only the bounded producer is complete. Source-orchestration integration,
production activation, native qualification and staging accounting in #316
remain separate gates. Delivery/review tasks in the archived plan remain
pending; the archive records the finished implementation slice rather than whole
M1 or Gate 6 completion. All other WIP, including the existing accepted
sequencing marker in `docs/wip/README.md`, retains its original bytes, paths and
disposition.

## Source-backed Codex instruction lifecycle

Task #325 adds selected local/Git Codex rule compilation and explicit source
orchestration as opt-in library interfaces. Its archived
[spec](specs/2026-10-18-codex-source-integration.md) and
[plan](plans/2026-10-18-codex-source-integration.md) retain the exact bytes from
`docs/wip/specs/2026-10-18-codex-source-integration.md` and
`docs/wip/plans/2026-10-18-codex-source-integration.md` at implementation head
`3efd46033d7dfd0801da45f3780c31b3156fb4fd`. Feature base:
`2c64620e3ba3d7e613e2c46abea6919ad2ed86bc`. These are local implementation
commits, with no claim of PR merge or hosted acceptance. References and
unchecked delivery steps inside the raw plan remain historical working material.

Durable successors:

- [Selected lifecycle rule source requirements](../topics/0003-sources.md#selected-lifecycle-rule-sources)
  and
  [source integration architecture](../topics/0003-sources.md#explicit-codex-source-integration).
- [Source-backed lifecycle requirements and architecture](../topics/0005-lifecycle-planning.md#source-backed-lifecycle-orchestration).
- [Captured Codex instruction boundary](../topics/0002-generation.md#captured-codex-instruction-boundary).
- [AD-0039](../adr/0039-compile-captured-codex-instructions.md), including the
  explicit route, retained representation, declared-root authority and separate
  reader-report decisions.
- [Command/library boundary](../book/commands.md).

Completion evidence: dedicated source/orchestration acceptance examples and
reviewed root-preflight corrections are committed at the implementation head.
The dispatcher's full `just fmt`, `just verify`, `just sanity` and
`just msrv-check` runs passed after those changes. Verification included all 17
documentation checker tests, rendered book checks and Rust 1.85 all-targets
checking. Task reports and scoped review evidence remain in the local
`.superpowers/sdd/2026-10-18-codex-source-integration/` evidence directory.

Production CLI routing, schema 1, generation clients and defaults remain
unchanged. Complete all-kind adapters, conflict aggregation, native
user/IDE/Linux/OpenCode qualification, activation and staging accounting in #316
remain separate gates. Final branch/PR review and hosted delivery remain
separate work. Archival completes only this bounded implementation pair.

The other 17 specs, 19 plans and `docs/wip/README.md` retain their exact paths
and bytes. Their existing sequencing disposition is recorded in the WIP
inventory; this archive does not resolve broader M1 or updater work.
