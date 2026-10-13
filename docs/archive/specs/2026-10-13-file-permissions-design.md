# Explicit file permission policy

Status: selected bounded prerequisite for qualified file application; not implemented.

## Intent

Resolve the permission-policy boundary left open by AD-0026 before introducing file effects. The captured compiler retains exact resource modes and emits no mode for generated output. A pure selection function must distinguish absent files from present files whose mode is unknown, preserve supported existing modes, and select private modes for new files without treating source permission evidence as execution authority.

User delegation authorizes controller lane decisions. The controller selects this small independently testable prerequisite; it does not replace the remaining journal, metadata qualification, state or application work.

## Public interface

Add `files::permissions` with these public types, deriving Debug, Clone, Copy, PartialEq and Eq:

```rust
pub enum DestinationMode { Absent, Present(Option<u32>) }
pub enum PermissionSource { Generated, Resource { captured_mode: Option<u32> } }
pub enum ResourceExecutionPolicy { Disabled, OwnerExecutable }
pub fn select_file_mode(
    destination: DestinationMode,
    source: PermissionSource,
    resource_execution: ResourceExecutionPolicy,
) -> anyhow::Result<u32>;
```

All inputs are supplied values. The result selects permission bits only and grants no filesystem, ownership, source authenticity or execution authority. The function performs no I/O, reads no process umask and changes no existing compiler/planner DTO or command.

## Selection and validation

1. Validate every supplied destination and captured resource mode is within 0o0000..=0o7777. Invalid evidence rejects even when its mode would not influence selection.
2. Present(None) is unknown existing mode and rejects. It is never treated as an absent file.
3. Present(Some(mode)) containing any 0o7000 special bit rejects as unsupported application policy. Do not silently clear special bits or infer authority to preserve them across inode replacement. Ordinary existing modes 0o0000..=0o0777 are returned exactly, including read-only, executable, zero and group/other bits. Resource policy does not change an existing mode.
4. An absent generated file selects 0o0600. ResourceExecutionPolicy affects resources only and cannot make generated content executable.
5. An absent resource selects 0o0600 when resource execution is Disabled. Captured ordinary and special bits are not copied to the output. Unknown captured mode is allowed in this non-executable case.
6. An absent resource with OwnerExecutable requires a known captured mode. Select 0o0700 only when its owner-execute bit 0o0100 is set; otherwise select 0o0600. Group/other execute bits, read/write bits and all special bits convey no additional authority.
7. Return contextual diagnostics distinguishing destination evidence, resource evidence, unknown existing mode, unsupported existing special bits and unknown executable-resource mode. Do not promise a particular ordering when multiple inputs are invalid.

Preserving an existing mode is a supplied-value policy decision. Future writers must separately qualify owner/group, ACLs, extended metadata, containment and filesystem operations before replacement; successful mode selection does not establish those conditions. The source and destination observation APIs currently qualify bytes and mode evidence, not atomic permission-preserving replacement.

## Acceptance examples

| ID  | Inputs and expected result                                                                                                                                    |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P01 | Absent generated output selects literal 0o0600 under either resource policy.                                                                                  |
| P02 | Existing ordinary modes 0o0000,0o0400,0o0640,0o0755,0o0777 remain exact under both policies and either source kind.                                           |
| P03 | Present(None) rejects; absent generated succeeds, proving unknown is not absence.                                                                             |
| P04 | Existing 0o1000,0o2000,0o4000 and mixed 0o2640 reject without an altered-mode result.                                                                         |
| P05 | Absent resource with execution Disabled returns 0o0600 for captured 0o7777 and unknown mode.                                                                  |
| P06 | OwnerExecutable plus captured 0o0100,0o0755 or 0o4777 returns 0o0700; captured 0o0011 or 0o0644 returns 0o0600.                                               |
| P07 | OwnerExecutable plus absent resource/unknown captured mode rejects; known existing ordinary mode still preserves exactly.                                     |
| P08 | Captured mode 0o10000 or u32::MAX rejects even for an existing file whose supported mode would otherwise be preserved. Invalid destination modes also reject. |
| P09 | Full literal expected values distinguish 0o0600 from 0o0644,0o0700 from source 0o0755, and owner execute from group/other execute.                            |
| P10 | Public API inputs are unchanged after success and error; no file, environment or clock is consulted.                                                          |

Independent controls omit existing-mode preservation, treat unknown-present as absence, copy source mode, use any execute bit instead of owner execute, ignore explicit Disabled policy, ignore special-bit rejection and ignore range validation. Each omission must compile, hit a named semantic assertion, restore exact source bytes and pass focused tests. Do not weaken production validation to make a control fail.

## Global constraints

- Rust edition 2024 and MSRV 1.85.
- No new dependencies, async runtime, subprocess, environment reads, filesystem I/O or stdout/stderr outside main.rs.
- No changes to source capture, manifest framing, compiler output, existing planner validation or lockfile schemas.
- No production CLI behavior, state persistence, migration, file effects, journal or recovery activation.
- Public supplied values convey no physical qualification or execution authority.
- Preserve the inherited 54 WIP/raw archive hashes, the broader application draft and all scratch/control evidence. No cleanup.

## Alternatives and cost

Copying source modes would import special/group/other bits and conflate captured evidence with permission authority. Applying process umask would introduce host-dependent state into a pure policy. Treating every new file as executable would grant authority without an explicit resource choice. Reject all three. Private defaults and owner-only executable resources are conservative; they may require a later explicit sharing policy. Existing special modes remain unsupported until an independently reviewed ownership/metadata contract can preserve them.

## Delivery boundary

One task implements and tests this pure interface with specification, ADR and usage-boundary documentation. No size/performance or hosted native filesystem claim follows from it. Qualified file effects, forward recovery, state checkpoints and lifecycle integration remain subsequent work; M1-A2, M1-A3 and M1 remain incomplete.
