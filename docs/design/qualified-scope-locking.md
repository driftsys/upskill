# Qualified scope roots and writer locking

Date: 2026-10-13. Status: Accepted for implementation under delegated delivery authority.
Baseline: PR #265, `e2ce18e`.

## Intent and boundary

Establish one process-safe writer exclusion primitive for the owned lifecycle,
without changing current CLI behavior or writing state. This is the first part
of the M1 filesystem stage. Destination observation, equivalence, containment,
source enumeration, file application, recovery and Windows qualification remain
separate gates. Completing this slice does not complete the filesystem stage.

The user delegated routine design and delivery decisions for the continuing loop.
Retain Rust 1.85, synchronous execution, a single binary, zero warnings and
library-only presentation. Current commands remain on schema 1.

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
checking the final component, so they cannot hide a symlink. Accept non-UTF-8
root paths without lossy conversion.

Ancestor symlinks are permitted during explicit ambient root selection, including
macOS temporary-directory aliases. Resolve them once, open the resolved directory
with no final symlink following, and compare metadata identity with the selected
root. Retain original normalized spelling, resolved path and open identity.
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

| ID | Required example |
| --- | --- |
| Q01 | Pin a real absolute root, verify binding, acquire/drop/reacquire without changing recursive directory contents or modes. |
| Q02 | Reject relative, empty, parent-traversing, missing, file and symlink roots, including trailing slash and dot disguises. |
| Q03 | Equivalent root spellings and different scope labels contend; independent roots can lock together. |
| Q04 | A second guard in the same process is busy while the first lives and succeeds after drop. |
| Q05 | A real child process holds the lock; parent is busy; graceful child exit and forced termination each permit reacquisition. Synchronize readiness and bound all waits. |
| Q06 | Replacement/rename/symlink substitution and ancestor-alias retargeting reject stale binding before a guard is returned. |
| Q07 | Non-UTF-8 roots work; unsupported platforms return explicit errors without effects. |
| Q08 | A child exec from a holder does not retain the lock after the holder exits. |

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

## References

- [AD-0028](../adr/0028-lock-qualified-scope-directories.md).
- [Linux flock](https://man7.org/linux/man-pages/man2/flock.2.html).
- [Apple flock](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html).
- [Rust File lock stabilization](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).
- [rustix 1.1.4](https://docs.rs/crate/rustix/1.1.4).
