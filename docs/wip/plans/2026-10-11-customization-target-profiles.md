# Customization target profiles implementation plan

> **For agentic workers:** Use the execution workflow task by task. Each task starts with a failing test, ends with verification, and receives review before the next task.

**Goal:** Add the F-A target identity and capability validation contract while preserving the currently shipped client-selection behavior.

**Architecture:** Introduce a small `target` module for family/surface identity and capability decisions. Connect existing `SelectedClient` values to surfaces and freeze the default selection explicitly, so later Codex integration cannot alter defaults merely by extending an enum. Keep Codex unselectable until its instruction lifecycle is implemented.

**Tech stack:** Rust 2024, existing standard library and test harness; no new dependencies.

**Spec:** [AC-00 foundation design](../specs/2026-10-11-agent-customization-foundation-design.md), sections 1, 2, 6, and F-A. [Roadmap](../specs/2026-10-11-agent-customization-roadmap.md).

## Global constraints

- Preserve the existing CLI flags and user/project `clients:` precedence.
- The no-selection default remains Claude, Copilot CLI, VS Code Local, and OpenCode.
- Codex is an identified future local target, with no CLI flag, output, or success claim in F-A.
- A required behavior or restriction that lacks an exact native mapping yields an error; optional cosmetic metadata may warn.
- No native conformance claim follows from Rust unit tests alone.
- Use the repository's fake-home integration harness and zero-warning gates.

## File responsibilities

| File                                                                 | Responsibility                                                                                        |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `src/target.rs` (new)                                                | Stable family/surface identifiers, contract revision, capability declarations and validation outcomes |
| `src/lib.rs`                                                         | Export the new target module                                                                          |
| `src/select.rs`                                                      | Map current consumer selections to surfaces and hold the frozen default set                           |
| `src/config.rs`                                                      | Keep config behavior and tests aligned with the frozen default                                        |
| `tests/cli_add.rs` or matching existing CLI file                     | Pin unchanged default install output from the user-facing command                                     |
| `docs/wip/specs/2026-10-11-agent-customization-foundation-design.md` | Record any implementation choice differing from the proposal                                          |

## Review focus

- A future surface added to the enum must not enter the existing default install.
- `--vscode` and `--copilot` retain one shared generation target and distinct MCP targets.
- Empty configured client lists keep their current default meaning.
- A required restriction cannot be converted into a warning or omitted native field.
- A profile with no declaration for a requested capability must fail closed.

## Task 1: Freeze selection and connect surface identity

**Files:** `src/target.rs`, `src/lib.rs`, `src/select.rs`, `src/config.rs`, `tests/cli_add.rs` (or the existing file that covers default add).

**Interfaces:** `TargetFamily`, `TargetSurface`, `TargetProfile { family, surface, contract_revision }`; `SelectedClient::surface() -> TargetSurface`; `ClientSelection::default_clients() -> &'static [SelectedClient]`. Existing `generation()` and `mcp()` remain.

- [ ] Write failing unit tests for stable identifier round trips and the four default selections. Include `TargetSurface::CodexCli` and `TargetSurface::CodexIde` in the identity table, but assert neither is selectable or in defaults.
- [ ] Run the focused tests and confirm failure before implementation.
- [ ] Add the types and explicit default list; route `ClientSelection::all()` membership checks through that list. Keep `ClientSelection::restrict(&[])` equivalent to the shipped default. Do not add Codex to `SelectedClient`, `Client`, or `McpTarget`.
- [ ] Run focused unit tests and the existing CLI default-add test. Check that `--copilot --vscode` still deduplicates generation but selects both integration targets.
- [ ] Commit the source/tests and any exact documentation correction.

## Task 2: Capability decisions fail closed

**Files:** `src/target.rs`, its unit tests.

**Interfaces:** `CapabilityCategory` (the seven requested categories); `CapabilityKey` (category plus native field/operation identifier); `Representation` (`Native`, `Adapted`, `NativeUnsupported`, `Deferred`); `Requirement` (`RequiredBehavior`, `OptionalMetadata`); a structured decision that carries surface, key, reason, and disposition. Validation accepts a profile's explicit capability entries and a requirement; it has no implicit success default.

- [ ] Write failing table tests: exact native support passes; adapted required behavior, missing declaration, native-unsupported, and deferred required behavior fail with target/key/reason; optional cosmetic data can warn; permissions/tool restrictions are required behavior.
- [ ] Run focused tests and confirm failure before implementation.
- [ ] Implement the smallest typed decision function and diagnostic data. Keep it pure. Profile version/evidence fields remain metadata; no native support is claimed without later fixture evidence.
- [ ] Run focused tests and `just test`. Verify no text-only restriction is reported as an enforcement capability.
- [ ] Commit.

## Task 3: Verify unchanged public behavior and document handoff

**Files:** Existing relevant tests and the foundation design; avoid new production modules.

- [ ] Run an integration test that installs a source without selection and observes the current Claude/Copilot/OpenCode output and current MCP target behavior. Run tests for `--vscode` only and configured client precedence. Add assertions only where coverage is missing.
- [ ] If a new assertion fails, fix the smallest production regression and rerun the focused red/green test.
- [ ] Document the F-A implementation boundary: profiles identify Codex, but Codex files and flags belong to the M1 lifecycle slice. Record which native version/OS checks remain open.
- [ ] Run `just fmt`, `just verify`, and inspect `git diff --check`; commit and open a stacked PR targeting the roadmap/design branch while PR #251 remains open.

The next plan covers M1 instruction aggregation, native discovery, and safe removal. It can add Codex selection only when its output and lifecycle work together. F-A does not implement schema-2 ownership or user-facing policy distribution.
