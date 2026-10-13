# Select explicit file permissions before file effects

AD-0031

Status: selected; pure policy implementation pending.

## Context

Captured resource modes and generated entrypoints have different provenance. AD-0026 intentionally left permission application to a future qualified writer. Pure operation plans preserve exact optional modes but do not grant authority to apply them.

## Decision

Introduce a separate pure permission-selection interface. Preserve supported ordinary existing modes exactly; reject unknown existing modes and existing special bits. Select 0o0600 for newly generated files and resources. An explicit resource execution choice selects 0o0700 only from a known captured owner-execute bit. Never copy source special, group or other bits into a new file. Reject out-of-range supplied modes.

The selection is deterministic and independent of process umask. It does not modify compiler output or existing planners and has no filesystem effects. Future application must persist actual resulting mode evidence and separately qualify owner/group, ACLs, extended metadata and replacement operations.

## Alternatives and consequences

Copying source modes conflates evidence with authority. Reading umask makes a supplied-value policy depend on ambient process state. Default execution grants more authority than a resource-only explicit choice. Private output may need an explicit sharing policy later. Existing special modes fail conservatively until a qualified metadata policy exists.

Related: [AD-0026](0026-separate-generated-permission-policy.md), [permission policy](../design/file-permission-policy.md).
