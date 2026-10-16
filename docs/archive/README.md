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
[specification §4.1](../specification/specification.md#41-pure-legacy-state-projection),
[generation design](../design/generation-pipeline.md#pure-legacy-state-projection)
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
[specification §4.2](../specification/specification.md#42-pure-owned-instruction-planning),
[planner design](../design/owned-instruction-planning.md),
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
[pure whole-file planner](../specification/specification.md#44-owned-whole-file-operation-plans)
is implemented for explicit complete file sets. The
[captured item compiler](../specification/specification.md#45-captured-item-compilation)
derives complete per-client entrypoint/resource outputs, and the
[mixed coordinator](../specification/specification.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../specification/specification.md#414-captured-rule-desired-sets)
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
[specification §4.3](../specification/specification.md#43-strict-captured-source-manifests),
[source manifest design](../design/strict-source-manifests.md),
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
[pure whole-file planner](../specification/specification.md#44-owned-whole-file-operation-plans)
is implemented for explicit complete file sets. The
[captured item compiler](../specification/specification.md#45-captured-item-compilation)
derives complete per-client entrypoint/resource outputs, and the
[mixed coordinator](../specification/specification.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../specification/specification.md#414-captured-rule-desired-sets)
producer is implemented. Lifecycle reconciliation and planning, immutable Git
evidence, source stabilization, qualified observation, destination
qualification, state/execution and Codex activation remain pending. The
source-manifest platform evidence above is retained as recorded; the whole-file
planning pair is archived in its own entry below.

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
[specification §4.4](../specification/specification.md#44-owned-whole-file-operation-plans),
[whole-file planner design](../design/owned-file-planning.md) and
[AD-0021 through AD-0023](../adr/README.md#owned-whole-file-planning). The
[command guide](../book/commands.md) describes the unchanged command boundary.

The
[operational lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
and [M1 plan](../wip/plans/2026-10-11-instructions-rules-m1.md) remain active.
The
[captured item compiler](../specification/specification.md#45-captured-item-compilation)
derives complete entrypoint/resource sets, and the
[mixed coordinator](../specification/specification.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../specification/specification.md#414-captured-rule-desired-sets)
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
[specification §4.5](../specification/specification.md#45-captured-item-compilation),
[compiler design](../design/captured-item-compilation.md) and
[AD-0024 through AD-0026](../adr/README.md#captured-item-compilation). The
[command guide](../book/commands.md) describes the unchanged command boundary.

The pure producer now derives complete generation entrypoint/resource sets from
captured values. The
[mixed coordinator](../specification/specification.md#46-mixed-owned-artifact-operation-plans)
composes explicit complete file/contribution sets. The
[captured-rule desired sets](../specification/specification.md#414-captured-rule-desired-sets)
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
[specification §4.6](../specification/specification.md#46-mixed-owned-artifact-operation-plans),
[mixed planning design](../design/mixed-owned-artifact-planning.md) and
[AD-0027](../adr/0027-compose-complete-owned-artifact-plans.md). The coordinator
composes complete explicit file/contribution sets; connecting captured compiler
outputs to lifecycle ownership and evidence remains pending. Source/target
qualification, complete reference authority, per-owner consumers, filesystem
safety, permission policy, writer locking, durable state and apply/recovery
remain deferred. No native, CLI or schema-2 activation follows; M1-A2, M1-A3 and
M1 remain incomplete. Other WIP retains its prior disposition; this entry
resolves only the exact named pair.

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
[specification §4.7](../specification/specification.md#47-qualified-scope-roots-and-advisory-writer-locking),
[scope-lock design](../design/qualified-scope-locking.md) and
[AD-0028](../adr/0028-lock-qualified-scope-directories.md). The working spec
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
[specification §4.8](../specification/specification.md#48-qualified-destination-observations),
[qualified destination design](../design/qualified-destination-observation.md)
and [AD-0029](../adr/0029-capture-qualified-destination-evidence.md). These
current records establish the implemented inactive qualified destination and
writer-lock boundaries. The separate
[qualified source design](../design/qualified-source-observation.md) and
[AD-0030](../adr/0030-capture-qualified-source-evidence.md) now establish
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
[specification §4.9](../specification/specification.md#49-qualified-source-tree-observations),
[source observation design](../design/qualified-source-observation.md) and
[AD-0030](../adr/0030-capture-qualified-source-evidence.md). These authorities
existed before implementation; archival does not redirect their citations.

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
[specification §4.10](../specification/specification.md#410-explicit-file-permission-selection),
[permission design](../design/file-permission-policy.md) and
[AD-0031](../adr/0031-select-explicit-file-permissions.md). These records
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
[Gate 5 specification](../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md)
remains active, as explicitly required by the approved slice-completion scope.
All other WIP retains its prior disposition.

Durable successors:
[specification §4.13](../specification/specification.md#413-combined-lifecycle-state-foundation)
and [combined lifecycle state design](../design/combined-lifecycle-state.md).
The [validation record](../technotes/2026-10-15-m1-gate5-state-validation.md)
records local checks, independent review and omission-control evidence.
Implementation through `490b8e29a27040dac8172dfc5b4f582991029f56` passed full
verification and locked Rust 1.85 checking; subsequent changes strengthened
tests. Final native/hosted CI, PR review and merge remain separate gates. These
are local slice records and make no merge claim.

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
[specification §4.14](../specification/specification.md#414-captured-rule-desired-sets),
[desired-set design](../design/captured-rule-desired-sets.md) and the
[validation note](../technotes/2026-10-15-m1-gate5-desired-validation.md).
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
[Gate 5 umbrella](../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md).
Only this completed graph slice is archived; shared-owner lifecycle planning,
guarded backup/recovery and observational adapters remain unfinished.

Durable successors are
[specification §4.15](../specification/specification.md#415-authoritative-lifecycle-reference-reconciliation),
the [reference design](../design/lifecycle-reference-reconciliation.md) and
[validation note](../technotes/2026-10-16-m1-gate5-reference-validation.md).
Implementation through `4bfc357` passed local full verification, locked Rust
1.85 checking and hosted CI. These checks do not establish native activation,
Gate 5 completion or a PR merge. Commands remain on schema 1.

## Gate 5 shared-owner reference removal

Source path: `docs/wip/plans/2026-10-16-m1-gate5-retirement.md`. The
[archived plan](plans/2026-10-16-m1-gate5-retirement.md) preserves its exact raw
bytes, including historical unchecked task boxes and relative spec links. The
current umbrella remains at
[Gate 5 lifecycle integration](../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md);
the
[M1 owned lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md)
also remains active. The approved scope archives only this completed plan. All
other WIP retains its prior disposition.

Durable successors:
[specification §4.16](../specification/specification.md#416-reference-driven-shared-owner-removal-planning),
[removal design](../design/lifecycle-reference-removal.md) and
[validation note](../technotes/2026-10-16-m1-gate5-retirement-validation.md).
Original local implementation `b5c9537`, rebased as `911915d`, passed full local
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
[Gate 5 umbrella](../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md) and
the
[M1 owned lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md).
The approved completion scope archives only this selection plan; all other WIP
retains its prior disposition.

Durable successors:
[specification §4.17](../specification/specification.md#417-captured-lifecycle-selection-preflight),
[selection design](../design/lifecycle-selection.md) and
[validation note](../technotes/2026-10-16-m1-gate5-selection-validation.md). The
20 public acceptance examples passed locally, and 23 controls failed their
intended assertions with exact source restoration. Full local verification and
locked Rust 1.85 checking passed. Independent review and hosted delivery remain
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
[Gate 5 umbrella](../wip/specs/2026-10-15-m1-gate5-lifecycle-integration.md),
the
[M1 owned lifecycle design](../wip/specs/2026-10-12-m1-owned-lifecycle-design.md),
[captured-rule desired-set design](../design/captured-rule-desired-sets.md) and
[selection design](../design/lifecycle-selection.md). Only this bounded plan is
archived; the umbrella and all other WIP retain their existing disposition.

Durable successors:
[specification §4.18](../specification/specification.md#418-complete-single-owner-lifecycle-upsert-planning),
[upsert design](../design/lifecycle-upsert.md) and
[validation record](../technotes/2026-10-16-m1-gate5-upsert-validation.md). The
supplied local evidence records 33 passing public acceptance examples and
twenty-one assertion/panic controls with exact source restoration. The
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
