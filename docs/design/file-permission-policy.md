# Explicit file permission selection

This design describes the implemented inactive pure library API.

[AD-0026](https://github.com/driftsys/upskill/blob/main/docs/adr/0026-separate-generated-permission-policy.md) separates captured resource modes from generated output permissions. The selected next prerequisite adds a pure selection interface before any qualified file effects.

Existing ordinary permission bits are preserved exactly. Unknown existing modes and existing special bits are errors. New generated output uses 0o0600. New resources also use 0o0600; an explicit resource-only execution policy may select 0o0700 when captured evidence contains the owner-execute bit. Source group/other and special bits are never copied. Supplied modes outside 0o0000..=0o7777 reject even when selection would not use them.

This is a supplied-value policy, with no I/O or process-umask input. It grants no ownership, containment, metadata-preservation or execution authority. Owner/group, ACLs, extended metadata and atomic replacement still require separate qualified writer decisions. The compiler, planners and current CLI remain unchanged. File application, recovery, state persistence and lifecycle activation remain pending.

## Public interface and examples

`files::permissions::select_file_mode` accepts `DestinationMode::{Absent,
Present(Option<u32>)}`, `PermissionSource::{Generated, Resource { captured_mode:
Option<u32> }}` and `ResourceExecutionPolicy::{Disabled, OwnerExecutable}` and
returns `anyhow::Result<u32>`. All three enums derive `Debug`, `Clone`, `Copy`,
`PartialEq` and `Eq`. The API has no serialized representation or effect capability.

An existing `0o000`, `0o400`, `0o640`, `0o755` or `0o777` remains exact for either
source kind and either resource policy. Existing `0o1000`, `0o2000`, `0o4000` and
`0o2640` reject rather than losing special bits. Unknown existing mode rejects.

For an absent resource with `OwnerExecutable`, captured `0o100`, `0o755` and
`0o4777` select `0o700`; captured `0o011`, `0o644`, `0o1000`, `0o2000` and `0o4000`
select `0o600`. Unknown captured mode rejects for that selection but is allowed
with `Disabled` or an existing ordinary destination mode. Out-of-range captured
evidence rejects even when an existing destination mode would otherwise win.
Contextual errors distinguish destination evidence, captured resource evidence,
unknown existing mode, existing special bits and unknown executable-resource mode.

The [implementation](https://github.com/driftsys/upskill/blob/main/src/files/permissions.rs)
selects from supplied values. The [public acceptance tests](https://github.com/driftsys/upskill/blob/main/tests/file_permissions.rs) exercise literal modes
and diagnostics. Independent omission controls pin preservation, unknown-mode
handling, explicit policy, owner execution, special-bit handling and both evidence
ranges. No mutable input exists: all inputs are `Copy` values and the function
consults no filesystem, environment or clock.

See [specification 4.10](https://github.com/driftsys/upskill/blob/main/docs/specification/specification.md#410-explicit-file-permission-selection)
and [AD-0031](../adr/0031-select-explicit-file-permissions.md). Metadata and qualified
writer work remains required before applying selected bits; M1-A2, M1-A3 and M1
remain incomplete.
