# Qualified destination observation

Implemented with local APFS evidence and hosted Linux and Windows boundary
checks. Windows qualification remains pending.

## Intent and scope

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

## Alternatives and decision

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
See the [Apple APFS FAQ](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/FAQ/FAQ.html)
and [Linux ext4 documentation](https://docs.kernel.org/admin-guide/ext4.html).
Use a conservative relation instead of claiming that byte inequality proves
distinctness. This intentionally rejects some valid case-sensitive or Unicode
batches. A later qualified comparison implementation may relax rejection without
changing the safety boundary.

## Public interface

[`QualifiedScope::observe_destinations`](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations.rs)
returns a `DestinationSnapshot` borrowing the scope. The snapshot exposes
`observations()` as a borrowed slice and `verify_unchanged()` as a fallible
revalidation operation.

Export `DestinationSnapshot` from `filesystem`. Reuse `files::plan::FileLocation`,
`ObservedFile` and `FileValue`. Existing regular files produce exact bytes and
`Some(mode & 0o7777)`; absent destinations produce `None`. Sort observations by
the existing exact location key: scope, raw root OsStr and portable path.
Permutation of locations must not change the returned data or acceptance.

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

## Path and filesystem qualification

Validate all portable relative paths using the ownership path rules before
accessing descendants. Reject empty, dot, parent, absolute, repeated-separator,
backslash, colon and control-character paths. Preserve native root paths without
lossy conversion. Error display may use the existing contextual presentation.

Qualify the pinned root with descriptor-based filesystem metadata. On Linux,
accept the ext-family magic or tmpfs magic and require a returned statx mount ID.
If the kernel cannot supply that evidence, return an explicit qualification
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
opened directories to prevent inode reuse from imitating the original chain.
Use close-on-exec descriptors throughout.

Inspect the final entry without following symlinks before opening it; reject a
known nonregular type before any file open. An existing final component must
likewise have exact stored spelling, be opened
without symlink following and be a regular file with link count exactly one.
Reject directories, symlinks, hard links and special files. Nonblocking opening
and type checks must prevent a FIFO or device fixture from hanging observation.
Compare entry and open identities, qualify its mount and retain the file handle.

At the first missing component, retain the deepest existing directory and the
entire unresolved suffix as absence evidence. Do not create directories or probe
files. Only an actual not-found result establishes absence; permission, I/O,
unsupported and malformed-path errors remain errors. A non-directory ancestor
does not mean absence. Revalidate the existing chain and first missing component
before returning the snapshot.

## Conservative overlap rule

All locations resolve under one physical root. Reject exact duplicate locations
and every pair whose portable paths may denote the same destination or an
ancestor/descendant pair. Compare components in order up to the shorter path:

- Two ASCII components may match when their ASCII lowercase strings match.
- If either component contains a non-ASCII character, treat that component pair
  as potentially matching, regardless of its spelling.
- Otherwise the components are distinct and the two paths cannot overlap.

If every compared component may match, reject the pair. This catches case-only
collisions, Unicode normalization/case uncertainty, aliases across root spellings
or scope labels, and potential prefix conflicts below missing parents. Apply the
rule to the whole supplied set, including existing files. It is a conservative
rejection rule, not a claim that every rejected pair is filesystem-equivalent.
For example, `a/x` and `b/y` are distinct; `a/X` and `a/x` conflict;
`a/é` and `a/e` conflict; `a/é` and `b/e` are distinct. A single Unicode path
remains eligible when its existing components have exact stored spellings.

Also reject duplicate opened file identities as a physical backstop. Existing
directory aliases introduced by mount traversal are excluded by mount checks.
No permission to remove or replace anything follows from successful observation.

## Capture and revalidation

Capture the regular file through its retained handle. Require stable identity,
type, link count, size, permission bits, modification time and change time,
including fractional seconds, across the read, and require the read byte count to match the sampled size. Do not use
access time as a precondition: reads can update it. Capture failure returns no
snapshot. File data may be binary; never decode or normalize it.

Before returning, revalidate all selected root bindings, every directory chain,
mount evidence and final entry identity or absence. Recheck captured file
metadata and exact bytes. `verify_unchanged()` repeats those checks against the
retained evidence and exact observations; it never refreshes the snapshot into a
new accepted state. Detect same-length byte edits even when modification time is
restored, timestamp-only changes, and edits with bytes and modification time
restored while change time differs. Also detect permission changes, identical-byte
inode replacement, ancestor replacement, missing-to-present transitions and
root-alias retargeting. Unrelated
sibling edits do not invalidate a snapshot merely by changing directory times.

These are sampled observations under the same namespace assumptions as AD-0028.
Pinned handles and revalidation do not freeze external writers, prevent hostile
ancestor relocation between checks or form an operating-system security sandbox.
A future executor must hold the writer guard, validate state/source authority and
permission policy, recheck destination evidence immediately before each effect,
and record interruptions. This API provides none of those effects or recovery.

Recoverable filesystem, read and descriptor-exhaustion failures return contextual
errors; only not-found establishes absence. Allocation errors reported by a
fallible read also return contextual errors. The API does not guarantee
recoverable memory exhaustion: ordinary Rust infallible allocations, including
captured-byte cloning, may abort, and operating-system process termination may
prevent a return of `Err`. Retained handles close on drop and on returned-error
paths; process abort or termination does not guarantee Rust destructors run.
No production retries or silent fallback to ambient path reads are allowed.

## Contract and acceptance coverage

The requirements are recorded in
[specification §4.8](https://github.com/driftsys/upskill/blob/765e70b9b7ce5386530ffea1fe9814b228724428/docs/specification/specification.md#48-qualified-destination-observations).
The [public acceptance suite](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/tests/filesystem_destinations.rs) covers
capture, absence, candidate validation, unsafe entries, alias handling,
adversarial changes and opaque evidence. Focused private tests in the
[backend](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations/backend_tests.rs),
[qualification](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations/qualification.rs) and
[candidate](https://github.com/driftsys/upskill/blob/8abad7b57f079ae67c59816e09ddeaf44ff996b2/src/filesystem/destinations/candidates.rs) modules independently
pin retained evidence and comparison rules. Filesystem fixtures use temporary
roots; privileged mount behavior remains outside the exercised contract.

## Implementation evidence and remaining gates

The library boundary is implemented in `src/filesystem/destinations.rs`, with
portable candidate validation, descriptor-relative acquisition/revalidation and
filesystem qualification in separate private modules. Scope reopening reuses the
original selected-spelling, canonical-path and pinned-identity comparisons.

Local APFS acceptance covers D01-D09, including exact binary bytes and modes,
absence, root aliases, conservative overlap, stored spelling, FIFO/socket
rejection, adversarial changes and observation under an existing writer guard.
Private tests independently pin no-follow openers, exact-byte verification with
unchanged real-file stamps, first-missing-component insertion, profile/mount
validators, descriptor flags and drop/error cleanup. A small private acquisition
phase hook pins namespace replacement between capture and its before-return
postcheck. Linux metadata-validator examples run on macOS with synthetic fields;
they do not establish kernel statx or privileged mount behavior.

macOS FIFO fixtures use a bounded `/usr/bin/mkfifo` child because rustix does not
export `mkfifoat` there. Linux fixtures use rustix directly. Descriptor cleanup is
measured in a bounded isolated child, with a positive retained-handle count and
baseline restoration after both success/drop and failure. This checks read
handle ownership independently of writer-lock reacquisition. Native non-UTF-8
root support remains conditional: local APFS rejects the fixture with errno 92.

Hosted evidence on 2026-10-13 passed for PR head
`765e70b9b7ce5386530ffea1fe9814b228724428` in
[Actions run 38330080515](https://github.com/driftsys/upskill/actions/runs/38330080515).
The jobs checked out PR merge commit
`242930ef9107aff6eff432484f474e47157983f2`, combining that head with baseline
`8298386cb4978dce6515415f0fa7af59b109e5c7`. Linux passed 419 library cases,
including 14 private destination cases, 23 destination acceptance cases, 12 scope
cases and four rustdoc cases. This exercises real Linux filesystem qualification
with a returned statx mount ID. Windows passed 400 library cases, the explicit
unsupported-platform scope case and four rustdoc cases; its destination suite
contains zero cases. This establishes compilation and the unsupported boundary,
not a qualified Windows backend. Windows qualification remains pending.

No privileged mount creation, device-node creation, hostile namespace relocation,
real I/O or descriptor-exhaustion injection or exotic filesystem qualification is
claimed. Synthetic metadata and error-classification tests do not replace those
experiments. Acquisition-read stamp/size isolation and negative controls for
production qualification wiring are covered by the issue #270 controls below. The API remains
inactive in existing commands and schema 1.

Earlier local validation passed `just fmt`, full `just verify`, warning-denying
all-target Clippy and `cargo +1.85.0 check --locked --all-targets`. The later local
APFS review fixes pass 26 destination cases, 16 private destination cases and four
warning-denying rustdoc cases; these additional cases were not in the historical
hosted run. The existing scope suite has 12 cases. The new tests independently
pin mtime revalidation after neutralizing the competing ctime difference in
private evidence, ctime rejection after restoring exact bytes and mtime, and
capture of all permission bits using a fixture confirmed to retain mode 07640.
Independent omissions of mtime, ctime, both timestamps and special-bit capture
produce assertion failures, with exact production source bytes restored before
focused green gates. Separate compile-fail rustdoc cases pin literal
construction, Clone rejection and private observation storage. Sixteen initial
independent semantic controls produced
assertion failures, including omission of capture's before-return postcheck,
exact-byte comparison and its production call, directory identity, profile and
mount evidence, first-missing verification and each retained descriptor flag
class and exact location tuple ordering. Mutated source bytes were restored and SHA-256 hashes compared before the
final green gates. Review follow-up controls additionally pin the association
between each exact location and its optional bytes/mode, and the inability of an
external caller to mutate private observation storage. The restored acceptance
suite now passes 26 cases and warning-denying rustdoc passes four cases. Local
permission denial was observed as EACCES.

Matched stripped release measurements use Rust 1.97.1, LLVM 22.1.6 and
`aarch64-apple-darwin`, with the repository release profile and matching package
versions. CLI baseline `8298386` and candidate both measure 3,215,120 bytes;
the inactive API is removed from the CLI by dead-code elimination, so this does
not measure its active cost. Standalone probes create identical temporary binary
input. The control constructs only the path/data; the live probe opens a scope,
captures destinations, verifies them and consumes captured bytes. Both run
successfully on APFS. Control: 302,960 bytes; live: 336,848 bytes; measured
increment: 33,888 bytes. The live code section grows by 30,288 bytes. Data,
constants, alignment padding and Mach-O linking/signature metadata contribute
to the file-size result; this is not a runtime memory or performance estimate.

Local all-target cross-compilation also passed for `aarch64-unknown-linux-musl`
and `x86_64-pc-windows-gnu`, using Zig C wrappers for the existing ring dependency.
These compile checks are distinct from the dated hosted runtime evidence above.
Neither establishes a qualified Windows backend.

## Independent destination regression controls

Additional controls for issue #270 exercise the production acquisition and
qualification entries. A real file starts with an explicit nonzero nanosecond
mtime, then changes only that fractional value within the same whole second.
The fixture asserts exact filesystem retention and unchanged identity, bytes,
mode, links and size, and normalizes only retained ctime so verification must
reject the fractional mtime change independently.

A private read-to-end parameter keeps the opened descriptor and its before-read
stamp together. The production reader remains `File::read_to_end`. Deterministic
read fixtures change real descriptor metadata while returning the same bytes and
count, or return fewer bytes while all compared descriptor metadata remains
unchanged. Both must reject during acquisition before the snapshot postcheck.
Stable reads preserve exact bytes, and read errors preserve context and cause.
Omission controls remove the stamp check, byte-count check and acquisition call
wiring separately; each must fail its intended assertion.

The private `Mount::of` entry accepts only native descriptor-query functions;
production supplies fstatfs and, on Linux, statx with MNT_ID. Tests alter only the
native profile or Linux returned mount-ID mask and require this same entry to
reject, with qualified real metadata as a success control. A different expected
mount binding must reject through `Directory::new`, retained directory
verification and `Evidence::verify`, with other real evidence unchanged.
Independent controls omit the profile/mask checks, their entry wiring and mount
comparison wiring. Linux mask runtime controls require hosted Linux for the
current head; cross-compilation and synthetic validators on macOS cannot replace
that gate. Injected query results and expected bindings do not establish real
kernel cross-mount rejection or privileged mount behavior.

Public examples additionally reject both permutations of two existing,
individually valid files with exact stored spellings `a/é` and `a/e`, and require
two consecutive stale-verification errors without changing captured observations.
The existing-file fixture has an independent control that exempts only pairs
whose files both exist from overlap rejection. A fresh-observation substitution
must fail the stale-verification example. Local APFS passes 28 public acceptance
cases and 22 private destination cases. Fourteen independent native production
omissions produce the intended assertion failures. Each mutation is restored
byte-for-byte before focused green checks; restored full repository verification
passes without warnings. These results establish the local regression controls,
not completion of the hosted merge gates.

Current-head hosted Linux must execute the missing returned mount-ID mask
example and independently omit the native profile and mask checks and their
production call wiring, then restore exact source bytes and pass focused checks.
These Linux runtime and assertion controls remain pending. Privileged mounts,
device nodes and hostile relocation remain unexercised.
