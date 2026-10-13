# Destination evidence regression controls

Status: bounded follow-up implementation in progress on test/destination-evidence, based on merged714c4c3. This working record does not claim completed validation.

## Purpose

Complete issue #270 before lifecycle activation. The binding existing behavior and public interface remain in the durable qualified destination observation design and AD-0029. This follow-up supplies independent regression controls and accurate current successor status; it does not broaden filesystem support or activate the CLI.

## Requirements

1. Preserve full fractional mtime precision. Capture real descriptor evidence, change only mtime nanoseconds within the same second, confirm exact native timestamps, normalize only competing ctime, and require rejection.
2. Compare retained metadata before and after the actual acquisition read, independently of later snapshot verification. A deterministic metadata change during reading must reject even with unchanged bytes/count; a short read must reject with unchanged metadata. Stable and read-error controls must remain meaningful.
3. Reject unknown native profiles and missing returned Linux statx MNT_ID mask through the actual production qualification entry. Reject a changed expected mount binding through production Directory/Evidence comparison. Native query results may be injected through a narrow private closure; no separate test-only validator path.
4. Reject potentially overlapping existing regular destinations as well as absent ones. Two distinct exact stored spellings a/é and a/e must reject in both permutations.
5. Verification must never refresh expected evidence after rejection. A single mutation must cause two consecutive errors and leave exported observations unchanged.
6. Independently removing each production check or call wiring must cause an assertion failure, with exact source restoration and focused GREEN afterward. Compiler errors and unrelated failures do not establish a control.
7. Correct current successor-status paragraphs in captured-item-compilation, owned-file-planning, strict-source-manifests and specification. Distinguish implemented inactive destination/writer-lock boundaries from pending lifecycle, source, permissions, effects and recovery.

## Decisions and alternatives

Use only private read-to-end and exact native metadata query closures on production paths where deterministic coverage requires them. Broad filesystem mocking would obscure the actual descriptor code; global fault injection would interfere with concurrent tests. Real privileged mounts and device/error experiments remain outside authorization and cannot substitute for deterministic unit controls here. Synthetic query results establish rejection wiring only; they do not establish real cross-mount kernel behavior.

The private Mount::of entry takes a native query closure directly; production callers supply Mount::query. This avoids testing a validator wrapper that production could bypass. The read closure accepts the opened descriptor and byte buffer and returns std::io::Result<usize>; the before/read/after checks remain in the production acquisition path.

## Constraints and evidence

Rust edition2024, MSRV1.85, existing rustix1.1.4; no new dependencies, unsafe code, public hooks or interfaces. Retain opaque borrowed snapshot evidence, no-follow opens, exact bytes/modes and conservative qualification. No filesystem probe writes by observation, new CLI behavior, lifecycle authority, source stabilization, state persistence, effects or recovery.

Run real local APFS focused checks and full isolated just fmt/just verify. Linux-specific query/mask controls require hosted Linux runtime before merge; Windows remains the explicit unsupported boundary. Record exact heads, commands, assertion RED, restored GREEN and hashes. Preserve earlier raw archives and all unrelated WIP/worktrees. The controller records independent reviews and exact-head hosted gates.
