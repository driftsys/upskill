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
