# Gate 5 final acceptance

Use the opt-in source orchestration and captured adapters intended for Gate 6.
Production commands retain schema 1. No native qualification is implied.

1. Add process acceptance using the executor's test-only checkpoint hook.
2. Record actual checkpoint events from a successful operation; interrupt a new
   process at each recorded boundary and recover through lifecycle
   orchestration.
3. Compare exact file bytes and modes, including combined state and migration
   backup; reject a concurrent writer and verify writer release after
   termination.
4. Exercise fresh and migrated add, update and remove. Change sources after
   interruption to prove recovery uses captured intent.
5. Map all ten criteria to concrete tests and report residual acceptance limits.
6. Run focused acceptance, formatting, lint and MSRV; leave hosted CI,
   independent final review and merge to the coordinating agent.

Decisions: process termination is tested, not hardware power loss. The ignored
probe is a unit-test subprocess entrypoint, never a production flag. Event
indices are discovered rather than inferred from output ordering. Existing
executor tests remain complementary protocol evidence.
