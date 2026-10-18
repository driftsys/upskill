# Source-backed Codex instruction lifecycle implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development to
> execute these tasks. Routine decisions, reviews and delivery are delegated.

**Goal:** Resolve and apply selected Codex rule sources while preserving
complete reader exposure separately from ownership.

**Architecture:** An additive captured-source compiler supplies the AD-0039
result. A separate public Codex orchestration wrapper shares internal session,
complete preflight, resolution and capture lifetimes with the historical
wrapper. Reports are available on opaque scoped preparation, never persisted as
ownership.

**Tech stack:** Existing synchronous Rust crate, edition 2024, MSRV 1.85; no
deps.

**Spec:** `docs/wip/specs/2026-10-18-codex-source-integration.md`.

## Global constraints

- Task #325; base main 2c64620e3ba3d7e613e2c46abea6919ad2ed86bc.
- Work only in `.claude/worktrees/codex-source-integration`; bootstrap ran.
- Production schema1, CLI routing, Client enum and target defaults stay frozen.
- Only CodexCli revision1 intentional; complete five/two-reader producer
  reports.
- Physical declared-root qualification is distinct from native HOME
  qualification.
- Remove performs no source work and ignores current compiler/target
  availability.
- Complete retained representation/selection preflight precedes all resolution.
- No cross-adapter conversions; retain historical instruction rejection.
- Captured source evidence cannot be substituted or refreshed during apply.
- SourcePrepared remains opaque and scoped; exposure never enters owned state.
- Rust checks use RUSTC_WRAPPER= and
  CARGO_TARGET_DIR=/private/tmp/upskill-gate6-target.

## Review focus

- Excluded installations have no artifacts but must retain route
  identity/reports.
- Multi-owner instruction artifacts need per-owner contract/consumer validation.
- Remove must succeed after source deletion and despite invalid compiler
  options.
- A later selected failure must prevent all fetches/preparation/publication.
- Declared user roots do not establish actual native HOME/CODEX_HOME authority.

### Task 1: Captured Codex source compiler

Own `src/lifecycle/sources.rs`, a crate-visible stable contribution-ID helper
extracted in `src/lifecycle/desired/codex.rs`, and new
`tests/lifecycle_codex_sources.rs`.

Public interface:

```rust
pub struct CodexSourceBinding<'a> {
    pub installation_id: &'a str,
    pub expected_source_name: &'a str,
    pub installed_name: &'a str,
    pub root: &'a InstallationRoot,
    pub requested: &'a [TargetSurface],
    pub profile: TargetProfile,
    pub location: &'a InstructionLocation,
}
// Within impl CapturedRuleSource:
pub fn compile_codex(&self, binding: CodexSourceBinding<'_>)
    -> Result<CompiledCodexInstructions>;
// In lifecycle::desired::codex, reuse for retained-state preflight:
pub(crate) fn contribution_id(installation_id: &str) -> Result<String>;
```

- [ ] Write dedicated compile-RED tests for local project/global exact
      destinations, real source manifest/hash identity, alias/folder binding,
      five/two-reader reports and CodexCli-only ownership. Write local
      generic/project exclusion examples.
- [ ] Refactor the current Installation construction into a private shared
      helper without changing historical compilation behavior; extract the
      existing framed contribution ID derivation without changing bytes; verify
      unchanged evidence in both public compiler paths. Feed canonical source
      folder into the pure producer.
- [ ] Cover moving Git branch sources with real local Git fixtures and native
      Git URL rewrite; actual commit updates, stable contribution identity and
      retained capture revision mismatch rejection. Do not fetch public
      repositories or models.
- [ ] Reject source edits after capture, wrong expected name, malformed profile,
      mixed target selection, wrong location and incompatible reader audience
      before returning compiled output. Include absent-name folder mismatch and
      excluded result complete report assertions.
- [ ] Run the dedicated target, existing lifecycle_sources and Codex boundary
      tests. Record RED/GREEN, conventional commit and report.

### Task 2: Shared orchestration with explicit Codex authority and reports

Own `src/lifecycle/orchestration.rs`, new
`tests/lifecycle_codex_orchestration.rs`, feature docs in sources/lifecycle
topics, command-library boundary and AD-0039. Historical tests change only if an
additive diagnostic needs a precise assertion.

Public interfaces:

```rust
pub struct CodexSourceOptions<'a> {
    pub targets: TargetSelection<'a>,
    pub add_defaults: &'a [TargetSurface],
    pub profile: TargetProfile,
    pub execution: ResourceExecutionPolicy,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInstructionReaders {
    pub installation_id: String,
    pub report: InstructionReaderReport,
}
// Within impl SourcePrepared:
pub fn instruction_readers(&self) -> &[SourceInstructionReaders];
pub fn with_prepared_codex<T>(
    scope: &QualifiedScope,
    guard: &ScopeWriterGuard<'_>,
    root: InstallationRoot,
    request: SourceRequest,
    options: CodexSourceOptions<'_>,
    use_prepared: impl FnOnce(SourcePrepared<'_>) -> Result<T>,
) -> Result<OrchestrationOutcome<T>>;
```

- [ ] Start ATDD missing-API/behavioral RED with two local generic rules beside
      manual AGENTS text; inspect literal five-reader reports before apply,
      add/update one/remove one with exact other/manual bytes, mode,
      identities/contracts and references. Confirm incidental readers never own
      or retain content.
- [ ] Implement a private route enum and shared orchestration body; keep
      historical wrapper/signatures intact. Validate Codex profile/effective
      target intent after recovery/remove branching but before source
      resolution, with typed Instructions CapabilityError profile/selection
      failures. Derive exact destination from the declared root, then use Task1
      compile_codex.
- [ ] Implement complete per-owner retained representation checks: requested
      exactly CodexCli; rule kind; canonical contribution root/path; per-owner
      consumers exactly CodexCli; matching owner contract revision1; zero
      artifacts or exactly one producer-derived contribution ID with sole
      installation ownership. Reject arbitrary IDs, multiple contributions and
      shared ownership. Validate all owners before any resolve. Historical route
      also rejects artifact-free Codex installations.
- [ ] Collect complete per-installation reader reports, including exclusions,
      sort by installation_id, and expose them through SourcePrepared. Only
      desired sets, owner contracts and direct references enter CapturedUpsert.
      Historical/remove callbacks receive no newly compiled reports. Preserve
      final capture revalidation.
- [ ] Test global .codex/AGENTS.md exact artifact/contract destinations and two
      readers, generic/project compatibility rejection, Codex-only global
      acceptance, exclusion/reinclusion stable identities, discarded
      preparation, local/Git source update, post-preparation source mutation and
      deleted-source removal with invalid options. Test preflight rejects
      unsupported later owner/mixed selection/profile, malformed retained
      contract/location, arbitrary/multiple/shared contributions, artifact-free
      historical-route rejection and whole-file conversion before source
      resolution/callback. Test a later incompatible captured source causes no
      writes.
- [ ] Mutation controls: drop reports including excluded results; add incidental
      ownership; corrupt global destination; skip final source verification;
      bypass complete retained preflight. Pin independent literal state/report
      assertions.
- [ ] Update durable source/lifecycle/command-library docs and AD-0039 with
      explicit library-only source integration and remaining activation/native
      gates. No CLI flag.
- [ ] Run dedicated and historical source/orchestration plus lower lifecycle
      tests; record covering commands/results/mutations, commit and report.

### Task 3: Review, garden and deliver

- [ ] Independent task reviews must pass before proceeding; resolve confirmed
      gaps.
- [ ] Run just fmt, just verify, just sanity and just msrv-check.
- [ ] Garden only this exact bounded spec/plan pair, preserving unrelated WIP.
- [ ] Publish one PR, full numbered review passes, one whole final fix wave,
      final-head hosted checks; track required advisory debt under Epic283.
- [ ] Squash merge under delegated policy, verify exact merged tree and
      post-merge CI/docs/security, then archive evidence/remove only this owned
      finished worktree.
- [ ] Report remaining native qualification/staging/activation gates honestly.
