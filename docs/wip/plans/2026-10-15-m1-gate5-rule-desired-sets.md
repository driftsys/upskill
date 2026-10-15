# Gate 5 captured rule desired sets implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> to implement this plan task by task. Steps use checkbox syntax for tracking.

**Goal:** Bind complete captured rule compilation to explicit installation roots
and target consumers, producing complete desired sets for the later lifecycle
coordinator.

**Architecture:** Add a pure producer under `lifecycle::desired` consuming the
merged combined-state records and existing captured-item compiler. Keep
client-family rendering separate from explicit surface consumption. A batch
boundary returns either all complete sets or contextual diagnostics, never a
successful subset.

**Tech Stack:** Rust edition 2024, Rust 1.85, existing anyhow/serde types and
compiler; no new dependencies.

**Spec:**
[Gate 5 lifecycle design](../specs/2026-10-15-m1-gate5-lifecycle-integration.md).
This plan is bounded task **2a** within its second delivery slice. PR #295
merged the combined-state prerequisite as `ff4671f`.

## Global constraints

- This is an opt-in pure library; production dispatch and
  `lockfile::CURRENT_SCHEMA` remain unchanged at schema 1.
- No filesystem reads/writes, fetch, backup, journal, migration or source
  scripts.
- Accept captured `SourceManifest` values and validate them before consuming
  their bytes. A digest alone is not source authenticity or containment
  evidence.
- Owned output production is limited to rules here. Skills, agents and native
  integrations remain outside this task; selected unsupported kinds fail
  explicitly.
- Preserve the existing compiler's exact rendered bytes, resource filtering,
  alias link rewriting and captured resource modes.
- Preserve requested surfaces even when audience filtering produces an empty
  desired set. Empty desired output does not mean delete the installation or its
  references.
- Do not enable Codex, infer incidental readers or claim native discovery from a
  family renderer.
- Do not weaken the existing mixed planner's affected multi-owner rejection.
  Reference reconciliation, shared-owner changes and instruction-contribution
  adapter integration remain task 2b.
- Source schema is independent of lockfile schema. This producer neither adopts
  legacy rows nor mutates combined state.

## Review focus

1. A correct source digest paired with a different source name or installed
   alias must fail rather than establish a mismatched owner.
2. Requested consumers must be accounted for exactly, including audience
   exclusions; unsupported surfaces must not disappear or fall back to another
   family.
3. Complete resources, binary bytes, executable mode evidence and alias paths
   must survive projection without rereading source files.
4. An active scoped OpenCode rule must fail rather than lose required scope
   through the historical renderer.
5. A batch with one blocked item or a selected destination collision must return
   no successful subset, while reporting independent item conflicts together.

## Files and public interfaces

- Create `src/lifecycle/desired/mod.rs`: producer request/result types and the
  single-rule boundary.
- Create `src/lifecycle/desired/batch.rs`: batch diagnostics and selected-output
  collision checks.
- Modify `src/lifecycle/mod.rs`: expose `pub mod desired`.
- Create `tests/lifecycle_desired.rs`: public acceptance tests and literal
  output assertions.
- Reuse `tests/compile_items.rs` and existing checked-in rule goldens; do not
  rewrite fixtures.
- Create `docs/design/captured-rule-desired-sets.md`; update specification and
  design index with the implemented inactive boundary.

Public values:

```rust
pub struct CompilerBinding {
    pub client: crate::generate::Client,
    pub consumers: Vec<crate::target::TargetSurface>,
}
pub struct RuleDesiredSetRequest<'a> {
    pub installation: &'a crate::lifecycle::state::Installation,
    pub source: &'a crate::source_manifest::SourceManifest,
    pub folder_name: &'a str,
    pub bindings: &'a [CompilerBinding],
}
pub struct DesiredInstallationSet {
    pub installation: crate::lifecycle::state::Installation,
    pub artifacts: Vec<crate::operations::plan::DesiredArtifact>,
    pub audience_excluded: Vec<crate::target::TargetSurface>,
}
pub fn compile_rule_desired_set(
    request: RuleDesiredSetRequest<'_>,
) -> anyhow::Result<DesiredInstallationSet>;

pub struct DesiredSetDiagnostic {
    pub request_indices: Vec<usize>,
    pub installation_ids: Vec<String>,
    pub message: String,
}
pub enum DesiredSetsOutcome {
    Ready(Vec<DesiredInstallationSet>),
    Blocked(Vec<DesiredSetDiagnostic>),
}
pub fn compile_rule_desired_sets(
    requests: &[RuleDesiredSetRequest<'_>],
) -> DesiredSetsOutcome;
```

These are supplied-value results, not serialized plans or execution authority.
Do not expose a constructor that interprets an empty set as
`InstallationChange::Remove`.

## Compiler binding policy

| Family compiler    | Explicit compatible consumer identities                          |
| ------------------ | ---------------------------------------------------------------- |
| `Client::Claude`   | `TargetSurface::ClaudeCli`                                       |
| `Client::Copilot`  | `TargetSurface::CopilotCli`, `TargetSurface::CopilotVscodeLocal` |
| `Client::OpenCode` | `TargetSurface::OpenCodeCli`                                     |

This table establishes family identity only. It is not native conformance
evidence. A Copilot compilation gets both consumers only when both were
explicitly requested and bound. Codex CLI/IDE have no compiler binding in this
task and receive an unsupported-surface error, including when their author
audience would exclude them.

Require one binding per family, nonempty unique consumer lists, compatible
family identities, and exact union equality with the installation's requested
surfaces. Compile exactly those families. Exclusion facts are projected only
onto the explicitly bound consumers for each excluded family.

## Task 1: Complete captured rule producer

**Consumes:** `Installation`, `validate_state`, `SourceManifest`,
`validate_manifest`, `compile::item::compile_item` and its `ItemCompileRequest`;
`DesiredArtifact::File`, `DesiredFile`, `FileLocation`, `FileValue`.

**Produces:** `compile_rule_desired_set` and its request/result types above.

- [ ] Write `captured_rule_projection_retains_exact_paths_bytes_and_consumers`.
      Capture the checked-in `api-conventions/RULE.md` and a literal binary
      resource `[0, 255, 17]` in a test fixture. Bind Claude and explicitly
      selected Copilot consumers. Assert entrypoint bytes against
      `fixtures/expected/claude/api-conventions.RULE.md` and
      `fixtures/expected/copilot/api-conventions.RULE.md`, exact resource bytes
      and literal destination paths from `tests/compile_items.rs`. Assert every
      file uses the supplied root and exactly its requested consumers. Snapshot
      inputs and assert unchanged values.
- [ ] Run `cargo test --test lifecycle_desired`; require missing
      module/interface failure before implementation.
- [ ] Implement installation validation through the existing combined-state
      validator using a minimal direct-reference graph, without artifacts or
      legacy data. Require `owner.kind == ItemKind::Rule`. Validate the full
      capture and require its digest to equal the installation's recorded
      source-manifest digest. Retain Git/local revision checks from state
      validation; do not infer a revision or claim verified Git origin.
- [ ] Validate bindings using the table above. Build
      `ItemCompileRequest { kind: Rule, folder_name, alias: Some(owner.installed_name.clone()), clients }`.
      Require returned source name, installed name and manifest identity to
      match the installation exactly. Reuse compiler output paths; do not build
      another layout table.
- [ ] Project every compiled entrypoint/resource to `DesiredArtifact::File`,
      preserving bytes and optional modes exactly. Keep exact root spelling and
      scope. Record explicit consumers from the corresponding binding, sorted by
      surface name. Return a cloned, unchanged installation and explicit
      audience-exclusion facts. Sort desired artifacts by portable relative
      path; any repeated path within an installation conflicts, even if bytes
      match.
- [ ] Write `source_and_alias_binding_mismatch_rejects` with independent valid
      fixtures for wrong digest, wrong declared source name, an invalid
      installed alias and unsupported item kind. Write
      `requested_surface_accounting_is_exact` for omitted/extra/duplicate
      consumers, wrong families, duplicate bindings, and unsupported Codex
      selection. Include positive Copilot CLI-only, VS Code-only and explicitly
      both cases.
- [ ] Write `audience_exclusion_keeps_installation_and_requested_selection`: an
      excluded family produces zero artifacts but retains the exact installation
      and reports only its explicitly bound consumers. Never manufacture a
      remove operation.
- [ ] Write `resources_and_aliases_keep_binary_bytes_and_mode_evidence` covering
      ignored files, hidden/Unicode/empty resources, executable and
      non-executable captured modes, multiple resources and aliased links. Use
      literal expected bytes/paths and checked-in goldens, not `compile_item` as
      the expectation builder. Generated entrypoint mode remains `None`;
      physical permission selection belongs to later qualified application.
- [ ] Write `active_opencode_scope_cannot_be_silently_dropped`. Parse the
      captured `RULE.md` using the existing Rule frontmatter parser to inspect
      required `scope.paths`. If OpenCode has actual compiler output and paths
      are nonempty, reject the set. Empty/absent scope is allowed;
      audience-excluded OpenCode produces exclusion facts without a scoped
      output. No renderer or current production behavior changes.
- [ ] Run focused tests and existing `cargo test --test compile_items`; require
      success. Prove that removing digest/name/consumer checks or omitting one
      resource causes an assertion failure, restore exact production bytes, then
      rerun green.
- [ ] Update specification/design with the pure producer and explicit
      unsupported/native-evidence boundary. Commit
      `feat: bind captured rules to complete desired sets`.

## Task 2: Complete batch outcome and contextual conflicts

**Consumes:** `compile_rule_desired_set`; **produces:**
`compile_rule_desired_sets`, `DesiredSetsOutcome`, `DesiredSetDiagnostic`.

- [ ] Write `batch_reports_independent_conflicts_without_a_ready_subset`: three
      requests, one valid and two independently invalid bindings/captures, must
      return `Blocked` with both input-index/installation contexts and no
      accessible successful-set vector. Assert inputs unchanged and stable
      diagnostic ordering.
- [ ] Run the focused test; require missing batch interface or wrong
      successful-partial-outcome failure before implementation.
- [ ] Compile all independent requests, collecting one contextual compile error
      per failed request. This does not claim exhaustive diagnostics within each
      rule; operation-wide selection aggregation remains task 2b. Reject
      duplicate installation IDs with both request indices. Do not silently
      deduplicate supplied requests.
- [ ] For successful captures, detect duplicate and ancestor/descendant selected
      file paths under the same raw root/scope across requests. Report both
      request contexts; reject equal-byte collisions too because this task
      establishes no shared-owner/native compatibility contract. Different exact
      roots/scopes remain separate at this lexical boundary. Physical
      aliases/case equivalence require later qualification.
- [ ] Return `Ready` in input order only when there are no diagnostics. Return
      `Blocked` with diagnostics ordered by request indices then message
      otherwise. An empty batch returns `Ready([])` without effects.
- [ ] Write `batch_rejects_equal_byte_collisions_and_ancestor_topology`,
      `batch_preserves_disjoint_roots_and_scopes`, and
      `batch_rejects_duplicate_ids_before_returning_sets`. Assert literal
      contexts and absence of a successful partial outcome; do not pin
      incidental complete error wording.
- [ ] Run `cargo test --test lifecycle_desired`; require success. Prove that
      returning the successful subset or ignoring a collision makes its targeted
      test fail; restore exact bytes and rerun green.
- [ ] Document batch semantics, scope limitations and remaining reconciliation
      work. Commit `feat: preflight complete rule desired-set batches`.

## Completion and next boundaries

- [ ] Self-check every review-focus item against named tests.
- [ ] Run `just fmt`, then `just verify`, and
      `cargo +1.85.0 check --all-targets --locked`; require zero warnings and
      success.
- [ ] Run the independent implementation review, garden only this completed plan
      into durable records, open one PR, complete required reviews and
      final-head CI. Apply the user's severity/effort disposition policy.
- [ ] Preserve the active Gate 5 umbrella, prior worktrees and unrelated WIP.

Task 2b still reconciles the authoritative reference graph, shared owners and
per-owner consumers, selected legacy conflicts, instruction contribution
adapters, and complete add/update/remove planning. Only then do guarded
backup/application/recovery and observational command adapters follow. This task
neither satisfies all of delivery slice 2 nor establishes the updater
prerequisite or production activation.
