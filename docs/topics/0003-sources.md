# Sources and captured evidence

This concern owns the requirements and implemented architecture below. Moving
these records does not change their implementation or approval status.
Library-only capabilities remain separate from production CLI activation; each
section records its own limits. Accepted decisions remain in
[ADRs](https://github.com/driftsys/upskill/blob/main/docs/adr/README.md).

## Requirements

<a id="43-strict-captured-source-manifests"></a>

### Strict captured source manifests

The separate library module `source_manifest` provides
`capture_source(&Path) -> anyhow::Result<SourceManifest>` and pure
`validate_manifest(&SourceManifest) -> anyhow::Result<()>`. It does not change
commands, legacy source hashing/discovery, source formats or lockfile writes.
This supplies the captured values consumed by the implemented
[item compiler](0002-generation.md#45-captured-item-compilation) to derive
complete per-client entrypoint and resource outputs. The pure planner for
explicitly supplied whole-file sets is implemented in
[owned whole-file operation plans](0006-artifact-planning.md#44-owned-whole-file-operation-plans).
Captured-rule desired sets are implemented in
[captured rule desired sets](0002-generation.md#414-captured-rule-desired-sets);
lifecycle reconciliation remains pending.

`SourceManifest` contains sorted `CapturedSourceFile` records and a lowercase
`content_sha256`. Each file contains a UTF-8 relative `path` joined with `/`,
owned `bytes`, their lowercase `content_sha256`, and optional `mode`. Capture
includes every observed regular file without ignore rules or entrypoint
exclusions, including nested entrypoints, overrides, empty and binary files.
Empty directories have no record. Each file is read once; on Unix its observed
mode is `PermissionsExt::mode() & 0o7777`, and on other hosts mode is absent.
Mode evidence does not authorize applying permissions or executing content.

Capture reconstructs the root from `Path::components()` before metadata lookup,
so trailing separators and dot suffixes do not hide an observed root symlink. It
rejects observed root symlinks and non-directory roots, every observed entry
symlink and special entry, and enumeration, metadata or file-read errors. Errors
return no partial manifest. Relative paths reject empty, dot and parent
components, backslashes, colons, control characters and non-UTF-8 names.
Hard-linked regular files are represented by each path without a topology claim.

The manifest hash is SHA-256 over these exact bytes, in order:

1. ASCII `upskill:source-manifest:v1` followed by one NUL byte.
2. File count as an unsigned 64-bit big-endian integer.
3. For each file in Rust string path order: path byte length as unsigned 64-bit
   big-endian, UTF-8 path bytes, content byte length in the same encoding, exact
   content bytes, then mode-presence byte (`0` absent or `1` present). A present
   mode follows as an unsigned 32-bit big-endian integer.

Length conversion is checked. Per-file hash fields are verified but are not
additional digest inputs. Root spelling, timestamps and enumeration order are
excluded. Pure validation borrows the public values without mutation or sorting
and checks lexical paths, strict ascending uniqueness, optional mode range
`0..=0o7777`, exact file hashes and the aggregate hash. Public records and valid
hashes do not establish trusted provenance or source authenticity.

Standard synchronous filesystem observation is not an atomic snapshot,
descriptor-relative containment or protection against concurrent replacement.
Root ancestors may be symlinks. Lexical validation does not qualify destination
case collisions, reserved names, aliases or filesystem limits. The separate
selected-source boundary records immutable Git evidence; integration of
captured-rule desired sets
([captured rule desired sets](0002-generation.md#414-captured-rule-desired-sets))
with lifecycle state and execution, and Codex activation remain pending.
Qualified scope roots and writer locking
([qualified scope roots and advisory writer locking](0007-filesystem-access.md#47-qualified-scope-roots-and-advisory-writer-locking))
and the inactive destination observation boundary
([qualified destination observations](0007-filesystem-access.md#48-qualified-destination-observations))
are implemented separately. Qualified source snapshots
([qualified source tree observations](0003-sources.md#49-qualified-source-tree-observations))
add retained source evidence without changing this freely mutable value
interface.

See the
[source manifest design](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#architecture-strict-source-manifests)
and its decision records for implementation boundaries and rationale.

<a id="49-qualified-source-tree-observations"></a>

### Qualified source tree observations

The inactive library boundary exposes
`QualifiedScope::observe_source_tree(&self) -> anyhow::Result<SourceSnapshot<'_>>`.
An opaque, non-Clone snapshot borrows exactly one independently selected
qualified source scope. Its `manifest(&self) -> &SourceManifest` accessor
borrows plain captured values, and
`verify_unchanged(&self) -> anyhow::Result<()>` checks the original evidence
without refreshing it. There is no public constructor, mutable accessor,
serialization, write API or implicit writer lock. Debug output contains no
captured content. Unsupported hosts expose the type and return explicit errors.

Capture covers every descendant: hidden and ignored names, .git contents, binary
and empty files, and empty directories. Names must be UTF-8 and satisfy the
portable relative-path rules before recursion or reading. Hard-linked regular
source paths remain distinct manifest records; this does not change destination
single-link eligibility. Exact sorted directory membership, topology, identities
and stamps remain private evidence, including empty directories absent from the
manifest. Repeated directory identities, symlinks and special entries reject.

Every access is relative to a pinned directory descriptor and one exact stored
component. Inspection uses SYMLINK_NOFOLLOW; opens use NOFOLLOW and CLOEXEC,
DIRECTORY for directories and NONBLOCK for regular entries. Selected/opened
identity and type must agree. The shared native qualification policy applies to
the root and every opened descendant, and requires one unchanged source mount
binding. The source mount may differ from any destination mount.

File acquisition compares metadata before open, immediately before read and
after read, then requires exact byte count and nonnegative observed size to
agree. Stamps retain device/inode, complete native mode, link count, size and
nanosecond mtime/ctime; atime is ignored. Directory stamps must remain stable
across enumeration and descendant capture. Permission evidence is
`Some(mode & 0o7777)`. Canonical assembly shares the existing strict-manifest v1
hash framing and pure validation, preserving legacy capture signatures,
diagnostics and digests.

Before returning, capture performs full revalidation. Every later check verifies
retained descriptors, reopens the bound root, acquires a fresh complete tree and
compares membership, identities, stamps, exact bytes and modes with original
evidence. The fresh tree is discarded. Both checks after one mutation must
reject and leave the manifest unchanged. Root and ancestor-alias rebinding,
same-byte inode replacement, same-descendant directory replacement,
file/empty-directory addition or removal, rename, mode and fractional timestamp
changes reject. Binding is checked again after the walk.

This read-only sampled boundary changes no CLI, schema, native adapter, source
fetch, lifecycle state, permission policy, application or recovery behavior.
Recoverable failures retain operation/path context and original causes and
return no partial snapshot. Ordinary infallible allocation may abort; OS
termination is outside Result guarantees. Checks provide neither an atomic
snapshot nor exclusion against external editors, relocation or hostile ABA
changes. Native Linux/macOS qualification uses existing profiles; Windows
qualification, privileged mounts/devices and physical I/O or exhaustion faults
remain separate evidence limits. Final-head hosted Linux/Windows gates remain
required. See
[AD-0030](https://github.com/driftsys/upskill/blob/main/docs/adr/0030-capture-qualified-source-evidence.md)
and the
[source observation design](0003-sources.md#architecture-qualified-source-observation).

Local APFS source coverage at `a393263` included 12 public acceptance tests and
31 private tests, with independent regular-descendant profile/mount
qualification before reading, complete permission-bit mapping, held-writer
capture/revalidation and non-content Debug checks. Independent link-count and
ctime omission isolation remains a coverage limit; both fields participate in
production stamps.

Scoped revalidation diagnostic tests retain source-root/operation context,
complete relative paths in tree comparisons and original causes, including
repeated public rejection without refreshing the manifest. Public root and
ancestor binding failures retain source-root/verification context and the
original scope cause. Retained nested-file diagnostics are independently pinned
with unchanged parent stamps and membership. A final-binding control now rebinds
an ancestor only after fresh capture/comparison and independently pins the last
revalidation check. Empty-root and directory-only positive fixtures exercise
qualified capture and revalidation of valid empty manifests.

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

<a id="selected-lifecycle-rule-sources"></a>

### Selected lifecycle rule sources

The opt-in applying source prerequisite resolves one explicitly selected local
or Git rule item root and retains its qualified source identity through capture
and compilation. Captured source evidence remains bound to the resolver that
supplied its source label and actual Git commit. Compilation returns complete
desired artifacts and matching owner contracts for explicitly supplied
installation, consumer and target bindings. Branches, tags and full commit SHAs
record the resolved checkout commit rather than treating a requested ref as
immutable.

Resolution rejects final local symlinks and Git subfolder escapes. Private Git
metadata stays outside the captured item tree; inherited repository-local Git
overrides cannot redirect checkout writes. Authentication continues through
Git's native global/system configuration, credential helpers, SSH and askpass.
Revalidation checks the retained source tree and checkout revision without
refreshing evidence. The boundary performs no destination observation,
registration, output or backup writes. Preview uses supplied captured evidence
and does not call this resolver. The
[source-backed orchestration boundary](0005-lifecycle-planning.md#source-backed-lifecycle-orchestration)
now connects explicit rule roots to captured command preparation. Discovery,
Codex qualification and schema-2 production activation remain pending.

## Architecture

<a id="architecture-strict-source-manifests"></a>

<a id="architecture-strict-source-manifests-strict-captured-source-manifests"></a>

### Strict captured source manifests

This record describes the implemented
[`source_manifest`](https://github.com/driftsys/upskill/blob/main/src/source_manifest.rs)
library boundary.
[strict captured source manifests](#43-strict-captured-source-manifests) owns
its requirements. Capture observes source files; validation checks supplied
values without I/O. Neither function participates in current command dispatch.
[AD-0019](https://github.com/driftsys/upskill/blob/main/docs/adr/0019-separate-strict-source-capture.md)
records that separation.

<a id="architecture-strict-source-manifests-public-interface-and-evidence"></a>

#### Public interface and evidence

`capture_source` takes a borrowed filesystem path and returns an owned
`SourceManifest` through `anyhow::Result`. Its `files` vector contains
`CapturedSourceFile` values with a portable relative path, exact owned bytes,
lowercase SHA-256 of those bytes, and optional observed permission bits. The
manifest also carries its framed SHA-256. Public values derive debugging,
cloning and equality; they have no serialization or trusted-provenance wrapper.
Callers can alter them, so consumers must validate supplied values.

`validate_manifest` borrows the manifest without mutation or sorting. For each
record it checks the shared ownership lexical-path rule, strict ascending path
order and uniqueness, mode range, and exact byte digest, then checks the
complete manifest digest. Invalid input returns contextual errors rather than a
partial validated result. Ownership uses the same extracted crate-private path
predicate at its original validation position with unchanged diagnostics.

<a id="architecture-strict-source-manifests-observation-and-assembly"></a>

#### Observation and assembly

Capture reconstructs the root from standard path components before metadata
inspection. This lexical step removes redundant trailing separators and dot
suffixes without canonicalizing the root or resolving its ancestors. Root
metadata must identify an existing directory rather than an observed symlink.

A private synchronous `ObservationIo` boundary supplies directory entries,
symlink metadata and file reads. Directory enumeration preserves individual
entry results inside an outer enumeration result, so traversal can report both
directory-open and iterator errors. Metadata and read errors also propagate with
operation and path context; no failure returns a partial manifest. The private
fault delegate exercises these paths after an earlier successful file read.

Traversal rejects observed symlinks before following or reading them, validates
relative names before either directory recursion or regular-file capture, and
rejects other entry types before opening them. It applies no ignore rules or
entrypoint exclusions. Hidden files, gitignore-matched files, binary and empty
files, nested entrypoints and overrides are captured. Empty directories create
no records; invalid directory names still fail validation. Hard-linked regular
files produce a record for each path, without a topology guarantee.

Each regular file is read once. Unix permission evidence comes from its observed
metadata masked to `0o7777`; other hosts record no mode. These bits authorize no
permission application or execution. The complete vector sorts by Rust string
path ordering, receives its framed digest and passes pure validation before
return.
[AD-0020](https://github.com/driftsys/upskill/blob/main/docs/adr/0020-framed-source-manifest-evidence.md)
records the versioned digest choice; its exact encoding belongs to specification
[strict captured source manifests](0003-sources.md#43-strict-captured-source-manifests).

<a id="architecture-strict-source-manifests-integration-and-evidence-limits"></a>

#### Integration and evidence limits

Captured bytes let later planning avoid rereading mutable source paths. They do
not establish authenticity, an immutable Git revision or a single observation
instant. Ordinary synchronous standard-library APIs provide no atomic snapshot,
descriptor-relative containment or protection against concurrent replacement.
Root ancestors can be symlinks. Lexical path checks do not qualify aliases, case
collisions, reserved names or destination filesystem limits.

Legacy hashing and discovery retain their existing runtime behavior and
persisted digest meanings. No CLI, lockfile, source-schema or native-client
activation is introduced. The separate
[pure whole-file planner](0006-artifact-planning.md#44-owned-whole-file-operation-plans)
is implemented for explicitly supplied complete file sets. The separate
[captured item compiler](0002-generation.md#architecture-captured-item-compilation)
derives complete per-client entrypoint and resource outputs from these captured
values. The inactive
[captured-rule desired-set producer](0002-generation.md#architecture-captured-rule-desired-sets)
binds complete rule outputs. Lifecycle reconciliation and execution remain later
work. The separate
[selected-source boundary](#architecture-selected-lifecycle-rule-sources)
records immutable Git revision evidence. Qualified scope roots, writer locking,
[destination observation](0007-filesystem-access.md#architecture-qualified-destination-observation)
and [qualified source observation](#architecture-qualified-source-observation)
are implemented separately. Source snapshots retain revalidation evidence around
this unchanged plain manifest format. The
[mixed coordinator](0006-artifact-planning.md#architecture-mixed-owned-artifact-planning)
now composes complete explicit file/contribution sets from supplied ownership
and observations; it does not derive those lifecycle inputs from compiler
outputs.

[`tests/source_manifests.rs`](https://github.com/driftsys/upskill/blob/main/tests/source_manifests.rs)
pins literal file and manifest vectors, path/content/mode changes, complete
inclusion, validation diagnostics and supported platform rejection fixtures.
Private module tests pin fault propagation and read counts. Actual non-UTF-8
filesystem-name fixtures are limited to Unix excluding macOS: the development
macOS filesystem rejected their creation, so a private pure path-conversion
fixture covers rejection there. FIFO and socket tests do not claim device-entry
or concurrent-containment qualification. Platform-gated tests describe coverage,
not proof that each host has executed them.

<a id="architecture-strict-source-manifests-qualified-observation-companion"></a>

#### Qualified observation companion

The inactive
[qualified source boundary](#architecture-qualified-source-observation) adds
`SourceSnapshot`, borrowing one qualified scope and exposing this same manifest
only by shared reference. It retains complete directory topology and native
descriptor evidence for later non-refreshing checks. Its canonical assembly
shares the strict source digest implementation; the existing ambient
`capture_source` behavior and freely mutable plain values remain unchanged. This
implements source observation only. Hosted qualification, lifecycle effects,
permission policy, application and recovery remain separate gates.

<a id="architecture-qualified-source-observation"></a>

<a id="architecture-qualified-source-observation-qualified-source-tree-observation"></a>

### Qualified source tree observation

The inactive library boundary is implemented independently of command dispatch.
Hosted Linux qualification and the Windows scope/compile boundary passed at
`a393263` on 2026-10-13. Hosted checks for the eventual final PR head, including
the new Windows source-method runtime assertion, remain merge gates. Decision:
[AD-0030](https://github.com/driftsys/upskill/blob/main/docs/adr/0030-capture-qualified-source-evidence.md).

<a id="architecture-qualified-source-observation-intent"></a>

#### Intent

Complete the source-observation prerequisite of the delegated owned-content
lifecycle. Existing strict manifests capture exact bytes for pure compilation
but use ambient paths and contain freely mutable values. Introduce an inactive,
read-only filesystem capability that binds a complete captured tree to an
existing qualified scope and can reject later namespace or value changes.
Success is independently tested acquisition and immutable revalidation, with no
change to current command dispatch, persisted digests, or the legacy capture
API.

<a id="architecture-qualified-source-observation-selected-approach"></a>

#### Selected approach

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

<a id="architecture-qualified-source-observation-interface"></a>

#### Interface

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

<a id="architecture-qualified-source-observation-root-and-mount-binding"></a>

#### Root and mount binding

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

<a id="architecture-qualified-source-observation-complete-descriptor-relative-enumeration"></a>

#### Complete descriptor-relative enumeration

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

<a id="architecture-qualified-source-observation-stable-regular-file-acquisition"></a>

#### Stable regular-file acquisition

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

<a id="architecture-qualified-source-observation-revalidation"></a>

#### Revalidation

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

<a id="architecture-qualified-source-observation-components-and-verification"></a>

#### Components and verification

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

<a id="architecture-qualified-source-observation-limits"></a>

#### Limits

This is a sampled read-only source stabilization prerequisite. It does not
record an immutable Git revision, establish authenticity, lock external source
writers, activate CLI/schema-2, write migration backup/state, establish
permission policy, apply files, implement recovery or complete M1-A2/M1-A3/M1.
Real privileged mounts, device nodes, physical I/O/exhaustion failures and
hostile namespace behavior are unexercised unless separately evidenced. Durable
status must remain honest.

Historical execution results are retained in the
[filesystem validation record](https://github.com/driftsys/upskill/blob/main/docs/technotes/filesystem-validation.md#architecture-qualified-source-observation-local-implementation-evidence).

<a id="architecture-selected-lifecycle-rule-sources"></a>

### Applying selected lifecycle rule sources

The opt-in Gate 5 library resolves explicitly selected local or Git rule item
roots through `lifecycle::sources::ResolvedRuleSource`. It retains a qualified
source scope and any private temporary checkout. This boundary performs no
registration, destination observation, output writes or backup writes. Read-only
preview does not use it. Registry discovery remains separate; the
[source-backed adapter](0005-lifecycle-planning.md#source-backed-lifecycle-orchestration)
now supplies complete literal selection before resolution.

`capture()` returns a private association between strict captured bytes and the
resolved source metadata. `CapturedRuleSource::compile` accepts an explicit
installation binding, compiler consumers and target profiles, and produces a
complete desired set with matching owner contracts. Requested branches, tags and
full commit SHAs become the actual immutable checkout commit in installation
evidence. Supported compiler families remain Claude, Copilot and OpenCode; Codex
compiler surfaces are rejected until Gate 6.

The retained `snapshot()` can be supplied to later executor revalidation.
`verify_unchanged()` samples the original source tree and checkout revision
without refreshing evidence. These checks do not provide an atomic filesystem
snapshot or exclusion against external writers. Local final symlinks and Git
subfolder escapes are rejected. Git metadata is kept outside the captured work
tree, and the temporary checkout lives until the resolver is dropped.

Implementation evidence and the separate permission-selection requirement are
recorded in the
[selected source validation note](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate5-sources-validation.md).

#### Resolution and retained evidence

Local resolution makes relative paths absolute and canonicalizes only their
ancestors, leaving the selected final component for qualified symlink rejection.
Git resolution uses synchronous init, shallow fetch and detached checkout with
structured command arguments. A private temporary directory owns separate `tree`
and `metadata` directories. Explicit subfolders must contain only normal
relative components and remain inside the checkout after canonicalization. The
resolver obtains the immutable revision through `HEAD^{commit}` and retains the
temporary directory until it is dropped.

Each Git subprocess removes the repository-local override set enumerated by
`git rev-parse --local-env-vars`, including index, object, common-directory and
repository-scoped config variables. A private coverage test compares that set
with the installed Git. Native authentication configuration remains available;
the resolver does not inject credentials.

`CapturedRuleSource` borrows its resolver and opaque qualified source snapshot.
Capture checks the Git revision before and after observing the source tree;
revalidation checks it around snapshot verification. Compilation revalidates,
binds the original manifest hash and resolved commit to the installation, then
calls the existing contracted desired-set producer. The borrowed `snapshot()`
accessor supplies source evidence to a later executor without transferring
ownership or extending the checkout lifetime.

Permission evidence is not executor policy. Generated rule entrypoints and their
contracts carry `mode: None`; resources retain observed permission bits. A later
applying adapter must select modes through
`files::permissions::select_file_mode` using qualified destination evidence and
explicit resource execution policy, and keep the desired artifacts and owner
contracts consistent before planning.

The implementation lives in
[`src/lifecycle/sources.rs`](https://github.com/driftsys/upskill/blob/main/src/lifecycle/sources.rs),
with public acceptance coverage in
[`tests/lifecycle_sources.rs`](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_sources.rs).

#### Requested bindings and scoped application

`normalized_source_label` derives a requested source binding without fetching or
capturing the tree. Local labels canonicalize ancestors and preserve the final
selected entry for qualification; Git labels retain their requested ref. The
resolved commit remains separate evidence. The orchestration adapter uses this
label to reject reuse of an installed alias by a different binding before source
resolution.

The source-backed adapter retains every resolver, temporary checkout and full
`CapturedRuleSource` through a scoped prepared-operation callback. Immediately
before consuming the captured command operation, it verifies the original tree
and actual Git revision for every capture. The opaque preparation accepts no
replacement source or compiled output. This connects compiler-produced rule sets
to existing execution without changing preview or production dispatch.
