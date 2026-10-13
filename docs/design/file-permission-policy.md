# Explicit file permission selection

Status: selected policy; implementation pending.

[AD-0026](../adr/0026-separate-generated-permission-policy.md) separates captured resource modes from generated output permissions. The selected next prerequisite adds a pure selection interface before any qualified file effects.

Existing ordinary permission bits are preserved exactly. Unknown existing modes and existing special bits are errors. New generated output uses 0o0600. New resources also use 0o0600; an explicit resource-only execution policy may select 0o0700 when captured evidence contains the owner-execute bit. Source group/other and special bits are never copied. Supplied modes outside 0o0000..=0o7777 reject even when selection would not use them.

This is a supplied-value policy, with no I/O or process-umask input. It grants no ownership, containment, metadata-preservation or execution authority. Owner/group, ACLs, extended metadata and atomic replacement still require separate qualified writer decisions. The compiler, planners and current CLI remain unchanged. File application, recovery, state persistence and lifecycle activation remain pending.
