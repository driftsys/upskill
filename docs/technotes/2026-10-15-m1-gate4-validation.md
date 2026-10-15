# M1 gate 4 validation evidence — 2026-10-15

## Tested revisions and limits

The original executor candidate is commit
`c4ce337d67fdefea69a8e8388d6c28e23db4cb7f`, based on
`00d4c2c5fe4184f63334464304a0e6bd009cd76e`. It passed 1027 tests with one
ignored subprocess entrypoint, `just verify`, all-target Clippy and the Rust
1.85 all-target check. A fresh independent review found three issues; their
regression fixes are included in that commit.

The subsequent quick-review fixes were tested on the source contents identified
by the SHA-256 manifest below. This record identifies contents directly because
the validation preceded its commit. Tests ran locally on macOS; hosted
Linux/macOS qualification is still pending. These checks do not establish
hardware power-loss durability or complete M1.

## Final quick-review fix checks

| Command                                      | Observed result                                                                                   |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `just fmt`                                   | Exit 0                                                                                            |
| `just verify`                                | Exit 0; 1034 tests passed, 1 ignored subprocess entrypoint; documentation checks and build passed |
| `cargo clippy --all-targets -- -D warnings`  | Exit 0                                                                                            |
| `cargo +1.85.0 check --all-targets --locked` | Exit 0; separate target directory                                                                 |

The new fixtures cover private unowned storage, mismatched owner/ignore records,
invalid persisted schema/root/topology/state ordering, state/journal aliases,
selected ordinary modes, recovery guard mismatch, chmod-only conflicts and
file/directory recovery sync failure ordering. Process interruption fixtures now
retain existing 0644/0755 modes as well as private modes.

## Executed semantic controls

The retained logs show compilation followed by runtime test failures for the
controls below, including assertion failures and unwrap panics. They do not
retain the inline runner, mutation patches, restoration steps or process exit
codes; those details cannot be independently established from these artifacts.

| Removed or weakened behavior                             | Regression that rejected it                             |
| -------------------------------------------------------- | ------------------------------------------------------- |
| Selected mode replaced with constant 0600                | `new_executable_and_existing_ordinary_modes_are_exact`  |
| Recovery file sync omitted                               | `recovery_sync_failures_do_not_publish_done`            |
| Recovery directory sync omitted                          | `recovery_sync_failures_do_not_publish_done`            |
| Evidence comparison ignores mode                         | `mode_only_edit_requires_reconciliation_without_repair` |
| Existing-store owner/ignore validation omitted           | `existing_unowned_state_directory_rejects`              |
| Both recovery entrypoint and runner guard checks omitted | `recovery_requires_matching_guard`                      |

The sync tests inject failures at the private sync adapter, observe file then
directory ordering before Done, and inspect the retained Pending receipt after
either failure. The adapter delegates to `File::sync_all` in production. This
pins the protocol calls and error handling; it is not a hardware durability
test.

The earlier Pending-publication omission control ran
`recovers_after_every_effect_and_receipt_boundary` and failed with:

```text
assertion `left == right` failed
  left: Reconcile { index: 0 }
 right: Complete
test result: FAILED. 0 passed; 1 failed
```

The restored `run.rs` content hash at that stage, before formatting, was
`efec5a5d2908ef30ce92f1ed999af211146dffa54f7b6f418c74b0bb6f5dfe20`.

## Deferred coverage

[Debt #292](https://github.com/driftsys/upskill/issues/292) tracks Darwin flags
and provenance equality fixtures: Minor severity, Medium effort. No confirmed
metadata-loss bug was identified. Caller ownership, lifecycle/schema-2
integration, native activation and exhaustive hostile races remain separate
boundaries.

## Tested source manifest

```text
d5acddd24defc3d509acf268a40f11dcab9464864ef6cdf841fa75ff7307fe7f  src/filesystem.rs
383ee4e8258425ff6b26c75f7c62fa4182a573ced671d3e3cb65cf4f948d8027  src/filesystem/scope.rs
ee69bcd977c3f113629e885383b421ead5b3732fb44771fc767f2afcb05912d5  src/filesystem/execution/backend.rs
d43661379c3333e941b80093a92b855ff5b36360f7a4ea3aafeda866d4d8886f  src/filesystem/execution/backend_tests.rs
332c81cdfccd3fab7f9e4e65a350ab631460a6419a9928fdc1714f9af9398abd  src/filesystem/execution/journal.rs
83f43f4dcf5222cf288f58774f24ed0d6822e39c384a1323f43e4eb504a78f9c  src/filesystem/execution/journal_tests.rs
2e1f1ddb46ec32fc0745becbfef4c5314301e4b222ab6d486b8d27c99fa6f734  src/filesystem/execution/macos_acl.rs
80489e2e4a96ce977ac8c823a9c62499904c0441c7c8eee98779e3ccbcd282b8  src/filesystem/execution/mod.rs
ec503ab32f68ff7fe839128c505c4b041d834f9c1f8bf8b0d60cb0a11b61c3d7  src/filesystem/execution/process_tests.rs
1aa22c5c099c1d682d45eab8e2b82178551d408c462ad23a92a4630e8949fe2c  src/filesystem/execution/request.rs
c0798e8561cb6e2aeb5119f6ec97a11632900abbc58fd095f1a8859f098b4fec  src/filesystem/execution/run.rs
aa3ef42fba3724ab9165d845d70c07aad1d11263647d306d3ffdbb325645d870  src/filesystem/execution/run_tests.rs
```

## Retained log fingerprints

These hashes identify the local raw logs used to compile this record. The raw
logs are scratch artifacts; this committed record retains the results and source
identity without treating future CI configuration as execution evidence.

```text
f70d9ee920db6861722f8b442a2f7fde53df9429b8e3498c42ec3f01ffa873ce  verify.log
d2b80bf38edc3431f10dcf2ecadb695aa0d98df69f108827e44edd7f70c93e79  msrv.log
878c77035a1757dd24a134cce54a788c8fc1fd22f2fbbbae68e73517c16168cb  pending-control.log
1ea14605b85fad8b4b075000af629b477a5f355643d102215085c311edfa8e25  triage-verify.log
45effa7d6b1e55a728dfb52434cb18ba48d63641ab7c0d8cfeb893670079a567  triage-clippy.log
750fd8fca28c61f6d98e589ef38a86ff2e9ab3e8c40e3f6d8f25d6a5811d4d0b  triage-msrv.log
a42f2bca962d726d5aa83b7594f5fed93fbf5ff55cc6b7864156a0334ac8ec3a  triage-control-mode.log
ff3218881645c570fb08fdb92c7d82374aa3919eecbbfc33cfb97c4b403d4479  triage-control-file-sync.log
527b5e2dab6e42a0146c0bd40643b91962f52b1178ac97be866c6bcb5fbf24bd  triage-control-directory-sync.log
e3d61eca85e0a40973d7534b547acd1d2855fa15f9810157e394b6c5caa516b6  triage-control-mode-conflict.log
4af809215df5545c35633cb5bfea0c68221b307755e63d306961b2f85aa26e06  triage-control-owner.log
5282a5c0a11c5334afdceb3be148a3df63869827455153213b0ad321cf183921  triage-control-guard.log
```

## Final sync-test refinement

After `0f44f1128d09fa17754a9e9df2b575624a79545f`, the sync regression was
expanded to inspect device/inode identity for the output file and its nested
parent. A separate deletion case checks that only the parent directory is
synced, and that a failed directory sync retains Pending. Both cases check sync
ordering before Done. The evidence wording above was narrowed to the artifacts
retained for the earlier controls.

The refined candidate passed `just fmt`, `just verify` (1034 passed, one ignored
subprocess entrypoint) and the Rust 1.85 all-target check locally. The source
manifest above identifies the earlier candidate; the changed test file now has
the following SHA-256:

```text
b8d67533b6c4b20392da2b56dea4e3b3d0dd25768ed147f3712437ebb4220f9b  src/filesystem/execution/run_tests.rs
```

Two additional controls were executed and their patch/result/restoration
metadata was retained:

```json
[
  {
    "name": "wrong-descriptor",
    "old": "sync_recovery(&file, \"file\")?;",
    "replacement": "sync_recovery(&dir, \"file\")?;",
    "exit": 101,
    "restored_sha256": "ee69bcd977c3f113629e885383b421ead5b3732fb44771fc767f2afcb05912d5"
  },
  {
    "name": "deletion-sync",
    "old": "sync_recovery(&dir, \"directory\")?;",
    "replacement": "if expected.is_some() { sync_recovery(&dir, \"directory\")?; }",
    "exit": 101,
    "restored_sha256": "ee69bcd977c3f113629e885383b421ead5b3732fb44771fc767f2afcb05912d5"
  }
]
```
