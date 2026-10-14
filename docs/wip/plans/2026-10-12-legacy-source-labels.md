# Legacy source-label compatibility plan

> For agentic workers: use superpowers:subagent-driven-development, then Codex
> PR review before merge.

Goal: restore updates of existing schema-1 installations recorded with
historical GitLab labels, closing #236 and satisfying foundation design section
5 before legacy-state projection.

Authority: `docs/wip/specs/2026-10-11-agent-customization-foundation-design.md`,
section 5; `docs/wip/plans/2026-10-11-instructions-rules-m1.md`, M1-A2. This
compatibility amendment supersedes ADR-0011's old-lockfile hard-fail policy
only. New user input keeps the current source syntax.

## Global constraints

- Rust edition 2024, minimum Rust 1.85; add no dependencies.
- Only main.rs writes to stdout/stderr. Preserve git-based authentication
  without credential injection or CLI fallbacks.
- Keep lockfile schema 1. Do not infer artifact ownership, adopt output files,
  write schema 2, or reconcile native integrations.
- Keep modern HTTPS, GitHub, and local source behavior unchanged; do not broaden
  the modern URL parser incidentally.
- Inspection and dry-run commands must not rewrite lockfiles or generated
  output.
- Work only in the assigned worktree; preserve unrelated branches and edits.

## Task 1: Decode historical state labels and restore offline update

Files: `src/source.rs`; `tests/pipeline_source.rs` or a dedicated
`tests/cli_legacy_source_labels.rs`; existing `tests/cli_exit_codes.rs`;
`src/pipeline/lifecycle.rs` only if acceptance evidence identifies a necessary
canonicalization/preservation correction; canonical source/lifecycle
documentation under `docs/book/` and `docs/specification/`;
`docs/adr/0011-generic-git-url-source.md`; the M1 plan and this plan. Report
concrete additional file needs before editing.

Acceptance criteria:

1. Begin with a failing real CLI acceptance example for #236 and focused parser
   failures. Record assertion-failure RED before implementation. Use the
   existing fake-HOME harness and only temporary git repositories. Never contact
   a remote host in these tests.
2. `parse_install_source_label` decodes
   `gitlab:<namespace>/<repo>[@ref][:subfolder]` to a generic HTTPS git source
   on gitlab.com, and `gitlab+<host>:<namespace>/<repo>[@ref][:subfolder]` to
   the explicit HTTPS host. Preserve nested namespace, optional ref, and
   subfolder values. Canonical Display emits the modern HTTPS label and
   round-trips through the modern label parser for supported historical forms.
3. Accept a numeric HTTPS port only when its authority delimiter is unambiguous.
   Reject malformed or ambiguous authorities rather than guessing a remote.
   Reject empty namespace/repository components, empty ref or subfolder,
   authority whitespace/control characters, credentials, embedded schemes, and
   malformed ports. Define the supported authority grammar in documentation and
   tests; include ordinary DNS/self-hosted names, numeric namespaces, bracketed
   validated IPv6 with or without a numeric port, and the reported
   nested-namespace case. A second colon before the first path slash identifies
   a port; without it a numeric namespace remains a namespace. If a historical
   authority form cannot be represented faithfully by the current parser, return
   an actionable reconciliation error rather than silently changing it. Do not
   modify the modern parser to accommodate it.
4. Modern `add gitlab:...` and `add gitlab+...` still exit 2 with the HTTPS
   replacement hint. Current HTTPS/GitHub/local source-label tests remain green.
   Invalid historical labels fail before fetch with actionable context naming
   the lockfile label and the HTTPS replacement or reconciliation step.
5. Offline update acceptance uses a real local git repository exposed through a
   per-command git `insteadOf` rewrite from its HTTPS URL. Scrub inherited git
   environment during fixture setup. Install with modern syntax, replace only
   recorded item/bundle source labels with historical forms, mutate and commit
   source content, then assert `update --dry-run` retains exact lockfile/output
   bytes and `update` applies the changed content and canonicalizes affected
   labels. Cover default-host and explicit self-hosted labels, including nested
   namespace and a numeric port. Prove installed alias and subfolder survive.
   Prove pinned refs remain selected when source history advances, using
   separate suitable fixtures if necessary.
6. Cover a historical bundle label matching
   `gitlab+gitlab.example.com:org/team/repo:skills/<bundle>.bundle.yaml`.
   Preserve bundle metadata and unrelated items/plugins/MCP entries during
   update. Keep schema 1. Verify inspection does not rewrite state. Tests must
   assert relevant content/metadata, not only command success.
7. Add a dated ADR-0011 amendment limited to persisted-label compatibility.
   Document canonicalization on successful update, unchanged modern input,
   supported historical grammar, actionable ambiguous-label handling, and the
   future ownership migration boundary. Update canonical specification/book
   guidance and the M1 plan with the delivered prerequisite and remaining
   projection/lifecycle work.
8. Run focused parser and CLI suites, `just fmt`, `just verify`, and
   `cargo +1.85.0 check --locked --all-targets` on final code. Report RED/GREEN
   evidence, exact commands/logs, exact commit, and any unverified behavior.
   Commit with Conventional Commits. Do not push or create a PR; the controller
   handles task review, final branch review, PR review, CI, and authorized
   merge.

Review focus: persisted-state compatibility remains separate from user syntax;
authority and suffix separation never guesses; aliases/pinned
refs/bundles/integrations survive; dry-run and inspection preserve bytes; real
offline fetch/update behavior; no early ownership/schema migration.

## Delivery boundary

Task 1 implements persisted-label decoding and exercises real offline git update
through per-command HTTPS URL rewrites. Applying updates canonicalize affected
item and bundle source labels; inspection and preview retain bytes. The modern
user parser, authentication policy and schema-1 state shape stay unchanged.
Legacy projection and ownership-aware add/update/remove/list/doctor integration
remain M1-A2 work, followed by M1-A3 preflight, locking, backup and recovery.
