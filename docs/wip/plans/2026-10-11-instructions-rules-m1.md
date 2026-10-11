# M1 instructions and rules delivery plan

> This plan implements the M1 milestone in the [agent customization roadmap](../specs/2026-10-11-agent-customization-roadmap.md). F-A target identities are its base. Keep each reviewable task in its own PR, with code, tests, and documentation together.

**Goal:** Install, update, inspect, and remove instruction contributions for Claude Code, Copilot CLI/VS Code Local, and Codex CLI without overwriting manual content or silently weakening scope.

**Contract:** Follow the [foundation design](../specs/2026-10-11-agent-customization-foundation-design.md), especially target selection, shared discovery, ownership, preflight, and schema-1 migration. Preserve OpenCode behavior and the fixed default selection.

## Acceptance examples

1. A project with manual `AGENTS.md` text adds two unconditional rules for Codex. The file retains the manual bytes and receives two deterministic, individually owned contributions. Updating one rule changes only its contribution; removing it retains the other and the manual text.
2. An item selected for Codex only never claims that other clients cannot discover a shared `AGENTS.md`. Preview and reports name known incidental readers. Codex remains outside the implicit default.
3. A Claude and Copilot scoped rule maps to their native path-specific instruction formats. Codex rejects an arbitrary file glob unless the author provides a native alternative with equivalent behavior; it does not widen the rule to repository-wide guidance.
4. A pre-existing unmarked output at a destination is a conflict. An edited owned contribution is a conflict on update/removal. A second bundle's contribution survives removal of the first.
5. A legacy schema-1 lockfile remains readable. Inspection does not rewrite it; schema-2 migration preserves old entries and blocks ambiguous ownership changes rather than deleting guessed paths.
6. Native discovery is tested in fresh Claude, Copilot, and Codex sessions at the recorded supported versions and scopes. The release claim is limited to combinations actually exercised.

## Task sequence

### M1-A: Owned artifact and state foundation

- M1-A1 is a pure Markdown contribution editor with tests for manual-byte preservation, independently owned blocks, edited-block conflict, and malformed markers. It does not change CLI behavior or claim file ownership until state integration lands.
- M1-A2 integrates ownership records and migration across add, update, remove, list, and doctor; M1-A3 adds preflight, writer locking, and interruption recovery. Keep these as separate task PRs.
- Start with CLI acceptance tests for conflicting unowned files, manual-content preservation, edited owned content, and legacy lockfile read/upgrade.
- Define schema-2 records for requested surfaces, installed files and contribution selectors, source identity, and tagged legacy integrations. Keep source schema independent.
- Add pure plan/precondition and Markdown contribution editing. Use deterministic markers and per-contribution hashes. Reject malformed, duplicate, missing, or edited markers. Preserve bytes outside owned regions.
- Migrate schema-1 reads without inferred ownership; write schema 2 only after add/update/remove/list/doctor retain its records. Keep an exact schema-1 backup on first write.
- Add a scope writer lock and recoverable journal before mutating shared files. Test stale preconditions, interrupted writes, and no recursive deletion of unowned content.
- Document lockfile and lifecycle semantics, then run `just fmt` and `just verify`.

The M1-A1 editor uses `<!-- upskill:begin ID -->` and
`<!-- upskill:end ID -->` lines around each contribution. Its returned record
contains the content SHA-256, the exact count of newlines inserted before
the block, and the count already present immediately before those newlines.
A future state writer must persist all three fields; the editor uses them
to reject edits and to restore manual bytes when a block is removed. Unrecorded
markers conflict rather than becoming implicitly owned. This is an internal
format until M1-A2 supplies durable state and CLI integration.

### M1-B: Project and user unconditional instructions

- Start with CLI examples: `--codex` opt-in, no-selection unchanged; add two rules to a manual `AGENTS.md`; update/remove one contribution; project/user scope paths; existing Claude/Copilot/OpenCode outputs remain sound.
- Add Codex CLI to selection and profile dispatch. Keep Codex IDE a separate observed surface, not a second CLI flag. Validate source audience and directives for every selected surface.
- Map native roots from the pinned contracts: Codex `AGENTS.md` and global Codex home; Claude `CLAUDE.md`/rules; Copilot CLI repository and personal instructions; Copilot VS Code Local separately. Reject unsupported scope/override combinations.
- Share an artifact only after validating its content for known readers. Report incidental discovery and conflicts. Preflight the whole operation before writing any instruction file.
- Test fresh native session discovery at each supported scope; record exact client versions and OS. Do not label file generation alone as native conformance.
- Update usage, format specification, and ADRs; run `just fmt` and `just verify`.

### M1-C: Scoped rules and lifecycle completion

- Start with matching and nonmatching native-session examples and CLI tests for update/remove preserving unrelated content.
- Keep Claude `paths` and Copilot `applyTo` semantics explicit. Codex nested `AGENTS.md` is directory-context guidance, not arbitrary file-glob activation. Reject unmappable globs or require an explicit author alternative.
- Test nested instruction discovery, override files, size limits, path traversal/symlink protections, shared output collisions, and two-bundle ownership.
- Complete `doctor`, list, preview, migration, and native evidence for all M1 destinations. Document unsupported semantics and release matrix.
- Run `just fmt`, `just verify`, native conformance, and the full PR review before merge.

## Scope boundaries

M1 handles prose instructions and rules. Codex `.rules` command policy belongs to permissions; hooks, MCP, skills, agents, and marketplace operations keep their later milestones. No automatic execution of source scripts is part of instruction installation. Existing bridge files and unmarked legacy output remain user-owned until explicit migration evidence proves otherwise.
