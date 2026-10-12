# AD-0029: Capture qualified destination evidence

Status: selected; implementation and hosted qualification pending.

## Context

AD-0028 pins one scope and coordinates cooperating writers. The pure mixed
planner still trusts supplied observations and exact lexical locations. Before
execution can use them, filesystem observation must bind reads to directory
identities and reject aliases and unsafe paths without making preview write.

Filename comparison differs across local filesystems and directory settings.
Two absent spellings cannot be proved distinct by inspecting only their absence.
Reproducing every versioned Unicode comparison table is outside the binary and
dependency budget.

## Decision

Implement the inactive snapshot boundary in the
[qualified destination design](../design/qualified-destination-observation.md).
Use descriptor-relative no-follow traversal, retained handles and exact
revalidation. Accept APFS and Linux ext-family/tmpfs profiles only when mount
evidence is available; reject cross-mount traversal and other profiles. Reuse
rustix 1.1.4 with no new dependencies or async runtime.

Conservatively reject candidate path pairs whose components may alias: ASCII
case variants and any differing non-ASCII component are treated as potentially
matching. Also reject potential ancestor/descendant pairs, stored-spelling aliases
and hard-linked final files. This deliberately rejects some otherwise valid
batches. The API describes rejection, not exact filesystem equivalence.

Return exact optional file values with an opaque, non-Clone snapshot borrowing
the qualified scope. Verification detects relevant identity/content/binding
changes and never refreshes stale evidence. The caller remains responsible for
complete lifecycle inputs, writer-lock lifetime and future effect checks.

## Consequences

Preview creates no probe files, directories or persistent state. Descriptor and
memory exhaustion remain errors. Linux requires statx mount-ID support; unknown
filesystems and Windows are explicit pending qualification boundaries. Retained
handles and sampled checks do not exclude external editors or provide a security
sandbox against hostile namespace relocation.

Source stabilization, permission policy, schema-2 persistence, apply/recovery and
CLI activation remain separate gates. Completion requires acceptance RED,
independent semantic controls, real APFS and hosted Linux evidence, unsupported
Windows checks, Rust 1.85, full repository verification and matched live-API size
measurements. No completion evidence exists yet.
