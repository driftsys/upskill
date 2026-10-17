# Filesystem access

This concern owns the requirements and implemented architecture below. Moving
these records does not change their implementation or approval status.
Library-only capabilities remain separate from production CLI activation; each
section records its own limits. Accepted decisions remain in
[ADRs](https://github.com/driftsys/upskill/blob/main/docs/adr/README.md).

## Requirements

<a id="47-qualified-scope-roots-and-advisory-writer-locking"></a>

### Qualified scope roots and advisory writer locking

`filesystem::QualifiedScope::open(&InstallationRoot)` pins an existing absolute
scope directory on Linux and macOS. Parent components, missing or non-directory
roots and final symlinks are rejected. Dot components and trailing separators
are normalized before final symlink checks. Ancestor aliases are permitted
during explicit selection; native non-UTF-8 paths are preserved where the
underlying filesystem accepts them. Filesystem refusals remain contextual
errors.

`verify_binding()` revalidates the normalized selected spelling, canonical path
and directory device/inode identity. A renamed, replaced or symlink-substituted
root, or retargeted ancestor alias, invalidates the old binding. The held handle
does not freeze the path namespace or establish descendant containment.

`try_lock_writer()` uses a fresh close-on-exec directory description and a
nonblocking exclusive advisory lock. It revalidates binding before and after
acquisition. Same-process and cross-process writers to one physical directory
contend, including aliases and different Project/User labels; distinct
directories remain independent. Contention returns a busy error. Other
filesystem failures remain errors. An opaque borrowing `ScopeWriterGuard`
retains exclusion until drop or holder termination; ordinary exec does not
retain it. Temporary fork inheritance is possible. No fork-safe lease, fairness,
network-filesystem guarantee or exclusion against external editors is promised.

These operations create no files, directories or persistent state and preserve
existing bytes and permissions. The API exposes no raw descriptor, cloning or
write operation. Other operating systems explicitly report unsupported behavior.
This library prerequisite activates no CLI command, schema change, lifecycle
execution, descendant containment, observation or recovery. It does not complete
the M1 filesystem stage or Windows qualification.

See
[AD-0028](https://github.com/driftsys/upskill/blob/main/docs/adr/0028-lock-qualified-scope-directories.md)
and the [qualified scope-lock design](#architecture-qualified-scope-locking) for
rationale, acceptance examples and platform evidence.

<a id="48-qualified-destination-observations"></a>

### Qualified destination observations

`QualifiedScope::observe_destinations(&[FileLocation])` captures exact optional
`FileValue` values beneath the borrowed physical scope. Regular files return
binary bytes and Unix permission bits (`Some(mode & 0o7777)`); absent
destinations return `None`. Observations preserve the supplied native roots and
portable paths, sorted by scope, raw root spelling and relative path. Every
supplied root must independently qualify and bind to the same physical
directory, including ancestor-alias spellings and different Project/User labels.
Empty input still validates the scope and filesystem profile.

Capture validates portable paths and rejects every potentially overlapping pair
before descendant reads. Components may match when either contains non-ASCII
characters or both compare equal ignoring ASCII case. If every component pair
through the shorter path may match, the batch is rejected, including duplicates,
prefix pairs and aliases across root spellings or labels. This conservative rule
can reject distinct files on case-sensitive filesystems.

The inactive API qualifies APFS on macOS and ext-family/tmpfs filesystems on
Linux. Linux requires a returned statx mount ID; macOS retains the fstatfs
filesystem ID and mount bindings. Unknown profiles, missing mount evidence,
cross-mount traversal and other operating systems return explicit errors.
Descriptor-relative enumeration requires exact stored component spellings.
No-follow traversal rejects symlinks, non-directory ancestors, directory leaves,
files with multiple links and special files. Only ENOENT establishes absence;
recoverable filesystem, permission, I/O, descriptor-exhaustion and unsupported
failures remain contextual errors, including allocation errors reported by a
fallible read. Recoverable memory exhaustion is not guaranteed: ordinary Rust
infallible allocation failures may abort, and the operating system may terminate
the process instead of returning `Err`.

An opaque, non-Clone `DestinationSnapshot` retains close-on-exec root, directory
and file handles, selected root bindings, mount evidence and either exact file
stamps or the first missing component with its unresolved suffix. Capture checks
identity, type, links, size, mode, modification/change times (including
fractional seconds) and byte count around each read. Before returning, and again
from `verify_unchanged()`, it checks fresh bindings and directory identities,
exact stored spellings, mounts, exact bytes, file stamps and absence evidence.
These checks never refresh stale expectations. Timestamp-only changes, edits
with bytes and mtime restored but changed ctime, same-size edits with restored
mtime, chmod, identical-byte inode replacement, ancestor replacement,
missing-to-present transitions and root-alias retargeting invalidate evidence.
Access times and unrelated directory timestamp changes do not invalidate it.
Retained handles close on drop and returned-error paths; process abort or
termination does not guarantee Rust destructors run.

Observation creates no files, directories, locks or persistent state and
acquires no writer guard. Plain observations convey no execution or permission
authority. These sampled checks do not exclude external writers or hostile
ancestor relocation. An applying executor must retain its writer guard and
independently establish complete lifecycle inputs, authority, permission policy,
immediate pre-effect checks and recovery. Existing commands and schema 1 remain
unchanged; Windows qualification and complete observational lifecycle
integration remain pending. The opt-in
[application boundary](0008-execution-and-recovery.md#guarded-lifecycle-application)
provides guarded execution and recovery for supplied plans. Hosted Linux runtime
qualification and Windows unsupported-boundary checks passed on 2026-10-13 for
PR head `765e70b9b7ce5386530ffea1fe9814b228724428` (merge checkout
`242930ef9107aff6eff432484f474e47157983f2`) in
[Actions run 38330080515](https://github.com/driftsys/upskill/actions/runs/38330080515).
That evidence does not establish a qualified Windows backend or cover later
local timestamp and special-bit tests. Independent regression controls now cover
fractional mtime changes, metadata changes during acquisition, short reads with
unchanged descriptor metadata, unknown native profiles and different retained
mount bindings through the production qualification and verification paths.
Existing-file overlap and repeated stale verification are also pinned. Linux's
missing returned mount-ID mask production-path case passed natively on Ubuntu in
[Actions run 38334882688](https://github.com/driftsys/upskill/actions/runs/38334882688)
on 2026-10-13 for PR #273 head `2a50ca6a5163e11912cbcee7987593c68e0a4197`.
Ubuntu job `115092610337` passed 28 public destination cases; four native
profile/mask check and production-call omissions failed their intended
assertions. Each restored `qualification.rs` to SHA-256
`1c5b288ed575ae1ccdd99a10170c13aa9054f92004a0927a64fb1587bac40bd5`, followed by
22 restored private cases GREEN. Local APFS separately passed 28 public and 22
private cases plus 14 independent native omission controls. All 13 head checks
succeeded, including Windows unsupported-boundary job `115092610428`. This dated
evidence covers that head; hosted CI for the final head after temporary harness
removal remains required before merge. Injected query results and expected mount
bindings establish neither real privileged mount or fault behavior nor a
qualified Windows backend. See
[AD-0029](https://github.com/driftsys/upskill/blob/main/docs/adr/0029-capture-qualified-destination-evidence.md)
and the
[destination observation design](#architecture-qualified-destination-observation).

<a id="410-explicit-file-permission-selection"></a>

### Explicit file permission selection

`files::permissions::select_file_mode(destination, source, resource_execution)`
is an inactive pure library API returning `anyhow::Result<u32>`. Its
supplied-value inputs are `DestinationMode::{Absent, Present(Option<u32>)}`,
`PermissionSource::{Generated, Resource { captured_mode: Option<u32> }}` and
`ResourceExecutionPolicy::{Disabled, OwnerExecutable}`. These types derive
`Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`; they carry no serialized state
or effect capability.

Every supplied destination and captured resource mode must be within
`0o0000..=0o7777`, including evidence unused by the selection branch. Errors
identify destination or captured resource evidence. When multiple inputs are
invalid, no particular diagnostic ordering is promised. `Present(None)` rejects
as unknown existing mode. Existing modes containing any `0o7000` special bit
reject as unsupported application policy; ordinary existing modes are returned
exactly, including `0o000`, `0o400`, `0o640`, `0o755` and `0o777`, under either
resource policy.

Absent generated output selects literal `0o600` under either policy. Absent
resources also select `0o600` when execution is `Disabled`, including unknown
captured mode. `OwnerExecutable` requires a known captured resource mode and
selects `0o700` only when its owner-execute bit `0o100` is set; otherwise it
selects `0o600`. Captured `0o755` and `0o4777` therefore select `0o700`;
`0o011`, `0o644`, `0o1000`, `0o2000` and `0o4000` select `0o600`. Source
group/other and special bits are never copied. Unknown captured resource mode
rejects only when selecting new resource execution; an existing ordinary
destination mode remains exact.

The function reads no environment or process umask and performs no I/O. Supplied
values and selected bits convey no source authenticity, ownership, physical
qualification or execution authority. Future writers must separately qualify
owner/group, ACLs, extended metadata, containment and atomic replacement and
persist actual resulting mode evidence. This API changes no compiler output,
planner validation, CLI behavior or lockfile schema. The opt-in executor in
[qualified file execution and recovery](0008-execution-and-recovery.md#412-qualified-file-execution-and-recovery)
supplies file application, journals and recovery. Combined lifecycle state,
activation and hosted qualification remain pending; M1-A2, M1-A3 and M1 remain
incomplete. See
[AD-0031](https://github.com/driftsys/upskill/blob/main/docs/adr/0031-select-explicit-file-permissions.md)
and the [permission design](#architecture-file-permission-policy).

## Architecture

<a id="architecture-qualified-scope-locking"></a>

<a id="architecture-qualified-scope-locking-qualified-scope-roots-and-writer-locking"></a>

### Qualified scope roots and writer locking

The library boundary is implemented; broader platform qualification remains
pending.

<a id="architecture-qualified-scope-locking-intent-and-boundary"></a>

#### Intent and boundary

Establish one process-safe writer exclusion primitive for the owned lifecycle,
without changing current CLI behavior or writing state. This is the first part
of the M1 filesystem stage. Destination observation, equivalence, containment,
source enumeration, file application, recovery and Windows qualification remain
separate gates. Completing this slice does not complete the filesystem stage.

Retain Rust 1.85, synchronous execution, a single binary, zero warnings and
library-only presentation. Current commands remain on schema 1.

<a id="architecture-qualified-scope-locking-components-and-interfaces"></a>

#### Components and interfaces

The
[filesystem module](https://github.com/driftsys/upskill/blob/4a80c9d2a385fe003df226ad2bf268c2e3e665e4/src/filesystem.rs)
exports `QualifiedScope` and `ScopeWriterGuard`. Their
[implementation](https://github.com/driftsys/upskill/blob/4a80c9d2a385fe003df226ad2bf268c2e3e665e4/src/filesystem/scope.rs)
keeps selection, binding validation and the Linux/macOS directory-lock backend
private. It consumes `ownership::InstallationRoot`; the scope label does not
determine physical lock identity. Unsupported-platform branches return
contextual errors. The
[real-filesystem integration tests](https://github.com/driftsys/upskill/blob/4a80c9d2a385fe003df226ad2bf268c2e3e665e4/tests/filesystem_scope_lock.rs)
exercise the public boundary and subprocess lifetime; private unit tests pin
acquisition ordering and final-symlink refusal independently.

<a id="architecture-qualified-scope-locking-alternatives-and-decision"></a>

#### Alternatives and decision

Select a pinned existing directory handle and nonblocking OS advisory exclusive
lock on that directory on Linux and macOS. It avoids creating a persistent lock
file, removing a lock inode during release, stale PID cleanup and read-only
preview writes. The inode, not a root spelling or Project/User label, identifies
the lock.

A create-new sentinel needs a separate crash/stale-owner protocol and is
rejected. A portable lock-file abstraction requires a securely established
persistent lock namespace and Windows qualification; retain that as a future
option. New APIs return an explicit unsupported error on other operating
systems. Existing CLI behavior remains available there. Do not claim all
supported-platform filesystem qualification from Linux/macOS evidence.

Admit the existing locked `rustix` 1.1.4 dependency directly, target-specific to
Linux/macOS, with filesystem support. Record the dependency decision in AD-0028.
Do not add an async runtime or hand-written unsafe syscall bindings. Standard
library file locks require Rust 1.89 and cannot satisfy Rust 1.85.

<a id="architecture-qualified-scope-locking-root-contract"></a>

#### Root contract

`filesystem::QualifiedScope::open(&InstallationRoot)` returns an opaque,
non-Clone pinned root or an error. Require a nonempty absolute existing
directory. Reject parent components, root symlinks, regular files, missing roots
and special files. Normalize redundant current-directory and trailing separators
before checking the final component, so they cannot hide a symlink. Preserve
non-UTF-8 root paths without lossy conversion where the filesystem accepts them;
propagate filesystem refusal contextually.

Ancestor symlinks are permitted during explicit ambient root selection,
including macOS temporary-directory aliases. Open both the normalized selected
path and its resolved directory with no final symlink following, and compare
both metadata identities with the selected root. Retain original normalized
spelling, resolved path and the resolved directory's open identity. This binds
the explicitly chosen root; it is not descendant containment or a security
sandbox against hostile namespace relocation.

`verify_binding()` reopens the selected root and checks its directory identity,
resolved path and nonsymlink final component. Renaming/replacing the root,
retargeting an ancestor alias or substituting a symlink must reject the old
scope. The pinned handle alone does not freeze the path namespace. Directory
edits by other processes remain possible, and later action code must revalidate
bindings and descendant preconditions immediately before each action.

<a id="architecture-qualified-scope-locking-writer-contract"></a>

#### Writer contract

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
lease, fairness, blocking wait, network-filesystem guarantee or protection
against external editors. If the filesystem refuses locking, return that error;
never pretend the lock succeeded.

Opening, validation, lock acquisition and release create no files, directories,
backups, journals or schema changes and preserve existing bytes and modes. They
do not establish state/source/destination preconditions or authorize writes.

<a id="architecture-qualified-scope-locking-acceptance-and-evidence"></a>

#### Acceptance and evidence

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

Historical execution results are retained in the
[filesystem validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/filesystem-validation.md#architecture-qualified-scope-locking-implementation-evidence).

<a id="architecture-qualified-destination-observation"></a>

<a id="architecture-qualified-destination-observation-qualified-destination-observation"></a>

### Qualified destination observation

Implemented with local APFS evidence and hosted Linux and Windows boundary
checks. Windows qualification remains pending.

<a id="architecture-qualified-destination-observation-intent-and-scope"></a>

#### Intent and scope

Capture existing file bytes and Unix modes, or verified absence, beneath one
qualified physical scope. Reject unsafe paths and potentially overlapping
destinations before a future executor receives observations. Retain opaque
evidence that can be revalidated while the caller retains its scope writer lock.
Preview may observe without acquiring that lock and performs no application
writes.

This is the next part of the M1 filesystem stage after AD-0028. It activates no
CLI behavior, ownership adoption, permission authority, schema-2 persistence,
source stabilization, file application or recovery. It does not establish that
the caller supplied the complete lifecycle destination union. A future lifecycle
orchestrator must derive that union and select exactly the affected observations
for the pure planner. Windows qualification remains pending.

<a id="architecture-qualified-destination-observation-alternatives-and-decision"></a>

#### Alternatives and decision

Select descriptor-relative traversal with no symlink following, retained
directory/file identities and a conservative batch overlap rule. Qualify APFS on
macOS and ext-family filesystems and tmpfs on Linux. Reject other filesystems,
cross-mount traversal and unknown qualification evidence explicitly. No new
dependency is required beyond the admitted rustix 1.1.4.

Ambient path canonicalization followed by ordinary reads is rejected: checking a
path does not bind subsequent accesses to that checked directory chain. Creating
temporary names to discover comparison behavior is rejected because preview must
not create files. Exact emulation of every filesystem's Unicode comparison is
also rejected: it adds version-dependent tables and still does not qualify an
unknown filesystem.

APFS preserves filename spelling while supporting normalization and case
insensitivity. ext4 can select Unicode case-insensitive lookup per directory.
See the
[Apple APFS FAQ](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/FAQ/FAQ.html)
and [Linux ext4 documentation](https://docs.kernel.org/admin-guide/ext4.html).
Use a conservative relation instead of claiming that byte inequality proves
distinctness. This intentionally rejects some valid case-sensitive or Unicode
batches. A later qualified comparison implementation may relax rejection without
changing the safety boundary.

<a id="architecture-qualified-destination-observation-public-interface"></a>

#### Public interface

[`QualifiedScope::observe_destinations`](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations.rs)
returns a `DestinationSnapshot` borrowing the scope. The snapshot exposes
`observations()` as a borrowed slice and `verify_unchanged()` as a fallible
revalidation operation.

Export `DestinationSnapshot` from `filesystem`. Reuse
`files::plan::FileLocation`, `ObservedFile` and `FileValue`. Existing regular
files produce exact bytes and `Some(mode & 0o7777)`; absent destinations produce
`None`. Sort observations by the existing exact location key: scope, raw root
OsStr and portable path. Permutation of locations must not change the returned
data or acceptance.

The snapshot borrows its qualified scope and privately owns retained evidence.
It is opaque, non-Clone and nonserializable, with redacted Debug. Returning or
cloning plain observations conveys no execution authority. There is no raw
descriptor, write method, lock acquisition or successful partial result.

Every location root must itself satisfy scope selection and remain bound to the
same physical directory as the supplied QualifiedScope. Different scope labels
and ancestor-alias spellings are permitted for distinct locations, but cannot
split physical identity or the batch overlap checks. Retain and revalidate the
selected spellings of additional roots. A different physical root is an error.
An empty input returns an empty snapshot only after validating the scope and its
filesystem profile.

<a id="architecture-qualified-destination-observation-path-and-filesystem-qualification"></a>

#### Path and filesystem qualification

Validate all portable relative paths using the ownership path rules before
accessing descendants. Reject empty, dot, parent, absolute, repeated-separator,
backslash, colon and control-character paths. Preserve native root paths without
lossy conversion. Error display may use the existing contextual presentation.

Qualify the pinned root with descriptor-based filesystem metadata. On Linux,
accept the ext-family magic or tmpfs magic and require a returned statx mount
ID. If the kernel cannot supply that evidence, return an explicit qualification
error. On macOS, require the APFS filesystem name and retain filesystem identity
and mount binding from fstatfs. Other operating systems remain unsupported.
Every opened descendant must stay on the selected mount: Linux mount IDs must
match, and macOS filesystem/mount identities must match. Comparing only device
numbers is insufficient for same-device bind mounts. Recheck these facts during
verification. Network, FUSE, overlay and other unqualified filesystems are not
implicitly accepted.

Walk one portable component at a time relative to a pinned directory descriptor.
Open each existing intermediate directory without following symlinks. Require
the requested component's exact bytes to appear in descriptor-relative directory
enumeration, so a case or normalization alias is rejected rather than silently
adopted. Compare the selected entry and opened directory identities. Retain the
opened directories to prevent inode reuse from imitating the original chain. Use
close-on-exec descriptors throughout.

Inspect the final entry without following symlinks before opening it; reject a
known nonregular type before any file open. An existing final component must
likewise have exact stored spelling, be opened without symlink following and be
a regular file with link count exactly one. Reject directories, symlinks, hard
links and special files. Nonblocking opening and type checks must prevent a FIFO
or device fixture from hanging observation. Compare entry and open identities,
qualify its mount and retain the file handle.

At the first missing component, retain the deepest existing directory and the
entire unresolved suffix as absence evidence. Do not create directories or probe
files. Only an actual not-found result establishes absence; permission, I/O,
unsupported and malformed-path errors remain errors. A non-directory ancestor
does not mean absence. Revalidate the existing chain and first missing component
before returning the snapshot.

<a id="architecture-qualified-destination-observation-conservative-overlap-rule"></a>

#### Conservative overlap rule

All locations resolve under one physical root. Reject exact duplicate locations
and every pair whose portable paths may denote the same destination or an
ancestor/descendant pair. Compare components in order up to the shorter path:

- Two ASCII components may match when their ASCII lowercase strings match.
- If either component contains a non-ASCII character, treat that component pair
  as potentially matching, regardless of its spelling.
- Otherwise the components are distinct and the two paths cannot overlap.

If every compared component may match, reject the pair. This catches case-only
collisions, Unicode normalization/case uncertainty, aliases across root
spellings or scope labels, and potential prefix conflicts below missing parents.
Apply the rule to the whole supplied set, including existing files. It is a
conservative rejection rule, not a claim that every rejected pair is
filesystem-equivalent. For example, `a/x` and `b/y` are distinct; `a/X` and
`a/x` conflict; `a/é` and `a/e` conflict; `a/é` and `b/e` are distinct. A single
Unicode path remains eligible when its existing components have exact stored
spellings.

Also reject duplicate opened file identities as a physical backstop. Existing
directory aliases introduced by mount traversal are excluded by mount checks. No
permission to remove or replace anything follows from successful observation.

<a id="architecture-qualified-destination-observation-capture-and-revalidation"></a>

#### Capture and revalidation

Capture the regular file through its retained handle. Require stable identity,
type, link count, size, permission bits, modification time and change time,
including fractional seconds, across the read, and require the read byte count
to match the sampled size. Do not use access time as a precondition: reads can
update it. Capture failure returns no snapshot. File data may be binary; never
decode or normalize it.

Before returning, revalidate all selected root bindings, every directory chain,
mount evidence and final entry identity or absence. Recheck captured file
metadata and exact bytes. `verify_unchanged()` repeats those checks against the
retained evidence and exact observations; it never refreshes the snapshot into a
new accepted state. Detect same-length byte edits even when modification time is
restored, timestamp-only changes, and edits with bytes and modification time
restored while change time differs. Also detect permission changes,
identical-byte inode replacement, ancestor replacement, missing-to-present
transitions and root-alias retargeting. Unrelated sibling edits do not
invalidate a snapshot merely by changing directory times.

These are sampled observations under the same namespace assumptions as AD-0028.
Pinned handles and revalidation do not freeze external writers, prevent hostile
ancestor relocation between checks or form an operating-system security sandbox.
A future executor must hold the writer guard, validate state/source authority
and permission policy, recheck destination evidence immediately before each
effect, and record interruptions. This API provides none of those effects or
recovery.

Recoverable filesystem, read and descriptor-exhaustion failures return
contextual errors; only not-found establishes absence. Allocation errors
reported by a fallible read also return contextual errors. The API does not
guarantee recoverable memory exhaustion: ordinary Rust infallible allocations,
including captured-byte cloning, may abort, and operating-system process
termination may prevent a return of `Err`. Retained handles close on drop and on
returned-error paths; process abort or termination does not guarantee Rust
destructors run. No production retries or silent fallback to ambient path reads
are allowed.

<a id="architecture-qualified-destination-observation-contract-and-acceptance-coverage"></a>

#### Contract and acceptance coverage

The requirements are recorded in
[qualified destination observations](https://github.com/driftsys/upskill/blob/765e70b9b7ce5386530ffea1fe9814b228724428/docs/specification/specification.md#48-qualified-destination-observations).
The
[public acceptance suite](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/tests/filesystem_destinations.rs)
covers capture, absence, candidate validation, unsafe entries, alias handling,
adversarial changes and opaque evidence. Focused private tests in the
[backend](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations/backend_tests.rs),
[qualification](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations/qualification.rs)
and
[candidate](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations/candidates.rs)
modules independently pin retained evidence and comparison rules. Filesystem
fixtures use temporary roots; privileged mount behavior remains outside the
exercised contract.

Historical execution results are retained in the
[filesystem validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/filesystem-validation.md#architecture-qualified-destination-observation-implementation-evidence-and-remaining-gates).

<a id="architecture-file-permission-policy"></a>

<a id="architecture-file-permission-policy-explicit-file-permission-selection"></a>

### Explicit file permission selection

This design describes the implemented inactive pure library API.

[AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md)
separates captured resource modes from generated output permissions. The
selected next prerequisite adds a pure selection interface before any qualified
file effects.

Existing ordinary permission bits are preserved exactly. Unknown existing modes
and existing special bits are errors. New generated output uses 0o0600. New
resources also use 0o0600; an explicit resource-only execution policy may select
0o0700 when captured evidence contains the owner-execute bit. Source group/other
and special bits are never copied. Supplied modes outside 0o0000..=0o7777 reject
even when selection would not use them.

This is a supplied-value policy, with no I/O or process-umask input. It grants
no ownership, containment, metadata-preservation or execution authority. The
separate
[qualified executor](0008-execution-and-recovery.md#architecture-qualified-file-execution)
enforces a restricted owner/group and metadata envelope before atomic
replacement. The compiler, planners and current CLI remain unchanged. Lifecycle
state persistence and activation remain pending.

<a id="architecture-file-permission-policy-public-interface-and-examples"></a>

#### Public interface and examples

`files::permissions::select_file_mode` accepts
`DestinationMode::{Absent,
Present(Option<u32>)}`,
`PermissionSource::{Generated, Resource { captured_mode:
Option<u32> }}` and
`ResourceExecutionPolicy::{Disabled, OwnerExecutable}` and returns
`anyhow::Result<u32>`. All three enums derive `Debug`, `Clone`, `Copy`,
`PartialEq` and `Eq`. The API has no serialized representation or effect
capability.

An existing `0o000`, `0o400`, `0o640`, `0o755` or `0o777` remains exact for
either source kind and either resource policy. Existing `0o1000`, `0o2000`,
`0o4000` and `0o2640` reject rather than losing special bits. Unknown existing
mode rejects.

For an absent resource with `OwnerExecutable`, captured `0o100`, `0o755` and
`0o4777` select `0o700`; captured `0o011`, `0o644`, `0o1000`, `0o2000` and
`0o4000` select `0o600`. Unknown captured mode rejects for that selection but is
allowed with `Disabled` or an existing ordinary destination mode. Out-of-range
captured evidence rejects even when an existing destination mode would otherwise
win. Contextual errors distinguish destination evidence, captured resource
evidence, unknown existing mode, existing special bits and unknown
executable-resource mode.

The
[implementation](https://github.com/driftsys/upskill/blob/main/src/files/permissions.rs)
selects from supplied values. The
[public acceptance tests](https://github.com/driftsys/upskill/blob/main/tests/file_permissions.rs)
exercise literal modes and diagnostics. Independent omission controls pin
preservation, unknown-mode handling, explicit policy, owner execution,
special-bit handling and both evidence ranges. No mutable input exists: all
inputs are `Copy` values and the function consults no filesystem, environment or
clock.

See
[specification 4.10](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#410-explicit-file-permission-selection)
and
[AD-0031](https://github.com/driftsys/upskill/blob/main/docs/adr/0031-select-explicit-file-permissions.md).
The opt-in executor applies selected bits only after qualification; hosted
executor qualification and lifecycle integration remain required. M1-A2, M1-A3
and M1 remain incomplete.
