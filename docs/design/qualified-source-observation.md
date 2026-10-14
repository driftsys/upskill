# Qualified source tree observation

The inactive library boundary is implemented independently of command dispatch.
Hosted Linux qualification and the Windows scope/compile boundary passed at
`a393263` on 2026-10-13. Hosted checks for the eventual final PR head, including
the new Windows source-method runtime assertion, remain merge gates. Decision:
[AD-0030](../adr/0030-capture-qualified-source-evidence.md).

## Intent

Complete the source-observation prerequisite of the delegated owned-content
lifecycle. Existing strict manifests capture exact bytes for pure compilation
but use ambient paths and contain freely mutable values. Introduce an inactive,
read-only filesystem capability that binds a complete captured tree to an
existing qualified scope and can reject later namespace or value changes.
Success is independently tested acquisition and immutable revalidation, with no
change to current command dispatch, persisted digests, or the legacy capture
API.

## Selected approach

Reuse the existing qualified scope and native mount profile policy. A separate
source snapshot traverses every descendant relative to pinned directory handles,
retains complete directory membership and file evidence, and exposes the
existing plain SourceManifest by shared borrow. This is selected over ambient
recursive rereads, which do not bind entry access to inspected parents, and over
filesystem snapshot or probe creation, which would add writes and platform
dependencies.

A source scope is selected independently from any destination scope and may be
on a different qualified mount. InstallationRoot's scope label remains caller
metadata; selecting a source root grants no ownership or effect authority.

## Interface

`QualifiedScope::observe_source_tree` returns a borrowing `SourceSnapshot`.
`SourceSnapshot::manifest` borrows its `SourceManifest`; `verify_unchanged`
returns success only when the captured evidence still agrees. The public
interface is defined in
[the source module](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/src/filesystem/sources.rs).

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
unsupported hosts return contextual errors. Source qualification does not
require sharing the destination mount and makes no network-filesystem promise.

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

Regular hard links are allowed as distinct source paths, matching strict
capture: source observation authorizes no mutation. Record each path's exact
bytes/mode and link-count stamp; duplicate file identities do not collapse
records. This is deliberately different from destination final-file eligibility.
Directory membership and identity evidence includes empty directories even
though the manifest has no empty-directory records.

Retain each directory's identity, native mount, exact name set and a stamp
covering mode, link count, size and nanosecond mtime/ctime. Require the
directory stamp to remain unchanged across its own enumeration and descendant
capture. Ignore atime. A conservative rejection after metadata-only changes is
permitted.

## Stable regular-file acquisition

Inspect the descriptor immediately before reading. Its stamp contains device,
inode, complete native mode, link count, size and nanosecond mtime/ctime. Read
exact bytes through the descriptor, recheck the stamp, and require byte length
to equal nonnegative observed size. Compare pre-open metadata with the opened
stamp as well. Capture permission evidence as Some(mode & 0o7777).

A narrow private read delegate on the actual production path supports
independent metadata-change, short-read and propagated-error controls.
Production directly returns File::read_to_end's result; no broad filesystem
mock, global hook, async runtime or new dependency. Recoverable
filesystem/read/descriptor errors retain operation/path context and original
causes. Ordinary infallible allocations can abort, and OS termination is outside
Result guarantees.

Assemble the manifest through a shared crate-private source_manifest helper that
computes the unchanged v1 per-file/framed digests and invokes the existing pure
validation. The existing capture_source and validate_manifest signatures, legacy
traversal rules, digest bytes and diagnostics remain unchanged.

## Revalidation

Verify retained descriptor identities/stamps/mounts and reopen the complete tree
from the currently bound root. Compare each directory's exact membership, path,
identity and stamp, plus each file's path, identity, stamp, bytes and mode
against the original evidence. Detect added/removed/renamed files or empty
directories, replaced directories with identical descendants, same-byte file
replacement, symlinks/special substitutions, mode-only changes and fractional
timestamp changes. A changed selected root or ancestor alias binding rejects.
Revalidate the root binding again after the complete check. Fresh temporary
observations are compared and discarded; never replace original manifest or
retained evidence. Repeated verification after one mutation must fail on both
calls with captured values unchanged. Checks sample sequentially and provide
neither an atomic tree snapshot nor exclusion against external editors or
hostile relocation/ABA changes.

## Components and verification

[The private backend](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/src/filesystem/sources/backend.rs)
owns the retained `Evidence`, directory tree, regular-file records, native
stamps and exact membership comparison.
[Shared qualification](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/src/filesystem/qualification.rs)
retains the destination policy.
[Manifest assembly](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/src/source_manifest.rs)
provides the unchanged v1 framing and pure validation.

[Public acceptance tests](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/tests/filesystem_sources.rs)
cover stable complete capture, captured compilation, namespace and value
changes, immutable stale checks, rejected entry kinds, permissions, held-writer
operation and non-content Debug output.
[Private tests](https://github.com/driftsys/upskill/blob/a4e84d9334f7717bfb170d59afcbaf7f829d1e82/src/filesystem/sources/backend_tests.rs)
isolate acquisition stability, counts, errors, identity, membership, exact
values, fractional timestamps, qualification wiring, descriptor flags and
cleanup.

## Limits

This is a sampled read-only source stabilization prerequisite. It does not
record an immutable Git revision, establish authenticity, lock external source
writers, activate CLI/schema-2, write migration backup/state, establish
permission policy, apply files, implement recovery or complete M1-A2/M1-A3/M1.
Real privileged mounts, device nodes, physical I/O/exhaustion failures and
hostile namespace behavior are unexercised unless separately evidenced. Durable
status must remain honest.

## Local implementation evidence

The historical local checks before the final source-binding correction at commit
`a4e84d9334f7717bfb170d59afcbaf7f829d1e82` passed 964 outer Rust tests (457
library, 500 integration and seven compile-fail doctests), with zero failed or
ignored tests, and 11 documentation-checker tests. Formatting, full
verification, locked warning-denying all-target Clippy and Rust 1.85 all-target
compilation passed. Nested subprocess summaries are excluded from these counts.
Before that final correction, focused native APFS coverage included 11 public
source tests, 30 source-private tests, 24 strict-manifest tests and 28
destination acceptance tests. The earlier combined independent omission evidence
contains 43 valid assertion failures with exact restoration and focused success;
two historical discarded experiments are not counted as controls.

The final source-binding correction preserves source-root/verification context
at the initial public `verify_unchanged` check while retaining the redundant
binding check and original scope cause. Public root replacement and ancestor
alias tests require the complete chain twice and preserve the original manifest.
An actual nested-file chmod fixture leaves complete parent stamps and membership
unchanged, then requires retained `nested/data` context, the original cause and
manifest preservation twice. Focused coverage at `a393263` included 12 public
source tests and 31 source-private tests. Three additional independent
context/path controls bring the combined valid count to 46, with exact
restoration and focused success; the same two historical discarded attempts
remain excluded. The earlier initial-binding exception and retained-descendant
assertion gap are resolved. Independent link-count and ctime omission isolation
remains a coverage limit; production stamps compare both fields.

The PR closeout adds a final-binding omission control that rebinds an ancestor
after fresh capture and comparison. It requires operation/root context and the
original scope cause, while retaining all production binding checks. Empty-root
and directory-only positive fixtures capture and revalidate valid empty
manifests. The two positive fixtures reject an injected empty-manifest refusal
independently. Restored local APFS coverage passes 14 public source tests and 32
source-private tests. Historical control totals above remain dated evidence.
Independent link-count and ctime omission isolation remains outside this
closeout.

On 2026-10-13,
[CI run 38346202172](https://github.com/driftsys/upskill/actions/runs/38346202172)
and
[CodeQL run 38346200154](https://github.com/driftsys/upskill/actions/runs/38346200154)
succeeded for PR head `a3932633e9cc9a269f8ba8d8b6dd2ec26d0b828e`. The tested
merge commit `604dcda92bbc3c08d4ce10b695b18f95fb4d5e83` and that PR head share
tree `48b716136a7822428e6ed163be2689beaaf9ddfa`. Native Linux job
[115128098194](https://github.com/driftsys/upskill/actions/runs/38346202172/job/115128098194)
executed the source missing mount-ID mask and non-UTF-8 cases. Windows job
[115128098069](https://github.com/driftsys/upskill/actions/runs/38346202172/job/115128098069)
executed scope rejection and compiled the opaque source API; its uncalled source
helper did not establish source-method runtime coverage. A new private
unsupported-host test now calls the source method and requires contextual
rejection. Its native Windows execution and all hosted checks for the eventual
final PR head remain merge gates. No qualified Windows backend is implemented.
FIFO subprocess fixtures own their deadlines and kill/reap only their child. No
privileged mount/device or physical fault claim follows.

## Historical matched local release size before the final binding fix

Matched Rust 1.97.1 locked release builds against baseline `fa2f322`, using the
same profile and pinned common dependencies, measure baseline and candidate CLI
binaries at 3,215,120 bytes each (delta zero). Their hashes differ. The control
public-API probe is 303,120 bytes; live capture plus explicit revalidation is
353,840 bytes (delta 50,720). Both runtime-input fixtures pass. These
measurements establish binary size only; inactive API code is stripped from the
CLI. Earlier measurements are historical and do not establish final-head hosted
qualification.

The binding-context fix measured at `a3932633e9cc9a269f8ba8d8b6dd2ec26d0b828e`
refreshes the candidate CLI and live public-API probe under the same Rust 1.97.1
locked dependencies, release profile and runtime inputs. Exact hash-verified
prior baseline/control binaries are reused: CLI baseline/candidate remain
3,215,120 bytes (delta zero); control is 303,120 bytes and live
capture/revalidation is 353,840 bytes (delta 50,720). Both runtime-input
fixtures pass. No byte-identity, performance or final-head hosted success claim
follows from these size measurements.

The PR closeout adds test fixtures and a generic no-op revalidation test seam,
without changing public runtime semantics. Size builds were not repeated; the
measurements above apply to the prior measured source at `a393263`, not the
eventual final PR head.
