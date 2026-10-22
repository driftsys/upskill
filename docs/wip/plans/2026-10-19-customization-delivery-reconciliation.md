# Customization delivery reconciliation and corrected Gate 6 plan

Use superpowers:executing-plans to execute bounded tasks with acceptance
examples first. This plan revises existing M1/Gate 6 delivery; it does not
restart the completed ownership/preflight/recovery foundation or authorize
activation.

**Goal:** implement and qualify independent Codex/OpenCode plugin rules beside
team instructions, with explicit cross-client discovery, activation and
preservation for every customization.

**Spec:**
[corrected Gate 6 design](../specs/2026-10-18-gate6-activation-design.md),
[delivery contract](../../topics/0009-customization-delivery.md) CD-01–CD-11 and
[AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md). The
product roadmap alone owns release ordering.

**Constraints:** Rust edition 2024/MSRV 1.85; shared synchronous Rust CLI, no
new Node/npm/async dependency. Client-required integration code stays in the
thin plugin. Preserve four equal initial clients, native permissions, every
current kind/command, manual config/content and unrelated ownership. Keep
production schema 1 until corrected activation gates pass. General
hooks/tools/permissions and expanded MCP management stay outside current
implementation scope.

## Reconciliation status

- [x] Review roadmap, AD-0039 and canonical ownership/generation/lifecycle
      contracts against intended plugin delivery and every customization.
- [x] Replace the active M1 acceptance examples and Gate 6 canonical-file
      activation assumptions; classify contribution code as experimental.
- [x] Record per-client delivery/discovery/activation, possible other readers,
      sharing intent, F/R/N/B preservation and I/X/O/Q/P implementation status.
- [x] Retain the reusable qualified executor, capture, preflight, backup,
      ownership and recovery contracts; retain historical evidence as bounded.
- [x] Obtain independent architecture/acceptance review, resolve contradictions
      and run `just fmt`, `just docs-check`, `just book` and link checks before
      downstream implementation. Documentation checks establish no native
      support.

Review focus: compatibility directories are not private; payload equality is not
consent; canonical-file discovery is not plugin activation; file journaling does
not recover unknown native side effects; scope/order/status labels cannot reduce
the four-client gate or turn planned agent/MCP/hooks/tool work into M1 features.

## Shared-root comparison follow-up (#349)

This extends the completed inventory without adopting a root or resetting
historical checks. Qualify native feasibility before adapter selection; qualify
integrated lifecycle after implementation and before route adoption. Gate 6
retains existing-kind preservation; new owned-skill lifecycle follows it and
blocks new skill-layout adoption and combined MVP release until qualified. No
unadopted route gains production effects.

Documentation reconciliation is the bounded deliverable of #349. Unchecked
native feasibility work continues under #337; new owned-skill lifecycle and its
integrated adoption controls remain with the later skill milestone (#287).

- [ ] Review the
      [alternative-root evaluation](../../technotes/shared-content-root-evaluation.md):
      intentional `.agents` sharing, intentional `.claude` sharing and selective
      Copilot/OpenCode compatibility exclusions.
- [ ] Record frozen-version exclusion support and effective configuration scope.
      Skills, agents, rules, commands and imports require separate controls.
- [ ] Qualify CD-11 native feasibility across all four clients and applicable
      platform/scope profiles: one shared item per client, no direct/plugin
      duplicates, variants outside shared catalogs and exclusions with working
      native/shared positives. Qualify scoped rules and actual delegated child
      restrictions; reject unsupported mappings.
- [ ] After each adapter is implemented, qualify integrated ownership,
      update/removal and recovery at its roadmap milestone before adoption.
      Preserve manual configuration, existing kinds and unrelated registrations.
- [ ] Reconcile every customization/client route and informative matrix
      candidate before implementation. Generated files and prior plugin-rule
      observations do not satisfy these controls. Production activation remains
      blocked.

## Shared-core authoring clarification (#324)

The following are planned acceptance criteria for candidate delivery patterns,
not native observations or additional Gate 6 implementation scope. Whole-body
overrides replace canonical content; they do not compose with it. Current
historical installation ignores override bodies and captured compilation rejects
active matching overrides under AD-0025. Implementing override execution or an
embedded composition/include feature requires separate work.

- [ ] For small orchestration differences, generated inline-directive examples
      preserve the common procedure for each intended client and select only its
      orchestration. Direct native shared discovery receives no raw client
      directives; generated-file checks alone do not qualify native behavior.
- [ ] For large differences, each candidate wrapper explicitly reads its common
      `references/review-procedure.md`. Native traces establish the actual read,
      relative resource identity and subsequent use of the intended native agent
      mechanism. Instructions alone do not establish child execution or grants.
- [ ] Test allowed and denied resource reads, a missing common resource and a
      relocated self-contained package with successful controls. Do not claim
      successful loading from a final marker or weaken native permissions.
- [ ] Keep variant families outside shared scanned catalogs and qualify one
      effective entrypoint per logical skill/client across direct and plugin
      discovery. Copied procedure resources are not extra entrypoints. Preserve
      manual resources, existing configuration and unrelated ownership during
      installation, update and removal, including surviving bundle references.
- [ ] Apply the four-client platform/scope and coexistence obligations before
      adopting any route. No new override suffix, route adoption, owned-skill
      lifecycle advancement or schema-2 activation follows from this
      clarification.

## Task 1: Pin profile and reader inventories before choosing plugin events

**Files:** extend `docs/technotes/customization-adapter-evidence.md` with exact
upstream references; create
`docs/technotes/agent-customization/customization-delivery-matrix.json` and a
human-readable evidence note. Add schema/fixture tests in
`scripts/tests/test_customization_delivery_matrix.py` only when the matrix
becomes executable input; do not invent a new product manifest format.

- [x] Resolve exact released versions common to macOS arm64/Linux x86_64 and
      freeze the campaign. Inventory each applicable project/user matrix cell
      for rules, skills/resources, agents, bundles, plugins and MCP, plus
      explicit deferred hook/tool/permission obligations. Record
      IDE/other-platform regression obligations separately.
- [x] For every cell, list entrypoints, native/config/package roots,
      compatibility directories, imports/globs, nested/ancestor discovery,
      potential readers, intended consumers, activation conditions, duplicate
      paths, preservation strategy and evidence status. Unknowns are unresolved,
      never empty sets.
- [x] Include the existing actual `.claude/skills`, `.github/skills`,
      `.agents/skills`, `.agents/rules`, agent and MCP/plugin paths in the
      inventory; do not assert historical layouts implement the intended route.
- [x] Record the exact supported installed-plugin rule event/API and trust
      requirements. A model-invoked skill is not evidence of always-on rule
      delivery. If the released runtime cannot provide the required route, mark
      it blocked; do not substitute managed team instructions or narrow scope.

Produces: versioned evidence/inventory records, not production target defaults.

## Task 2: Qualify installed independent rule delivery before adapter code

**Files:** extend or create a separate opt-in `scripts/qualify-plugin-rules.py`,
its offline fixture/classification tests in
`scripts/tests/test_qualify_plugin_rules.py`, disposable native plugin fixtures
under `tests/fixtures/native-plugins/`, and a dedicated technote. Keep
`scripts/qualify-native-instructions.py` identified as canonical-file background
probes; no reinterpretation of old summaries.

- [x] First write failing offline examples named `team_instructions_unchanged`,
      `failed_positive_cannot_qualify_negative`,
      `disabled_plugin_retains_team_guidance`,
      `unintended_reader_never_activates_payload` and
      `direct_and_packaged_rule_activate_once`. They validate harness mechanics,
      not native behavior.
- [ ] Implement disposable installed-package fixtures using Task 1's native
      contract. Do not edit real personal instruction roots, copy auth or
      disable existing user's discovery to force a pass. Isolated authenticated
      user-root provisioning is an explicit prerequisite, not an inferred global
      pass.
- [ ] Run CD-02/CD-03/CD-04 paired native controls: two rules beside
      unpredictable manual team markers, plugin enabled/disabled/removed,
      fresh/resume/ compaction as applicable, and every unintended client's
      compatible paths, imports/globs and nested roots. Require parseable
      version and successful positive/fallback turns with no unapproved
      tool/native side effects.
- [ ] Preserve sanitized evidence, exact commands/configuration and hashes
      outside the repository; retain reviewable sanitized receipts in the
      evidence directory. Test timeout/interruption process cleanup.
- [ ] Stop the affected route at blocked/inconclusive outcomes, and retain the
      full release gate. Native package install alone does not qualify
      injection.

Produces: qualified adapter constraints or explicit blocking evidence. No
production CLI switch, schema migration or public plugin publication.

Preparation progress:
[fixture task #339](https://github.com/driftsys/upskill/issues/339) adds offline
controls and disposable packages. The
[probe note](../../technotes/gate6-plugin-rule-probes.md) records installation
observations and authentication/runtime blockers. Task 2 remains incomplete; no
native route is qualified.

### Authenticated project evidence slice (#345)

After normal isolated-profile login and ordinary hook trust, retain the bounded
macOS project controls in a separate evidence note and sanitized receipt set.
Use the existing fixture and offline classifier unchanged. Record exact model
routes, native completed-turn/session data, per-session hook counts and team
snapshots. Separate unsuccessful CLI override controls from persisted native
configuration controls. Record native remove/reinstall/cache effects and exact
configuration restoration without claiming unknown cloud effects are recovered.
Keep unrelated cloud plugin registrations visible and unchanged. No adapter,
production dispatch or whole-profile qualification follows from this slice. The
remainder of Task 2, including compaction, user scopes, Linux, unintended
readers, duplicate paths and other clients, remains incomplete.

### Shared-skill native evidence slice (#351)

Retain the
[bounded Copilot observations](../../technotes/gate6-shared-skill-controls.md)
with sanitized event receipts and distinct raw/retained hashes. Nine completed
sessions cover shared invocation/removal, relative resource reads, narrow
Claude-source exclusions, surviving manual/shared skills and restored baseline.
Native config write/readback confirms the selected setting, but explicit native
activation still succeeds: keep both isolation negatives failed. Do not treat
these observations as automatic-selection, actual-subagent or complete
preservation/lifecycle proof. Independent before/after bytes, nonempty user
settings, other readers/platforms/scopes and integrated ownership remain
pending. Task 2 and CD-11 stay incomplete; all 160 campaign cells remain
unqualified.

### Claude skill-route native evidence slice (#353)

Retain the
[bounded Claude controls](../../technotes/gate6-claude-skill-controls.md) after
ordinary isolated reauthentication: native positive/failed shared-route
candidate, development plugin positive/absent/restored, and normal project
marketplace install/disable/enable/uninstall. Require successful completed
native turns and matching Skill tool results, separate development loading from
installed behavior, and retain before/after snapshots where captured. Record
native removal's empty configuration objects; preserved manual semantics/mode
are not byte restoration. No complete duplicate, resource, unintended-reader,
rule/subagent, user/Linux or integrated lifecycle result follows. Task 2/CD-11
remain incomplete; no adapter or shared layout is selected and all 160 cells
remain unqualified.

### Skill coexistence and duplicate evidence slice (#356)

Retain
[bounded common-project and duplicate controls](../../technotes/gate6-skill-coexistence-controls.md)
with declared expectations, catalog/native invocation counts, exact model and
raw/retained hashes. Preserve the revoked-login calibration, failed Claude
bare-name/exact-fallback expectations and Copilot same-name count mismatch.
Distinguish model selection of an existing namespace from direct path discovery.
Record alias exposure, surviving unrelated manual/shared content and native
configuration residue. The private helper's post-capture mode error and guarded
restoration do not establish uninterrupted preservation or recovery. Correct
pre-existing planned/observed wording (#355) without qualifying any cell. Task
2/CD-11, other clients/scopes/platforms/IDE, rules/subagents/resources and
integrated ownership/recovery stay incomplete; no shared layout is adopted.

### OpenCode child-execution evidence slice (#358)

Retain the
[bounded actual-child controls](../../technotes/gate6-opencode-child-controls.md)
with six parent/six child identities, selected model, exact responses, child
idle outcomes, native tool IDs and before/after fixture snapshots. Distinguish a
configured child from a file-backed child, a no-attempt denial response from an
actual `permission.rejected` patch, and the independent narrow positive from the
path pair's broader positive policy. Retain the original export-parser error,
its correction and separate actual patch-resource bindings; do not reinterpret
earlier empty debug catalogs or repair old manifests. Manual native fixtures do
not qualify legacy generated permissions or implement owned-agent management.
Task 2/CD-11, four-client coexistence, rules/MCP/delegation, other scopes,
platforms/IDE and integrated lifecycle remain incomplete; all 160 cells remain
unqualified and schema-2 activation stays blocked.

### Unchanged-policy OpenCode path pair (#363)

Retain the [new path pair](../../technotes/gate6-opencode-policy-parity.md) as a
separate experiment: one unchanged file-backed agent/body, ordered permission
policy and parent configuration across two fresh parent/child sessions. Require
actual sentinel patch success and a nonmatching protected-file patch receiving
native `permission.rejected`, with selected patch resource bindings and exact
before/after bytes/modes. Preserve the earlier positive's extra edit glob and
its failed policy-equality comparison; new observations do not repair old data.
The new bounded pair does not qualify historical generated permissions, shared
catalogs, rules/MCP/delegation, other clients/scopes/platforms/IDE or integrated
lifecycle. Task 2/CD-11 remain incomplete; all 160 cells stay unqualified and
production schema-2 activation remains blocked.

### OpenCode shared-skill/runtime-resource evidence (#366)

Retain
[native skill/resource controls](../../technotes/gate6-opencode-shared-resources.md):
native-directory calibration, the same entry moved to shared discovery, a
supporting-file error, a missing-entrypoint native error and a restored shared
positive. Require actual skill/read tool IDs and resource paths, exact declared
responses, selected model/idle outcomes and unchanged manual fixture snapshots.
Keep one fixture entry exposed at a time; do not infer native catalog counts,
configured/default source exclusivity or duplicate/coexistence qualification.
Portable source and runtime reads do not implement overrides, embedded includes,
generated adapters, review-child orchestration or new owned-skill lifecycle.
Task 2/CD-11 and remaining four-client/platform/scope/IDE/integrated obligations
remain incomplete; all 160 cells stay unqualified and schema-2 activation stays
blocked.

## Task 3: Add reader intent and qualified rule adapters to the foundations

**Files:** extend the relevant `src/target/`, `src/lifecycle/contracts.rs`,
`src/lifecycle/preflight.rs`, `src/lifecycle/orchestration.rs` and
`src/lifecycle/desired/` modules; add only the plugin adapter files required by
Task 2's chosen route. Tests belong beside those modules and in
`tests/lifecycle_preflight.rs`, `tests/lifecycle_orchestration.rs`,
`tests/lifecycle_codex_orchestration.rs`, and a separate OpenCode route target.
Exact structs/signatures require a bounded file-level subplan after Task 2; this
plan does not pretend the native event/API is already selected.

- [ ] Write failing examples
      `compatible_bytes_do_not_authorize_incidental_reader`,
      `excluded_client_cannot_activate_compatibility_copy`,
      `plugin_update_preserves_team_agents_bytes_and_mode`,
      `plugin_remove_retains_manual_configuration` and
      `two_rule_plugins_keep_unrelated_owner_references`.
- [ ] Keep intended ownership separate from possible readers. Explicit sharing
      requires qualified contracts, not merely identical canonical text.
      Preflight the complete selected set, including package/config side
      effects, before journal or publication. No successful-subset execution.
- [ ] Implement only routes qualified by Task 2. Use independent rule payloads
      and registration; never route production through experimental managed
      `AGENTS.md` contributions. Keep recorded experimental removal/recovery
      safe.
- [ ] Extend native-effect observation/recovery with typed pre/post outcomes and
      explicit inverse/replay policy before performing those effects. Retain
      source/destination drift, root authority, writer lock and exact backup.
- [ ] Run affected tests plus `lifecycle_application`, `lifecycle_state_origin`,
      `lifecycle_sharing`, `lifecycle_end_to_end` and complete preflight
      controls.

Produces: opt-in qualified rule adapters and safety evidence, still no
activation.

## Task 4: Qualify all-kind compatibility without advancing new features

**Files:** extend existing generation/resource/bundle/plugin/MCP/CLI tests and
installed-state inspection/selection code only as needed for compatible state
preservation; maintain the CD inventory and native evidence note. Start each
behavior change with its CLI or library acceptance example.

- [ ] Exercise mixed rules, skills/resources, agents, bundles, plugins and MCP
      rows and ownership; test direct-plus-plugin, same alias/name, project/user
      copies, multiple bundles and intentional sharing. Include existing IDE
      behavior and preserve native restrictions and manual configuration.
- [ ] Add named examples `resource_glob_does_not_inject_instructions`,
      `manual_mcp_registration_survives_content_removal`,
      `legacy_projection_cannot_replace_owned_state`,
      `unrelated_agent_permissions_survive_update` and
      `last_bundle_reference_controls_native_removal`.
- [ ] Test install/update/remove/list/doctor/preview and `remove-mcp` with mixed
      schema-2 state through their actual prospective adapters. Keep ambiguous
      legacy rows inspectable and block unknown ownership/native effects.
- [ ] Run each applicable native discovery/activation/negative/duplicate cell;
      missing native cells stay blocked. Offline generation proves only output.
      Do not implement general agent/skill/MCP expansion, hooks/tools or
      permission management merely to complete the inventory; document
      capability limits.

Produces: compatibility proof and concrete blockers for CD-05–CD-09. If
preserving existing behavior requires broader adapter work, create a bounded
prerequisite; never silently trim existing support or the four-client gate.

## Task 5: Combined production dispatch and guarded migration

**Files:** prospective `src/main.rs`, `src/pipeline/`, existing lifecycle
application/state modules, `tests/cli_*`, specification/user guide and migration
records. Requires Tasks 1–4 and applicable G6/CD native evidence first.

- [ ] Write failing CLI examples for G6-01–G6-05/G6-08, CD-06/CD-07: explicit
      selectors/defaults, independent plugin rules, mixed-kind state,
      original-byte backup, interrupted/reopened operation, held lock, offline
      source removal, external-home authority and incomplete native-effect
      outcomes.
- [ ] Implement all installed-state command routing and schema-1 migration in
      one reviewed activation task through qualified sessions. Do not mix new
      writers with recursive legacy removal or legacy-only state saves.
- [ ] Run `just fmt`, `just verify`, `just sanity`, `just msrv-check`,
      `just book` and the pinned native campaign. Retain exact-head receipts,
      independent review and hosted platform checks before claiming completion
      or merging.
- [ ] Publish accurate supported/unsupported matrix, migration, preservation,
      failure/recovery and reload guidance. Garden only completed task WIP;
      retain broader unfinished plans. Release scope does not shrink when a
      prerequisite is blocked.

No task above is marked implemented or qualified by this document. Further
implementation and native requests proceed only against the reviewed corrected
architecture and the task's prerequisites; production activation remains
blocked.

Architecture check: an isolated builder review found nothing important left in
the corrected contract/design/plan. Documentation checks establish link/render
consistency only. Initial version/plugin-help observations are retained in the
[reconciliation note](../../technotes/customization-architecture-reconciliation.md);
Task 1 records the frozen campaign, explicit reader/profile inventory and pinned
API candidates in the
[profile inventory](../../technotes/gate6-plugin-profile-inventory.md).
Unresolved native roots, trust and execution are Task 2 prerequisites; no later
task is complete or qualified by static inventory evidence.
