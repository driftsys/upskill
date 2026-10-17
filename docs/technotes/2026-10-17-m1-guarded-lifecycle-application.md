# Guarded lifecycle application boundary

The opt-in `lifecycle::application` boundary binds a complete owned-artifact
plan to the combined state written by the same qualified execution request. It
requires schema-1 state evidence for the first schema-2 transition and requires
the resulting state's artifact records to equal the planner output. The request
is applied under the matching `QualifiedScope` writer guard. The existing
executor creates the private exact-byte backup before output actions and uses
its durable journal for state-last recovery.

`lifecycle::application::recover` delegates to the same qualified executor
recovery API under its matching writer guard. It does not perform inspection or
recovery without an explicit caller invocation.

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
