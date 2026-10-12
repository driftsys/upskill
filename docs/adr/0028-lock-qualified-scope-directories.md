# Lock qualified scope directories before owned lifecycle effects

AD-0028

The library boundary is implemented with local macOS evidence;
hosted Linux and Windows unsupported-boundary evidence remain pending.

## Context

Owned lifecycle writers need process exclusion on Rust 1.85 without preview
writes, stale sentinel cleanup or independently replaceable lock-file inodes.
Directory identity and scope-label identity are different: two aliases may select
one physical scope. Current CLI lifecycles remain inactive consumers of this API.

## Decision

Use opaque pinned directory identities and nonblocking exclusive directory
`flock` on Linux and macOS. Add the already locked `rustix` 1.1.4 as a direct
Linux/macOS dependency with filesystem support, retaining its safe synchronous
bindings. Open a fresh close-on-exec description for every guard and revalidate
the selected root before returning it. Unsupported platforms fail explicitly.

## Alternatives

Standard library file locking stabilized in Rust 1.89. A create-new sentinel
requires a crash and stale-owner protocol. A portable lock-file crate still
requires a stable contained lock namespace. Both add machinery that directory
locking avoids for these two qualified operating systems.

## Consequences

Locking creates no state directory or lock file. Aliases and Project/User labels
cannot split writer exclusion. Descriptor closure releases the lock after normal
exit or termination. Directory replacement invalidates the root binding; the
lock does not stop external namespace edits. Destination containment, complete
precondition validation, executor integration and Windows qualification remain
separate activation gates. No network filesystem or fork-safe lease is promised.

The implemented backend uses the exact direct dependency `rustix = 1.1.4`
with filesystem support, gated to Linux/macOS. The existing lock resolution is
unchanged apart from the direct edge. Local Rust 1.85 all-target compilation and
warning-denying clippy passed. Same-toolchain release CLI size stayed at 3,215,120
bytes; matched control/live probes measured 302,848/303,168 bytes, a 320-byte file
delta that includes root/lock/error machinery and linker padding. The durable
design records compiler, target, process controls and filesystem limits.

Complete admission evidence must include Rust 1.85 compilation, real-process contention,
normal and forced release, close-on-exec, local macOS and hosted Linux tests, and
matched release CLI plus live-API probe size measurements. Record evidence in the
durable scope-lock design before marking this decision implemented.

Related: [qualified scope-lock design](../design/qualified-scope-locking.md).
