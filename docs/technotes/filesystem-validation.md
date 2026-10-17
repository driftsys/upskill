# Filesystem qualification validation

This informative record consolidates historical execution evidence previously
embedded in the scope, destination and source architecture records. Observation
dates, revisions, reported results and qualification limits remain those stated
below; this migration performs no new native qualification. Current contracts
and architecture live in
[filesystem access](../topics/0007-filesystem-access.md) and
[sources](../topics/0003-sources.md).

## Qualified scope locking

<a id="architecture-qualified-scope-locking-implementation-evidence"></a>

### Implementation evidence

Local evidence recorded on 2026-10-13 uses
`rustc 1.97.1 (8bab26f4f 2026-07-14)`, LLVM 22.1.6, target
`aarch64-apple-darwin`. Rust 1.85.0 `cargo check --locked --all-targets` passed.
The complete `cargo test --locked` suite passed at initial implementation
`eeabc81` (862 main-harness tests), as did
`cargo clippy --locked --all-targets -- -D warnings` and the rendered
documentation checker.

The initial real-filesystem integration binary passed all 10 tests covering
Q01-Q08, including exact recursive entries, bytes and Unix modes, alias and
label contention, independent roots, graceful and forced child release, root
replacement, ancestor retargeting and a live exec successor. Helpers run only
under explicit test environment variables with temporary HOME/USERPROFILE.
Readiness and exit polling have deadlines; cleanup kills and reaps unfinished
children. Tests serialize process setup because temporary pre-exec fork
inheritance can overlap another fixture's immediate release check. Close-on-exec
prevents retention after exec; the API does not promise a fork-safe lease.

One-at-a-time semantic controls produced assertion failures: omitting flock
failed same-process and subprocess contention, cloning the pinned description
failed same-process contention, omitting binding validation failed replacement
and ancestor-retargeting checks, and omitting close-on-exec failed reacquisition
while the exec successor remained alive. Restored focused and complete tests
passed. These controls are scratch evidence, not production toggles.

The local Data volume was verified as APFS with
`diskutil info
/System/Volumes/Data`. Native `root-0xff` directory creation was
refused before qualification with `EILSEQ`, errno 92, “Illegal byte sequence”.
The macOS test records that refusal and verifies contextual API failure without
effects. On filesystems accepting that native name, including the required Linux
fixture, the same test must pin, verify and lock it successfully. No lossy
conversion is used. Hosted Linux contention, release and native-path success
evidence and the Windows unsupported-boundary test remain pending actual CI.
Other filesystems and operating systems are not qualified by this local run.

<a id="architecture-qualified-scope-locking-review-follow-up-controls"></a>

#### Review follow-up controls

Subsequent review added two public integration examples and two focused unit
examples. A stale root with an existing guard must fail binding validation
before attempting the contended lock. A small private acquisition closure lets
the unit test replace the namespace after the initial check and then acquire a
real lock on the pinned directory; the final check must reject the guard and
release that lock. Only this unit test's final cleanup reacquisition tolerates
busy errors for up to two seconds: unrelated library tests can fork while a
descriptor is open. Other errors fail immediately, and a persistent descriptor
leak must fail at the deadline. Integration release assertions remain immediate
under serialized fixtures. The public method always passes the real backend
acquisition operation. There is no public test hook or new runtime policy.

A same-inode canonical relocation example renames an ancestor and retargets its
alias to the renamed directory, preserving the selected child's device/inode
while changing its canonical path. Binding verification and acquisition both
reject the old scope. A direct backend directory-open test rejects a final
symlink independently of selection-time metadata validation.

Independent mutations removing only the pre-lock check, only the post-lock
check, only the canonical-path comparison and only NOFOLLOW each produced
runtime assertion failures. A separate deliberate descriptor-leak control failed
at the cleanup deadline, proving the bounded test wait does not accept a
persistent leak. Restored focused runs passed 12 integration and two scope unit
tests. Updated all-target Rust 1.85 compilation and clippy passed. The
controller owns the next complete-suite gate after this focused review fix.

<a id="architecture-qualified-scope-locking-release-size"></a>

#### Release size

Baseline source is commit `a18a888`. Baseline and candidate used the compiler
and target above with the repository release profile (`opt-level = "z"`, LTO,
codegen-units 1, stripping and aborting panic). Two separate temporary probe
packages used that same profile and matched native-argument, path and Result
scaffolding: the control performs basic absolute-path/directory checks; the live
probe calls root qualification, writer locking and guard binding verification.
Both probes ran successfully against a temporary directory. Measurements were
refreshed after the private acquisition-helper refactor and remained unchanged.
Probe sources are not part of the shipped binary.

| Artifact              | Baseline/control bytes | Candidate/live bytes | Delta bytes |
| --------------------- | ---------------------: | -------------------: | ----------: |
| Release CLI           |              3,215,120 |            3,215,120 |           0 |
| Matched release probe |                302,848 |              303,168 |         320 |

The inactive CLI can eliminate this unused API, so its unchanged size alone is
not admission evidence. The live-probe file delta includes root, lock and error
machinery, as well as ordinary linker and Mach-O padding effects; it is not an
exact code-size estimate and does not establish future executor size. No Linux
or Windows size claim follows from these local macOS measurements.

<a id="architecture-qualified-scope-locking-references"></a>

### References

- [AD-0028](https://github.com/driftsys/upskill/blob/main/docs/adr/0028-lock-qualified-scope-directories.md).
- [Linux flock](https://man7.org/linux/man-pages/man2/flock.2.html).
- [Apple flock](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html).
- [Rust File lock stabilization](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).
- [rustix 1.1.4](https://docs.rs/crate/rustix/1.1.4).

## Qualified destination observation

<a id="architecture-qualified-destination-observation-implementation-evidence-and-remaining-gates"></a>

### Implementation evidence and remaining gates

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

macOS FIFO fixtures use a bounded `/usr/bin/mkfifo` child because rustix does
not export `mkfifoat` there. Linux fixtures use rustix directly. Descriptor
cleanup is measured in a bounded isolated child, with a positive retained-handle
count and baseline restoration after both success/drop and failure. This checks
read handle ownership independently of writer-lock reacquisition. Native
non-UTF-8 root support remains conditional: local APFS rejects the fixture with
errno 92.

Hosted evidence on 2026-10-13 passed for PR head
`765e70b9b7ce5386530ffea1fe9814b228724428` in
[Actions run 38330080515](https://github.com/driftsys/upskill/actions/runs/38330080515).
The jobs checked out PR merge commit `242930ef9107aff6eff432484f474e47157983f2`,
combining that head with baseline `8298386cb4978dce6515415f0fa7af59b109e5c7`.
Linux passed 419 library cases, including 14 private destination cases, 23
destination acceptance cases, 12 scope cases and four rustdoc cases. This
exercises real Linux filesystem qualification with a returned statx mount ID.
Windows passed 400 library cases, the explicit unsupported-platform scope case
and four rustdoc cases; its destination suite contains zero cases. This
establishes compilation and the unsupported boundary, not a qualified Windows
backend. Windows qualification remains pending.

No privileged mount creation, device-node creation, hostile namespace
relocation, real I/O or descriptor-exhaustion injection or exotic filesystem
qualification is claimed. Synthetic metadata and error-classification tests do
not replace those experiments. Acquisition-read stamp/size isolation and
negative controls for production qualification wiring are covered by the issue
#270 controls below. The API remains inactive in existing commands and schema 1.

Earlier local validation passed `just fmt`, full `just verify`, warning-denying
all-target Clippy and `cargo +1.85.0 check --locked --all-targets`. The later
local APFS review fixes pass 26 destination cases, 16 private destination cases
and four warning-denying rustdoc cases; these additional cases were not in the
historical hosted run. The existing scope suite has 12 cases. The new tests
independently pin mtime revalidation after neutralizing the competing ctime
difference in private evidence, ctime rejection after restoring exact bytes and
mtime, and capture of all permission bits using a fixture confirmed to retain
mode 07640. Independent omissions of mtime, ctime, both timestamps and
special-bit capture produce assertion failures, with exact production source
bytes restored before focused green gates. Separate compile-fail rustdoc cases
pin literal construction, Clone rejection and private observation storage.
Sixteen initial independent semantic controls produced assertion failures,
including omission of capture's before-return postcheck, exact-byte comparison
and its production call, directory identity, profile and mount evidence,
first-missing verification and each retained descriptor flag class and exact
location tuple ordering. Mutated source bytes were restored and SHA-256 hashes
compared before the final green gates. Review follow-up controls additionally
pin the association between each exact location and its optional bytes/mode, and
the inability of an external caller to mutate private observation storage. The
restored acceptance suite now passes 26 cases and warning-denying rustdoc passes
four cases. Local permission denial was observed as EACCES.

Matched stripped release measurements use Rust 1.97.1, LLVM 22.1.6 and
`aarch64-apple-darwin`, with the repository release profile and matching package
versions. CLI baseline `8298386` and candidate both measure 3,215,120 bytes; the
inactive API is removed from the CLI by dead-code elimination, so this does not
measure its active cost. Standalone probes create identical temporary binary
input. The control constructs only the path/data; the live probe opens a scope,
captures destinations, verifies them and consumes captured bytes. Both run
successfully on APFS. Control: 302,960 bytes; live: 336,848 bytes; measured
increment: 33,888 bytes. The live code section grows by 30,288 bytes. Data,
constants, alignment padding and Mach-O linking/signature metadata contribute to
the file-size result; this is not a runtime memory or performance estimate.

Local all-target cross-compilation also passed for `aarch64-unknown-linux-musl`
and `x86_64-pc-windows-gnu`, using Zig C wrappers for the existing ring
dependency. These compile checks are distinct from the dated hosted runtime
evidence above. Neither establishes a qualified Windows backend.

<a id="architecture-qualified-destination-observation-independent-destination-regression-controls"></a>

### Independent destination regression controls

Additional controls for issue #270 exercise the production acquisition and
qualification entries. A real file starts with an explicit nonzero nanosecond
mtime, then changes only that fractional value within the same whole second. The
fixture asserts exact filesystem retention and unchanged identity, bytes, mode,
links and size, and normalizes only retained ctime so verification must reject
the fractional mtime change independently.

A private read-to-end parameter keeps the opened descriptor and its before-read
stamp together. The production reader remains `File::read_to_end`. Deterministic
read fixtures change real descriptor metadata while returning the same bytes and
count, or return fewer bytes while all compared descriptor metadata remains
unchanged. Both must reject during acquisition before the snapshot postcheck.
Stable reads preserve exact bytes, and read errors preserve context and cause.
Omission controls remove the stamp check, byte-count check and acquisition call
wiring separately; each must fail its intended assertion.

The private `Mount::of` entry accepts only native descriptor-query functions;
production supplies `Mount::queries`, which calls fstatfs and, on Linux, statx
with MNT_ID. Tests alter only the native profile or Linux returned mount-ID mask
and require this same entry to reject, with qualified real metadata as a success
control. A different expected mount binding must reject through
`Directory::new`, retained directory verification and `Evidence::verify`, with
other real evidence unchanged. Independent controls omit the profile/mask
checks, their entry wiring and mount comparison wiring. Linux mask runtime
controls require native hosted Linux execution; cross-compilation and synthetic
validators on macOS cannot replace that gate. Injected query results and
expected bindings do not establish real kernel cross-mount rejection or
privileged mount behavior.

Public examples additionally reject both permutations of two existing,
individually valid files with exact stored spellings `a/é` and `a/e`, and
require two consecutive stale-verification errors without changing captured
observations. The existing-file fixture has an independent control that exempts
only pairs whose files both exist from overlap rejection. A fresh-observation
substitution must fail the stale-verification example. Local APFS passes 28
public acceptance cases and 22 private destination cases. Fourteen independent
native production omissions produce the intended assertion failures. Each
mutation is restored byte-for-byte before focused green checks; restored full
repository verification passes without warnings. These results establish the
local regression controls, with hosted evidence recorded separately below.

Hosted evidence on 2026-10-13 for PR #273 head
`2a50ca6a5163e11912cbcee7987593c68e0a4197` passed in
[Actions run 38334882688](https://github.com/driftsys/upskill/actions/runs/38334882688).
Ubuntu job `115092610337` passed 28 public destination cases, including native
qualification, and the missing returned mount-ID mask case. Four separate native
profile/mask check and production-call omissions produced their intended
assertion RED. After each omission, `qualification.rs` was restored to SHA-256
`1c5b288ed575ae1ccdd99a10170c13aa9054f92004a0927a64fb1587bac40bd5`; the final
restored private destination suite passed all 22 cases. All 13 head checks
succeeded, including Windows unsupported-boundary job `115092610428`. This dated
evidence covers that head. Removing the temporary Linux CI harness requires
hosted CI for the resulting final head before merge. Windows has no qualified
backend; privileged mounts, device nodes, real I/O or exhaustion faults and
hostile relocation remain unexercised.

The archived
[bounded spec](https://github.com/driftsys/upskill/blob/2a50ca6a5163e11912cbcee7987593c68e0a4197/docs/archive/specs/2026-10-13-destination-evidence-controls-design.md)
and
[plan](https://github.com/driftsys/upskill/blob/2a50ca6a5163e11912cbcee7987593c68e0a4197/docs/archive/plans/2026-10-13-destination-evidence-controls.md)
retain the provisional supplier name `Mount::query`. The implemented private
supplier is `Mount::queries`; the singular name was a sketch, not an additional
interface. The archive index records this as-built clarification while
preserving the raw working bytes.

## Qualified source observation

<a id="architecture-qualified-source-observation-local-implementation-evidence"></a>

### Local implementation evidence

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

<a id="architecture-qualified-source-observation-historical-matched-local-release-size-before-the-final-binding-fix"></a>

### Historical matched local release size before the final binding fix

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
