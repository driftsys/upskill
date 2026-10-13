# Qualified source tree observation

Date: 2026-10-13. Status: inactive library boundary implemented; hosted qualification pending.
Baseline: `1d8ac581a2111b149e5bcceb6d789a0063814696` (PR #273).
Decision: [AD-0030](../adr/0030-capture-qualified-source-evidence.md).
The source snapshot API is implemented separately from command dispatch. The
recorded baseline predates this implementation; final-head hosted Linux and
Windows evidence remains pending.

## Intent

Complete the source-observation prerequisite of the delegated owned-content
lifecycle. Existing strict manifests capture exact bytes for pure compilation but
use ambient paths and contain freely mutable values. Introduce an inactive,
read-only filesystem capability that binds a complete captured tree to an
existing qualified scope and can reject later namespace or value changes.
Success is independently tested acquisition and immutable revalidation, with no
change to current command dispatch, persisted digests, or the legacy capture API.

## Selected approach

Reuse the existing qualified scope and native mount profile policy. A separate
source snapshot traverses every descendant relative to pinned directory handles,
retains complete directory membership and file evidence, and exposes the existing
plain SourceManifest by shared borrow. This is selected over ambient recursive
rereads, which do not bind entry access to inspected parents, and over filesystem
snapshot or probe creation, which would add writes and platform dependencies.

A source scope is selected independently from any destination scope and may be
on a different qualified mount. InstallationRoot's scope label remains caller
metadata; selecting a source root grants no ownership or effect authority.

## Interface

```rust
impl QualifiedScope {
    pub fn observe_source_tree(&self) -> anyhow::Result<SourceSnapshot<'_>>;
}
impl SourceSnapshot<'_> {
    pub fn manifest(&self) -> &SourceManifest;
    pub fn verify_unchanged(&self) -> anyhow::Result<()>;
}
```

Export SourceSnapshot from upskill::filesystem. It borrows exactly one
QualifiedScope, is opaque and non-Clone, and provides a non-content Debug view.
No mutable manifest accessor, public constructor, serialization, write method,
writer-lock acquisition, or successful partial capture is provided. Cloning the
plain manifest cannot forge or refresh the snapshot's private evidence. Capture
performs a complete revalidation before returning; later checks never refresh
captured values or retained baselines. Other operating systems expose the same
opaque type and return explicit unsupported errors.

## Root and mount binding

Reopen the selected root using QualifiedScope::reopen_directory and verify its
binding before and after capture/revalidation. Retain its descriptor identity.
Apply the existing descriptor-based Mount::of policy to the root and every
opened descendant. Require the selected root mount binding throughout the tree
and on later checks. Share the existing native qualification implementation by
moving it to a narrowly scoped private filesystem module; do not duplicate its
profile policy or weaken destination checks. Keep rustix pinned at 1.1.4.
Unknown profiles, missing returned Linux mount-ID evidence, mount changes and
unsupported hosts return contextual errors. Source qualification does not require
sharing the destination mount and makes no network-filesystem promise.

## Complete descriptor-relative enumeration

Enumerate each retained directory using rustix directory iteration. Ignore only
the native dot and dot-dot entries. Record and sort every other exact stored
name, including hidden and ignored names, empty directories and .git contents.
Reject non-UTF-8 or nonportable relative names before recursion/read. Preserve
strict source-manifest path order; enumeration order is not semantic.

For every entry, inspect with SYMLINK_NOFOLLOW and open exactly that single
stored component relative to its inspected parent with NOFOLLOW and CLOEXEC.
Directory opens require DIRECTORY; regular-file opens use NONBLOCK so a
replacement FIFO cannot block capture. Compare selected and opened identities
and kinds, qualify their mount, and retain the handles. Reject symlinks and
special entries without following or reading them. Reject repeated directory
identities to avoid directory aliases/cycles. All entry and iterator failures
abort without returning a partial manifest.

Regular hard links are allowed as distinct source paths, matching strict capture:
source observation authorizes no mutation. Record each path's exact bytes/mode
and link-count stamp; duplicate file identities do not collapse records. This
is deliberately different from destination final-file eligibility. Directory
membership and identity evidence includes empty directories even though the
manifest has no empty-directory records.

Retain each directory's identity, native mount, exact name set and a stamp
covering mode, link count, size and nanosecond mtime/ctime. Require the directory
stamp to remain unchanged across its own enumeration and descendant capture.
Ignore atime. A conservative rejection after metadata-only changes is permitted.

## Stable regular-file acquisition

Inspect the descriptor immediately before reading. Its stamp contains device,
inode, complete native mode, link count, size and nanosecond mtime/ctime. Read
exact bytes through the descriptor, recheck the stamp, and require byte length
to equal nonnegative observed size. Compare pre-open metadata with the opened
stamp as well. Capture permission evidence as Some(mode & 0o7777).

A narrow private read delegate on the actual production path supports independent
metadata-change, short-read and propagated-error controls. Production directly
returns File::read_to_end's result; no broad filesystem mock, global hook, async
runtime or new dependency. Recoverable filesystem/read/descriptor errors retain
operation/path context and original causes. Ordinary infallible allocations can
abort, and OS termination is outside Result guarantees.

Assemble the manifest through a shared crate-private source_manifest helper
that computes the unchanged v1 per-file/framed digests and invokes the existing
pure validation. The existing capture_source and validate_manifest signatures,
legacy traversal rules, digest bytes and diagnostics remain unchanged.

## Revalidation

Verify retained descriptor identities/stamps/mounts and reopen the complete tree
from the currently bound root. Compare each directory's exact membership, path,
identity and stamp, plus each file's path, identity, stamp, bytes and mode against
the original evidence. Detect added/removed/renamed files or empty directories,
replaced directories with identical descendants, same-byte file replacement,
symlinks/special substitutions, mode-only changes and fractional timestamp changes.
A changed selected root or ancestor alias binding rejects. Revalidate the root
binding again after the complete check. Fresh temporary observations are compared
and discarded; never replace original manifest or retained evidence. Repeated
verification after one mutation must fail on both calls with captured values
unchanged. Checks sample sequentially and provide neither an atomic tree snapshot
nor exclusion against external editors or hostile relocation/ABA changes.

## Acceptance and independent controls

1. Stable nested, hidden, ignored, binary, empty-file and hard-link fixtures yield
   the same literal records/digest as strict capture and pure validation succeeds.
   Verify twice, then compile captured fixture input without ambient rereads.
2. Empty directory additions/removals, file additions/removals/renames, same-byte
   inode replacement, directory replacement, root/ancestor rebinding and modes
   each reject twice while preserving the original manifest and fixture bytes.
3. Final/ancestor symlinks, socket/FIFO entries, invalid names and traversal errors
   return contextual errors without writes, follows, blocking or partial results.
4. Production-path private controls independently pin before/read/after stability,
   byte-count agreement and read-error context. A same-second nonzero fractional
   mtime-only verification fixture normalizes competing ctime evidence.
5. Independently omit complete membership comparison (empty-directory insertion),
   identity comparison (same-byte replacement with competing stamps normalized),
   exact value comparison, acquisition stamp/byte-count checks and call wiring,
   pre-return verification, native qualification call and mount comparison,
   source root-binding calls, and stale non-refresh behavior. Each omission must
   produce the named assertion RED; restore exact source bytes and focused GREEN.
6. Preserve independent destination qualification/mask controls after sharing the
   module. Source-specific native profile/mount wiring controls use real qualified
   metadata with only the rejected field/binding altered. Linux executes its
   missing returned mask case natively in hosted CI; cross-compilation is insufficient.
7. Unsupported-host compilation/opacity, descriptor CLOEXEC/NONBLOCK/NOFOLLOW and
   cleanup after failed acquisition are pinned. No privileged mount/device action.
8. Run just fmt then just verify, warning-denying all-target Clippy, Rust 1.85,
   native APFS and final-head hosted Linux/Windows CI. Measure matched release
   baseline/candidate probes with the new API live; record bytes without claiming
   runtime performance or a qualified Windows backend.

## Delivery and limits

One self-contained implementation task owns the source capability, private shared
qualification extraction, source-manifest assembly helper, tests and current docs.
Use a fresh implementer then independent task review, whole-branch final review,
scoped gardening, two PR review passes, exact-head hosted checks and squash merge.
Preserve all 34 unrelated WIP files, earlier raw archives and other worktrees.

This is a sampled read-only source stabilization prerequisite. It does not record
an immutable Git revision, establish authenticity, lock external source writers,
activate CLI/schema-2, write migration backup/state, establish permission policy,
apply files, implement recovery or complete M1-A2/M1-A3/M1. Real privileged mounts,
device nodes, physical I/O/exhaustion failures and hostile namespace behavior are
unexercised unless separately evidenced. Durable status must remain honest.

## Local implementation evidence

Native APFS focused coverage includes 10 public source acceptance tests, 27
source-private tests (including the public pre-return seam), 24 existing strict
manifest tests and 28 existing destination acceptance tests. Source-private tests
exercise stable reads, changed metadata, short reads, returned counts, contextual
errors, retained flags, failed-capture cleanup, exact values, complete membership,
independent inode identity, fractional mtime, changed mounts and native profile
rejection. FIFO subprocesses own their deadlines, kill and reap only their child.
Linux-only non-UTF-8 and missing returned mount-ID mask cases require native Linux
execution. Windows exposes the opaque API but has no qualified backend.

Independent omission evidence normalizes only competing stamp differences for
membership and identity fixtures. Redundant source root-binding checks remain:
their end-to-end omission control bypasses all source verification binding
frontiers and rereads through original retained descriptors. Existing scope tests
pin the underlying binding implementation. Individual redundant checks are not
claimed independently necessary. Final task counts, restored checks and matched
release-size evidence are recorded with implementation review; these local
results do not complete hosted gates or M1.

## Matched local release size

Rust 1.97.1 release builds with identical pinned root dependency resolution and
release profile produce byte-identical baseline/candidate CLI binaries of
3,215,120 bytes (delta 0). Minimal public-API probes use the same pinned package
versions and profile: control 303,120 bytes, source capture plus revalidation
353,840 bytes (delta 50,720). Both runtime-input probe fixtures succeed. This
records binary size only, not performance; inactive API code is stripped from the
current CLI. Final-head hosted Linux/Windows qualification remains pending.

The correction for issue #274 independently pins regular-descendant native
profile and mount-binding qualification before any reader invocation, exact
special permission bits by path, capture and verification under an existing
writer guard, and Debug excluding captured content. These tests do not establish
privileged mount behavior. Independent link-count and ctime omission isolation
remains an advisory coverage limit; production stamps retain and compare both
fields.
