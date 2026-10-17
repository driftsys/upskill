# AD-0029: Capture qualified destination evidence

Implemented with local APFS and hosted Linux evidence, plus hosted Windows
unsupported-boundary checks. Windows qualification remains pending.

## Context

AD-0028 pins one scope and coordinates cooperating writers. The pure mixed
planner still trusts supplied observations and exact lexical locations. Before
execution can use them, filesystem observation must bind reads to directory
identities and reject aliases and unsafe paths without making preview write.

Filename comparison differs across local filesystems and directory settings. Two
absent spellings cannot be proved distinct by inspecting only their absence.
Reproducing every versioned Unicode comparison table is outside the binary and
dependency budget.

## Options

Ambient path canonicalization followed by ordinary reads does not bind later
accesses to the inspected directory chain. Creating temporary comparison probes
would violate preview's no-write contract. Exact emulation of every filesystem's
Unicode comparison would require version-dependent tables without qualifying
unknown filesystems. Descriptor-relative traversal with retained identities and
conservative overlap rejection preserves the selected boundary within the
existing dependency budget.

## Decision

Implement the inactive snapshot boundary in the
[qualified destination design](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#architecture-qualified-destination-observation).
Use descriptor-relative no-follow traversal, retained handles and exact
revalidation. Accept APFS and Linux ext-family/tmpfs profiles only when mount
evidence is available; reject cross-mount traversal and other profiles. Reuse
rustix 1.1.4 with no new dependencies or async runtime.

Conservatively reject candidate path pairs whose components may alias: ASCII
case variants and any differing non-ASCII component are treated as potentially
matching. Also reject potential ancestor/descendant pairs, stored-spelling
aliases and hard-linked final files. This deliberately rejects some otherwise
valid batches. The API describes rejection, not exact filesystem equivalence.

Return exact optional file values with an opaque, non-Clone snapshot borrowing
the qualified scope. Verification detects relevant identity/content/binding
changes and never refreshes stale evidence. The caller remains responsible for
complete lifecycle inputs, writer-lock lifetime and future effect checks.

## Consequences

Preview creates no probe files, directories or persistent state. Recoverable
filesystem, read and descriptor-exhaustion failures return contextual errors,
including allocation errors reported by a fallible read. This is not a guarantee
of recoverable memory exhaustion: ordinary Rust infallible allocations,
including captured-byte cloning, may abort, and the operating system may
terminate the process instead of returning `Err`. Linux requires statx mount-ID
support; unknown filesystems and Windows are explicit pending qualification
boundaries. Retained handles and sampled checks do not exclude external editors
or provide a security sandbox against hostile namespace relocation.

Source stabilization, permission policy, schema-2 persistence, apply/recovery
and CLI activation remain separate gates. Completion requires acceptance RED,
independent semantic controls, real APFS and hosted Linux evidence, unsupported
Windows checks, Rust 1.85, full repository verification and matched live-API
size measurements. Local APFS acceptance, private evidence tests and independent
semantic controls now cover the implemented boundary. Linux profile, absent
mount-ID mask and same-device/different-mount-ID validators are tested
separately with synthetic metadata. On 2026-10-13,
[Actions run 38330080515](https://github.com/driftsys/upskill/actions/runs/38330080515)
passed Linux runtime qualification with returned statx mount IDs and Windows
compilation/unsupported-boundary checks for PR head
`765e70b9b7ce5386530ffea1fe9814b228724428` (merge checkout
`242930ef9107aff6eff432484f474e47157983f2`). This is historical evidence for
that head; later local timestamp and special-bit assertion controls are
separate. A qualified Windows backend remains pending. Privileged mounts, device
nodes and hostile namespace relocation remain unexercised. The issue #270
regression implementation now independently pins fractional mtime,
acquisition-read stamp/size and call wiring, native profile rejection, retained
mount comparison, existing-file overlap and repeated stale verification. Local
APFS passes 28 public and 22 private destination cases; 14 independent
production omissions produce assertion failures, with exact source restoration
and focused green checks. Restored full repository verification passes without
warnings. On 2026-10-13,
[Actions run 38334882688](https://github.com/driftsys/upskill/actions/runs/38334882688)
passed for PR #273 head `2a50ca6a5163e11912cbcee7987593c68e0a4197`. Ubuntu job
`115092610337` passed 28 public destination cases and the native missing
returned mount-ID mask case. Four profile/mask check and production-call
omissions failed their intended assertions; each restored `qualification.rs` to
SHA-256 `1c5b288ed575ae1ccdd99a10170c13aa9054f92004a0927a64fb1587bac40bd5`,
followed by 22 restored private cases GREEN. All 13 head checks succeeded,
including Windows unsupported-boundary job `115092610428`. This dated evidence
covers that head; hosted CI for the final head after temporary harness removal
remains required before merge. It establishes neither a qualified Windows
backend nor real privileged mount or fault behavior. See the durable design for
evidence scope, counts and limits.

Local full verification, warning-denying all-target Clippy and Rust 1.85
all-target checks passed. Matching Rust 1.97.1 macOS release probes measure
33,888 additional bytes when snapshot capture and verification are live; the
inactive CLI remains 3,215,120 bytes at both baseline `8298386` and candidate.
This measures local binary size; it does not measure runtime cost.

Satisfies:
[specification §4.8](https://github.com/driftsys/upskill/blob/765e70b9b7ce5386530ffea1fe9814b228724428/docs/specification/specification.md#48-qualified-destination-observations).
Related: [AD-0028](0028-lock-qualified-scope-directories.md).
