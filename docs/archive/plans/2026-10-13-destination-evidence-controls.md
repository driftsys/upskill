# Destination evidence controls implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development to implement this plan task-by-task.
> Steps use checkbox syntax for tracking.

**Goal:** Complete the three remaining independent destination regression
controls in issue #270 before lifecycle activation.

**Architecture:** Preserve the qualified destination API and its existing
conservative filesystem contract. Add deterministic controls around the
production read-stability and qualification paths, using only narrowly scoped
private read/metadata seams where needed. Do not introduce a filesystem
abstraction, public test hooks or global fault injection.

**Tech Stack:** Rust edition 2024, MSRV 1.85, existing rustix 1.1.4 and standard
library; no new dependencies or unsafe code.

**Spec:** `docs/wip/specs/2026-10-13-destination-evidence-controls-design.md`,
`docs/design/qualified-destination-observation.md`, AD-0029, and issue #270
acceptance criteria. The durable design remains the binding authority; this is
regression coverage of existing checks.

## Global constraints

- Preserve all public interfaces, exact byte/mode semantics, no-follow
  traversal, mount qualification and opaque borrowed evidence.
- No CLI activation, state persistence, permission authority, effects or
  recovery.
- Only main.rs writes product stdout/stderr. Tests may capture subprocess
  diagnostics in owned temporary artifacts.
- No privileged mount/device experiments, network fixtures, filesystem
  comparison probes or general filesystem mocks.
- Isolate subprocess HOME and USERPROFILE, capture output, enforce deadlines and
  kill/reap only owned process groups.
- Use `common::upskill_cmd` for any CLI acceptance fixture. This task needs
  filesystem API acceptance, not a new CLI command.
- Preserve every unrelated working document and other worktree. Keep complete
  evidence and scratch after completion.

## Review focus

1. Subsecond mtime changes must not be masked by a changed ctime or whole
   seconds.
2. Acquisition stability must compare metadata before and after the actual read,
   independently of the snapshot postcheck.
3. Returned byte count must match the opened file size even if all compared
   metadata stays identical.
4. Unknown profiles and missing returned Linux mount-ID mask must reject through
   the production qualification path.
5. Different mount bindings must reject through actual Directory/Evidence
   wiring, without claiming a synthetic fixture proves a privileged kernel mount
   experiment.

## Task 1: Independently pin the three remaining destination controls

**Files:**

- Modify: `src/filesystem/destinations/backend.rs` only for narrow private seams
  if deterministic coverage requires them.
- Modify: `src/filesystem/destinations/backend_tests.rs` for read and evidence
  controls.
- Modify: `src/filesystem/destinations/qualification.rs` for qualification-path
  controls and narrowly scoped private metadata seams if necessary.
- Modify: `tests/filesystem_destinations.rs` only if a public acceptance fixture
  adds independent observable coverage.
- Modify: `docs/design/qualified-destination-observation.md` and
  `docs/specification/specification.md` to describe completed controls and exact
  evidence limits.

**Interfaces:**

- Consumes existing `Evidence::capture`, `Evidence::verify`, `Stamp::of`,
  `Directory::new`, `Mount::of` and `Mount::require_same`.
- Produces no public interface. If extracting the read operation, keep the
  before-read Stamp associated with the opened descriptor and use a private
  reader parameter implementing std::io::Read. If injecting qualification
  metadata, accept only the exact descriptor query results in a private
  qualification entry used by Mount::of; do not duplicate the qualification
  logic in test code.

- [x] **Step 1: Establish baseline and record acceptance examples.** Run the
      existing 26 public and 16 private destination tests on clean main714c4c3.
      Write a coverage matrix mapping each #270 criterion and the five
      review-focus rows to its named new test and independent omission control.
      Record all commands in the task report.

```bash
cargo test --locked --test filesystem_destinations
cargo test --locked --lib filesystem::destinations::
```

- [x] **Step 2: Add fractional-mtime acceptance first.** Capture real file
      evidence, set an explicit representable timestamp with nonzero
      nanoseconds, change only nanoseconds within the same whole second, and
      verify the filesystem retained that exact change. Preserve identity,
      bytes, mode, links and size; normalize only competing retained ctime.
      Require production verification to reject. Do not skip if the qualified
      local filesystem fails the claimed precision: report the platform
      limitation. Independently zero only mtime nanoseconds in Stamp::of and
      require an assertion failure, then restore exact production bytes.

```rust
assert_eq!(before.st_mtime, after.st_mtime);
assert_ne!(before.st_mtime_nsec, after.st_mtime_nsec);
assert_eq!(Identity::of(&before), Identity::of(&after));
assert_eq!(before.st_mode, after.st_mode);
assert_eq!(before.st_size, after.st_size);
assert_eq!(before.st_nlink, after.st_nlink);
assert!(evidence.verify(&scope, &observations).is_err());
```

- [x] **Step 3: Pin acquisition-read stability independently.** Add a
      deterministic reader that changes the actual descriptor metadata during a
      read while retaining exact same bytes and byte count. Exercise the
      production before/read/after acquisition check directly, without reaching
      capture's later verification. Require rejection. Separately supply a
      shorter reader with unchanged actual file metadata and require byte-count
      rejection. If a private read helper is necessary, extract the existing
      check intact; production must use it. Include a stable-reader success case
      and retain contextual read-error propagation. Independently omit
      before/after stamp comparison, byte-count comparison and production call
      wiring; each must fail its intended assertion rather than compile.

```rust
// The fixture reader performs the bounded metadata change during Read::read.
// The second fixture returns fewer bytes than the stable descriptor size.
assert!(unstable_capture_result.is_err());
assert!(short_capture_result.is_err());
assert_eq!(stable_capture_result.unwrap(), expected_bytes);
```

- [x] **Step 4: Pin production qualification rejection.** Exercise the
      production qualification entry with an unknown native profile; on Linux
      additionally supply a statx result whose returned mask omits MNT_ID
      despite a populated numeric ID. Establish a qualified success control.
      Mutate an expected retained Mount binding to a different binding and
      exercise Directory/Evidence production verification, with all file
      identity/value evidence otherwise unchanged. If narrow descriptor query
      injection is needed, its implementation must be the production
      qualification path used by Mount::of, not a test-only copy of validators.
      Independently omit the profile rejection, mask rejection, mount comparison
      and their call wiring; each relevant supported-platform assertion must
      fail. Linux-only runtime controls must run in hosted Linux CI before
      merge; local synthetic helper validation is insufficient.

```rust
assert!(unknown_profile_result.is_err());
assert!(missing_returned_mask_result.is_err()); // Linux only.
assert!(directory_or_evidence_verification.is_err());
```

- [x] **Step 5: Restore, document and verify.** Restore all temporary mutations
      and record file hashes plus assertion-only RED output and restored GREEN
      output. Update the durable design/spec with exact controls and pending
      real privileged/error experiments. Never claim synthetic injected metadata
      establishes real kernel mount rejection. Run focused suites and
      warning-denying rustdoc/Clippy, then `just fmt` and `just verify` once
      with isolated HOME. Preserve the raw archives and all unrelated WIP
      hashes. Report native local and hosted coverage separately.

```bash
cargo test --locked --test filesystem_destinations
cargo test --locked --lib filesystem::destinations::
RUSTDOCFLAGS='-D warnings' cargo test --locked --doc
cargo clippy --locked --all-targets -- -D warnings
just fmt
just verify
```

- [x] **Step 6: Commit and return for independent review.** Stage only the task
      files and use a Conventional Commit. Return exact head, tests, each
      omitted-check assertion control, restored hashes, remaining limitations
      and all artifact paths. The controller performs independent task review,
      gardening, whole-branch review, PR FULL review and exact-head hosted
      gates; do not dispatch reviewers or push.

```bash
git commit -m "test: independently pin destination evidence rejection"
```

## Task 1 scope amendment from landed-commit QUICK advisory

The immutable8298386..714c4c3 QUICK advisory adds acceptance within this same
bounded regression task. Extend owned files to
`docs/design/captured-item-compilation.md`, `docs/design/owned-file-planning.md`
and `docs/design/strict-source-manifests.md`, editing only current
successor-status paragraphs; update matching specification paragraphs, including
already-implemented writer-lock status. Preserve historical archives.

Before final gates, create two distinct regular files `a/é` and `a/e`, assert
both exact stored spellings and existing regular values, and require overlap
rejection in both input permutations. Independently skip candidate overlap
comparison only when both destinations exist and require an assertion RED, then
restore. Capture another file, mutate it once, call verification twice and
require both errors plus unchanged borrowed observations; an expectation-refresh
omission control must assert-fail. These additions pin existing semantics, not
new behavior.

Private qualification seam ruling: put the exact native metadata query closure
on `Mount::of(file, query)`, with production callers using `Mount::query`. Test
the actual Mount::of entry with one native result altered, and
Directory/Evidence mount comparison with only the retained expected binding
changed. Avoid an outer wrapper that could bypass the tested qualification
entry.

## Implementation checkpoint

Task implementation621f19c and acceptance35d7309 passed local focused checks and
full just verify. Independent task review approved; hosted Linux mask runtime
and native Linux profile/mask omission controls remain explicit merge gates. No
full lifecycle activation claim.
