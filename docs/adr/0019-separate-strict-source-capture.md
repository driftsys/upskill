# Separate strict source capture from legacy discovery

AD-0019

## Context

Future owned file and resource planning needs captured bytes and explicit
failure when source observation is incomplete. Existing best-effort discovery
and hashing can skip errors and already define persisted legacy digest meanings.

## Options

Change the legacy helpers to be strict, provide only a pure manifest model, or
add a separate strict observer together with pure validation. Changing helpers
would alter existing installations and hashes. A model alone cannot establish
which files were read or expose skipped observation failures.

## Decision

Provide an opt-in library capture boundary and pure validator. Capture owns each
regular file's bytes and rejects observed symlinks, special entries and
traversal or read errors instead of silently omitting them. Preserve legacy
behavior and leave current command dispatch unchanged.

## Consequences

Later planners can consume exact captured bytes without rereading source paths.
Memory use grows with captured content. Sources containing unsupported entries
fail capture until their content or an explicit policy changes. Successful
capture does not establish authenticity, an atomic snapshot or qualified
containment; stabilization and qualified observation remain activation gates.

Satisfies:
[Specification §4.3](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#43-strict-captured-source-manifests).
Related:
[source manifest design](https://github.com/driftsys/upskill/blob/main/docs/topics/0003-sources.md#architecture-strict-source-manifests).
Rationale:
[archived spec](../archive/specs/2026-10-12-strict-source-manifests-design.md).
