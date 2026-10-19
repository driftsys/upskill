# M1 instructions and rules delivery plan

> This plan implements the M1 milestone in the
> [agent customization roadmap](../specs/2026-10-11-agent-customization-roadmap.md).
> F-A target identities are its base. Keep each reviewable task in its own PR,
> with code, tests, and documentation together.

Reconciled 2026-10-19: the
[delivery contract](../../topics/0009-customization-delivery.md), AD-0040 and
[corrected Gate 6 design](../specs/2026-10-18-gate6-activation-design.md)
supersede earlier production use of managed `AGENTS.md` contributions. Completed
foundation tasks retain their implementation evidence, not native route
approval.

**Goal:** Install, update, inspect and remove rules for Claude Code, Copilot
CLI, Codex CLI and OpenCode v2 without changing manual content or weakening
scope. Codex/OpenCode independent rules require qualified plugin delivery.
Copilot VS Code Local remains a separately qualified existing surface.

**Contract:** Follow the
[foundation design](../specs/2026-10-11-agent-customization-foundation-design.md),
especially target selection, shared discovery, ownership, preflight, and
schema-1 migration. Preserve OpenCode behavior and the fixed default selection.

## Acceptance examples

1. A project with manual team `AGENTS.md` installs two independent rules through
   each intended Codex/OpenCode plugin. Team bytes/modes stay identical;
   updating or removing one rule preserves the other. Disabling/removing the
   plugin stops managed-rule activation while team guidance remains available.
2. For each customization/client inventory compatibility directories, imports,
   globs, package exports and shared paths. Unintended consumers must pass
   native negative controls. Identical output or incidental-reader reporting
   does not authorize cross-client consumption. Test standalone-plus-plugin and
   project/user duplicate activation.
3. Claude/Copilot scoped rules retain native matching/nonmatching semantics;
   independent Codex/OpenCode plugins must prove equivalent scope or reject the
   profile. Nested team instructions do not substitute for arbitrary rule globs.
4. Unmarked destinations and edited owned artifacts conflict. Aliases,
   resources, direct and multiple-bundle references preserve manual content,
   existing configuration and unrelated ownership through update/removal.
5. Every installed-state command preserves mixed rule/skill/resource/agent/
   bundle/plugin/MCP state through guarded migration/recovery. Exact original
   schema-1 bytes survive in the backup; typed legacy projection is not an
   authorization to overwrite outer schema-2 ownership.
6. All four clients pass applicable native version/OS/scope/kind cells and CD-01
   through CD-10. Existing IDE and binary-platform obligations remain separate.
   Canonical-file/generated-output checks cannot qualify plugin delivery.
   Missing cells block activation without reducing four-client scope.

## Task sequence

### M1-A: Owned artifact and state foundation

- M1-A1 is a pure Markdown contribution editor with tests for manual-byte
  preservation, independently owned blocks, edited-block conflict, and malformed
  markers. It does not change CLI behavior or claim file ownership until state
  integration lands.
- M1-A2a delivers the pure validated owned-artifact record model and
  deterministic internal JSON codec; see the
  [owned artifact records plan](2026-10-12-owned-artifact-records.md). M1-A2
  continues with legacy projection and lifecycle integration across add, update,
  remove, list, and doctor. M1-A3 adds preflight, writer locking, and
  interruption recovery. Keep these as separate task PRs.
- Start with CLI acceptance tests for conflicting unowned files, manual-content
  preservation, edited owned content, and legacy lockfile read/upgrade.
- Define schema-2 records for requested surfaces, installed files and
  contribution selectors, source identity, and tagged legacy integrations. Keep
  source schema independent.
- Add pure plan/precondition and Markdown contribution editing. Use
  deterministic markers and per-contribution hashes. Reject malformed,
  duplicate, missing, or edited markers. Preserve bytes outside owned regions.
- Migrate schema-1 reads without inferred ownership; write schema 2 only after
  add/update/remove/list/doctor retain its records. Keep an exact schema-1
  backup on first write.
- Add a scope writer lock and recoverable journal before mutating shared files.
  Test stale preconditions, interrupted writes, and no recursive deletion of
  unowned content.
- Document lockfile and lifecycle semantics, then run `just fmt` and
  `just verify`.

The M1-A1 editor uses `<!-- upskill:begin ID -->` and `<!-- upskill:end ID -->`
lines around each contribution. Its returned record contains the content
SHA-256, the exact count of newlines inserted before the block, and the count
already present immediately before those newlines. A future state writer must
persist all three fields; the editor uses them to reject edits and to restore
manual bytes when a block is removed. Unrecorded markers conflict rather than
becoming implicitly owned. This is an internal format until M1-A2 supplies
durable state and CLI integration.

M1-A2a records a concrete project or user installation root, a portable relative
artifact path, explicit installation owners with separate source names and
installed aliases, and consuming target surfaces. Its tagged content identifies
either a whole file with a SHA-256 and optional permission mode, or one Markdown
contribution with its ID, SHA-256, and both newline counts. Multiple
contribution IDs may share a location; whole-file ownership cannot share that
location with contributions. A repeated installation ID must retain its exact
source, revision, item kind, source name, and installed alias binding across all
records.

The codec reads and writes an internal JSON array of these records, rejects
unknown fields and invalid or ambiguous records, and serializes a sorted copy as
pretty JSON with one trailing newline. It does not activate CLI behavior or
schema-2 lockfile writes. Root validation uses the current host's absolute-path
rules; location comparisons use exact lexical root spellings and portable
relative paths. Validation performs no filesystem access, canonicalization, case
or alias checks, symlink containment, source verification, or writer locking.
Those checks remain executor and state integration work.

The
[legacy source-label compatibility prerequisite](2026-10-12-legacy-source-labels.md)
restores schema-1 updates from unambiguous persisted `gitlab:` and `gitlab+`
labels. The applying lifecycle canonicalizes reinstalled item and bundle labels
to HTTPS while preserving aliases, pins and unrelated records. Inspection and
preview preserve bytes. New user syntax stays unchanged; ambiguous or
unrepresentable labels require explicit reconciliation. This prerequisite does
not adopt output files or introduce schema 2.

The
[pure legacy-state projection prerequisite](../../archive/plans/2026-10-12-legacy-state-projection.md)
is implemented: it preserves the typed snapshot and indexed rows, decodes source
labels, and describes the fixed historical Claude/Copilot/OpenCode candidate
layouts. Requested selection remains unresolved; diagnostics retain malformed
and duplicate state. It performs no I/O, source verification or ownership
adoption.

The
[bounded owned instruction planning prerequisite](../../archive/plans/2026-10-12-owned-instruction-planning.md)
joins the aggregate editor and owned-artifact model through a pure public
planner and shared verified body reader. It plans complete installation
contribution sets from exact supplied observations, preserves untouched
bodies/owners/consumers and manual bytes, rebuilds separator metadata, and
retains exact before bytes for no-ops. Last-region removal retains manual bytes
and keeps the file, which is empty only when no content remains. Affected whole
files and multi-owner regions conflict; unrelated records survive. It enables no
CLI or native support and does not complete M1-A2 or M1-A3.

The
[strict captured source manifest prerequisite](../../design/strict-source-manifests.md)
is implemented as a separate opt-in library API: complete captured regular-file
bytes, optional observed Unix modes, framed versioned digests and pure
validation. Observed links, special entries and read/traversal errors fail
rather than return partial evidence. Current commands and legacy hashes remain
unchanged. This prerequisite does not complete whole-file/resource planning or
qualify an atomic snapshot, source authenticity or containment. Immutable Git
evidence, source stabilization, destination qualification, executor/state
integration and Codex activation remain pending.

The
[owned whole-file planning prerequisite](../../archive/plans/2026-10-12-owned-file-planning.md)
provides pure `files::plan` operation plans for explicit complete desired sets
and supplied before-state observations. It preserves untouched ownership,
requires exact byte/hash and optional-mode evidence, rejects unowned
destinations and affected shared/contribution records, and enumerates only
individual file changes. The observations remain unqualified. Lifecycle
desired-set integration, combined durable state and Codex activation remain
pending. Qualified destination observation and opt-in
[execution/recovery](../../design/qualified-file-execution.md) are implemented
separately; final-head hosted qualification remains required. The complete mixed
coordinator is described below. Existing commands, lockfiles and native behavior
are unchanged; this does not complete M1-A2, M1-A3 or M1.

The
[captured item compilation prerequisite](../../design/captured-item-compilation.md)
provides an opt-in pure producer for one validated captured rule, skill or
agent. It reuses current rendering, historical paths, resource classification
and ignore semantics, retains exact captured resource bytes and modes, and
reports explicit audience exclusions. Aliases change paths and resource link
namespaces while rendering retains source identity. Active matching overrides
fail explicitly; body override precedence remains deferred. It does not reread
sources, write outputs, activate CLI/native behavior or supply ownership and
qualified destination evidence. Connecting complete compiled outputs to the
lifecycle coordinator, requested-selection reconciliation, filesystem
qualification, durable state, executor/recovery and target gates remains
pending. This bounded prerequisite does not complete M1-A2 or M1-A3.

The
[mixed owned-artifact planning prerequisite](../../specification/specification.md#46-mixed-owned-artifact-operation-plans)
provides one pure complete-operation boundary over dedicated files and
instruction contributions. Sets replace both forms, including omitted old
artifacts. It validates global installation bindings, exact physical evidence,
cross-form collisions and affected ancestor topology before composing the
existing planners. It preserves aggregate modes and manual bytes, every affected
no-op, untouched owners and prior interleaved row order. This does not connect
compiled outputs to lifecycle desired sets or activate commands, native targets,
state or effects. M1-A2, M1-A3 and M1 remain incomplete.

The
[operational lifecycle design](../specs/2026-10-12-m1-owned-lifecycle-design.md)
remains active WIP after the bounded pair completes and must not be archived
with it. Complete reference-graph authority, per-owner consumer association,
qualified source/filesystem evidence, compiler-to-lifecycle desired sets,
durable state/lifecycle and transactional execution remain activation work.

M1-A2 still requires verified source resolution, reproducible historical
rendering and resource enumeration, requested-selection reconciliation, and
adoption. Lifecycle integration must preserve records through add, update,
remove, list and doctor before schema-2 writes can be enabled with exact
schema-1 backup and the M1-A3 containment, preflight, writer-lock and
recoverable journal boundary. These prerequisites do not complete M1-A2 or
provide schema-2 support.

The [gate-4 qualified executor](../../design/qualified-file-execution.md) now
provides opt-in file application, strict desired-payload journals and forward
recovery under the matching scope writer guard. Local full tests, process
termination checkpoints and the Rust 1.85 all-target check passed for the
candidate implementation. Hosted Linux/macOS qualification remains pending; gate
4 is not complete. The schema-2 lifecycle integration gate and native Codex gate
remain separate, and M1-A2, M1-A3 and M1 remain incomplete. This executor does
not activate the current schema-1 commands or adopt legacy ownership.

### M1-B: Independent rules alongside team instructions

- Execute the
  [corrected implementation plan](2026-10-19-customization-delivery-reconciliation.md)
  after architecture review, starting with installed-plugin entrypoint probes
  and failing CD-02/CD-03/CD-04 examples. Production schema-2 dispatch stays
  off.
- Use qualified native rule loading for Claude/Copilot and independently
  registered plugin rule delivery for Codex/OpenCode. Define effective root,
  package identity, lifecycle event/API, trust, reload and rule activation scope
  before implementing the adapter. Never silently rewrite team `AGENTS.md`.
- Bind requested consumers separately from potential readers. Require explicit
  intended sharing and successful unintended-reader/duplicate controls,
  including compatibility directories, imports/globs and packaged resources.
- Reuse captured source evidence, owner contracts, complete preflight and
  qualified application/recovery. Experimental contribution code remains
  available only within its stated library boundary; recorded removal/recovery
  retains safety without converting an installation to plugin ownership.
- Complete all-kind command/configuration compatibility and native-effect
  recovery before the combined production routing/migration task.

### M1-C: Scoped rules and lifecycle completion

- Start with matching and nonmatching native-session examples and CLI tests for
  update/remove preserving unrelated content.
- Keep Claude `paths` and Copilot `applyTo` semantics explicit. Qualified
  Codex/OpenCode plugin rules must establish matching scope independently of
  nested team `AGENTS.md`, which is not arbitrary file-glob activation. Reject
  unmappable globs or require an explicit author alternative.
- Test nested instruction discovery, override files, size limits, path
  traversal/symlink protections, shared output collisions, and two-bundle
  ownership.
- Complete `doctor`, list, preview, migration, and native evidence for all M1
  destinations. Document unsupported semantics and release matrix.
- Run `just fmt`, `just verify`, native conformance, and the full PR review
  before merge.

## Scope boundaries

M1 handles rules and their required internal plugin delivery. Codex `.rules`
command policy belongs to permissions. General hooks/tools/permissions, expanded
MCP management and owned skill/agent lifecycle retain their roadmap milestones;
existing skill/resource/agent/bundle/plugin/MCP behavior remains a regression
obligation. Internal rule hooks are qualified plugin implementation, not a new
general hook-management feature. No automatic execution of source scripts is
part of instruction installation. Existing bridge files and unmarked legacy
output remain user-owned until explicit migration evidence proves otherwise.

Validation results and tested source identities are retained in the
[gate-4 evidence record](https://github.com/driftsys/upskill/blob/main/docs/technotes/m1-gate4-validation.md).
