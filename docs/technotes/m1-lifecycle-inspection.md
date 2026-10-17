# M1 read-only lifecycle inspection validation

This slice adds opt-in captured list and doctor descriptions and qualified
read-only scope inspection. It does not activate production commands.

[Public acceptance examples](https://github.com/driftsys/upskill/blob/main/tests/lifecycle_inspection.rs)
cover exact schema-1 byte retention, unknown fields, duplicate legacy rows,
invalid/unsupported schemas, validated schema-2 records, missing evidence versus
missing outputs, content and mode drift, unknown modes, duplicate observations,
and manual Markdown preservation versus owned-region edits. A healthy
multi-contribution aggregate remains healthy; a group-level aggregate failure
diagnoses each recorded contribution at that location. Qualified Linux/macOS
cases check that inspection preserves state and output bytes and creates no
backup, lock or execution storage. They also cover inspection while a writer
guard is already held and rejection of foreign artifact roots without writes.

The executor unit test
`pending_inspection_is_observational_for_absent_and_interrupted_storage` checks
absent storage, validated journal presence and corrupt-journal rejection without
changing journal bytes or applying output/state actions.

These tests do not prove an atomic view across external writers, source
freshness, native client discovery, hardware power-loss recovery, or complete
Gate 5 command adapters. Production schema remains 1.

The
[durable inspection contract](../topics/0005-lifecycle-planning.md#read-only-lifecycle-inspection)
is reconciled against the local implementation based on `f61afbf`.
`cargo test --test lifecycle_inspection` passed all ten integration examples;
the focused executor unit test and `just msrv-check` passed. The healthy
multi-contribution regression failed before the correction and passed afterward.
Full `RUSTC_WRAPPER= just verify` passed outside the sandbox, including tests,
lint, hooks, documentation checks and build. Initial sandbox process tests could
not acquire the required permissions; the complete outside-sandbox rerun passed.
These are local checks, not hosted or merged qualification.
