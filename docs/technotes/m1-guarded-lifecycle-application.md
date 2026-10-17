# Guarded lifecycle application boundary

Validation record date: 2026-10-17.

The
[guarded application architecture](../topics/0008-execution-and-recovery.md#guarded-lifecycle-application)
records the implemented integration boundary.

Later schema-2 transitions include the retained original backup as a journaled
no-op action, so its exact bytes and mode are revalidated with the other
preconditions. The integration tests exercise successful first-write backup,
rejection of an existing mismatched backup without changing the lockfile, and
rejection of a changed retained backup before a later state transition. The
executor's process-level recovery tests exercise interruption at its file,
backup, state, and receipt checkpoints. No production command calls this
boundary. This slice does not yet provide the complete
add/update/remove/list/doctor adapters or source-resolution orchestration
required to complete Gate 5.
