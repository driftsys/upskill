# Per-client package preservation contract

Date: 2026-10-25. Task [#415](https://github.com/driftsys/upskill/issues/415).
Status: proposed for explicit user approval. This draft supplies the
preservation work required by the [slice-2 handoff][handoff]. Recommendations
below are the author's proposals, not decisions already made by the user.
Retaining or merging this document does not approve its allowances, qualify an
adapter or activate a production route. Production remains lockfile schema 1
([AD-0040][direction]).

## Purpose and scope

Define what a package operation may change, what it must preserve and how it
reports incomplete native effects. The first implementation profile is Claude
skills/resources, subject to its entry gates. Codex and OpenCode retain their
own route gates. Package defaults, new owned subagents, authorable bundles and
OpenCode skill-source selection remain the separate pending amendments listed in
[AD-0040][direction]. This contract does not decide them.

The deliverable is a design and approval record. Implementation, native control
execution, Q11 [#410](https://github.com/driftsys/upskill/issues/410), preflight
inventory implementation and durable-record amendments are separate work. Q5
remains deferred. The retained observations below concern macOS arm64; the
combined release still needs the applicable CD-09/CD-11 evidence
([delivery contract][contract], [handoff][handoff]).

## Approval overview

These recommendations are independently reviewable; all remain unapproved.
Policy approval does not authorize an unidentified configuration selector. If a
client profile's concrete path, field shape or mode contract is still missing,
complete its evidence and obtain profile approval before implementing effects.

| Item | Recommendation                                                                                                                                                                                              |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1   | Distinguish exact protected-file preservation from separately reported native cache, authentication, trust, log and session effects. No whole-profile equality claim.                                       |
| D2   | Allow only operation-created Claude empty containers inside a pre-existing settings file, under the guarded conditions below. No new-file residue allowance.                                                |
| D3   | Allow bounded Copilot `installedPlugins` empty-container residue only after its concrete configuration/route profile passes. No project-overlay allowance.                                                  |
| D4   | Start Codex with no configuration-residue exception and no notice/trust repair authority.                                                                                                                   |
| D5   | Retain managed bytes and an incomplete operation while existing/resumed-session use is unresolved.                                                                                                          |
| D6   | Keep bounded shared-configuration preservation checks in memory and persist only non-secret native-effect facts; a credential-bearing whole-file writer needs a separate approved capture/storage contract. |

## Existing requirements

The following requirements already bind this draft:

- Preserve team `CLAUDE.md`, `AGENTS.md`, nested equivalents, manual content,
  unselected owners and surviving references. Independent delivery never
  rewrites team guidance. Reader visibility does not create ownership
  ([preservation and reader policy][contract], [AD-0040][direction]).
- Enumerate owned files/resources individually. Reject unowned collisions even
  when bytes match; reject edited owned artifacts on update/remove. Preserve
  exact existing ordinary modes and presence. Do not recursively delete mixed
  directories or infer ownership from a package-name prefix
  ([file preservation][contract], [owner contracts][owners]).
- Select modes for new upskill files through [AD-0031][permissions]: `0o0600`
  normally; `0o0700` for an explicitly authorized resource execution choice with
  captured owner-execute evidence. Native-created configuration needs its own
  observed mode contract. An existing fixture's mode does not select that mode
  for a new file. Unsupported ownership, ACL or extended-metadata conditions
  retain the qualified executor's rejection rules ([AD-0033][execution]).
- Observe native registration separately from file publication. Preserve other
  keys, credentials, comments and meaningful order. Unknown or pre-existing
  registrations are conflicts or externally satisfied dependencies, not owned
  registrations. Trust, enablement, discovery and invocation have distinct
  outcomes ([native preservation and CD-08][contract]).
- Reuse complete selected-set preflight, qualified roots, writer locking, exact
  State backup and journaled recovery. Native effects require observed pre/post
  state in addition to file receipts. No distributed atomicity or hardware
  power-loss guarantee follows ([handoff][handoff], [AD-0033][execution],
  [AD-0037][backup]).

## What preservation measures

An adapter profile must identify every intended reader and resolved user/project
configuration layer before effects, including imports, globs and compatibility
roots. The separate inventory must cover `.claude/rules`, `.github/instructions`
and the managed root. An unsafe or unresolved profile stops before writes; it is
never repaired by disabling existing discovery ([handoff][handoff],
[CD-03][contract]).

For each package file, shared configuration object and native registration,
record its client pin, scope, qualified location, logical identity, ownership
provenance, selected owners and surviving references. File observations include
presence, digest, mode and the executor's supported identity/metadata evidence.
Identity checks guard operations; they do not promise the same inode after a
qualified replacement. Shared objects need explicit agreement, not equal bytes
([AD-0034][owners], [AD-0033][execution]).

| Object                                            | Required boundary                                                                                                                                                                       |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unrelated files and team guidance                 | Exact bytes, supported modes and presence remain unchanged; no import, bridge or glob is added to them.                                                                                 |
| Owned package entrypoints/resources               | Only enumerated artifacts may change, under captured source and owner evidence. Preserve survivors and block edited outputs.                                                            |
| Shared client configuration                       | Ownership is an exact logical entry at an identified layer and scope, not the entire file or a whole top-level table. Unrelated values, comments and meaningful order remain protected. |
| Native registries and caches                      | Read bounded inventory/metadata for observation; use native management for registration. Do not patch or restore client-owned registry/cache files directly.                            |
| Authentication, trust and application bookkeeping | Separate native outcomes. No automatic trust grant, credential export, account repair or notice-counter reset is authorized by file ownership.                                          |

The shared-configuration writer must demonstrate how it preserves unrelated
bytes, comments and meaningful order while changing an owned entry. Parsed-value
equality alone does not authorize reformatting a manual file. If the native
serializer or proposed writer cannot meet the declared profile, that profile is
unsupported until an explicit, bounded allowance is approved and qualified. No
blanket reserialization exception is proposed here
([native preservation][contract]).

Exact settings restoration in a short fixture is not authority to overwrite a
user's later edits with an old whole-file snapshot. Revalidate the relevant
identity, owned entry, unrelated projection and mode before any compensation.
Unexpected changes remain untouched and require reconciliation
([AD-0032][recovery], [Q6][Q6], [Q8][Q8]).

## Pinned observations and proposed client profiles

Every result in this section is retained evidence, not a new native run. The
concrete adapter profile must bind resolved paths and exact supported key/value
shapes; a native namespace alone does not identify a writable field.

### Claude Code 2.1.308

Q6 observes project/user marketplace registration and fresh-session visibility
of a combined content/version edit without reinstall. Q10 adds same-workspace
user/project package overlap and each surviving scope after removal. Each of its
six fresh sessions has one visible target entry, one skill expansion and one
resource read. This does not qualify active/resumed-session cleanup or
provider-effective uniqueness ([Q6][Q6], [Q10][Q10]).

The proposed profile distinguishes:

- User `$CLAUDE_CONFIG_DIR/settings.json` and project `.claude/settings.json`:
  only the captured package ID in `enabledPlugins` and the exact marketplace
  identity in `extraKnownMarketplaces` can be eligible owned entries.
- `plugins/installed_plugins.json` and `plugins/known_marketplaces.json` under
  the client configuration root: native-managed observation targets, never
  direct upskill write targets.
- Native cache paths reported by inventory: client-owned effects, not managed
  package storage or authority for deletion ([Q6 preservation][Q6],
  [Q10 preservation][Q10]).

Observed uninstall leaves added empty `enabledPlugins` and
`extraKnownMarketplaces` objects. The fixture then restores measured settings
bytes/modes exactly. Q6's native cache survives removal, including
later-observed `0.0.1` storage; Q10 records persisting `0.0.0` storage. Native
registry files return to their measured baselines ([Q6][Q6], [Q10][Q10]).

Decision required: **D2, Claude empty-container allowance.** Recommend allowing
those two empty objects inside a pre-existing settings file only when the
operation introduced the container, all owned entries and references in it are
released, unrelated content remains preserved and native remaining-reader checks
pass. Existing empty objects remain manual baseline. This is declared residue,
not exact file equality. It does not permit a newly created empty settings file
to remain, arbitrary mode changes or a whole settings rewrite. The
[Q10 restoration receipt][Q10 restoration] records initially absent user
settings, a native file observed at `0o0644` after cleanup, and restored
absence. Project settings already existed at `0o0640`. This is bounded
absence-restoration and cleanup-time mode evidence, not an observation of mode
at first creation or a universal native-created mode policy. A qualification
profile must pin the remaining creation, metadata and recovery cases separately.

A marketplace is a separate shared dependency. In Q10 the project binding
survives user-package removal while a user-scoped marketplace remains known.
Removing a user's package reference does not authorize removing a marketplace
still needed by a project or another owner. Preserve that dependency until its
last authorized reference is released; refuse an unqualified overlap profile
([Q10][Q10], [composition preservation][contract]).

### Copilot CLI 1.0.103

Q3 observes native installation, an owned empty `installedPlugins` container
after uninstall, and exact measured configuration restoration after unrelated
value checks. Its rule component fails at both scopes; project-overlay
activation is unverified. User catalog and bounded child observations do not
supply a passing general skill/update lifecycle ([Q3][Q3]).

The proposed profile allows ownership of individual native plugin registration
entries only after the adapter records the actual user configuration path and
supported `installedPlugins` representation. Q3's public restoration receipt
labels it `configuration`; it does not define a universal writable schema.
Project `enabledPlugins` overlays remain a separate, unqualified profile. There
is no permission to replace the whole native configuration, adopt manual
registrations or patch an installation/cache directory ([Q3][Q3],
[handoff][handoff]).

Decision required: **D3, Copilot empty-container allowance.** Recommend the same
bounded treatment for an operation-created empty `installedPlugins` container
inside a pre-existing file: preserve other entries, comments, meaningful order,
mode and presence; retain it as declared residue only after a passing profile
control. No allowance for project overlays or newly created empty files follows.
The required route, schema and preservation qualification remains outstanding.

### Codex CLI 0.164.0

Q8 observes a project activation with its new user binding disabled. A resource
and package-version edit remains invisible until native remove/re-add; the skill
body is unchanged. The control restores its own user configuration baseline and
new project configuration's original absence. Q7 separately retains an initial
pre-UI restoration failure and a later owned notice-counter correction. Q8's
later baseline is not the original pre-Q7 baseline ([Q8][Q8], [Q7][Q7]).

The proposed profile identifies `$CODEX_HOME/config.toml`, the selected
project's `.codex/config.toml` and exact native plugin/marketplace entries
through captured scope and source identity. The future adapter record must
specify concrete TOML entry selectors and native inventory before mutation. It
may not claim whole plugin, project-trust or notice tables. Remove/re-add is a
multi-effect update that must preserve selected project scope and surviving
references; Q8 does not qualify every update form ([Q8][Q8],
[native preservation][contract]).

Decision required: **D4, Codex configuration allowance.** Recommend no
configuration residue exception initially. Owned registration entries must
return to the recorded baseline or preserve surviving owners under the qualified
entry writer. Unrelated notice/trust changes block reconciliation; do not reset
them merely to recreate a historical hash. Q7's fixture-specific correction is
not a production repair policy. Normal user trust review remains mandatory;
index-only hook trust is still a separate unscheduled control ([Q7][Q7],
[Q8][Q8], [pending controls][contract]).

### OpenCode 2.1.4

Q5's positive fails and component lifecycle controls remain unrun. The retry
records unchanged measured configuration/manual/rule bytes and modes; it does
not prove install/update/remove preservation. Keep the profile blocked while Q5
is deferred ([Q5][Q5]).

Candidate configuration includes the selected `opencode.json`/`opencode.jsonc`
and applicable user configuration, with explicit rule paths and potential
package plugin/skill-source entries. No array, field shape, serializer, cache or
residue allowance is approved by this draft. A later profile must distinguish
each owned path/registration from manual entries, preserve comments and
meaningful ordering, and reject duplicate/ambiguous ownership. Existing globs
and bridges keep their separate step-8 gate; this contract grants no migration
or removal ([handoff][handoff], [native-first direction][direction]).

## Native residue and live-use boundary

Decision required: **D1, measured boundary.** Recommend approving the
per-profile reporting distinction between exact protected files/configuration
and declared native effects outside that equality. Native cache copies,
authentication refresh, native trust interaction, logs and session bookkeeping
can persist only as separately reported native outcomes. This does not authorize
upskill to edit them, ignore changes inside protected configuration or claim
whole-profile restoration. Required registrations, file changes and unknown
effects still need their own result ([preservation review][contract], [Q6][Q6],
[Q7][Q7], [Q10][Q10]).

Decision required: **D5, unresolved live-use outcome.** Recommend retaining the
managed package files and an incomplete operation when running/resumable-session
use cannot be ruled out. Report observed registration removal separately from
pending file reclamation. Do not publish a completed removal or drop ownership
facts while cleanup safety is unresolved. A later qualified native detachment
observation can allow continuation. This proposes no automatic garbage
collector, timeout-based deletion or client-cache cleanup. Fresh-session Q6/Q8/
Q10 results do not prove running/resumed-session detachment
([lifecycle constraints][proposal], [CD-07][contract]).

## Capture and credential boundary

Decision required: **D6, shared-configuration capture.** Recommend allowing
bounded client configuration observation in memory for preservation checks, with
only non-secret identities, digests and explicit owned values in native-effect
receipts. Never extract credential values, export authentication stores or put
them in public evidence, logs or new native-effect journals. Ordinary native
credential use is separate. A shared configuration requiring credential-bearing
whole-file payloads needs a separate approved storage/capture contract before
that new writer is supported. Do not redact a snapshot and call it exact
restoration ([Q3][Q3], [Q8][Q8], [handoff][handoff]).

[AD-0037][backup] still requires exact original authoritative State backup under
its existing opt-in contract; it is not authorization to back up client homes,
authentication stores or arbitrary shared client configuration. Its execution
journal version is not a production lockfile-schema change. A new native-effect
journal extension and its qualification remain implementation-plan work; this
draft selects no serialization or new state fields ([AD-0033][execution],
[handoff][handoff]).

## Operation and recovery requirements

The following describes required future outcomes, not current CLI behavior. The
file executor does not already implement this native-effect protocol
([AD-0032][recovery], [AD-0033][execution], [CD-07][contract]).

| Phase or observation                            | Required action                                                                                                                                                                                                                                                                       |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Prepare                                         | Preflight the complete selected set, source/ownership agreements, readers, configuration layers, permission support and native pre-state. Qualify scopes and lock cooperating writers. Stage complete owned content outside discovery; retain the existing State-backup prerequisite. |
| Native effect pending                           | Persist qualified non-secret intent and expected pre/post outcomes before invoking the native operation. Record source, package identity, scope, surviving references and the exact owned native entry.                                                                               |
| Native post-state established                   | Reobserve registration, scope and enablement. A completed command or file publication alone does not prove that state. Acknowledge only the qualified matching post-state and continue with native trust/discovery outcomes separately.                                               |
| Native pre-state established after interruption | Retry only under a qualified idempotence/inverse-action contract and after rechecking files, ownership, readers and configuration. A timeout does not establish absence.                                                                                                              |
| Unknown or conflicting native state             | Preserve files, ownership facts, journal and unrelated configuration; stop for reconciliation. Do not guess whether an effect ran or repeatedly install/uninstall. Queued work matching a desired post-state is not automatically adopted.                                            |
| Removal                                         | Release only selected references. Disable the last owned activation before considering file reclamation, retain shared marketplace dependencies, then apply D5. Never reclaim client caches.                                                                                          |
| Finish                                          | Publish final authoritative state only after required native/file outcomes and observations are established. Preserve conflicting receipts and unknown files. Report exact preservation, declared residue and unresolved effects separately.                                          |

External clients/editors do not honor the upskill writer lock. Revalidate around
native effects and recovery; do not promise to exclude every race. A native
command that changes unrelated registration data is a preservation failure, not
permission to restore an entire native registry from a saved copy
([AD-0033][execution], [native preservation][contract]).

## Acceptance evidence required before implementation

All examples below are planned, not results from this draft. Extend matching
pure planners/tests first; native controls require their own retained pinned
packets and ordinary user trust/authentication. Another session owns Q11; this
task consumes only its eventual merged result ([handoff][handoff]).

| Example                                                | Required evidence                                                                                                                                                                                                                                           |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Manual collision or local edit                         | Equal-byte unowned output, edited owned resource and unknown registration each reject before native/file effects; original bytes/modes/presence remain exact.                                                                                               |
| Shared configuration                                   | Owned-entry edits preserve unrelated values, lexical content, comments and meaningful order. Concurrent manual edits or an unsupported serializer stop restoration rather than replacing the whole file.                                                    |
| Native-created configuration                           | Pin exact new-file mode and original-absence restoration separately. Unknown modes, ownership or unsupported metadata reject; no inferred `0o644` default.                                                                                                  |
| Empty containers                                       | For each approved D2/D3 profile, prove the container was operation-created, no surviving reference needs it, manual/native entries remain active and only the approved residue remains. Pre-existing containers and unrelated added keys cannot be removed. |
| Shared marketplace/scope                               | Removing user or project ownership preserves the remaining package and marketplace dependency. Retain Q10's bounded PASS; add foreign/manual-owner cases and qualify other pins/profiles separately.                                                        |
| Refresh and visibility                                 | Test source edits, fresh/running/resumed sessions and native refresh separately. Native refresh interruption cannot silently widen project activation to user scope.                                                                                        |
| Interrupted native effect                              | Kill before/after invocation and before/after observation publication. Recovery distinguishes observed pre, post and unknown outcomes, never repeats an unproven effect and preserves drift.                                                                |
| Live-use removal                                       | Establish safe detachment or retain managed bytes and incomplete recovery state under D5. Missing catalog entry is not proof that a resumed session cannot read an old resource.                                                                            |
| Trust, authentication and secret-bearing configuration | Denied/offline/expired cases have distinct outcomes and preserve manual configuration. No automatic trust bypass or secret-bearing journal payload is accepted by this draft.                                                                               |
| Mixed kinds and references                             | Unselected rule/skill/resource/agent/bundle/plugin/MCP state, direct/bundle edges and explicit integration-removal policy remain intact; no inferred retention from incidental readers.                                                                     |

Offline examples do not qualify a client. Failure of a native positive leaves
subsequent absence/duplicate controls unqualified. Q10 is a merged bounded PASS;
Q11, collaborator onboarding, update/live-use controls, adapter preservation
profiles and remaining adoption gates still require their own evidence or user
approval. Four-client release qualification remains unchanged ([Q10][Q10],
[CD-06 through CD-11][contract], [AD-0040][direction]).

## Approval and next records

The user owns D1-D6 approval. Record each accepted/rejected item with its exact
client pin, scopes, configuration selector, residue boundary and qualification
references in the approval review. An item left unanswered remains unapproved.
Approval of a policy still does not establish a missing native PASS. Neither
merging this draft nor the Q10 packet adopts the pending package/MVP amendments
([AD-0040][direction], [handoff][handoff]).

After explicit approval, the implementation plan must name complete adapter
profiles, native-effect journal design, remaining acceptance controls and the
code/record amendment sequence. Later amend Topic 0009's preservation/recovery
clauses, the selected adapter design, applicable state/execution records and
user-facing lifecycle/doctor guidance to match approved implemented behavior.
This proposal itself leaves those durable records unchanged.

[handoff]: 2026-10-23-customization-delivery-implementation-spec.md#slice-2-outline-skills-packaging-on-passed-routes
[proposal]: 2026-10-23-customization-delivery-proposal.md#lifecycle-and-implementation-constraints
[direction]: ../../adr/0040-separate-customization-delivery-from-discovery.md
[contract]: ../../topics/0009-customization-delivery.md
[permissions]: ../../adr/0031-select-explicit-file-permissions.md
[recovery]: ../../adr/0032-model-recovery-checkpoints-before-effects.md
[execution]: ../../adr/0033-qualified-file-execution-and-journals.md
[owners]: ../../adr/0034-record-explicit-owner-contracts.md
[backup]: ../../adr/0037-require-create-only-backup-journals.md
[Q3]: ../../technotes/gate6-copilot-plugin-components.md
[Q6]: ../../technotes/gate6-claude-managed-root.md
[Q7]: ../../technotes/gate6-codex-agent-registration.md
[Q8]: ../../technotes/gate6-codex-skill-updates.md
[Q10]: ../../technotes/gate6-claude-scope-overlap.md
[Q5]: ../../technotes/gate6-opencode-explicit-paths-retry.md
[Q10 restoration]: ../../technotes/evidence/2026-10-25-claude-scope-overlap/retry-preservation.json
