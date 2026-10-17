# Shared updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans`
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Status:** retained draft; refresh against the completed prerequisites before
execution.

**Goal:** Add one scope-aware Rust maintenance command that refreshes installed
upskill content and updates only binaries registered as upskill-managed.

**Architecture:** Add a `maintenance` module for policy, release lookup, state,
binary replacement, and orchestration. `upskill maintain --scope project|user`
coordinates these pieces while reusing the existing content update pipeline;
`upskill update` remains content-only. Client plugins call the same command.

**Tech Stack:** Rust 2024, clap 4, serde/serde_json, sha2 0.11, ureq 2, existing
qualified filesystem locks, shell installer and GitHub Actions release workflow.
Add no runtime dependencies.

**Spec:**
[docs/wip/specs/2026-10-14-shared-updater-design.md](../specs/2026-10-14-shared-updater-design.md)

**Start gate:** Do not start #284 implementation until the M1 ownership-aware
lifecycle completes its combined-state, qualified-apply, recovery, and adapter
activation gates, and owned-skill lifecycle support is available. M1 covers
rule/instruction effects and does not itself provide owned-skill updates. Do not
deliver a binary-only slice while content refresh is blocked on either
prerequisite. Schema-1 content is reported as `legacy-unverified` and left
untouched. Do not route it through the current schema-1 update pipeline or
create a parallel ownership sidecar.

## Global Constraints

- Rust edition 2024; MSRV 1.85.
- Preserve the single static binary goal; do not add a runtime dependency.
- Support the existing Linux and macOS release targets; native Windows remains
  out of scope and WSL uses Linux assets.
- Keep manual `upskill update` content-only.
- Require explicit `--scope project|user`; never infer maintenance scope from
  the current directory.
- Replace only a binary with a valid upskill-installer ownership record that
  matches the running executable.
- Check stable release assets with SHA-256 before replacement.
- Preserve source labels, refs, user-owned file changes, and the last usable
  installation on failed or interrupted work.
- Skip schema-1 content without fetching or writing; refresh only content
  handled by the fully activated ownership-aware lifecycle.
- Use existing temporary-home CLI test harnesses; do not access the developer's
  real home from tests.
- Run `just fmt` and `just verify` before opening a PR.

## Review Focus

- A project-scoped startup while the caller is outside a Git repository must
  require and update only the explicitly selected project lockfile.
- A user-scoped update from inside a repository must not update that
  repository's lockfile.
- A binary without a valid managed-install record must remain byte-identical,
  including under `~/.cargo/bin`.
- A release with a corrupt checksum or wrong target must leave the current
  executable runnable and unchanged.
- Concurrent and interrupted maintenance must preserve local modifications and
  recover the prior valid binary/content without reporting a false success.

---

### Task 1: Record the accepted update policy and CLI acceptance contract

**Files:**

- Create: `docs/adr/<next-id>-shared-maintenance-and-binary-ownership.md`
- Modify: `docs/adr/README.md`
- Create: `tests/cli_maintain.rs`

**Interfaces:**

- The CLI accepts `upskill maintain --scope project` or
  `upskill maintain --scope user`.
- `upskill maintain` without a scope is a usage error.
- The eventual handler delegates to
  `upskill::maintenance::maintain(scope, context)` and prints separate binary
  and content outcomes.
- Existing `upskill update` arguments and output remain unchanged.

- [ ] **Step 1: Add the failing CLI acceptance tests**

Add `maintain_requires_explicit_scope`,
`maintain_skips_schema1_content_without_fetch_or_write`, and
`update_remains_content_only` to `tests/cli_maintain.rs`. Use
`common::upskill_cmd(&home)`, a temporary project with a schema-1 lockfile and
sentinel generated output, and a local source fixture that records fetches.
Assert missing scope exits 2 and names `--scope`; maintenance reports
`legacy-unverified`, makes no source request, and leaves the lockfile and output
byte-identical; `update --help` still describes source/content refresh and has
no binary-update option.

- [ ] **Step 2: Run the new acceptance tests and confirm they fail for missing
      CLI behavior**

Run: `cargo test --test cli_maintain`

Expected: the command parser reports `maintain` as unknown, so the tests fail
before production code is added.

- [ ] **Step 3: Record the accepted policy in the ADR**

Allocate the next unused ADR identifier at implementation time and record the
reviewed policy: 24-hour success interval, one-hour retry delay, stable latest
by default, explicit installer tag pin, explicit project/user scope, and
registered installer ownership only. Link it from the ADR index.

- [ ] **Step 4: Re-run the acceptance tests and confirm the intended red state**

Run: `cargo test --test cli_maintain`

Expected: the tests still fail only because `maintain` is not implemented; the
test binary compiles and the `update` help compatibility assertion passes.

- [ ] **Step 5: Commit the CLI contract and ADR**

```bash
git add tests/cli_maintain.rs docs/adr/
git diff --cached --stat
git commit -m "feat: define shared maintenance command"
```

### Task 2: Add maintenance state and due-check policy

**Files:**

- Create: `src/maintenance/mod.rs`
- Create: `src/maintenance/state.rs`
- Create: `src/maintenance/policy.rs`
- Modify: `src/lib.rs`
- Test: unit tests in `src/maintenance/state.rs` and `src/maintenance/policy.rs`

**Interfaces:**

- `MaintenanceScope` is `Project(PathBuf)` or `User(PathBuf)` and provides a
  stable state key from the canonical root.
- `MaintenanceState` stores the last successful check and next retry time for
  each component.
- `DueDecision` is `Due`, `NotDue`, or `RetryDue`; policy functions accept an
  injected `SystemTime` so tests use fixed instants.

- [ ] **Step 1: Write failing policy and state tests**

Add `project_state_key_does_not_contain_root_path`,
`successful_check_is_not_due_for_24_hours`,
`failed_check_retries_after_one_hour`, and
`state_round_trip_preserves_component_timestamps`. Assert the same canonical
project gets the same key, different roots get different keys, and serialized
state contains no raw project path.

- [ ] **Step 2: Run the focused tests and confirm expected missing-type
      failures**

Run:
`cargo test maintenance::policy::tests && cargo test maintenance::state::tests`

Expected: compilation fails because the maintenance types and functions do not
yet exist.

- [ ] **Step 3: Implement pure due decisions and state serialization**

Use `serde`/`serde_json` for the state format and `sha2` for the project key.
Implement atomic state writes with a temporary sibling file followed by rename.
Use a 24-hour successful interval and one-hour failure retry; do not perform I/O
in due-decision functions.

- [ ] **Step 4: Run the focused tests and add corrupt-state recovery coverage**

Run:
`cargo test maintenance::policy::tests && cargo test maintenance::state::tests`

Expected: all tests pass; corrupt or truncated state returns an actionable error
and is not interpreted as a successful fresh check.

- [ ] **Step 5: Commit the state and policy layer**

```bash
git add src/maintenance/mod.rs src/maintenance/state.rs src/maintenance/policy.rs src/lib.rs
git commit -m "feat: track maintenance check state"
```

### Task 3: Track managed installs and publish raw release binaries

**Files:**

- Modify: `install.sh`
- Modify: `.github/workflows/release.yml`
- Modify: `tests/cli_install.rs` or create it if absent
- Test: shell installer tests and release-asset workflow checks

**Interfaces:**

- A successful official install writes `<install-dir>/upskill.install.json`
  atomically.
- The record contains schema version, absolute executable path, installed tag,
  and `channel` (`stable` or `pinned`).
- Each release publishes `upskill-<rust-target>` and its `.sha256` alongside the
  existing compressed archive and checksum.

- [ ] **Step 1: Write failing installer tests for managed metadata**

Stub `uname`, `curl`, `tar`, and checksum tools through a temporary `PATH`.
Assert a latest install writes a `stable` record, `UPSKILL_VERSION=v0.8.1`
writes a `pinned` record with that exact tag, and a failed archive or checksum
step leaves any existing binary and record unchanged.

- [ ] **Step 2: Run the shell tests to verify failure**

Run: `cargo test --test cli_install`

Expected: the existing installer does not write a managed record, so the new
metadata assertions fail.

- [ ] **Step 3: Implement atomic installer metadata and raw release assets**

After verifying the existing archive checksum, install the binary and write its
metadata through temporary sibling files and rename. In the release workflow,
copy the built target executable into `dist/upskill-${target}` and write a
SHA-256 sidecar while retaining the archive assets used by `install.sh`.

- [ ] **Step 4: Run installer tests and validate each target's asset names**

Run: `cargo test --test cli_install`; inspect the four workflow matrix entries
and assert each produces one archive, one archive checksum, one raw binary, and
one raw-binary checksum.

Expected: tests pass; the raw names exactly match `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, and `aarch64-apple-darwin`.

- [ ] **Step 5: Commit installer ownership and release assets**

```bash
git add install.sh .github/workflows/release.yml tests/cli_install.rs
git commit -m "feat: record managed binary installs"
```

### Task 4: Implement verified binary replacement and crash recovery

**Files:**

- Create: `src/maintenance/release.rs`
- Create: `src/maintenance/binary.rs`
- Modify: `src/maintenance/mod.rs`
- Test: unit tests in `src/maintenance/release.rs` and
  `src/maintenance/binary.rs`

**Interfaces:**

- `ReleaseSource::latest(target) -> Result<ReleaseAsset>` supplies the stable
  tag, binary URL, and checksum URL.
- `BinaryUpdater::update(executable, record, release) -> Result<BinaryOutcome>`
  stages and replaces only the executable named by the valid managed record.
- `BinaryOutcome` distinguishes `Updated { from, to }`, `UpToDate { version }`,
  and `Unmanaged { reason }`.

- [ ] **Step 1: Write failing release and binary tests**

Add tests named `release_asset_selects_exact_target`,
`checksum_mismatch_keeps_current_binary`, `wrong_target_keeps_current_binary`,
`replacement_failure_restores_previous_binary`,
`interrupted_replacement_recovers_backup`, and
`missing_or_mismatched_record_is_unmanaged`. Use a local HTTP fixture serving a
JSON latest-release response, raw binary bytes, and the matching checksum; use a
temporary executable that writes a known sentinel when run.

- [ ] **Step 2: Run the focused tests and confirm the expected missing-module
      failures**

Run:
`cargo test maintenance::release::tests && cargo test maintenance::binary::tests`

Expected: compilation fails because release lookup and binary replacement do not
exist.

- [ ] **Step 3: Implement release lookup and checksum verification**

Use `ureq` with a fixed user agent to retrieve the latest non-prerelease release
metadata, select only the current Rust target, then fetch its raw bytes and
`.sha256` sidecar. Reject missing assets, malformed checksums, unexpected asset
names, and hash mismatch before creating a replacement executable.

- [ ] **Step 4: Implement staged atomic replacement and recovery**

Stage the verified bytes beside the managed executable, set executable mode, and
persist a small replacement journal before renaming. Move the current executable
to a recoverable backup, move the staged file into place, verify its version,
then clear the journal and backup. On the next invocation, recover the backup if
the journal shows the replacement stopped before the new executable was
verified.

- [ ] **Step 5: Run focused tests and verify old-binary survival**

Run:
`cargo test maintenance::release::tests && cargo test maintenance::binary::tests`

Expected: all failure injection cases leave at least one runnable version and
report an error instead of success.

- [ ] **Step 6: Commit binary update support**

```bash
git add src/maintenance/release.rs src/maintenance/binary.rs src/maintenance/mod.rs
git commit -m "feat: update managed binaries safely"
```

### Task 5: Orchestrate scoped maintenance and coordinate processes

**Files:**

- Create: `src/maintenance/run.rs`
- Modify: `src/maintenance/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `tests/cli_maintain.rs`
- Test: unit tests in `src/maintenance/run.rs` and CLI acceptance tests

**Interfaces:**

- `maintain(scope, context) -> Result<MaintenanceReport>` checks state, acquires
  the scope lock, performs due binary/content work, records outcomes, and
  returns independent component statuses.
- User scope resolves to the fakeable home target; project scope uses only the
  explicit project root supplied by the CLI.
- Content updates call the activated ownership-aware lifecycle under its
  qualified writer lock, preserving source selection and manual command
  semantics. The schema-1 `pipeline::update` path is not used for automatic
  maintenance.

- [ ] **Step 1: Write failing orchestration and scope tests**

Add `maintain_project_updates_only_project_lockfile`,
`maintain_user_updates_only_home_lockfile`,
`maintain_not_due_makes_no_source_fetch`, `maintain_disabled_performs_no_write`,
`maintain_reports_offline_content_failure`, and
`concurrent_maintain_reports_busy`. Use local git repositories and independent
fake homes; exercise schema-2 mixed state with an owned item and an unresolved
legacy item, assert owned content can update while legacy content and records
remain byte-identical, and compare exact bytes for the untouched scope.

- [ ] **Step 2: Run the new tests and confirm missing orchestration behavior**

Run: `cargo test --test cli_maintain && cargo test maintenance::run::tests`

Expected: CLI tests fail because the handler has no maintenance implementation;
unit tests fail until the runner exists.

- [ ] **Step 3: Implement the CLI command, scope resolution, and maintenance
      lock**

Add the required clap `--scope project|user` enum and dispatch from `main.rs`.
Validate project scope from the explicit current directory and require its
lockfile; resolve user scope from the configured home. Acquire a persistent
state-directory file lock backed by the existing `rustix` dependency. Return
`busy` without changing timestamps or files when the lock is already held. For
schema-1 content, return `legacy-unverified` without invoking fetch or write
operations.

- [ ] **Step 4: Implement due orchestration and independent statuses**

For user scope, check/update the managed binary and ownership-aware global
content. For project scope, skip binary work and update only ownership-aware
content in that project. Preserve unresolved legacy records and skip their
content. If `UPSKILL_AUTO_UPDATE=0`, return `disabled` without network or
writes. Record success only after the associated component completes; record the
one-hour retry time after a failure.

- [ ] **Step 5: Run the focused CLI and orchestration suites**

Run: `cargo test --test cli_maintain && cargo test maintenance::run::tests`

Expected: project and user scopes remain separate, no-work runs perform no
fetch, and offline or busy outcomes are reported without claiming successful
updates.

- [ ] **Step 6: Commit orchestration**

```bash
git add src/maintenance/run.rs src/maintenance/mod.rs src/cli.rs src/main.rs tests/cli_maintain.rs
git commit -m "feat: orchestrate scoped automatic maintenance"
```

### Task 6: Verify real transitions and update user documentation

**Files:**

- Modify: `docs/book/commands.md`
- Modify: `docs/book/getting-started.md`
- Modify: `docs/specification/specification.md`
- Modify: `tests/cli_maintain.rs`
- Modify: release workflow tests or CI assertions

**Interfaces:**

- User documentation distinguishes manual `update` from lifecycle-triggered
  `maintain`, names the default cadence, and explains managed versus
  package-manager-owned binaries.
- The acceptance suite exercises a real binary transition and a real local-git
  installed-skill transition with temporary state and homes.

- [ ] **Step 1: Add the end-to-end transition test**

Use a controlled local release fixture to install binary v1 and update it to v2
through `maintain --scope user`. In the same test, update ownership-aware
content from a local Git repository, verify an edited owned artifact is
preserved or reported as a conflict, verify an unresolved schema-1 item is
untouched and not fetched, and assert project and user state remain separate.
Verify a pinned source ref remains selected.

- [ ] **Step 2: Run the transition test and inspect actual artifacts**

Run:
`cargo test --test cli_maintain maintain_observes_real_binary_and_skill_transitions`

Expected: the temporary binary reports v2 when executed; the generated skill
contains the committed mutation; the checksum, source pin, and untouched-scope
assertions pass.

- [ ] **Step 3: Document command and ownership behavior**

Describe `upskill maintain --scope project|user`, automatic due checks, the
stable/pinned binary behavior, the unmanaged/package-manager status, failure
retention, and how manual `upskill update` differs. State that a client plugin
may call maintenance only on its qualified lifecycle event and that reload
requirements are client-specific.

- [ ] **Step 4: Run focused tests and repository verification**

Run: `cargo test --test cli_maintain`; `cargo test --test cli_update`;
`just fmt`; `just verify`.

Expected: all named tests pass; formatting is clean; full verification passes
with no warnings. If a supported Linux runner is not available locally, record
CI evidence for its release target and replacement tests before claiming
cross-platform completion.

- [ ] **Step 5: Commit tests and documentation**

```bash
git add tests/cli_maintain.rs docs/book/commands.md docs/book/getting-started.md docs/specification/specification.md .github/workflows/release.yml
git commit -m "docs: explain automatic maintenance"
```

## Self-review

- **Spec coverage:** command separation and explicit scope are covered by Tasks
  1 and 5; cadence/state by Task 2; ownership, target assets and raw downloads
  by Task 3; integrity and interruption recovery by Task 4; offline,
  concurrency, scope isolation and disabled status by Task 5; real transitions,
  supported OS targets and user guidance by Task 6.
- **Placeholder scan:** the plan contains no TODO, TBD, or deferred
  implementation steps.
- **Interface consistency:** Tasks 1, 2 and 4 define the scope, state, release,
  and outcome boundaries used by Task 5; Task 6 consumes the same CLI.
- **Review focus coverage:** Task 5 tests explicit project/user scope; Tasks 3–4
  test unowned binaries and asset corruption; Tasks 4–6 test interruption and
  preservation.
- **Open qualification:** Codex desktop lifecycle support and a measured client
  startup budget remain #285 gates. They must be completed before the epic
  claims a qualified four-client release; they do not justify changing
  `maintain` scope or moving update logic into client plugins.
