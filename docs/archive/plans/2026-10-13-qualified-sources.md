# Qualified source observation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture and non-refreshingly revalidate a complete source tree beneath
a qualified physical scope without activating lifecycle effects.

**Architecture:** A private descriptor-relative backend retains directory
membership, identity and mount evidence plus stable file bytes. An opaque
SourceSnapshot borrows QualifiedScope and exposes the existing SourceManifest by
immutable borrow. Share native qualification and manifest assembly without
changing destination checks or legacy capture behavior.

**Tech Stack:** Rust edition 2024, Rust 1.85 minimum, rustix =1.1.4 fs, existing
anyhow and sha2; synchronous APIs.

**Spec:**
[qualified source design](../specs/2026-10-13-qualified-sources-design.md).
Durable authority:
[source contract](../../design/qualified-source-observation.md) and
[AD-0030](../../adr/0030-capture-qualified-source-evidence.md).

## Global Constraints

- Rust edition 2024; MSRV 1.85; rustix pinned at 1.1.4; no new dependency or
  async runtime.
- No CLI/schema-2/native adapter activation, filesystem writes, probe creation
  or writer-lock acquisition.
- Opaque non-Clone SourceSnapshot borrowing exactly one QualifiedScope;
  immutable manifest accessor only.
- Preserve legacy capture_source/validate_manifest signatures, diagnostics and
  v1 digest bytes.
- Linux/macOS qualify using existing profiles; Windows and other hosts return
  explicit unsupported errors.
- Preserve all 34 unrelated WIP files, earlier raw archives and other worktrees.
- Recoverable errors retain operation/path context; no claim of recoverable
  infallible allocation or OS termination.
- Per task: acceptance first, meaningful assertion RED, implementation, semantic
  omission RED, exact restoration/GREEN, fresh independent review.

## Review Focus

- Empty-directory topology changes must invalidate evidence even though no
  manifest file changes.
- Same-byte inode or directory replacement must fail independently of content
  comparisons.
- Hard-linked source paths remain distinct records; destination one-link
  rejection remains unchanged.
- A regular entry replaced by a FIFO or symlink must neither block nor follow
  it.
- Two verification failures after one mutation must retain original values and
  never refresh stale evidence.

## File structure

- Create src/filesystem/sources.rs: public opaque snapshot and QualifiedScope
  method, unsupported boundary, pre-return check.
- Create src/filesystem/sources/backend.rs: complete retained tree and
  descriptor-relative acquisition/revalidation.
- Create src/filesystem/sources/backend_tests.rs: focused native
  metadata/read/mount/failure/control seams.
- Move src/filesystem/destinations/qualification.rs to
  src/filesystem/qualification.rs: same policy, filesystem-private access only;
  preserve native control tests.
- Modify src/filesystem.rs and
  src/filesystem/destinations.rs/backend.rs/backend_tests.rs only for module
  visibility/import adaptation required by the shared qualification module.
- Modify src/source_manifest.rs: crate-private canonical assembly helper used by
  legacy and qualified capture; unchanged public behavior.
- Create tests/filesystem_sources.rs: public acceptance and immutable evidence
  checks.
- Modify tests/source_manifests.rs only for needed literal compatibility
  assertions; prefer new acceptance file.
- Update docs/design/qualified-source-observation.md,
  docs/adr/0030-capture-qualified-source-evidence.md,
  docs/specification/specification.md, docs/design/strict-source-manifests.md
  and docs/book/SUMMARY.md with honest implemented/pending status.
- Controller owns this plan/spec and scratch/review/GitHub/CI; gardener owns
  only assigned archival pair/index after green.

### Task 1: Qualified complete source capture and revalidation

**Consumes:** QualifiedScope::reopen_directory() -> Result<File>,
QualifiedScope::verify_binding() -> Result<()>, unchanged native
Mount::of/queries/require_same, CapturedSourceFile/SourceManifest and pure
validation.

**Produces:** QualifiedScope::observe_source_tree(&self) ->
Result<SourceSnapshot<'_>>, SourceSnapshot::manifest(&self) -> &SourceManifest,
SourceSnapshot::verify_unchanged(&self) -> Result<()>. Existing source capture
and destination APIs remain compatible.

- [x] **Step 1: Write public acceptance tests before implementation.** Use
      direct library calls and tempfile; no CLI because this boundary remains
      inactive. Include helpers selecting the real source scope:

```rust
fn source_scope(path: &std::path::Path) -> upskill::filesystem::QualifiedScope {
    upskill::filesystem::QualifiedScope::open(&upskill::ownership::InstallationRoot {
        scope: upskill::ownership::OwnershipScope::Project,
        path: path.to_owned(),
    }).unwrap()
}
```

Pin complete stable records against existing capture_source with nested binary,
hidden, ignored and empty files plus two hard-linked regular paths. Include a
real minimal skill tree and feed snapshot.manifest() to the existing captured
compiler interface without rereading it. Pin empty-directory membership and two
stale checks with explicit assertion messages:

```rust
let tmp = tempfile::tempdir().unwrap();
std::fs::write(tmp.path().join("data"), b"original").unwrap();
let scope = source_scope(tmp.path());
let snapshot = scope.observe_source_tree().unwrap();
let original = snapshot.manifest().clone();
snapshot.verify_unchanged().unwrap();
std::fs::create_dir(tmp.path().join("new-empty")).unwrap();
for call in 1..=2 {
    assert!(snapshot.verify_unchanged().is_err(),
        "empty directory insertion accepted on verification {call}");
    assert_eq!(snapshot.manifest(), &original);
}
assert_eq!(std::fs::read(tmp.path().join("data")).unwrap(), b"original");
```

Add independent file addition/removal/rename, same-byte inode replacement,
empty-directory removal, same-descendant directory replacement, mode change,
selected-root and ancestor-alias rebind fixtures. Assert exact manifest remains
unchanged after every failure and no fixture entry is written by observation. On
Unix, symlink root/descendant, socket/FIFO and invalid-name fixtures reject;
Linux alone runs actual non-UTF-8 entry creation if macOS rejects fixture
creation. Unsupported hosts compile opaque API and return errors without
artifacts.

- [x] **Step 2: Establish assertion RED.** Missing API compile errors are setup
      feedback, never semantic RED. Add the smallest opaque public scaffold
      returning contextual unsupported/not-implemented errors on capture;
      compile the tests, then record the stable-tree acceptance failing at
      successful capture. Run captured commands with isolated HOME and
      restricted PATH:

```bash
python3 /tmp/upskill-destination-controller-run.py sources-atdd-red cargo test --test filesystem_sources
```

Do not count scaffolding compiler errors as rejection-control evidence.
Implement no traversal before obtaining the runtime failure. Add compile-fail
doctests for construction, Clone and mutable access.

- [x] **Step 3: Share qualification and canonical manifest assembly.** Move the
      existing qualification module without policy changes, preserving private
      tests and adapting imports/visibility to filesystem ancestry. Extract the
      final legacy capture assembly into this crate-private helper and call it
      from both paths:

```rust
pub(crate) fn assemble_manifest(mut files: Vec<CapturedSourceFile>) -> anyhow::Result<SourceManifest> {
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let manifest = SourceManifest {
        content_sha256: manifest_hash(&files)?,
        files,
    };
    validate_manifest(&manifest).context("validate captured source manifest")?;
    Ok(manifest)
}
```

Compute each captured source file's content digest using the existing hash
helper with crate-private visibility, or a similarly narrow canonical
constructor; do not reimplement framing. Preserve existing diagnostics and
order. Run source manifest and destination suites after mechanical extraction.

- [x] **Step 4: Implement complete retained acquisition.** Represent a retained
      directory as descriptor, identity/full stamp, exact sorted child names and
      retained children; a regular file as descriptor, full stamp and captured
      record. Retain one native Mount at tree level and compare it for every
      opened entry. Use an explicit recursive tree or iterative stack with
      per-directory evidence; reject a repeated directory identity before
      descent. Every new source file stays within the listed ownership.

```rust
fn directory_flags() -> rustix::fs::OFlags {
    rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY
        | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC
}
fn regular_flags() -> rustix::fs::OFlags {
    rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW
        | rustix::fs::OFlags::CLOEXEC | rustix::fs::OFlags::NONBLOCK
}
```

Use Dir::read_from, statat with SYMLINK_NOFOLLOW and single-component openat.
Filter only dot/dot-dot, convert exact stored names to UTF-8 and validate
complete relative paths before recursion/read. Compare selected/opened
identities and kinds. Do not require regular-file nlink==1; retain link count in
its stamp. Directory stamps include full mode/link count/size/nanosecond
mtime/ctime and remain unchanged across enumeration plus descendant acquisition.
Ignore atime.

The actual reader path takes a narrow private closure returning
io::Result<usize> on (&File, &mut Vec<u8>); default directly calls
File::read_to_end. Require the before/after fstat stamp and nonnegative
size/byte length to agree. Errors include operation/path and original cause.
Scope/mount/identity and actual read fields must participate in production, not
only a test helper.

- [x] **Step 5: Implement non-refreshing full revalidation and pre-return
      check.** Verify original handles and mounts; capture a fresh complete
      descriptor-relative tree from the revalidated root. Compare exact
      directory names/topology/stamps and file identity/stamps/bytes/modes
      against original evidence, then discard fresh values. Recheck scope
      binding after the walk. Public code retains one immutable original
      snapshot; neither success nor failure replaces its manifest/evidence:

```rust
let (evidence, manifest) = backend::Evidence::capture(self)?;
let snapshot = SourceSnapshot { scope: self, evidence, manifest };
snapshot.verify_unchanged()?;
Ok(snapshot)
```

Private tests exercise a change between capture completion and this exact public
postcheck. Repeated-stale omission controls account for this initial
verification and must fail specifically on later verification 2. No source lock
is acquired.

- [x] **Step 6: Add focused independent native tests.** Cover acquisition
      metadata change while returned bytes/count stay equal, short-read while
      metadata stays equal, exact error/cause propagation, same-second
      fractional mtime with competing ctime normalized, retained descriptors'
      CLOEXEC/NONBLOCK flags and no-follow opens, iterator/stat/open failures,
      released owned handles after failed capture, native profile/mask rejection
      and changed mount binding on source acquisition/verify. Use real
      descriptors/metadata and alter only the intended field/query, with a
      stable success control. Private seams remain narrow; no broad filesystem
      mock. Bound FIFO-related subprocess checks with owned-process
      timeouts/kill/reap; never leave blocking children or terminate unrelated
      processes.

- [x] **Step 7: Record independent omission RED and exact restored GREEN.**
      Store a machine-readable manifest under /tmp/upskill-sources-controls.json
      with each unique mutation, intended test/assertion, exit status and
      pre/restored SHA256. Controls separately remove membership comparison,
      identity comparison with competing stamps normalized, value comparison,
      acquisition stamp comparison, byte-count comparison, each acquisition call
      wiring, pre-return verification, source native qualification call, mount
      comparison, source root binding calls and stale verification 2. Each
      mutation must fail the intended assertion, not compile or fail an earlier
      assertion. Restore exact bytes in finally, rerun focused suites, and
      document discarded invalid experiments separately. Preserve normal
      source/destination tests and hosted native Linux missing-mask execution.
      No persistent temporary CI harness unless root authorizes its precise
      scope.

- [x] **Step 8: Update current docs and run final task gates.** Add
      specification section 4.9 with interface, complete source coverage,
      hard-link distinction, private topology, stable acquisition, immutable
      revalidation, no-effects and platform/evidence limits. Update the early
      durable source design/AD30 with actual local counts and remaining hosted
      gates; replace strict-source status only where this new boundary is
      implemented. Add durable design/ADR links to book Summary. Never link
      durable files to WIP, rewrite earlier raw archives, or claim M1 done.

```bash
python3 /tmp/upskill-destination-controller-run.py sources-fmt just fmt
python3 /tmp/upskill-destination-controller-run.py sources-verify just verify
python3 /tmp/upskill-destination-controller-run.py msrv-sources cargo +1.85.0 check --locked --all-targets
python3 /tmp/upskill-destination-controller-run.py sources-clippy cargo clippy --all-targets --locked -- -D warnings
```

Reuse isolated captures; preserve 34 baseline hashes after full formatting.
Produce matched Rust 1.97.1 release CLI baseline/candidate and minimal
public-API control/live probes with identical pinned dependency
resolution/profile. Runtime inputs keep capture+verify live. Record binary sizes
and source hashes; do not claim performance. Existing destination size
report/probe under /tmp/upskill-destination-probes-pinned is methodology only;
create uniquely named source artifacts and never overwrite earlier evidence.

- [x] **Step 9: Commit only owned files with normal hooks and return complete
      report.** Use Conventional Commit feat: capture qualified source tree
      evidence. Do not push, open PR, archive, edit unrelated WIP or dispatch
      agents. Report commit/hash, exact files, acceptance RED, every
      valid/invalid omission result, restored gates, counts, size deltas,
      limitations and remaining hosted gates to
      .superpowers/sdd/2026-10-13-qualified-sources/task-1-report.md.

## Controller closeout

Fresh independent task reviewer gates task 1 before another task or gardening;
fix required findings through a fresh scoped implementer, no repeated QUICK
cycle. Then root runs fresh appropriate verification and dispatches scoped
gardener for only this assigned spec/plan pair. Preserve raw bytes and
unresolved foreign WIP. Use the most capable whole-branch review, then normal
push/PR, FULL passes 1/2, exact-final-head Linux/Windows checks and admin squash
under delegated authority. No current acceptance waiver or release/CLI
activation follows from this plan.

## Self-review

All eight spec acceptance groups map to task 1 steps 1 and 4–8. Review Focus has
explicit public/private fixtures in steps 1, 5 and 6. Interface names are
identical throughout. New types/functions are defined in this task, with native
policy and assembly reused. Baseline is PR273 merge; task is independently
testable. Ordinary allocation, real privileged/fault/hostile behavior and
qualified Windows remain named evidence limits, rather than untested completion
claims.
