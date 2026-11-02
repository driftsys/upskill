# Four-client skills and resources implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development or superpowers:executing-plans to
> implement this plan task by task. This document does not authorize execution.

**Status:** Design and implementation sequence approved on 2026-10-31. The user
authorized starting Stage 1 with inline implementation and independent review.
Native qualification and production activation retain their separate gates.

**Goal:** Deliver selected skills and their resources through independently
qualified Claude Code, Copilot CLI, Codex CLI and OpenCode CLI routes.

**Architecture:** Compile captured source into separate immutable item packages.
Coordinate native binding receipts with existing qualified file ownership and
reference planning. Keep production schema 1 unchanged until admission gates
pass.

**Tech stack:** Rust 2024, MSRV 1.85, synchronous native CLI adapters, existing
capture/hash/ownership machinery. No new dependency is admitted by this plan.

**Spec:**
[Four-client skills/resources design](../specs/2026-10-31-four-client-skills-resources-design.md).

## Global constraints

- One self-contained package per logical item and intended client; copy only.
- Managed root: `<scope>/.upskill/exports/<client>/<installation-id>/`.
- One selected body and one effective exposure per logical item/client.
- No Codex addition to legacy `Client::ALL`; schema-1 behavior stays unchanged.
- Existing preservation D1-D6, Q10/Q11, CD-01 through CD-11 and four-client
  release gates remain mandatory.
- No paid native calls without a fresh allowance check and authorized bounded
  experiment. Respect the operator's current reserve limits.
- Separate worktree and bootstrap per task; tests, docs and code in one task PR.
  Run `just fmt` and `just verify`; independently review before squash merge.

## Review focus

- Aliases and native passthrough must not change identity or widen permissions.
- A copied clone or nested CWD must not select another checkout's resources.
- Shared references and retained sessions must not lose required package bytes.
- Native timeout or crash must not be mistaken for absence or safe rollback.
- Duplicate/commented config and concurrent user edits must preserve manual
  bytes.

Each stage is independently reviewable. Stages 1-2 are pure library work, 3-5
build inactive infrastructure, and 6 is admission/integration. Do not execute
blocked native profiles while developing the offline stages. Obtain design
approval before product implementation; reconcile pending step-7 decisions and
state compatibility before wiring owned CLI effects.

## Stage 1: Pure selected-skill compilation

**Local status:** Implemented in `6029866`; QUICK review findings are addressed
by the follow-up tests and documentation changes tracked in #458 and #459. All
22 selector tests pass, all seven deliberate regressions are caught, and
`just verify` passes. This is local verification, not merge or native admission.

**Files:** Create `src/compile/skill_selection.rs`; modify `src/compile/mod.rs`;
create `tests/skill_selection.rs`; update the content-format topic with the new
opt-in boundary. Keep `src/compile/item.rs` rejection behavior intact.

**Interfaces:** Define `SkillSelectionRequest` containing captured manifest,
`TargetSurface`, installed name and permission/capability context. Produce
`SelectedSkill` containing canonical metadata, selected-body provenance,
rendered body and ordered captured resources. Export
`select_skill(request: &SkillSelectionRequest) -> Result<Option<SelectedSkill>>`.
An audience-excluded target returns `None`; invalid source or projection returns
an error. The request owns or borrows the captured evidence; it never rereads
source paths.

- [x] Write failing tests: audience exclusion precedes override selection;
      existing three overrides are verbatim; canonical Codex directives filter;
      `SKILL.codex.md` rejects; alias binds native name and directory;
      passthrough cannot replace identity; nonempty native blocks and unknown
      fields reject; canonical license/metadata emit through capability
      validation or reject; changing original source cannot alter captured
      resources.
- [x] Run `cargo test --test skill_selection`; confirm failures identify the
      absent selector or unmet behavior, not fixture/setup errors.
- [x] Implement selection and explicit metadata projection, preserving resource
      bytes/modes and subtractive ignore semantics from the design.
- [x] Rerun the new tests plus `cargo test --test compile_items` and
      `cargo test --test generate_skills`; require all to pass unchanged.
- [x] Update source-format documentation and commit
      `feat: compile selected skill representations`.

## Stage 2: Immutable package planning

**Local status:** Implemented in `07c7dd8`. QUICK review test gaps are addressed
with independent aliased/resource digest values and isolated validation tests.
All 16 package tests pass, all ten deliberate regressions are caught, and
`just verify` passes. The catalog metadata is at the item root so revision
sources remain within that root. This is local verification, not native
qualification or merge. GitHub task/PR tracking is pending API connectivity.

**Files:** Create `src/compile/skill_package.rs` and `tests/skill_packages.rs`;
modify `src/compile/mod.rs`; document the candidate layout in the customization
delivery topic.

**Interfaces:** Consume `SelectedSkill`. Define `SkillPackageRequest` with the
selected skill, target, qualified scope identity and `ArtifactOwner`. Export
`compile_skill_package(request: &SkillPackageRequest) -> Result<SkillPackage>`.
`SkillPackage` exposes full payload digest, full package digest, immutable
relative files, stable catalog contribution and native identity. No I/O occurs.

- [x] Add failing fixtures for all four layouts, full digest/version identity,
      mode-sensitive hashing, alias paths, no duplicate default component path,
      exact resource bytes, path collision and repeatable output. Assert stable
      native identity across revision, alias and bundle changes, with different
      scope or installation IDs producing distinct identities.
- [x] Run `cargo test --test skill_packages`; confirm intended failures.
- [x] Implement domain-framed hashing and client-specific envelopes. OpenCode
      produces an explicit-source descriptor, without a JavaScript plugin.
- [x] Rerun fixtures and existing resource tests; assert legacy goldens
      unchanged.
- [x] Document the layout as inactive and commit
      `feat: plan immutable client skill packages`.

## Stage 3: Portable intent and native receipt contracts

**Local status:** Implemented in `2de2226`, with QUICK coverage findings
addressed by exact staging/selector assertions, independent native projections,
and isolated receipt/origin/ownership tests. All 24 acceptance cases pass and 15
deliberate regressions are caught. `just verify` passes. AD-0042 records the
reviewed in-memory boundary; no serializer, public migration or native admission
is selected. GitHub tracking/publication remains unavailable.

**Files:** Create `src/lifecycle/skill_delivery.rs`,
`src/lifecycle/native_receipt.rs`, `tests/skill_delivery_state.rs`; modify
`src/lifecycle/mod.rs`. Reconcile with the existing state-compatibility work
before choosing any persisted schema or public migration.

**Interfaces:** Consume `SkillPackage` and existing reference changes. Define
`SkillDeliveryIntent` (portable selection), `PackageRevision` (owned immutable
files) and `NativeBindingReceipt` (machine-local operation/prestate/outcome).
Export pure
`plan_skill_delivery(input: &SkillDeliveryInput) -> Result<SkillDeliveryPlan>`.
The input contains current intent, receipts, package candidates, reference graph
and captured native observations. The plan separates files, bindings and
retained revisions; it does not execute them.

- [x] Add failing tests for diamond bundles, direct-plus-bundle ownership,
      conflicting revision/alias selection, override removal without fallback,
      missing/edited bytes, copied absolute-root state rejection and explicit
      clone materialization using the new checkout's scope.
- [x] Run `cargo test --test skill_delivery_state`; confirm intended failures.
- [x] Specify persistence compatibility in a reviewed record, then implement the
      pure records and planner without a second authoritative installation lock.
- [x] Rerun state tests and `cargo test --test lifecycle_references`; require
      reference reachability and existing state round trips to remain unchanged.
- [x] Commit `feat: plan skill delivery intent and native receipts`.

## Stage 4: Owned configuration contributions

**Files:** Create `src/native_config/mod.rs`, `src/native_config/jsonc.rs`,
`src/native_config/toml.rs`, `tests/native_config_contributions.rs`; modify
`src/lib.rs`. A parser dependency requires its own ADR and size/MSRV evidence
inside this stage before implementation; do not add a partial TOML parser.

**Interfaces:** Define `ConfigContribution` (format, exact selector, expected
prestate, desired value), `CapturedConfig` (bytes and file identity), and
`ConfigPatch` (precondition and resulting bytes). Export
`plan_config_patch(config: &CapturedConfig, contribution: &ConfigContribution) -> Result<ConfigPatch>`.
Planning is pure; application uses qualified file preconditions.

- [x] Add failing tests for JSONC/TOML comments and unrelated byte preservation,
      duplicate keys, conflicting selectors, root-vs-overlay precedence, tracked
      OpenCode overlay rejection, external XDG root rejection and concurrent
      edits.
- [x] Run `cargo test --test native_config_contributions`; confirm failures.
- [x] Implement parsed-span edits and explicit ownership checks. Capture all
      effective native config layers before calculating a contribution.
- [x] Rerun tests; if a dependency is admitted, run `just msrv-check` and
      compare release binary size against the pre-dependency baseline, recording
      both.
- [x] Commit `feat: preserve native configuration contributions`.

## Stage 5: Durable native coordination

**Local status:** Implemented in `cf2f877` and the preceding durable-store
commits. The real coordinator has fake-adapter lifecycle acceptance tests and
fresh-process forward/inverse checkpoint sweeps. It preserves uncertain native
outcomes, explicit recovery choices, resource-retention gates and exact owned
configuration preconditions. This completes the inactive coordinator stage; it
does not establish any real client profile or authorize production dispatch. The
[retained verification evidence](../../technotes/2026-11-01-native-coordination-verification.md)
records recovered RED excerpts, fresh GREEN results and their limitations.

**Files:** Create `src/native_delivery/mod.rs`,
`src/native_delivery/adapter.rs`, `src/native_delivery/journal.rs` and
`src/native_delivery/recovery.rs`; modify `src/lib.rs`; create
`tests/native_delivery_recovery.rs`. Reuse existing execution primitives without
claiming that their File/State journal already covers native effects.

**Interfaces:** Consume `SkillDeliveryPlan`, `ConfigPatch` and receipts. Define
`NativeDeliveryAdapter` with capture, apply, observe and inverse operations and
typed outcomes `Confirmed`, `Rejected` and `Unknown`. Define
`NativeDeliveryCoordinator::apply` and `recover` accepting qualified roots, a
durable operation store and adapter. Return `Complete` or `RecoveryRequired`
with attributable observations; no stdout from library code.

- [x] Write failing fake-adapter acceptance tests for every interruption between
      prepare, durable intent, native effect, observation, selection commit and
      retirement. Include timeout-after-effect, failed inverse, manual changes,
      cross-root contention, no-op drift and uncertain live-session retention. A
      fake native cache invalidated by reinstall must block refresh until cache
      retention or session clearance is proven, despite intact export revisions.
- [x] Run coordinator acceptance before implementation; confirm the absent
      adapter/coordinator APIs fail compilation in `native_coordinator`. The
      retained logs do not preserve the historical command argument vector.
- [x] Implement deterministic multi-root locking, durable intent before native
      mutation and observation-first recovery. Preserve unknown outcomes and old
      resource revisions; prohibit automatic retry and file-only success claims.
- [x] Rerun checkpoint tests plus existing file execution recovery tests.
- [x] Commit `feat: coordinate recoverable native skill delivery`.

## Stage 6: Native adapters, qualification and CLI admission

**Local status:** In progress. Exact-profile production admission remains
closed. The bounded subprocess runner, Claude metadata decoder and fixed
version/plugin-list reader are implemented. A fixed marketplace-list reader now
shares the same bounded version and transport checks, returning a distinct
metadata view without capturing registry/settings file prestate. Claude command
planning and an offline actual-compiler probe fixture cover fixed explicitly
scoped argv. Copilot and Codex candidate planners now cover their captured
user-context command families; both reject project operations, and Codex
local-package refresh rejects instead of using Git upgrade or remove/re-add.
These pieces do not yet implement the complete adapter trait. OpenCode has a
candidate batch planner for owned absolute sources that preserves existing
legacy configuration form. Pure Claude metadata decoders now distinguish
marketplace list output, known-marketplace records and installed-plugin registry
schema 2. Exact name, directory and scope lookups reject ambiguity; parsed
emptiness is separate from file absence. Integrated native local-catalog
capture, effects/inverses, effective observations, other client adapters and
runtime qualification remain open. The
[Claude contract probe](2026-11-01-claude-local-catalog-contract-probe.md) ran
once after explicit approval, with no model spending. Its
[bounded observations](../../technotes/2026-11-01-claude-local-catalog-contract.md)
show full-name/version acceptance and project registry metadata, but exact
settings preservation failed. It qualifies no native profile or stable cleanup;
repeat execution requires separate authorization. A separate
[Copilot contract input](2026-11-02-copilot-local-catalog-contract-probe.md) is
now prepared from the actual compiler in a disposable user root. Its
eleven-command, zero-model proposal retains full names. The local collector adds
one separately bounded offline Git setup and immutable command chronology. Its
review fixes passed full local verification at `5e2d9c2`; the retained packet
also passed read-only preflight. Explicit execution approval is still required.
No Copilot native command has run from that packet.

**Files:** Create `src/native_delivery/claude.rs`, `copilot.rs`, `codex.rs` and
`opencode.rs`; add adapter contract fixtures and `tests/cli_owned_skills.rs`;
modify `src/cli.rs` and `src/main.rs` only after admission. Update the
specification, commands guide and qualification index.

**Interfaces:** Each adapter implements Stage 5's trait. Add a profile admission
registry keyed by target surface, native version, scope and platform. A rejected
or missing qualification produces an actionable typed capability error before
any effects. Preserve existing schema-1 dispatch.

Read-only command inspection is retained in the
[2026-11-01 command-contract note](../../technotes/2026-11-01-native-adapter-command-contracts.md).
It pins help for the installed releases, including changed Copilot/Codex
versions, without qualifying a route. Concrete adapters and the acceptance and
qualification cells below remain open.

- [ ] Add failing fake-native CLI tests for all four routes, including native
      command availability, project admission without transient user activation,
      nested-CWD absolute OpenCode sources, version/name acceptance, cache
      refresh, native trust refusal, duplicate readers and unchanged legacy
      commands.
- [ ] Run `cargo test --test cli_owned_skills`; confirm intended failures.
- [ ] Implement adapters against captured help/fixtures. Do not silently skip a
      missing native executable, install trust, edit caches or weaken Q11.
- [ ] Publish bounded qualification plans with exact positives, negatives,
      preservation controls, limits and safe cleanup. Check live allowances
      before any approved paid experiment; stop before reserve thresholds. No
      cost is authorized by this checklist itself.
- [ ] Execute only approved profiles. Record native resource paths and ordered
      fresh-session receipts. Leave failed/unrun cells explicitly blocked.
- [ ] Reconcile the remaining step-7 decisions, state compatibility and every
      applicable gate. Only then expose the owned-skills CLI and public
      examples.
- [ ] Run `just fmt`, `just verify`, independent review and hosted checks. Keep
      four-client release admission blocked until all required cells pass.
- [ ] Commit `feat: integrate qualified native skill delivery` only when its
      admission conditions hold; otherwise deliver adapter infrastructure with
      the production gate closed and record remaining qualification work.

## Handoff

Continue Stage 6 from the implemented inactive infrastructure. Complete the
client adapters and fake-native lifecycle acceptance, then execute only approved
qualification profiles. Preserve failed/unrun gates explicitly and keep the
production registry closed until the complete route qualifies. Final independent
review, gardening, hosted checks and delivery are still required. This plan
chooses no release date and does not convert retained failures into support
claims.
