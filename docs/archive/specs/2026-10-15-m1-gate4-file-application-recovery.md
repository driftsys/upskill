# M1 gate 4: qualified file application and recovery

Durable successors:
[requirements](../../specification/specification.md#412-qualified-file-execution-and-recovery),
[as-built design](../../design/qualified-file-execution.md), and
[AD-0033](../../adr/0033-qualified-file-execution-and-journals.md). This raw
approved working document preserves planning history. Hosted qualification
remains pending; archival does not complete gate 4 or M1.

Date: **2026-10-15**. Status: **scope approved; implementation plan for
review**.

This scope implements the executor and recovery contract in the
[M1 lifecycle design](../../wip/specs/2026-10-12-m1-owned-lifecycle-design.md).
Gate 5 consumes its reviewed interfaces. PR #280 supplies pure recovery
decisions, not file effects or durable checkpoint receipts.

## Goal and boundary

Build an opt-in library executor that applies a complete qualified file plan and
resumes interrupted application using durable observations and receipts. It must
preserve manual edits and report the exact applied prefix. Existing CLI dispatch
and schema-1 commands remain inactive consumers of this API until the later
lifecycle and adapter gates pass.

Inputs comprise the complete planned action sequence, exact source/state/file
preconditions, desired bytes and permissions, and a qualified scope writer
guard. The caller supplies final state bytes as a checkpointed file action; this
task does not define schema 2 or decide installation ownership.

## Required behavior

- Use existing qualified scope, source and destination observations, permission
  policy, and `operations::recovery` decisions. Reject unsupported platforms or
  affected forms before effects rather than weakening qualification.
- Validate the complete sequence and every source, state and destination
  precondition under the scope lock before the first output effect. Reobserve
  each destination immediately before acting.
- Replace files atomically where qualified, preserving the selected permission
  mode. Delete only individually planned unchanged files. Keep directories,
  extra resources, and empty shared Markdown aggregates.
- Persist the operation journal under `<scope-root>/.upskill/state/`, with a
  generated ignore file and restricted journal/staging permissions. Record
  operation/action identities, pre/post evidence, state hashes, and durable
  checkpoint outcomes. Do not archive unrelated configuration or credentials.
- Order durable intent, file effects, and completion receipts so recovery can
  distinguish an unapplied action from an effect that completed before its
  receipt. Include the final state action in the same protocol.
- Recover forward by comparing current qualified observations with recorded
  pre/post evidence. Preserve unexpected bytes and return reconciliation
  guidance. Do not claim that a multi-file operation is atomic.
- Report completed and verified prefixes separately, matching the recovery
  model. File completion never implies native registration or trust success.

## Acceptance examples

1. A valid multi-file plan applies exact desired bytes and modes, then commits
   its final state action; unrelated files and manual aggregate text survive.
2. A stale source, lockfile, destination, root, or alias observation rejects the
   complete operation before its first output effect.
3. An external edit between actions stops application at that action, reports
   the applied prefix, and preserves the edit.
4. A second process cannot apply or recover the same scope while its writer
   guard is held; releasing or terminating the first process releases the lock.
5. Interruptions before and after every intent, file replacement/deletion, and
   receipt checkpoint recover to the expected result without duplicate effects.
6. Interruptions immediately before and after final-state replacement retain
   sufficient evidence to recognize the committed state and prevent replay.
7. Recovery encountering bytes or modes matching neither recorded state stops
   with reconciliation guidance and leaves them untouched.
8. Extra resource files, unowned directories, and empty shared aggregates
   survive removal. No execution path performs recursive output removal.
9. Corrupt, truncated, unsupported, or mismatched journals cause no output
   effects. Recovery never executes arbitrary paths or actions from unvalidated
   persisted data.
10. Read-only planning and inspection create no journal, staging file, ignore
    file, or migration backup. Restricted transient files are cleaned only when
    their identity and ownership are established.

## Exclusions and integration

Schema-2 codecs, combined legacy/new state, reference selection, ownership
adoption, and the first-migration backup policy belong to gate 5. Native plugin
or MCP effects, Codex selection, and automatic maintenance are separate work.
The executor rejects unsupported native actions rather than inferring success.

The existing untracked `docs/design/qualified-file-application.md` files in the
file-application and file-recovery worktrees retain their current ownership.
This draft uses the canonical M1 contract and merged APIs; it does not adopt
those unreviewed drafts.

## Completion evidence

Start with real filesystem acceptance tests in temporary scopes, then focused
unit tests. Use process interruption and cross-process locking tests rather than
only mocked checkpoint decisions. Verify supported Linux/macOS behavior, Rust
1.85, `just fmt`, `just verify`, hosted CI, and independent review before
claiming gate 4 complete. Document unsupported filesystem/platform guarantees.

Implementation planning follows review of this scope. The plan must specify the
journal format, durability ordering and exact executor interfaces before
implementation begins.
