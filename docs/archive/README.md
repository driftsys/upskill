# Documentation archive

This directory retains completed or superseded working documents for provenance.
An archive entry records its source paths, delivered or superseding commit or
PR, durable successor records, and any unfinished scope retained in WIP.

The [triage of 2026-11-23](#triage-of-2026-11-23) classified the feature
documents that the documentation refactor had left in WIP. Archive a feature
only after its completion evidence has been checked; dates, approval labels and
checked boxes are insufficient on their own. The [WIP index](../wip/README.md)
lists the documents that are still active.

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

The [foundation](specs/2026-10-11-agent-customization-foundation-design.md) and
[M1 plan](plans/2026-10-11-instructions-rules-m1.md) were active until
2026-11-11 (the foundation was archived on 2026-11-23): source verification,
historical rendering and resources, adoption, requested-selection
reconciliation, schema-2 lifecycle preservation, exact backup and transactional
execution are unfinished. Other WIP material retains its existing disposition;
this archive entry concerns only the named pair.

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
[operational lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](plans/2026-10-11-instructions-rules-m1.md) were active until
2026-11-11. Closeout status recorded before the whole-file planner:

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
lifecycle design and M1 plan were active until 2026-11-11; the whole-file
planning pair is archived in its own entry below.

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
[operational lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](plans/2026-10-11-instructions-rules-m1.md) were active until
2026-11-11. Closeout status recorded before the whole-file planner:

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
and [AD-0021 through AD-0023](../adr/README.md). The
[command guide](../book/commands.md) describes the unchanged command boundary.

The
[operational lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](plans/2026-10-11-instructions-rules-m1.md) were active until
2026-11-11. The
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
and [AD-0024 through AD-0026](../adr/README.md). The
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
[operational lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](plans/2026-10-11-instructions-rules-m1.md) were active until
2026-11-11; this prerequisite completes neither M1-A2 nor M1-A3. Other WIP
retains its prior disposition; this entry resolves only the exact named pair.

## Mixed owned-artifact planning

The archived [spec](specs/2026-10-12-mixed-owned-planning-design.md) and
[plan](plans/2026-10-12-mixed-owned-planning.md) preserve the exact working
material. Their parent-context links are historical relative references; the
operational lifecycle design and M1 delivery plan were active in WIP until
2026-11-11.

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
umbrella was
[Gate 5 lifecycle integration](specs/2026-10-15-m1-gate5-lifecycle-integration.md);
the [M1 owned lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md)
was also active until 2026-11-11. The approved scope archives only this
completed plan. All other WIP retains its prior disposition.

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
scope remained in the
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md) and the
[M1 owned lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md),
which were active until 2026-11-11. The approved completion scope archives only
this selection plan; all other WIP retains its prior disposition.

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
links. The active scope remained in the
[Gate 5 umbrella](specs/2026-10-15-m1-gate5-lifecycle-integration.md), the
[M1 owned lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md),
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
[original raw bytes](https://github.com/driftsys/upskill/blob/archive/lifecycle-2026-11/docs/technotes/evidence/2026-10-16-m1-owner-contracts/original-archived-plan.md.gz)
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
[execution evidence](https://github.com/driftsys/upskill/blob/archive/lifecycle-2026-11/docs/technotes/evidence/2026-10-17-m1-shared-reconciliation/manifest.json).
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
[compressed evidence](https://github.com/driftsys/upskill/blob/archive/lifecycle-2026-11/docs/technotes/evidence/2026-10-17-m1-guarded-backup/original-archived-spec.md.gz),
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
repository anchors and 235 captured asset hashes from the parent revision. On
2026-11-13 the 196 hashes of the removed M1 evidence were removed with that
evidence; 39 asset hashes remain, and the removed evidence is on the branch
`archive/lifecycle-2026-11`. Documentation regression checks reject unreferenced
anchor removal and changed license or provenance bytes; the inventory is
enforced by `just docs-check`.

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

## Rust CodeQL extraction coverage

Source path: `docs/wip/plans/2026-11-04-codeql-extraction-coverage.md`. The
[plan](plans/2026-11-04-codeql-extraction-coverage.md) was reconciled before
archival, including updates to its completion checkboxes. Requirements remain in
[issue #344](https://github.com/driftsys/upskill/issues/344); there is no
separate WIP spec. The durable successor is the
[extraction note and linked receipts](../technotes/codeql-rust-extraction.md).

Implementation and plan reconciliation are complete. Candidate `f6fdb61` passed
full local verification, Rust 1.85 checking and Linux/macOS/Windows CI. Matched
CodeQL 2.27.4 runs report 193 clean / zero errored files for that candidate and
188 clean / five errored files with the original source, retaining all 193 Rust
paths. The seven conditional blocks preserve runtime behavior. Delivery requires
the restoration and record commit plus exact final-head hosted checks, tracked
in [PR #462](https://github.com/driftsys/upskill/pull/462). This archive records
the experiment and does not assert a merge. Only the named plan is archived. All
other WIP retains its original paths, bytes and existing disposition.

## Customization re-steer: withdrawn lifecycle working documents

Source paths: `docs/wip/plans/2026-10-11-instructions-rules-m1.md`,
`docs/wip/plans/2026-10-12-owned-artifact-records.md`,
`docs/wip/plans/2026-10-19-customization-delivery-reconciliation.md`,
`docs/wip/plans/2026-10-25-q11-cross-client-isolation.md`,
`docs/wip/plans/2026-10-30-q11b-native-name.md`,
`docs/wip/specs/2026-10-12-m1-owned-lifecycle-design.md`,
`docs/wip/specs/2026-10-18-gate6-activation-design.md`,
`docs/wip/specs/2026-10-25-q11-registration-isolation-follow-up.md`,
`docs/wip/specs/2026-10-27-q11b-project-profile.md` and
`docs/wip/specs/2026-10-30-q11b-native-name-profile.md`. They were moved with
only their relative links updated for this location. The documents that were
already in this archive before 2026-11-11 keep their original bytes. A relative
link in one of them to a document that this entry moved from `docs/wip` no
longer resolves; the moved document is in `plans/` or `specs/` of this archive
under the same file name.

- [Instructions and rules M1 plan](plans/2026-10-11-instructions-rules-m1.md)
- [Owned artifact records plan](plans/2026-10-12-owned-artifact-records.md)
- [Customization delivery reconciliation plan](plans/2026-10-19-customization-delivery-reconciliation.md)
- [Q11 cross-client isolation plan](plans/2026-10-25-q11-cross-client-isolation.md)
- [Q11b native-name plan](plans/2026-10-30-q11b-native-name.md)
- [M1 owned lifecycle design](specs/2026-10-12-m1-owned-lifecycle-design.md)
- [Gate 6 activation design](specs/2026-10-18-gate6-activation-design.md)
- [Q11 registration isolation follow-up](specs/2026-10-25-q11-registration-isolation-follow-up.md)
- [Q11b project profile](specs/2026-10-27-q11b-project-profile.md)
- [Q11b native-name profile](specs/2026-10-30-q11b-native-name-profile.md)

These documents belong to the gated lifecycle approach that
[AD-0042](../adr/0042-resteer-customization-delivery.md) withdrew on 2026-11-11.
The library code that the lifecycle and owned-artifact documents planned was
removed from `main` and is kept on the branch `archive/lifecycle-2026-11`. The
Gate 6 activation design planned production activation on that library code, and
the customization delivery reconciliation plan planned changes to modules that
were removed. The Q11 and Q11b documents planned native trials; their scripts
and tests stay on `main`. The trial results they produced stay in
`docs/technotes/` as research.

## Customization storage and delivery candidates

The
[storage and delivery candidates](specs/2026-10-23-customization-storage-delivery-candidates.md)
draft is a byte copy of a file that existed only as an untracked file in another
checkout. It is an unapproved comparison. It was never adopted, and it is
superseded by the
[re-steer design](specs/2026-11-11-customization-resteer-design.md) of
2026-11-11. Its relative links are not updated for this location.

The following observation was kept in `docs/wip/README.md` until 2026-11-23 and
is retained here unchanged.

### OpenCode explicit-path observation

The draft reports that explicit OpenCode instruction paths avoid the glob's
cache/token behavior in the user's experience. This remains user-reported and
unqualified; Q5's failed positives do not compare cache behavior. The
[2026-10-24 user decision](specs/2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24)
now selects explicit paths, subject to Q5 retry PASS, and lists the amendment
from AD-0040's plugin-only wording to delivery independent of team `AGENTS.md`.
Criterion 7 remains: compare native and hook using identical rule bytes and
tasks, cached and uncached tokens, one client at a time. No performance
advantage or production route is qualified by this retained context
([Q5 packet](https://github.com/driftsys/upskill/blob/6ec187f5422d72e9329c4c7f9cb706468e2ebb6b/docs/technotes/gate6-opencode-current-routes.md),
[AD-0040](../adr/0040-separate-customization-delivery-from-discovery.md)).

## Customization adapter model and delivery roadmap

Source path: `docs/wip/specs/2026-10-14-customization-adapter-roadmap.md`. The
[adapter roadmap](specs/2026-10-14-customization-adapter-roadmap.md) was moved
with only its relative links updated for this location. It is superseded by the
[re-steer design](specs/2026-11-11-customization-resteer-design.md) of
2026-11-11, which replaces its four-client MVP sequencing and its proposed
qualification gates.

## Per-source update prompts

Source paths: `docs/wip/specs/2026-11-12-per-source-update-prompts-design.md`
and `docs/wip/plans/2026-11-12-per-source-update-prompts.md`. The archived
[spec](specs/2026-11-12-per-source-update-prompts-design.md) and
[plan](plans/2026-11-12-per-source-update-prompts.md) retain the working
material; the spec's link to the re-steer design points to the archived copy of
that design.

Delivered by PR #502 (commit `dbb6504`, keep an edited file of an unchanged item
on update) and PR #503 (commit `b727999`, one question per source in a terminal
update). Completion evidence: tests passed on Linux, macOS and Windows. Open
debt is tracked in issues #504 to #514.

Durable successors: [AD-0043](../adr/0043-per-source-update-prompts.md),
[specification §2.2](../specification/specification.md#22-update) and the
[command guide](../book/commands.md#upskill-update-name). The archived spec
records the review decisions made during implementation as "Author's call".

## Customization MVP re-steer

Source paths: `docs/wip/specs/2026-11-11-customization-resteer-design.md` and
the plans `docs/wip/plans/2026-11-11-customization-resteer-part1.md` (delivery
steps 0 to 4), `docs/wip/plans/2026-11-13-customization-resteer-step5.md` (step
5) and `docs/wip/plans/2026-11-13-customization-resteer-part2.md` (steps 6 to
8). The archived [design](specs/2026-11-11-customization-resteer-design.md) and
plans ([part 1](plans/2026-11-11-customization-resteer-part1.md),
[step 5](plans/2026-11-13-customization-resteer-step5.md),
[part 2](plans/2026-11-13-customization-resteer-part2.md)) are retained
unchanged. The approved plan names the three plan files as one set.

Delivered on `main` by these pull requests: step 0 #466 (remove the unreachable
lifecycle library), step 1 #467 (fix #465), step 2 #468 (ownership record,
`remove` by recorded files), step 3 #489 (`update` keeps modified files and
refuses unowned destinations), step 4 #490 (interrupted install or update
completes on the next run), step 5 #523 (scope in every command, global Copilot
paths), step 6 #524 (recorded commits, `update --check`), step 7 #525, #529 and
#531 (support levels, smoke test, first smoke runs), step 8 #526
(`update
--notice`), #528 (session-start hook trial) and #532 (sources of the
hook plugins). Completion evidence: the tests of these pull requests passed in
hosted CI, and the smoke runs of 2026-11-14 and 2026-11-15 are recorded in the
[support levels](../book/support-levels.md). No release holds this work yet; the
last release is v0.8.1, and the release is a separate step.

Durable successors: [AD-0042](../adr/0042-resteer-customization-delivery.md),
the [command guide](../book/commands.md),
[support levels](../book/support-levels.md), the
[recipes](../book/recipes.md#report-available-updates-when-a-session-starts),
[specification](../specification/specification.md) §2 to §4, and the technotes
[T1](../technotes/t1-copilot-duplicate-visibility.md) and
[T2](../technotes/t2-session-notice-display.md). The design's section 7 (parking
unused code) is recorded in AD-0042 Decision 5.

## Gate 6 complete preflight and conflict reporting

Source path: `docs/wip/plans/2026-10-18-gate6-complete-preflight.md`. The file
was never committed under `docs/wip/`; it was an untracked file in the worktree
of the branch `docs/gate6-activation-plan`. The raw
[plan](plans/2026-10-18-gate6-complete-preflight.md) is retained unchanged. Its
SHA-256 is `05ad05dbb8696a3b420d3d5f57cee270a4481c2fcbba47277fbc321e716f3c0a`.

Delivered by pull request #331, merge commit
`4d26d640919469febd56b6b6783bbfbbed12dd5b`, on 2026-10-19. The plan has no
separate spec; it names the
[Gate 6 activation design](specs/2026-10-18-gate6-activation-design.md).

The plan describes code under `src/lifecycle/`. Pull request #466 (commit
`2a4b29e`) later removed that directory, so the code that #331 delivered is not
in the current tree. The
[re-steer entry](#customization-re-steer-withdrawn-lifecycle-working-documents)
and [AD-0042](../adr/0042-resteer-customization-delivery.md) record that
removal. The plan has no durable successor: it is kept for provenance only.

## Bundle plugin output (`upskill pack`)

Source paths: `docs/wip/specs/2026-11-18-bundle-plugin-output-design.md` and
`docs/wip/plans/2026-11-19-upskill-pack.md`. The raw
[spec](specs/2026-11-18-bundle-plugin-output-design.md) and
[plan](plans/2026-11-19-upskill-pack.md) are retained unchanged, except that one
relative link was changed to follow the moved files: the plan's link to the hook
item kind design. It pointed into `docs/wip/specs/` until 2026-11-24 and now
points to the archived draft (see "Hook item kind draft" below). Step 7 of the
plan still says that the hook item kind spec stays in `docs/wip/`; that was true
when the plan was written.

Delivered by pull request [#569](https://github.com/driftsys/upskill/pull/569),
squash commit `553590a`, on 2026-11-19, for story
[#563](https://github.com/driftsys/upskill/issues/563). Completion evidence: all
CI checks of the pull request passed on its last commit `69b80e3`. Review
findings that were not fixed are tracked in
[#570](https://github.com/driftsys/upskill/issues/570). The delivered scope is
`upskill pack`, `add --kind`, the run lock of `update --notice`, and the `kinds`
record that `update` keeps for a bundle. The spec's open item "whether `pack`
accepts a bundle from a remote source" was settled as follows: `pack` resolves
the entry bundle in the current directory, and a `requires` entry with a
`source` is fetched from that source.

Durable successors: [AD-0044](../adr/0044-plugin-output-of-a-bundle.md), the
[command guide](../book/commands.md#upskill-pack-bundle---source-source---out-dir),
the [recipe](../book/recipes.md#publish-a-bundle-as-client-plugins),
[support levels](../book/support-levels.md#bundle-plugin-output) and
[specification](../specification/specification.md) §2. The
[hook item kind design](specs/2026-11-18-hook-item-kind-design.md) was a later
slice; [AD-0046](../adr/0046-plugin-mvp-components.md) replaced it on 2026-11-24
(see below).

## Hook item kind draft

Task [#623](https://github.com/driftsys/upskill/issues/623). The
[hook item kind draft](specs/2026-11-18-hook-item-kind-design.md) of 2026-11-18
was never approved for implementation. The design session of 2026-11-24 replaced
it. Nothing of it is implemented. The draft is retained unchanged, except for
its status line, which names its successors, and one relative link that follows
the move.

Durable successor: [AD-0046](../adr/0046-plugin-mvp-components.md), Decisions 3
to 7. The [plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md)
holds the detail and stays in WIP until the stories of stage 2 are delivered.

## MCP servers in `pack` output

Source path: `docs/wip/plans/2026-11-24-pack-mcp-servers.md`. The raw
[plan](plans/2026-11-24-pack-mcp-servers.md) is retained unchanged, except that
its link to the plugin MVP design now points to `docs/wip/specs/`. It is the
plan of story [#633](https://github.com/driftsys/upskill/issues/633), one of the
seven stories of the
[plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md). That design
is not archived: the other six stories (#631, #632, #634 to #637) are not
implemented.

Implementation: pull request
[#639](https://github.com/driftsys/upskill/pull/639). Completion evidence: the
CI checks of that pull request.

Durable successors: [AD-0046](../adr/0046-plugin-mvp-components.md), Decisions
2, 9 and 11; [specification §2.8](../specification/specification.md); the
[command guide](../book/commands.md#upskill-pack-bundle---source-source---out-dir);
the [recipe](../book/recipes.md#publish-a-bundle-as-client-plugins).

## LSP servers in `pack` output

Source path: `docs/wip/plans/2026-11-24-pack-lsp-servers.md`. The raw
[plan](plans/2026-11-24-pack-lsp-servers.md) is retained unchanged, except that
its link to the plugin MVP design now points to `docs/wip/specs/`, and that its
architecture paragraph and Task 2 record the move described below. It is the
plan of story [#634](https://github.com/driftsys/upskill/issues/634), one of the
seven stories of the
[plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md). That design
is not archived: the stories #631, #632 and #635 to #637 are not implemented.

Task 2 of the plan, the stderr report of the LSP servers that `add` and `update`
skip, is not delivered by this plan. It moved to story
[#632](https://github.com/driftsys/upskill/issues/632), which adds the skip
mechanism that the report needs. Story #634 therefore stays open for that part.

Implementation: the pull request of branch `feat/pack-lsp-servers` (Task 1
only). Completion evidence: the CI checks of that pull request.

Durable successors: [AD-0046](../adr/0046-plugin-mvp-components.md), Decisions
10 and 11; [content format](../topics/0001-content-format.md#lsps-map);
[format spec](../specification/format-spec.md);
[specification §2.8](../specification/specification.md); the
[command guide](../book/commands.md#upskill-pack-bundle---source-source---out-dir);
the [recipe](../book/recipes.md#publish-a-bundle-as-client-plugins).

## Command item kind in lint, fmt, new and `pack` output

Source path: `docs/wip/plans/2026-11-24-command-item-kind.md`. The raw
[plan](plans/2026-11-24-command-item-kind.md) is retained unchanged, except that
its link to the plugin MVP design now points to `docs/wip/specs/`. It is the
plan of story [#632](https://github.com/driftsys/upskill/issues/632), one of the
seven stories of the
[plugin MVP design](../wip/specs/2026-11-24-plugin-mvp-design.md). That design
is not archived: the stories #631 and #635 to #637 are not implemented.

The same pull request also delivered the stderr report of the LSP servers that
`add` and `update` skip. That report is the last criterion of story
[#634](https://github.com/driftsys/upskill/issues/634); the
[LSP servers entry](#lsp-servers-in-pack-output) records the move.

Implementation: the pull request of branch `feat/pack-commands`. Completion
evidence: `just verify` and `just sanity` passed on commit `5748408`.

Durable successors: [AD-0046](../adr/0046-plugin-mvp-components.md), Decisions
2, 8 and 11; [content format](../topics/0001-content-format.md);
[format spec](../specification/format-spec.md);
[specification §2.8](../specification/specification.md); the
[command guide](../book/commands.md); the [conventions](../book/conventions.md).

## Triage of 2026-11-23

Issue [#536](https://github.com/driftsys/upskill/issues/536). The 36 documents
below were moved from `docs/wip/specs/` and `docs/wip/plans/` to the same file
names under `docs/archive/specs/` and `docs/archive/plans/`. Their bodies are
unchanged, except that every path of the form `wip/specs/<name>` or
`wip/plans/<name>` to another moved document, in a link or in plain text,
follows the move, and the formatter wrapped the changed lines again. Links from
the ADRs, topics and technotes were updated in the same way; the evidence JSON
files under `docs/technotes/evidence/` keep the old paths. Each entry was
checked against the merged pull requests and the code at commit `4c07051`.
Status lines inside the documents ("Approved", "pending implementation") are
historical.

The triage also ends two statements of earlier entries. The entries "Captured
Codex instruction boundary" and "Source-backed Codex instruction lifecycle" say
that the other working documents and `docs/wip/README.md` retain their paths and
bytes. That was true until this triage moved the documents and rewrote the
index.

### Delivered

| Documents                                                                                                                                                                                          | Delivered by                               | Durable records                                                                                                                     | Differences from the documents                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Install script: [spec](specs/2026-05-16-install-script-design.md), [plan](plans/2026-05-16-install-script.md)                                                                                      | #135, #136                                 | [Getting started](../book/getting-started.md)                                                                                       | No release asset and no installer exists for native Windows.                                                                                                                          |
| Bundle discovery by name: [spec](specs/2026-05-22-bundle-by-name-discovery-design.md), [plan](plans/2026-05-22-bundle-by-name-discovery.md)                                                        | #160                                       | [Specification](../specification/specification.md) §2.1, [command guide](../book/commands.md)                                       | None found.                                                                                                                                                                           |
| Windows support: [plan](plans/2026-05-22-windows-support.md)                                                                                                                                       | #157                                       | [Specification](../specification/specification.md) §1.2, [getting started](../book/getting-started.md)                              | The plan has no spec.                                                                                                                                                                 |
| Plugin descriptor modes: [plan](plans/2026-05-23-plugin-descriptor-modes.md)                                                                                                                       | #171 (issue #168)                          | [Content format](../topics/0001-content-format.md) §3.7, [ADR-0008](../adr/0008-plugin-install-shellout.md)                         | The design that the plan names was never committed. #584 later changed the location and the key of the OpenCode config entry.                                                         |
| Comment-preserving `fmt`: [spec](specs/2026-05-23-comment-preserving-fmt-design.md)                                                                                                                | #170 (issue #169)                          | [Content format](../topics/0001-content-format.md) §3.8, [ADR-0004](../adr/0004-cli-surface.md)                                     | The key order lists of the spec are older than the current ones.                                                                                                                      |
| Item conflict resolution: [spec](specs/2026-05-24-item-conflict-resolution-design.md), [plan](plans/2026-05-24-item-conflict-resolution.md)                                                        | #174                                       | [Specification](../specification/specification.md) §2.1 (partial)                                                                   | `--force` also covers modified and unowned files since #468 and #489. The lockfile is `schema: 2`. The durable records do not describe the conflict error; see the open points below. |
| Multi-registry search: [spec](specs/2026-05-24-multi-registry-search-design.md), [plan](plans/2026-05-24-multi-registry-search.md)                                                                 | #175                                       | [Command guide](../book/commands.md) (`search`, `index`), [getting started](../book/getting-started.md)                             | The index holds no bundles. The `gitlab:` source examples are obsolete since #233.                                                                                                    |
| Upgrade UX: [spec](specs/2026-05-24-upgrade-ux-design.md), [plan](plans/2026-05-24-upgrade-ux.md)                                                                                                  | #172                                       | [AD-0043](../adr/0043-per-source-update-prompts.md), [specification](../specification/specification.md) §2.2                        | Removal of items that left the source is kept. #468 and #489 replaced clean-and-regenerate with recorded files; #503 replaced the single question with one question per source.       |
| Supporting resources: [spec](specs/2026-06-02-supporting-resources-design.md), [plan](plans/2026-06-02-supporting-resources.md)                                                                    | #201, #224, #225 (issues #199, #200, #203) | [Content format](../topics/0001-content-format.md) §2.4                                                                             | #225 replaced the `--as` guard with alias relocation. Client-named files are ordinary resources since #580.                                                                           |
| Coupling redesign: [spec](specs/2026-06-03-coupling-redesign-design.md), plans [slice 1](plans/2026-06-03-coupling-redesign-slice1.md) and [slice 2](plans/2026-06-03-coupling-redesign-slice2.md) | #204, #210, #231                           | [ADR-0009](../adr/0009-coupling-tiers-and-dependencies.md), [content format](../topics/0001-content-format.md) §3.1                 | `requires.mcps` was not delivered. The status line of ADR-0009 still says "Proposed"; see the open points below.                                                                      |
| MCP servers: specs [support](specs/2026-06-03-mcp-support-design.md) and [local-install convention](specs/2026-06-03-mcp-v2-convention-design.md), [plan](plans/2026-06-03-mcp-support-v1.md)      | #218, #219, #239, #242 (issues #237, #241) | [ADR-0010](../adr/0010-mcp-config-write.md), [content format](../topics/0001-content-format.md) §3.7, [recipes](../book/recipes.md) | The "v2" sketch of the support spec (`requires.mcps`, generated preflight, install agent) was not delivered; the convention spec rejects it. Debt #244 is open.                       |
| `fmt` formats the body: [spec](specs/2026-06-04-fmt-format-body-design.md), [plan](plans/2026-06-04-fmt-format-body.md)                                                                            | #221                                       | [Content format](../topics/0001-content-format.md) §3.8, [ADR-0004](../adr/0004-cli-surface.md)                                     | None found.                                                                                                                                                                           |
| Generic git URL source: [spec](specs/2026-06-10-generic-git-url-source-design.md), [plan](plans/2026-06-10-generic-git-url-source.md)                                                              | #232, #233, #234                           | [ADR-0011](../adr/0011-generic-git-url-source.md), [specification](../specification/specification.md) §2.1                          | The amendment of 2026-10-12 to ADR-0011 (#257) reversed the hard failure for `gitlab:` labels in existing lockfiles.                                                                  |
| Customization target profiles: [plan](plans/2026-10-11-customization-target-profiles.md)                                                                                                           | #252                                       | [ADR-0012](../adr/0012-consumer-client-filtering.md), `src/target.rs`                                                               | #583 removed the capability validation of the plan, because no command called it.                                                                                                     |
| Legacy source labels: [plan](plans/2026-10-12-legacy-source-labels.md)                                                                                                                             | #257 (issue #236)                          | [ADR-0011](../adr/0011-generic-git-url-source.md) amendment, [specification](../specification/specification.md)                     | None found.                                                                                                                                                                           |
| Rust 1.85 compatibility: [plan](plans/2026-10-12-rust-185-compatibility.md)                                                                                                                        | #256                                       | `Cargo.toml` (`rust-version`), the `msrv` job of `.github/workflows/ci.yml`                                                         | None found.                                                                                                                                                                           |

### Replaced by the re-steer of 2026-11-11

[AD-0042](../adr/0042-resteer-customization-delivery.md) and the
[re-steer design](specs/2026-11-11-customization-resteer-design.md) replaced the
gate-based approach that these documents belong to. None of them is planned
work. The [roadmap](../roadmap.md) holds the current delivery order.

- [Agent customization foundation design](specs/2026-10-11-agent-customization-foundation-design.md).
  Its target identities are in `src/target.rs` (#252). Its planning, ownership,
  migration and recovery contracts were built as library code and removed by
  #466. AD-0042 Decisions 1 and 2 replace them. Its permission preservation and
  its Cursor and Gemini targets were not delivered.
- [Agent customization delivery roadmap](specs/2026-10-11-agent-customization-roadmap.md).
  The [roadmap](../roadmap.md) replaces it. Its own header already names it
  historical planning.
- [Shared updater design](specs/2026-10-14-shared-updater-design.md) and
  [plan](plans/2026-10-14-shared-updater.md). AD-0042 Decision 8 delivered
  `update --check`, `update --notice` and the session notice plugins in their
  place (#524, #526, #532; stories #284 and #285 are closed). The `maintain`
  command, the update of the upskill binary and the release channel were not
  delivered; the re-steer design lists the binary update as a non-goal.
- [Customization delivery proposal](specs/2026-10-23-customization-delivery-proposal.md)
  and
  [implementation handoff](specs/2026-10-23-customization-delivery-implementation-spec.md).
  AD-0042 replaced the gates Q1 to Q11 and CD-01 to CD-11, the one-copy rule
  policy and the Codex policy. The trial results stay in `docs/technotes/`. The
  proposed amendments 1, 3 and 4 of
  [AD-0040](../adr/0040-separate-customization-delivery-from-discovery.md)
  (managed packages for each client, OpenCode skill sources, bundles outside the
  MVP) were neither adopted nor implemented.
- [Per-client package preservation contract](specs/2026-10-25-customization-preservation-contract.md)
  (#415, #416). A design record without code. It covers native plugin
  registration for all content, which AD-0042 rejects as an alternative. AD-0042
  Decisions 1 and 2 state the preservation behaviour that was built.
- [Preflight discovery and import inventory](specs/2026-10-25-preflight-discovery-inventory-design.md)
  (#417). Never implemented. It was the entry design of the gated lifecycle;
  AD-0042 replaced the clause that requires a complete preflight.

Two technotes that the WIP index listed as active proposals stay in
`docs/technotes/` as research:
[the Copilot plugin exclusion profile](../technotes/copilot-project-plugin-overlay-profile.md)
(#446, closed; its native trial never ran) and
[the delivery policy comparison](../technotes/customization-delivery-policy-comparison.md)
(#442, closed; AD-0041 adopted its recommendation).

### Open points found by the triage

These points are not fixed by the triage and have no issue yet:

- ADR-0009 and ADR-0006 have the status "Proposed", and the
  [ADR index](../adr/README.md) repeats it, although the code implements both.
- No durable record describes the error for an item that is already installed
  from another source, or the `--as` flag of `add`. For the `--exclude` flag,
  [ADR-0045](../adr/0045-record-add-excludes-in-lockfile.md) (merged on
  2026-11-23, after the triage) is the durable record.
- `upskill search --kind` and `upskill index` do not cover bundles. No
  integration test covers the registry index.

## Stale documentation inventory of 2026-11-23

The [inventory](2026-11-23-stale-docs-inventory.md) was a working note for the
same session and is retained unchanged. Result for each of its items:

| Item | Result                                                                                                                                             |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1, 2 | The paragraph in section 7 of the content format now cites AD-0041 and AD-0042.                                                                    |
| 3    | The "Planned ancillary transition" block was removed from the specification. The `CLAUDE.md` bridge and the OpenCode glob entry are kept.          |
| 4    | Topic 0009 has a new status paragraph, and its replaced sections are marked. The file was not moved, because the book and many records link to it. |
| 5    | No change: the three lines are inside sections that are already marked withdrawn.                                                                  |
| 6    | The content format states that upskill generates no output for Codex.                                                                              |
| 7    | The capability matrix has no version label; two cells were corrected (Copilot subagents, OpenCode path scoping).                                   |
| 8    | No change: the table is under the heading for installing Superpowers, a third-party plugin.                                                        |

The same session added a withdrawn mark to the "Native-first transition" section
of ADR-0003 and corrected the lint rule table of the command guide. The redirect
pages at the `docs/` root, the pages of `docs/design/` and topics 0005 to 0008
stay in place: `docs/compatibility.json` and the book depend on their paths. The
part of the inventory named "Not covered by the sweep" is still not covered.
