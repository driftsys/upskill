# Select explicit file permissions before file effects

AD-0031

This decision is implemented as an inactive pure library API.

## Context

Captured resource modes and generated entrypoints have different provenance.
AD-0026 intentionally left permission application to a future qualified writer.
Pure operation plans preserve exact optional modes but do not grant authority to
apply them.

## Options

Copying source modes conflates evidence with authority. Reading umask makes a
supplied-value policy depend on ambient process state. Default execution grants
more authority than a resource-only explicit choice. A supplied-value interface
with private defaults preserves ordinary existing modes and bounds new resource
execution to an explicit choice backed by owner-execute evidence.

## Decision

Expose `files::permissions::select_file_mode` with `DestinationMode`,
`PermissionSource` and `ResourceExecutionPolicy` as a separate pure
permission-selection interface. Preserve supported ordinary existing modes
exactly; reject unknown existing modes and existing special bits. Select 0o0600
for newly generated files and resources. An explicit resource execution choice
selects 0o0700 only from a known captured owner-execute bit. Never copy source
special, group or other bits into a new file. Reject all out-of-range supplied
modes, including captured evidence unused by existing-mode preservation. Unknown
existing mode always rejects; unknown captured mode rejects when selecting
execution for a new resource. Captured special bits alone never grant owner
execution.

Public acceptance tests pin literal mode selections and contextual diagnostics.
Independent compiled omission controls pin validation and selection branches.

The selection is deterministic and independent of process umask. It does not
modify compiler output or existing planners and has no filesystem effects.
Future application must persist actual resulting mode evidence and separately
qualify owner/group, ACLs, extended metadata and replacement operations.

## Consequences

Private output may need an explicit sharing policy later. Existing special modes
fail conservatively until a qualified metadata policy exists.

Related:
[AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md),
[permission policy](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#architecture-file-permission-policy).

See
[specification 4.10](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#410-explicit-file-permission-selection).
[Qualified file effects and journal/recovery](0033-qualified-file-execution-and-journals.md)
are implemented as opt-in library capabilities. Combined lifecycle state,
activation and hosted qualification remain pending; M1-A2, M1-A3 and M1 remain
incomplete.

Satisfies:
[specification §4.10](https://github.com/driftsys/upskill/blob/main/docs/topics/0007-filesystem-access.md#410-explicit-file-permission-selection).
Related decisions: AD-0026.
