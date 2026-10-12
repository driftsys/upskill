# Qualified scope roots and writer locking

The library boundary is implemented; broader platform qualification remains pending.

## Intent and boundary

Establish one process-safe writer exclusion primitive for the owned lifecycle,
without changing current CLI behavior or writing state. This is the first part
of the M1 filesystem stage. Destination observation, equivalence, containment,
source enumeration, file application, recovery and Windows qualification remain
separate gates. Completing this slice does not complete the filesystem stage.

Retain Rust 1.85, synchronous execution, a single binary, zero warnings and
library-only presentation. Current commands remain on schema 1.

## Components and interfaces

The [filesystem module](https://github.com/driftsys/upskill/blob/4a80c9d2a385fe003df226ad2bf268c2e3e665e4/src/filesystem.rs) exports `QualifiedScope` and
`ScopeWriterGuard`. Their [implementation](https://github.com/driftsys/upskill/blob/4a80c9d2a385fe003df226ad2bf268c2e3e665e4/src/filesystem/scope.rs) keeps
selection, binding validation and the Linux/macOS directory-lock backend private.
It consumes `ownership::InstallationRoot`; the scope label does not determine
physical lock identity. Unsupported-platform branches return contextual errors.
The [real-filesystem integration tests](https://github.com/driftsys/upskill/blob/4a80c9d2a385fe003df226ad2bf268c2e3e665e4/tests/filesystem_scope_lock.rs)
exercise the public boundary and subprocess lifetime; private unit tests pin
acquisition ordering and final-symlink refusal independently.

## Alternatives and decision

Select a pinned existing directory handle and nonblocking OS advisory exclusive
lock on that directory on Linux and macOS. It avoids creating a persistent lock
file, removing a lock inode during release, stale PID cleanup and read-only preview
writes. The inode, not a root spelling or Project/User label, identifies the lock.

A create-new sentinel needs a separate crash/stale-owner protocol and is rejected.
A portable lock-file abstraction requires a securely established persistent lock
namespace and Windows qualification; retain that as a future option. New APIs
return an explicit unsupported error on other operating systems. Existing CLI
behavior remains available there. Do not claim all supported-platform filesystem
qualification from Linux/macOS evidence.

Admit the existing locked `rustix` 1.1.4 dependency directly, target-specific to
Linux/macOS, with filesystem support. Record the dependency decision in AD-0028.
Do not add an async runtime or hand-written unsafe syscall bindings. Standard
library file locks require Rust 1.89 and cannot satisfy Rust 1.85.

## Root contract

`filesystem::QualifiedScope::open(&InstallationRoot)` returns an opaque,
non-Clone pinned root or an error. Require a nonempty absolute existing directory.
Reject parent components, root symlinks, regular files, missing roots and special
files. Normalize redundant current-directory and trailing separators before
checking the final component, so they cannot hide a symlink. Preserve non-UTF-8
root paths without lossy conversion where the filesystem accepts them; propagate
filesystem refusal contextually.

Ancestor symlinks are permitted during explicit ambient root selection, including
macOS temporary-directory aliases. Open both the normalized selected path and its
resolved directory with no final symlink following, and compare both metadata
identities with the selected root. Retain original normalized spelling, resolved
path and the resolved directory's open identity.
This binds the explicitly chosen root; it is not descendant containment or a
security sandbox against hostile namespace relocation.

`verify_binding()` reopens the selected root and checks its directory identity,
resolved path and nonsymlink final component. Renaming/replacing the root,
retargeting an ancestor alias or substituting a symlink must reject the old scope.
The pinned handle alone does not freeze the path namespace. Directory edits by
other processes remain possible, and later action code must revalidate bindings
and descendant preconditions immediately before each action.

## Writer contract

`try_lock_writer()` acquires a fresh open description of the pinned directory,
checks identity, obtains a nonblocking exclusive OS lock, then checks the root
binding again before returning an opaque non-Clone `ScopeWriterGuard` borrowing
the scope. Reopening must use the pinned descriptor (for example `openat(".")`),
not a cloned description that shares a flock with another guard.

A competing same-process or cross-process writer to the same physical root
returns a contextual busy error promptly. Different roots do not contend. Alias
spellings and different scope labels cannot split exclusion. A dropped guard or
terminated holder releases the lock through descriptor close. Do not expose the
locked handle or allow cloning it. Use close-on-exec descriptors so ordinary
subprocess execution does not prolong exclusion. This API promises no fork-safe
lease, fairness, blocking wait, network-filesystem guarantee or protection against
external editors. If the filesystem refuses locking, return that error; never
pretend the lock succeeded.

Opening, validation, lock acquisition and release create no files, directories,
backups, journals or schema changes and preserve existing bytes and modes.
They do not establish state/source/destination preconditions or authorize writes.

## Acceptance and evidence

| ID  | Required example                                                                                                                                                      |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q01 | Pin a real absolute root, verify binding, acquire/drop/reacquire without changing recursive directory contents or modes.                                              |
| Q02 | Reject relative, empty, parent-traversing, missing, file and symlink roots, including trailing slash and dot disguises.                                               |
| Q03 | Equivalent root spellings and different scope labels contend; independent roots can lock together.                                                                    |
| Q04 | A second guard in the same process is busy while the first lives and succeeds after drop.                                                                             |
| Q05 | A real child process holds the lock; parent is busy; graceful child exit and forced termination each permit reacquisition. Synchronize readiness and bound all waits. |
| Q06 | Replacement/rename/symlink substitution and ancestor-alias retargeting reject stale binding before a guard is returned.                                               |
| Q07 | Native non-UTF-8 roots work where the filesystem accepts them; filesystem refusal remains contextual. Unsupported platforms return explicit errors without effects.   |
| Q08 | A child exec from a holder does not retain the lock after the holder exits.                                                                                           |

Tests use temporary roots and never write developer HOME. Capture failing
acceptance tests before implementation, assertion-failure semantic controls for
lock omission, missing binding validation and accidental description sharing,
Rust 1.85 checks, full repository verification, Linux hosted contention/release
and local macOS evidence. Windows tests pin the unsupported boundary.

Measure release CLI size against the same-toolchain baseline. Also measure a
small release probe that actually calls root qualification and locking against a
matched probe, because dead-code elimination hides an inactive library feature
from CLI size. Record toolchain, target, exact bytes and interpretation; these
measurements do not establish future executor size.

## Implementation evidence

Local evidence recorded on 2026-10-13 uses `rustc 1.97.1 (8bab26f4f 2026-07-14)`,
LLVM 22.1.6, target `aarch64-apple-darwin`. Rust 1.85.0
`cargo check --locked --all-targets` passed. The complete `cargo test --locked`
suite passed at initial implementation `eeabc81` (862 main-harness tests), as did
`cargo clippy --locked --all-targets -- -D warnings` and the rendered
documentation checker.

The initial real-filesystem integration binary passed all 10 tests covering Q01-Q08,
including exact recursive entries, bytes and Unix modes, alias and label
contention, independent roots, graceful and forced child release, root replacement,
ancestor retargeting and a live exec successor. Helpers run only under explicit
test environment variables with temporary HOME/USERPROFILE. Readiness and exit
polling have deadlines; cleanup kills and reaps unfinished children. Tests
serialize process setup because temporary pre-exec fork inheritance can overlap
another fixture's immediate release check. Close-on-exec prevents retention after
exec; the API does not promise a fork-safe lease.

One-at-a-time semantic controls produced assertion failures: omitting flock
failed same-process and subprocess contention, cloning the pinned description
failed same-process contention, omitting binding validation failed replacement
and ancestor-retargeting checks, and omitting close-on-exec failed reacquisition
while the exec successor remained alive. Restored focused and complete tests
passed. These controls are scratch evidence, not production toggles.

The local Data volume was verified as APFS with `diskutil info
/System/Volumes/Data`. Native `root-0xff` directory creation was refused before
qualification with `EILSEQ`, errno 92, “Illegal byte sequence”. The macOS test
records that refusal and verifies contextual API failure without effects. On
filesystems accepting that native name, including the required Linux fixture,
the same test must pin, verify and lock it successfully. No lossy conversion is
used. Hosted Linux contention, release and native-path success evidence and the
Windows unsupported-boundary test remain pending actual CI. Other filesystems
and operating systems are not qualified by this local run.

### Review follow-up controls

Subsequent review added two public integration examples and two focused unit
examples. A stale root with an existing guard must fail binding validation before
attempting the contended lock. A small private acquisition closure lets the unit
test replace the namespace after the initial check and then acquire a real lock
on the pinned directory; the final check must reject the guard and release that
lock. Only this unit test's final cleanup reacquisition tolerates busy errors
for up to two seconds: unrelated library tests can fork while a descriptor is
open. Other errors fail immediately, and a persistent descriptor leak must fail
at the deadline. Integration release assertions remain immediate under serialized
fixtures. The public method always passes the real backend acquisition operation.
There is no public test hook or new runtime policy.

A same-inode canonical relocation example renames an ancestor and retargets its
alias to the renamed directory, preserving the selected child's device/inode
while changing its canonical path. Binding verification and acquisition both
reject the old scope. A direct backend directory-open test rejects a final symlink
independently of selection-time metadata validation.

Independent mutations removing only the pre-lock check, only the post-lock check,
only the canonical-path comparison and only NOFOLLOW each produced runtime
assertion failures. A separate deliberate descriptor-leak control failed at the
cleanup deadline, proving the bounded test wait does not accept a persistent leak.
Restored focused runs passed 12 integration and two scope
unit tests. Updated all-target Rust 1.85 compilation and clippy passed. The
controller owns the next complete-suite gate after this focused review fix.

### Release size

Baseline source is commit `a18a888`. Baseline and candidate used the compiler and
target above with the repository release profile (`opt-level = "z"`, LTO,
codegen-units 1, stripping and aborting panic). Two separate temporary probe
packages used that same profile and matched native-argument, path and Result
scaffolding: the control performs basic absolute-path/directory checks; the live
probe calls root qualification, writer locking and guard binding verification.
Both probes ran successfully against a temporary directory. Measurements were
refreshed after the private acquisition-helper refactor and remained unchanged.
Probe sources are
not part of the shipped binary.

| Artifact              | Baseline/control bytes | Candidate/live bytes | Delta bytes |
| --------------------- | ---------------------: | -------------------: | ----------: |
| Release CLI           |              3,215,120 |            3,215,120 |           0 |
| Matched release probe |                302,848 |              303,168 |         320 |

The inactive CLI can eliminate this unused API, so its unchanged size alone is
not admission evidence. The live-probe file delta includes root, lock and error
machinery, as well as ordinary linker and Mach-O padding effects; it is not an
exact code-size estimate and does not establish future executor size. No Linux
or Windows size claim follows from these local macOS measurements.

## References

- [AD-0028](../adr/0028-lock-qualified-scope-directories.md).
- [Linux flock](https://man7.org/linux/man-pages/man2/flock.2.html).
- [Apple flock](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html).
- [Rust File lock stabilization](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).
- [rustix 1.1.4](https://docs.rs/crate/rustix/1.1.4).
