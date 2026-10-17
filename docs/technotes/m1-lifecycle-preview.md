# M1 read-only lifecycle preview validation

This slice exposes opt-in captured preview for restrictive/shared installation
upserts and exact reference removals. It does not activate production commands,
fetch sources or qualify native discovery.

The public integration tests in `tests/lifecycle_preview.rs` cover complete
ordered actions at distinct locations with reversed observations; content, mode,
combined, create, remove and unchanged classifications; exact schema-1 bytes and
retained legacy/native rows; typed legacy selection blockers; semantically
invalid schema-2 state; edited output rejection; explicit shared owner release;
metadata-only reference edits; and semantic no-ops despite serialization or
record-order differences.

The physical temporary-root example snapshots state, source, output and extra
resource bytes plus the directory inventory before successful and failing
preview requests. It verifies preservation and no execution-storage or backup
creation. This proves observational behavior for supplied evidence; it does not
prove freshness against concurrent editors or establish a reusable apply lease.

Initial public acceptance compilation failed only because the preview API was
absent. After implementation, all ten preview examples and the matching 33
upsert and 18 retirement regressions passed locally. Full `just verify` passed
locally: 80 Rust suites reported 1,283 passing tests, no failures and one
existing ignored test. Clippy, formatting, lint, hooks, documentation checks and
the build passed. `just msrv-check` passed the locked all-targets check with
Rust 1.85.0. Independent review found no findings. The separate documentation
check passed 17 checker tests plus repository links, rendered book, publication
isolation, snapshots and compatibility inventory. These checks cover the local
slice based on `a74350e90b5b577cc4babb41e30dc1753be3547f`; they do not establish
merged or hosted qualification evidence.

Applying command adapters, source-resolution orchestration, complete lifecycle
acceptance and native qualification remain pending. Production schema is 1.
