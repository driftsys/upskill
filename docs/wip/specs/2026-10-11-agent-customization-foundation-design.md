# Agent customization foundation design

Date: 2026-10-11. Status: **Proposed**.
Roadmap: [AC-00 / M0](2026-10-11-agent-customization-roadmap.md).
Code baseline: `b866f5e`. This document specifies future behavior; it does not
claim implementation, accepted ADR changes, or native conformance.

## Purpose and boundaries

Establish the minimum shared contracts needed to deliver instructions, skills,
subagents, MCP, plugins/marketplaces, hooks, and permissions for Claude Code,
Copilot, and Codex. The first implementation slice applies these contracts to
content installation. Later categories add their adapters without changing the
meaning of selection, ownership, or a successful operation.

The user approved proceeding from the prioritized roadmap. All detailed choices
below are design recommendations. Retain existing OpenCode functionality and
account for Cursor/Gemini CLI requirements without implementing their adapters.
Keep a synchronous Rust implementation, copied outputs, and the repository's
static-binary/dependency constraints. No client launcher, hook runtime, server
installer, marketplace hosting service, or runtime adapter SDK is introduced.

## Baseline observations

- `src/select.rs` maps four selections to three generation clients and four MCP
  targets. Both Copilot selections share generation output; integrations differ.
- `src/pipeline/output.rs` computes content paths from kind/client/name, without
  a scope parameter. OpenCode skills use `.agents/skills`, also discovered by
  other clients according to the research.
- `src/lockfile.rs` stores schema 1 item/source/client data and integration
  records. It has no per-output installed hash, owned config-key identity, or
  exact selected-surface record. Empty item `clients` historically means all
  three generation clients.
- `src/pipeline/lifecycle.rs` reconstructs paths to remove content and uses
  recorded generation clients for diagnostics. Its source hash is not evidence
  that a generated file has remained unchanged.
- `src/model/common.rs` rejects unsupported source schema versions; this is a
  useful compatibility boundary for future required semantics.

These are reasons for explicit contracts rather than a wholesale rewrite of
the pipeline. Existing reports, source parsing, fetch, and pure renderers remain
useful and should be adapted incrementally.

## 1. Target identity and compatibility

### Native target profiles

A target profile identifies **family + surface + contract revision**. Installation
context supplies project/user scope, OS, resolved roots, and an optional observed
client version. These are separate values: changing scope changes discovery,
not the identity of an item; one family can expose incompatible surfaces.

Initial internal surface identifiers are `claude.cli`, `copilot.cli`,
`copilot.vscode-local`, `codex.cli`, `codex.ide`, and `opencode.cli`.
Future `cursor.editor`, `cursor.cli`, and `gemini.cli` profiles remain design-only.
They are internal contract identifiers, not new CLI spelling promises.

Each profile records:

| Field             | Meaning                                                                     |
| ----------------- | --------------------------------------------------------------------------- |
| Contract revision | Upskill-owned revision selecting a fixed adapter contract                   |
| Client versions   | Explicit tested versions/ranges; no implicit claim about newer releases     |
| Scope/OS support  | Native discovery roots and supported installation combinations              |
| Capabilities      | Category, native field/event/operation, constraints, packaging form         |
| Evidence          | Official documentation/schema/source revision and native test result        |
| Lifecycle         | File or native-operation strategy, observation, inverse action, limitations |

Profiles ship with upskill. Installation does not download schemas or accept a
new vendor contract automatically. A detected version outside the tested range
produces an unverified-version diagnostic. Content generation may use an explicit
profile without a locally installed client; it cannot report native discovery
as verified. Native execution operations require the profile's version checks.
Permission-sensitive operations cannot bypass compatibility validation by
selecting an unknown version or passing a generic force option.

### Capability results

Separate **representation support** from **evidence status**:

- Representation: native, adapted with stated limitations, native-unsupported,
  or not-yet-implemented by upskill.
- Evidence: documented, schema-validated, native-discovery-tested, or
  semantic-runtime-tested, with version/OS/scope and date.

Category-level support is a summary of individual capability entries. For
example, supporting command hooks does not imply support for every event or
handler type. An adapter must not infer standalone support from plugin support,
or editor support from a CLI using a similar path.

For a selected item, validation produces a plan or diagnostics before effects:

| Condition                                    | Result                                                      |
| -------------------------------------------- | ----------------------------------------------------------- |
| Item audience excludes every selected family | Existing warn-skip behavior for the whole item              |
| Required behavior cannot be represented      | Error naming source field, surface, and remedy              |
| Restriction or approval meaning would change | Error; no automatic weakening or stronger substitute        |
| Cosmetic metadata cannot be represented      | Warning identifying the omitted field                       |
| Client CLI absent for file-only generation   | Files may be installed; native discovery remains unverified |
| Required native operation unavailable        | Incomplete operation, never installed/successful            |

An author can supply an explicit native alternative or exclude a target. There
is no blanket “ignore restrictions” fallback. Native extension data is validated
against the chosen contract, including behavior-bearing keys in otherwise
permissive formats.

### Version evidence for this proposal

Read-only local probes on 2026-10-11 returned Claude Code **2.1.287**, Copilot CLI
**1.0.95**, Codex CLI **0.162.1**, and VS Code **1.141.0** on macOS arm64.
Commands: `claude --version`, `copilot --version`, `codex --version`, and
`code --version`. VS Code emitted an Electron codesign diagnostic before returning
its version. No discovery or behavior test followed these probes.

These are candidate validation versions, not minimum supported versions or
release promises. The Copilot extension and Codex IDE versions still need to be
recorded independently of the editor. The implementation plan must select exact
Linux/macOS/Windows test combinations before claiming their support. The
[research snapshots](../../technotes/2026-10-11-agent-customization/README.md)
remain the contract inventory, not substitutes for those tests.

## 2. Consumer selection and author targeting

Keep existing boolean selectors and config precedence. Add `--codex` and config
`clients: [codex]` in M1; this selects local Codex CLI configuration. Codex IDE
is a companion conformance surface for shared files, not an assumed clone of
CLI behavior. `--vscode` means Copilot VS Code Local, not Agent Host or cloud.

Freeze the no-selection default to the currently shipped set: Claude, Copilot
CLI, VS Code Local, and OpenCode. Codex requires explicit selection initially.
This deliberately refines ADR-0012's “all clients” wording and requires an
accepted amendment before implementation. Adding an adapter must never expand
existing defaults as a side effect of extending an enumeration.

Existing family `audience` and content directives remain valid. Resolve family
filters and consumer surface selection independently. Do not add a generic
surface-filter syntax until a category needs it; native surface differences
still require distinct validation before deduplicating a shared output.

Record **requested surfaces** and **effective artifact consumers** separately.
For update, resolve selection in this order: invocation flags, project config
`clients`, global config `clients`, then the installation's recorded requested
selection. A saved config selection therefore overrides installation history,
as in ADR-0012. “Bare update preserves selection” means no flags and no configured
client selection. Preserve the existing empty-list-as-default behavior; an empty
effective config list is not an explicit selection override. Legacy unresolved
history uses the migration rules below rather than the latest default set.
These operation-level choices do not change the user's saved preferences.
Diagnostics and removal use recorded ownership;
they do not reinterpret the latest global preference as install history.

## 3. Artifact ownership

An artifact is identified by its destination scope/root plus path and, for shared
files, a stable owned selector. Artifact identity is independent of the client
requesting it. Each artifact records its consuming surfaces and owners.

An owner is an installed source + item kind/name + install identity. Retain the
source name separately from aliases. Bundle membership, dependency edges, and
direct installation roots remain separate references, so removing one bundle
does not erase another bundle's dependency. Preserve co-located removal groups
as an explicit input to removal planning, not a reason to ignore other owners.

| Artifact form               | Ownership and edit contract                                                                  |
| --------------------------- | -------------------------------------------------------------------------------------------- |
| Dedicated file              | Record installed bytes hash; replace/remove only if unchanged                                |
| Resource tree               | Record each owned file and mode; never delete a directory recursively based only on its name |
| Instruction contribution    | Own a marked aggregate region; preserve bytes outside it                                     |
| Structured config           | Own a key/table or identity-selected entry; preserve unrelated bytes/comments                |
| Native package registration | Record native identity, scope, pre-existing state, and operation outcome                     |

Unowned existing content is a conflict even when its name matches. Matching
content may be observed as externally satisfied, but does not become owned
without explicit adoption. No operation removes an externally satisfied object.
M0 does not add a generic force-adoption switch.

Shared files require syntax-aware edits for JSON, JSONC, and TOML. The first
implementation only adds the editor needed by M1's Markdown aggregates; later
adapters introduce their editors after dependency review. If an editor cannot
preserve a file's unrelated content, report a conflict rather than serialize
the whole file and discard user formatting/comments. Array selectors use native
identity or tracked contributions, never a stored numeric position alone.

### Shared discovery

Resolve destinations before deduplication. Two selected surfaces may share an
artifact only if their rendered bytes, resources, and behavioral requirements
are compatible. Equal bytes alone do not prove equal runtime interpretation.
Otherwise use verified client-specific locations where possible, or stop with
a conflict requiring an author/consumer choice.

Selection controls upskill's writes; it cannot stop an unselected client from
discovering a native shared directory. Preview must identify known incidental
consumers of shared paths. Client-specialized output must not enter a shared
path without validating it for the profile's known discovery consumers. This
includes compatibility directories, not just `.agents/skills`. Unknown future
clients cannot be guaranteed compatible and remain outside the contract.

Incidental discovery is exposure metadata, not a retention reference. Only
recorded installation owners and their requested consumers keep an artifact
alive. Removing its last such reference can remove an unchanged owned artifact
even when an unselected client could discover its directory.

Canonicalize roots and compare destinations using the filesystem's case and
aliasing behavior. Reject path traversal, symlink escapes, conflicting resource
paths, and multiple writes to the same selector. Respect explicit native root
overrides only where the selected profile defines them.

## 4. Installation plans and execution

Use an internal installation plan shared by add, update, and remove. Extend
preview to add/remove as their slices land; existing `update --dry-run` remains.
An install plan contains source resolutions, desired ownership, ordered actions,
preconditions, permission changes, diagnostics, and expected state transitions.
It is not a user-editable script or an arbitrary shell command list.

Action forms are file write/remove, contribution/key edit, resource copy/remove,
and a typed native operation with explicit arguments. Renderers return artifacts;
they do not run commands or write files. Native operations keep the existing
shellout approach, subject to the relevant accepted integration ADRs. Never
execute bundle-provided installers or scripts while constructing a plan.

Execution sequence:

1. Resolve source content and dependencies. Preview may fetch into a temporary
   directory, as current update preview does; it may not change client installation
   state, authenticate, launch MCP servers, or execute hooks.
2. Validate required capabilities and aggregate all destination conflicts.
3. Build deterministic actions and show any loss of coverage or change of access.
4. Acquire the scope's single-writer lock and recheck source, lockfile, destination
   hashes, and relevant native inventory. A changed precondition requires a new
   plan; never apply a stale preview.
5. Apply actions, checkpoint outcomes, and commit state. Use atomic per-file
   replacement where supported and preserve file permissions. Atomicity of one
   file is not a claim of a transaction over several files or external CLIs.

Recheck an action's destination preconditions immediately before applying it.
If a later action conflicts, stop and report the already-applied actions through
the journal. The writer lock coordinates upskill processes; it cannot make an
uncooperative external editor participate in a cross-file transaction.

Use a local operation journal under `<scope-root>/.upskill/state/`, excluded
from version control through a generated `.gitignore` inside that state
directory. Project scope uses the project root; user scope uses the user's home.
The journal stores action identities, pre/post hashes, native outcomes, and
owned declarative values. It does not store credential values or copies of
unrelated user configuration. Restrict permissions on transient state files.

The writer lock covers normal operations and recovery; a second writer reports
the running operation. A stale lock requires owner-liveness verification before
recovery. After interruption, inspect journal and actual state before retrying.
An external operation with an unknown result is observed first, never blindly
repeated. If native observation is unavailable, report unresolved state and the
required manual reconciliation. Do not report successful rollback unless every
affected action has a verified inverse.

### Outcomes and observation

Track configured files, native registration, enablement, trust, authentication,
and successful runtime use separately. A native CLI exit code alone cannot
prove discovery or enforcement. Missing CLI, skipped, failed, pending trust,
and unknown outcomes remain visible.

`doctor` remains non-mutating and does not connect to servers or run hooks.
Offline checks compare owned artifacts and state; native inventory queries are
allowed only when the profile establishes them as observational. Connection and
semantic tests belong to an explicit verification workflow. Library code returns
structured reports; `main.rs` formats them.

## 5. State and source migrations

Introduce **lockfile schema 2** with explicit installation roots, requested
surfaces, item/bundle references, owned artifact records, and operation outcomes.
Schema 1 remains readable. Schema 2 writing begins only once content lifecycle
can preserve these records on add, update, remove, list, and doctor. Integration
entries not yet migrated retain a tagged legacy representation and cannot be
silently omitted during save.

Source schema and lockfile schema evolve independently. Keep source schema 1
for currently defined content. New required semantics that an old reader could
ignore must use a higher source schema and fail on unsupported readers. Decide
the concrete hook/policy grammar in their category designs; M0 does not introduce
speculative fields that schema-1 tools would silently discard.

Migration rules:

1. Parse legacy source labels before the modern source parser rejects them;
   normalize only unambiguous historical forms, preserving host, ref, and path.
   Ambiguous input receives an actionable diagnostic rather than a guessed URL.
   This includes the case tracked in [#236](https://github.com/driftsys/upskill/issues/236).
2. Expand schema-1 empty item clients to the original three generation clients,
   never to newly introduced targets. Mark requested surfaces as unresolved when
   CLI versus VS Code selection cannot be recovered. Preserve recorded MCP/plugin
   targets as separate facts; do not infer permission to expand or prune them.
3. Reconstruct candidate artifact paths from the legacy contract. Missing paths
   remain missing. Source hashes do not count as generated-byte ownership proof.
   Only adopt artifacts matching reproducible legacy output and a verified source
   revision; otherwise preserve them and require explicit reconciliation.
4. Old ancillary files remain user-owned. Never claim ownership of an entire
   existing `AGENTS.md`, `CLAUDE.md`, settings file, or native config by filename.
5. Preview migration and conflicts before writes. Pure list/doctor reads do not
   rewrite a lockfile. Ambiguous state blocks only operations needing that
   missing ownership/selection fact; it remains inspectable.
6. Retain an exact schema-1 state backup before the first schema-2 write. Old
   upskill versions already reject newer schemas; explain that restoring the
   lockfile alone cannot undo newly generated output or native operations.
   Downgrade requires a verified inverse plan; otherwise it is unsupported.

Target narrowing removes unchanged owned content contributions with no remaining
consumers. For MCP/plugin operations, retain the existing explicit-removal
behavior until M4/M5 provide ownership-aware reconciliation. Report retained
integrations in preview and doctor; do not silently declare narrowing complete.
This is the proposed disposition of [#244](https://github.com/driftsys/upskill/issues/244)
for the foundation, with an ADR-0010/0012 clarification before implementation.
Removing a reference never authorizes deletion of a shared marketplace or an
installation that existed before upskill.

## 6. Permission preservation from the first slice

Represent the following independently in validation and plans: prose guidance,
tool exposure, execution authorization, approval behavior, and filesystem/network
boundaries. Native restrictions on skills, agents, MCP, plugins, and hooks are
subject to this contract before M7 adds general policy management.

An omitted tool list inherits; an explicit empty list grants no exposure; a
nonempty list requests the stated set. Each adapter must prove that mapping for
the relevant field rather than applying these meanings to arbitrary fields with
similar names. Never silently drop an unknown tool or convert a restriction
into an omitted native field. A stronger restriction is still a behavioral
change and needs an explicit author alternative, not automatic substitution.

Show permission deltas as unchanged, narrowed, broadened, or unknown, with
native source and scope. Unknown is not safe-by-default evidence. Applying a
broadened or unknown policy requires explicit acceptance of that plan; unattended
operations fail with a diagnostic unless a category-specific authorization
contract has been defined. Existing content-only installs do not gain a generic
interactive approval ceremony.

Do not distribute saved personal approvals, disable native hook trust, change
managed constraints, or report prose as sandbox enforcement. Keep references to
credentials intact and redacted in diagnostics. Native configuration can make
code run later: installation status must not imply that executable content has
been reviewed, trusted, or run. Permission and hook semantics remain
version-sensitive as documented in the
[roadmap](2026-10-11-agent-customization-roadmap.md#m7--permissions-and-execution-policies).

## 7. Acceptance examples

The implementation plan must turn these into failing tests before code changes.
Tests touching client homes use the repository's fake-home harness.

| ID  | Scenario and required result                                                                                                        |
| --- | ----------------------------------------------------------------------------------------------------------------------------------- |
| F01 | No selection after adding Codex: only the historical default set is planned                                                         |
| F02 | Explicit Codex selection: only its supported scope artifacts are planned; other client discovery exposure is reported               |
| F03 | Copilot CLI plus VS Code: compatible content writes once; distinct native integration actions remain separate                       |
| F04 | Two renderings target one shared path with incompatible behavior: error before any destination changes                              |
| F05 | Remove one of two owners: shared artifact survives; last-owner removal touches only unchanged owned files                           |
| F06 | User edits an owned file or adds a resource after installation: conflict/preservation, never recursive deletion                     |
| F07 | Empty/omitted/restricted tool sets: three distinguishable results; unknown restrictions cannot become unrestricted                  |
| F08 | Edit outside an instruction region: survives update/removal byte-for-byte; an edited owned region conflicts                         |
| F09 | Unsupported version, scope, event, or policy: precise capability diagnostic with no silent translation                              |
| F10 | A destination changes after preview: applying the stale plan stops before its first write                                           |
| F11 | Crash after a file action or native operation: recovery reconciles actual outcome and never duplicates an unknown operation blindly |
| F12 | Schema-1 empty clients: excludes Codex; ambiguous Copilot/VS Code history stays unresolved without guessed writes                   |
| F13 | Legacy source label and unavailable original source: state remains readable; migration does not fabricate ownership                 |
| F14 | Narrow selection: owned content is reconciled; retained legacy integrations are explicitly reported                                 |
| F15 | Missing client CLI: file generation can remain unverified; required native action cannot be recorded as installed                   |
| F16 | Symlink escape, case-alias collision, or competing writer: no writes beyond owned/locked destinations                               |
| F17 | Plugin supplies an already-present item: ownership conflict or verified sharing, never two untracked copies                         |
| F18 | Broadening/unknown policy delta without explicit authorization: unattended apply stops and emits no secret values                   |

Native conformance fixtures must additionally demonstrate discovery and effective
restrictions. Their absence keeps a profile unverified even if every deterministic
test above passes. Record contracts for both Cursor and Gemini against the
artifact model: native rule/command formats, extension packaging, hook events,
and policy files must fit without pretending their adapters exist.

## 8. Incremental implementation boundaries

Plan focused changes in this order. Do not implement schema 2 or replace every
integration path in the first PR.

| Slice | Responsibility and likely owning files                                                                       | Gate                                                     |
| ----- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| F-A   | Profiles/capability diagnostics and explicit defaults in new `src/target/`, `src/select.rs`, `src/config.rs` | F01/F03/F07/F09; legacy selectors still pass             |
| F-B   | Artifact/contribution planning in `src/pipeline/`; adapt pure renderer results and scope paths               | F02/F04/F08/F10/F16; M1 instruction decisions supplied   |
| F-C   | Ownership/state migration and lifecycle in `src/lockfile.rs`, `src/source.rs`, `src/pipeline/lifecycle.rs`   | F05/F06/F12/F13/F14; legacy state readable               |
| F-D   | Apply/recovery and preview reporting in `src/pipeline/`, `src/main.rs`                                       | F10/F11/F15/F16/F18; no native-operation success fiction |
| F-E   | M1 adapters and native qualification; then M2–M7 category plans                                              | Remaining applicable fixtures and native evidence        |

These are design boundaries, not independently approved executable plans. F-B,
F-C, and F-D must land behind an inactive path until a complete content lifecycle
uses them; a release must not mix ownership-aware installation with legacy
recursive removal. Do not expose Codex selection before its first supported
instruction artifact and lifecycle are available together.

Before implementation, prepare a bounded plan for F-A, select the initial native
profile test versions, and propose ADR amendments for default selection and
legacy integration retention. F-A can land as internal contract machinery
without changing public defaults or claiming Codex support. Exact Rust APIs,
state serialization details, and test files belong in those slice plans.

## Decisions still requiring evidence

- Native discovery and effective policy tests for the candidate versions and
  operating systems; version probes alone do not close this requirement.
- Copilot/Codex IDE extension versions and their separate surface contracts.
- A demonstrated JSONC/TOML editing library choice before shared-config slices,
  including dependency size and preservation tests.
- Category-specific layouts for Codex instruction aggregation and safe
  client-specialized skill placement when compatibility discovery overlaps.

These evidence gates prevent an M0 completion claim today. They do not prevent
reviewing the foundation model or writing the first bounded implementation plan.
