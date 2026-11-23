# Preflight discovery and import inventory

Date: 2026-10-25. Task [#417](https://github.com/driftsys/upskill/issues/417).
Status: proposed design, not implemented. The user authorized this design while
Q11 [#410](https://github.com/driftsys/upskill/issues/410) proceeds in another
session. This document neither supplies its result nor changes a production
route. The [preservation policies][preservation] D1-D6 have separate
policy-level approval; concrete adapter profiles remain subject to approval and
qualification.

## Purpose and authority

Before publishing an owned artifact or changing a native registration, identify
which configured clients/surfaces can discover it and whether that exposure is
intended. Inventory the complete selected operation and affected surviving
owners, not only the client requested on the command line. Reject unsafe or
unresolved affected profiles before effects, preserving existing discovery
settings ([reader policy][readers], [handoff][handoff]).

The controlling requirements are [AD-0040][direction], Topic 0009's
[reader/activation policy][readers] and
[CD-01/CD-03/CD-04/CD-07/CD-10/CD-11][gates]. Always-on rules remain
native-first. Shared unscoped Claude/Copilot rules have one physical
`.claude/rules` file and per-item intended readers; a second Copilot copy is
refused. Scoped shared rules remain unsupported, and `paths:` metadata is not
proof that Copilot cannot read a file ([rules decision][rules], [Q1][Q1]). This
inventory implements no new route, default package layout or release-scope
amendment.

The scope is the capture/evaluation contract, diagnostics and acceptance design.
It adds no CLI flag, parser, native observer, journal schema or production
lockfile field. Existing schema-1 behavior, bridge/glob retirement gates,
credentials and native client configuration remain unchanged. General source
fetching, package generation, onboarding, update visibility and native
qualification have their own work ([handoff][handoff], [AD-0040][direction]).

## Separate capture, evaluation and authority

The proposed separation follows the existing pure-plan and qualified-observation
boundaries, without claiming a general discovery implementation already exists
([AD-0027][planning], [AD-0028][scope], [AD-0030][capture]).

| Component                  | Required contract                                                                                                                                                                                                                                                                                                         |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Read-only capture producer | Receive explicit roots/profile context. Read only declared discovery/configuration inputs and relevant directory membership through qualified observations. Return immutable evidence and retained revalidation guards. No probing writes, profile edits, client startup, network traversal or automatic source fetching. |
| Pure inventory evaluator   | Consume captured views, client profile revisions, candidate paths, ownership/reference facts, audience declarations and existing qualification references. Derive reader edges, ambiguity and duplicate hazards deterministically without I/O.                                                                            |
| Admission result           | Report complete or incomplete inventory, conditional discovery admissibility and every blocking reason. An admissible discovery graph is not native qualification, write authority or proof that all implementation/release gates passed.                                                                                 |
| Lifecycle caller           | Preserve existing admission/recovery order, complete selected-set planning and locks. Revalidate bound inputs before effects; a caller cannot reuse a report for different source, targets, roots or configuration. Native effects still need their own approved protocol.                                                |

A supplied native catalog/registration view is a separate input with a declared
producer, pin, scope, time/identity binding and freshness condition. The
inventory collector does not start clients to obtain it. A command called `list`
is not assumed side-effect-free. Producing new native views needs its own
qualified observer and approved preservation boundary; missing or stale views
remain explicitly unresolved ([CD-08][gates], [preservation][preservation]).

The existing [orchestration][orchestration] accepts explicit selected rule
roots, recovers before new source work and retains captures through application.
Its `SourceInstructionReaders` report is not this general configuration
inventory. The current [preflight report][preflight] has diagnostics and
instruction-reader reports; it has no generic inventory or admission-proof
field. Implementation must introduce an explicit, reviewed integration rather
than relabel those experimental reports as complete native discovery.

Implementation retains the single static Rust binary and existing dependency
policy. This design does not introduce a parser service or delegate collection
to a running client ([handoff][handoff]).

## Inventory domain

A profile declares client/runtime/surface, exact supported version and parser
revision, OS, project/user scopes, effective roots and non-secret substitutions.
The caller supplies this context explicitly; the evaluator never rereads ambient
`HOME`, configuration overrides or environment variables. Unknown effective
context is not replaced by a guessed default ([reader policy][readers]).

Capture the following for the prospective operation:

- Every applicable project/user configuration layer, ancestor/nested root,
  compatibility directory, import, configured glob and package export. Record
  precedence and exclusions only when the pinned profile defines them. Capture
  shadowed inputs too if a planned change could expose them.
- Existing entrypoints and the complete desired entrypoint/resource paths,
  including paths that do not yet exist. Capture current absence and relevant
  directory membership; an existing-files-only scan cannot prove safe placement.
- The candidate managed root and its possible descendants, including new
  versions/resources introduced by later operations. Keep managed source
  distinct from intentionally discoverable native output such as
  `.claude/rules`.
- Existing owned registrations, manual/third-party exposure and surviving
  direct/bundle references. Include user/project overlap and other selected
  installations affected by the same physical object.
- All four clients and known IDE/compatibility surfaces with their discovery
  conditions. Non-selection is not an exclusion and does not prove absence.
  Unknown behavior blocks an isolation claim ([reader policy][readers],
  [preservation][preservation]).

The supported search domain must be explicit and complete for its profile. Do
not recursively scan an entire home, enumerate authentication/session stores or
invent extra roots. A reference outside declared qualified roots, an unbounded
search or an exhausted resource budget makes the inventory incomplete, not
partially safe. Implementation may choose documented bounded limits; those
limits and exhaustion diagnostics must be deterministic.

### Client source checklist

This is a checklist of inputs to qualify, not a universal filesystem/schema
mapping. The versions name retained observations, not newly tested versions.
Concrete extractors, precedence and match behavior still need profile evidence.

| Client/source                             | Required coverage and retained boundary                                                                                                                                                                                                                                                                                                                          |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code 2.1.308                       | Project/user native rule roots, applicable guidance/import sources, package marketplace/enablement and ancestor/nested conditions. Observe any existing bridge/import; never create or rewrite one. Q6/Q10 prove bounded managed-root behavior, not every configuration language ([Q6][Q6], [Q10][Q10]).                                                         |
| Copilot CLI 1.0.103                       | `.github/copilot-instructions.md`, `.github/instructions`, Claude compatibility readers including `.claude/rules`, and declared package roots/registrations. Q1 establishes the unscoped cross-reader vector; Q3 does not qualify project-overlay activation or the retired rules component ([Q1][Q1], [Q3][Q3]).                                                |
| Codex CLI 0.164.0                         | Applicable user/project native configuration and guidance, plugin/skill exports and explicitly registered agent/configuration paths. A trusted generated hook/index has an explicit declared source edge, not a general script-analysis exemption. Q8 cache paths and Q7 agent pointers remain distinct observations ([Q8][Q8], [Q7][Q7], [AD-0040][direction]). |
| OpenCode 2.1.4                            | Applicable project/user JSON/JSONC, `instructions` paths/globs, potential skill-source/plugin references and compatibility roots. Do not guess precedence between present files or infer a passing route from the legacy glob. Q5 failed its positive and remains deferred ([Q5][Q5], [handoff][handoff]).                                                       |
| IDE/cloud/review and other known surfaces | Record relevant conditions and unresolved behavior separately; a CLI test does not qualify them. Do not hide a possible affected reader by declaring it outside the selected profile without evidence supporting that boundary ([CD-03/CD-09][gates], [rules decision][rules]).                                                                                  |

## Capture records and confidentiality

Each input record needs its resolved qualified location/layer, presence or
absence, content/metadata fingerprint, parser/profile revision, observation
provenance and revalidation dependency. Empty content differs from absence.
Record relevant membership guards so a new nested root, guide or import target
cannot evade a file-hash-only check. Preserve physical identity separately from
scope labels and logical source identity; aliases cannot split exclusion or
invent different logical items ([AD-0028][scope], [AD-0030][capture]).

Capture/evaluate discovery selectors in memory under D6. Reports and any future
native-effect receipts contain only bounded non-secret identities, digests and
diagnostics. Do not persist whole client configurations, guidance bodies,
credential values or arbitrary environment contents as inventory evidence. A
secret-bearing selector cannot be logged verbatim. Unsupported capture/storage
requirements stop the new profile; no redacted snapshot is described as exact
preservation ([preservation][preservation]).

This is not a new manifest or journal serialization. Preserve existing source
identity/framing and ownership agreement contracts; the implementation plan must
specify how discovery evidence is bound alongside them. Pure supplied values
alone do not grant filesystem, source-authenticity or native-effect authority
([AD-0027][planning], [AD-0030][capture], [AD-0034][owners]).

## Derive reader edges conservatively

For every logical item, derive edges describing the reader surface, its scope,
source condition, candidate entrypoint/resource, intended audience, route and
supporting evidence. Keep discovery/availability separate from activation and
scope enforcement. A catalog reference or file match does not prove invocation.

Classify each possible edge as intended, unintended or unresolved under the
existing reader policy. Intent requires explicit per-item consent and compatible
qualified behavior. Equal bytes, an audience label, a package namespace or a
warning cannot supply that consent. Incidental readers gain no owners or
retention references ([reader policy][readers], [AD-0034][owners]).

### Imports and configuration interpretation

Resolve imports only under the selected native profile's grammar and base-root
rules. Traverse transitive references with cycle detection and provenance. Do
not execute code, expand arbitrary shell expressions, follow remote imports or
fetch an imported repository. Qualified literal substitutions may be supplied as
captured non-secret inputs; unknown substitutions remain unresolved.

Malformed documents, ambiguous duplicate selectors, unsupported syntax or
unqualified precedence block affected profiles rather than producing an empty
reader set. Supported JSONC comments, fenced examples and escaped import syntax
need native-profile-specific interpretation; a generic regex or
comment-stripping pass cannot silently substitute for it. Symlinks, mount
changes and escaping paths retain the qualified capture/containment rejection
boundaries ([AD-0030][capture], [reader policy][readers]).

An arbitrary manual tool's ability to read files is not itself a native
discovery edge. An automatic hook or computed import that can supply affected
content but cannot be bounded by the profile is unresolved. This evaluator does
not certify arbitrary scripts or the absence of every possible filesystem
reader.

### Globs and prospective paths

A qualified matcher must distinguish proven disjointness, possible intersection
and unsupported semantics. Its language/base/case/recursion/exclusion rules
belong to the pinned client profile. Conservative possible matches may block a
profile; an unsupported pattern can never become a proof of disjointness.

Compare patterns with the managed-root prefix and potential descendants, not
only today's files. Also evaluate every desired native entrypoint/resource and
the effective configuration after the planned delta. An empty current glob
result is not evidence that a future `SKILL.md`, rule or resource cannot match.
`.gitignore`, naming conventions and empty directories are not privacy barriers
unless their relevant native behavior is qualified and unchanged
([handoff root constraint][handoff], [reader policy][readers]).

The managed root remains outside documented automatic discovery roots and
pre-existing import/glob exposure. A selected, owned explicit package
registration pointing to its managed source is an expected delivery edge, not an
accidental-discovery exception. It still needs ownership, intended-reader
consent and qualification. Pre-existing manual imports/globs cannot be excused
as intended by adding that registration. Unsafe existing settings remain
unchanged; the operation is blocked ([AD-0040][direction],
[preservation][preservation]).

### Duplication and unresolved surfaces

Group exposures by logical item, reader and relevant event/invocation, retaining
physical aliases and scope bindings separately. Detect direct-plus-package,
compatibility-copy, shared-plus-variant and duplicate-native-output hazards.
Native precedence or deduplication is not a portable assumption. Q10 permits
only its bounded, qualified same-package scope profile; it does not waive
planner deduplication or qualify another client/layout ([Q10][Q10],
[CD-04][gates]).

The accepted Codex recorded-history limitation remains a profile annotation, not
an exception for a second planner delivery or a missing control. Inventory
cannot infer provider-effective uniqueness from a catalog or configuration
([AD-0040][direction], [CD-04][gates]).

A shared unscoped Claude/Copilot rule has two declared readers and one physical
output. A second `.github/instructions` copy for that logical rule is refused.
Other IDE/cloud/review readers remain unresolved until their conditions and
profile boundary are established; an admissible inventory cannot silently omit
them. Do not widen a scoped rule, redefine an alias as a new item, or convert a
requested exclusion into a warning ([rules decision][rules], [Q1][Q1]).

## Admission and drift

Report inventory completeness separately from discovery admissibility and from
implementation/release eligibility. Missing profile evidence, unresolved
potential readers, unintended exposure, duplicate routes, unsafe placement or
unqualified matching block the affected complete operation. Independently safe
items do not cause a partial application. A later explicitly narrowed request
requires a new complete evaluation ([AD-0027][planning],
[reader policy][readers]).

Preserve existing early target/state/capability rejection before source
fetching. The collector itself performs no fetching. Authorized SSOT resolution
remains in its existing caller boundary; it never follows discovery imports as
new source-fetch instructions. Complete generated-path inventory is required
before publishing any outputs or invoking native mutation
([orchestration][orchestration], [AD-0027][planning]).

Existing pending recovery stops a new request before new source work. Recovery
uses its bound recorded intent plus fresh qualified observations, not a fresh
installation request disguised as cleanup. Native-effect recovery and required
State backup remain separate contracts ([preservation][preservation],
[AD-0033][execution], [AD-0037][backup]).

The admission result is valid only for its bound selected operation and captured
configuration/source/native views. Recheck content, relevant membership,
presence, roots, profile substitutions, scopes and native-view freshness before
effects. A newly created import, changed glob or new ancestor guide invalidates
the report even if candidate output bytes are unchanged. Expected owned native
changes require qualified post-observation and a renewed remaining-operation
inventory; do not disable guards because a client wrote the file.

Directory locks coordinate cooperating upskill writers, not external clients or
editors. Rechecks do not promise a race-free global snapshot. Detect drift,
retain evidence and stop before further effects; never overwrite an external
change to restore a stale inventory ([AD-0028][scope], [AD-0030][capture],
[preservation][preservation]).

## Diagnostics

Return structured diagnostics through the library; presentation stays in the
CLI. Identify the logical items/owners, reader, scope/layer, bounded source
condition, possible affected path and missing evidence or conflict. Preserve
stable deterministic ordering and safe independent diagnostics, following the
existing [preflight reporting][preflight] pattern. Any required unavailable
input keeps the result incomplete even when other diagnostics can be computed.

Distinguish unsupported parser/pattern/precedence, unresolved reader, unsafe
root, duplicate exposure, changed input and missing native evidence. Describe
what a new qualified plan or profile must resolve. Do not propose editing manual
discovery, deleting guides, granting trust or patching client caches as a
repair. These are proposed diagnostic obligations, not existing CLI messages or
codes.

## Planned acceptance examples

These examples are future offline acceptance tests, not results of this design.
Native discovery/activation/isolation still needs separate pinned controls with
successful positives and retained failures. The implementation should extend
matching existing planners/tests rather than building another native harness
([CD-10][gates], [handoff][handoff]).

| Input or change                                                                      | Expected result                                                                                                          |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Root outside standard client directories but covered by a manual broad Markdown glob | Block with the originating selector and prospective descendant witness; do not change the glob.                          |
| No current matches, but a proposed/future managed descendant could match             | Block unsafe placement; absence does not prove disjointness.                                                             |
| Approved exact native package registration to a neutral managed root                 | Record the intended explicit delivery edge; still require profile/owner evidence and evaluate all incidental selectors.  |
| Claude+Copilot unscoped native rule, both readers declared                           | One shared delivery; detect/refuse an added Copilot-native copy. Keep other relevant unresolved readers visible.         |
| Claude-only rule with a possible Copilot reader, or scoped shared rule               | Block rather than invent consent or widen to always-on.                                                                  |
| Compatibility copy plus package export with identical bytes or aliases               | Preserve logical identity and detect duplicate exposure; no ownership inferred from visibility.                          |
| User/project bindings or physical root aliases                                       | Combine affected reference/physical views, preserve scope semantics and require the applicable Q10/profile evidence.     |
| Shadowed configuration becomes effective after a planned entry removal               | Evaluate the prospective effective layers and block newly exposed unintended content.                                    |
| Nested root/ancestor guide/import appears after capture                              | Invalidate admission through relevant membership/absence guards; do not rely solely on hashes of known files.            |
| Unsupported pattern, duplicate selector, unknown base or precedence                  | Incomplete/blocked, never an empty reader set or guessed native semantics.                                               |
| Cyclic, remote, executable or escaping import; exceeded traversal budget             | Record bounded provenance and block; no execution, network traversal, probing write or partial-safe report.              |
| Existing ignore/exclusion setting                                                    | Use only its qualified observed semantics and capture it for revalidation; never edit it to manufacture a pass.          |
| Known IDE/cloud reader not selected for CLI installation                             | Retain its condition and unresolved status unless evidence establishes the boundary; no CLI-derived isolation claim.     |
| Config/native view changes during preparation or after a native effect               | Reject stale admission and reobserve; preserve unrelated content and incomplete operation state.                         |
| Secret-bearing configuration or selector                                             | No secret material in reports or new native-effect receipts; unsupported persistence needs a separate approved contract. |
| Invalid legacy/target state with an unavailable source                               | Reject before source fetching under the existing admission contract; inventory does not relax compatibility checks.      |

The implementation plan must add meaningful mutation controls: treating an empty
scan as disjoint, ignoring user/nested/compatibility inputs, excluding
unselected clients, trusting an unsupported pattern, dropping membership
revalidation and ignoring a duplicate route must each fail an independent
assertion.

## Approval and implementation sequence

This design needs user approval before code. D1-D6 policy approval is recorded
in the [preservation contract][preservation]; concrete profile selectors, modes,
owned-key behavior and native qualification remain separate gates. Package and
release-scope amendments are not adopted here. Q11 belongs to the other session;
consume only its eventual merged packet/index result.

After design approval, prepare the effect-free capture/evaluator plan and its
acceptance tests, explicitly define supported profile grammars and observation
producers, then integrate with complete lifecycle preflight and revalidation
without activating production routes. Any native profile work has its own
retained control task. Owned skills/package implementation and release retain
all handoff and CD-11 gates; this design or an offline PASS cannot supply them
([handoff][handoff], [CD-11][gates]).

[direction]: ../../adr/0040-separate-customization-delivery-from-discovery.md
[readers]: ../../topics/0009-customization-delivery.md#reader-and-activation-policy
[gates]: ../../topics/0009-customization-delivery.md#acceptance-and-evidence-gates
[handoff]: 2026-10-23-customization-delivery-implementation-spec.md
[preservation]: 2026-10-25-customization-preservation-contract.md
[rules]: 2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24
[planning]: ../../adr/0027-compose-complete-owned-artifact-plans.md
[scope]: ../../adr/0028-lock-qualified-scope-directories.md
[capture]: ../../adr/0030-capture-qualified-source-evidence.md
[owners]: ../../adr/0034-record-explicit-owner-contracts.md
[execution]: ../../adr/0033-qualified-file-execution-and-journals.md
[backup]: ../../adr/0037-require-create-only-backup-journals.md
[orchestration]: ../../../src/lifecycle/orchestration.rs
[preflight]: ../../../src/lifecycle/preflight.rs
[Q1]: ../../technotes/gate6-copilot-claude-rules-reader.md
[Q3]: ../../technotes/gate6-copilot-plugin-components.md
[Q5]: ../../technotes/gate6-opencode-explicit-paths-retry.md
[Q6]: ../../technotes/gate6-claude-managed-root.md
[Q7]: ../../technotes/gate6-codex-agent-registration.md
[Q8]: ../../technotes/gate6-codex-skill-updates.md
[Q10]: ../../technotes/gate6-claude-scope-overlap.md
