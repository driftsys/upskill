# Qualified destination observation

Status: implemented with local APFS evidence; hosted Linux and Windows checks
remain pending.

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

```rust
impl QualifiedScope {
    pub fn observe_destinations<'scope>(
        &'scope self,
        locations: &[FileLocation],
    ) -> anyhow::Result<DestinationSnapshot<'scope>>;
}

impl DestinationSnapshot<'_> {
    pub fn observations(&self) -> &[ObservedFile];
    pub fn verify_unchanged(&self) -> anyhow::Result<()>;
}
```

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
type, link count, size, permission bits, modification time and change time across
the read, and require the read byte count to match the sampled size. Do not use
access time as a precondition: reads can update it. Capture failure returns no
snapshot. File data may be binary; never decode or normalize it.

Before returning, revalidate all selected root bindings, every directory chain,
mount evidence and final entry identity or absence. Recheck captured file
metadata and exact bytes. `verify_unchanged()` repeats those checks against the
retained evidence and exact observations; it never refreshes the snapshot into a
new accepted state. Detect same-length byte edits even when modification time is
restored, permission changes, identical-byte inode replacement, ancestor
replacement, missing-to-present transitions and root-alias retargeting. Unrelated
sibling edits do not invalidate a snapshot merely by changing directory times.

These are sampled observations under the same namespace assumptions as AD-0028.
Pinned handles and revalidation do not freeze external writers, prevent hostile
ancestor relocation between checks or form an operating-system security sandbox.
A future executor must hold the writer guard, validate state/source authority and
permission policy, recheck destination evidence immediately before each effect,
and record interruptions. This API provides none of those effects or recovery.

Resource exhaustion and every non-not-found filesystem failure remain contextual
errors. Retained handles close on drop and on every error path. No production
retries or silent fallback to ambient path reads are allowed.

## Acceptance and evidence

| ID  | Required example                                                                                                                                                                        |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D01 | Capture exact binary bytes and Unix mode; immutable borrowed observations; entries, bytes and permission modes remain unchanged.                                                        |
| D02 | Capture absent leaf and absent intermediate parents without creating anything; verify absence; empty input validates scope/profile.                                                     |
| D03 | Reject every invalid relative path and a different physical root; allow distinct locations through equivalent roots and labels, preserving exact observation locations.                 |
| D04 | Reject final and intermediate symlinks, file ancestors, directory leaves, hard links and special files without escaping or hanging.                                                     |
| D05 | Reject duplicate, ASCII case-only, Unicode-uncertain and prefix pairs, including absent parents and root aliases; accept provably distinct examples and permutation-equivalent input.   |
| D06 | Reject existing case/normalization aliases where the filesystem resolves them; preserve a single exact Unicode spelling and native root paths where supported.                          |
| D07 | Reject byte edits with restored mtime, mode edits, identical-byte inode replacement, ancestor replacement, absence changes and root-alias retargeting; allow unrelated sibling changes. |
| D08 | Unsupported filesystem/profile/mount evidence and non-not-found failures stay errors; no partial result, leaked handles or application writes on failure.                               |
| D09 | Opaque/non-Clone API and close-on-exec evidence are independently pinned; existing commands and schema 1 stay unchanged.                                                                |

Start with failing acceptance tests before implementation. Demonstrate separate
assertion failures for no-follow traversal, overlap omission, exact-byte
revalidation, missing-to-present verification, mount/profile rejection and
descriptor close-on-exec. Use real temporary files for ordinary behavior and
small private deterministic seams only where an otherwise unrepeatable check
needs independent coverage. Do not replace the filesystem with a general mock.
Tests must not change process-global cwd, touch developer HOME, mount filesystems
or require elevated privileges. Any unexercised privileged mount behavior remains
explicitly unswept rather than inferred from synthetic metadata.

Run full repository verification, Rust 1.85 all-target checks, real local APFS
tests and hosted Linux and Windows checks. Linux runtime qualification must be
observed on the hosted filesystem, including required mount-ID availability.
Windows checks establish the explicit unsupported/compilation boundary only.
Measure matched release CLI and live snapshot/verification probe sizes using the
same toolchain and baseline. Record exact scope, bytes and remaining limits.

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

Hosted Linux runtime qualification, including a returned statx mount ID, and
hosted Windows compilation/unsupported-boundary checks remain pending. No
privileged mount creation, device-node creation, hostile namespace relocation,
real I/O/resource-exhaustion injection or exotic filesystem qualification is
claimed. Synthetic metadata and error-classification tests do not replace those
experiments. The API remains inactive in existing commands and schema 1.

Local validation passed `just fmt`, full `just verify`, warning-denying
all-target Clippy and `cargo +1.85.0 check --locked --all-targets`. The focused
APFS suites pass 22 destination cases, 15 private destination cases and the
existing 12 scope cases. Separate compile-fail rustdoc cases pin literal
construction and Clone rejection. Sixteen independent semantic controls produced
assertion failures, including omission of capture's before-return postcheck,
exact-byte comparison and its production call, directory identity, profile and
mount evidence, first-missing verification and each retained descriptor flag
class and exact location tuple ordering. Mutated source bytes were restored and SHA-256 hashes compared before the
final green gates. Local permission denial was observed as EACCES.

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
These compile checks do not establish hosted execution, Linux mount-ID
availability or Windows runtime behavior.
