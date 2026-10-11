# Documentation lifecycle proposal

Research date: 2026-10-11. Status: historical audit and proposal, followed by
the implementation disposition below. The audit itself does not ratify product
requirements or establish implementation completion.

## Disposition after the audit

The user requested this refactor and explicitly selected `docs/technotes/` for
the first-class agent research. The implementation separates book sources,
canonical specifications, design, WIP and archive navigation; retains ADR IDs;
and adds a documentation check to local verification and PR CI. The 25 older
working documents remain pending reconciliation in WIP. No historical feature
was bulk-archived. The original proposal below remains dated audit evidence;
its suggested WIP home for research is superseded by the user's technotes choice.

## Evidence and scope

RIDL remote `main` was verified with `gh api repos/driftsys/ridl/commits/main`:
`c0569f765d6640857ba42f5f26ad3d34b79ac275`. Its local HEAD matched; RIDL content
was read from that git object, excluding untracked working notes. Upskill was
inspected at `ae31bb5c31e2210b9a1dae16348ec839eb661bbd` in the
`research-agent-customization` worktree, plus the new, uncommitted research pack
and ADR-0013 added during this session.
Remote freshness is established only for RIDL at inspection time. This audit
inspects instructions and automation; it does not claim either repository's
checks were executed or that every old plan has been implemented.

Source references below identify repository paths and lines at those revisions.
The RIDL and upskill revision links are immutable:
[RIDL tree](https://github.com/driftsys/ridl/tree/c0569f765d6640857ba42f5f26ad3d34b79ac275),
[upskill tree](https://github.com/driftsys/upskill/tree/ae31bb5c31e2210b9a1dae16348ec839eb661bbd).

## What RIDL actually does

RIDL separates a document's authority from its audience and lifecycle:

| Location              | Ownership and lifecycle                                                                                                                                                                                       |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/specification/` | Normative requirements and language references; the family overview indexes shared doctrines and open questions. Specification does not imply implementation: the frame reference explicitly remains unbuilt. |
| `docs/decisions/`     | Binding choices and their rationale. Each ADR's own status is authoritative; Proposed entries remain visibly distinguished.                                                                                   |
| `docs/design/`        | Detailed architecture of a component as built. Code and tests are the source of truth for this description; binding choices are linked ADRs.                                                                  |
| `docs/technotes/`     | Informative explanations and implementation notes; these do not establish binding requirements.                                                                                                               |
| `docs/wip/`           | Preliminary drafts, research and Superpowers working memory. Settled requirements and decisions are promoted; completed raw design/plan material is archived after its durable content is recorded.           |
| `docs/archive/`       | Superseded references and completed working memory, retained for provenance. The index explains what each document became and any remaining work.                                                             |
| `docs/book/`          | Published reader documentation about the system as built, with explicit labels for specified but unimplemented behavior.                                                                                      |

These definitions are explicit in RIDL `docs/{specification,design,technotes,
wip,archive}/README.md:1-9`, `docs/decisions/README.md:1-4`, and
`AGENTS.md:43-45,275-295`. They are more useful than reproducing directory names
alone. In particular, correcting an inaccurate as-built design record is
different from approving a change to a normative requirement.

The book is a publication view. `book.toml:6` sets `src = "docs/book"`;
`docs/book/SUMMARY.md:3-25` owns navigation. Eight reference chapters include the
canonical specification sources rather than duplicate them: for example,
`docs/book/reference/typl.md:1-6` adds implementation status and includes the
typl reference. There is **no tracked `docs/book/README.md`** at this revision;
the root README, SUMMARY and reading guide provide that navigation.

RIDL's working-memory index describes archiving at epic close or when the
covered story lands, with links to durable records (`docs/wip/README.md:8-59`).
The archive index records provenance, supersession and exceptions. A useful
partial-completion example is `docs/archive/README.md:531-555`: the completed
encoding stage is archived, while the unbuilt next stage is retained separately
in WIP. Dates and checked task boxes alone do not establish completion.

There are limitations worth preserving in the comparison. The WIP index admits
that `family-general-form.md` is effectively normative through ADR citations
despite its folder: ratification was deliberately left unresolved
(`docs/wip/README.md:68-77`). Also, `book.toml:3` still says five languages while
`AGENTS.md:3-7` says four. Directory classification and passing checks cannot
eliminate semantic drift by themselves.

## Rules, automation and coverage limits

RIDL instructions require one owning record per fact, links from other records,
as-built book prose, and removal of story/stage names from shipped explanations
(`AGENTS.md:275-295`). The justfile is the gate definition; CI invokes recipes
and `gate-parity` checks the build membership (`AGENTS.md:218-231`).

Actual enforcement includes:

- `book-check` builds a temporary copy and checks logged errors, unresolved
  directives, missing chapters silently created by mdBook, and include anchor
  pairs (`justfile:740-824`).
- `link-check` scans tracked Markdown relative links. Its extractor strips
  fragments and uses a limited inline-link syntax; it is not a complete
  cross-document anchor or external-URL validator (`justfile:1122-1149`).
- `doc-path-check` checks bare documentation paths, excluding historical WIP,
  archive and changelog citations. Slashless directory mentions are outside its
  coverage (`AGENTS.md:129-140`; `justfile:1405-1590`).
- `story-id-check` scans specified implementation and durable documentation
  trees, including specification files included by the book; its exclusions
  intentionally retain planning history (`justfile:1834-1929`).
- Book language examples are compiled by a Rust harness. It does not expand
  includes, so included specification fences are outside that guarantee
  (`AGENTS.md:254-270`).

CI runs the four documentation checks in its book job and requires aggregate
success before publishing (`.github/workflows/ci.yml:159-183,320-369`). No
empty-WIP, ratification or automatic gardening gate was found in the inspected
tracked workflow, justfile or Claude session hook. The hook provisions tools;
it does not promote documents. Gardening remains an evidence-driven editorial
process, not an automatic consequence of CI success.

The separately installed `sdd-gardening` skill was read as workflow background,
not invoked. Its local `SKILL.md` requires green implementation tests,
feature-scoped spec/plan reconciliation, and preservation of unrelated WIP. It
is not pinned RIDL repository policy, and its generic non-empty-WIP language
must not be misreported as RIDL's implemented CI rule.

## Upskill's current position

Upskill already distinguishes user guidance, behavior specification, portable
format and ADRs (`README.md:76-84`; `docs/SUMMARY.md:5-17`). Its conventions are
explicitly informative (`docs/conventions.md:3-18`). The structural problem is
that `book.toml:4` makes all of `docs/` the publication source, while durable
contracts, ADRs, working plans and the new research sit beneath it. SUMMARY
selects chapters, but publication isolation should also consider copied assets
and links, not just the visible table of contents.

The 25 files in `docs/superpowers/` comprise 12 specs and 13 plans, without a
directory lifecycle index. Their headers use Draft, Approved, implementation
pending, or no status. These are historical claims requiring reconciliation.
For example, the generic Git URL design says implementation planning is pending
(`docs/superpowers/specs/2026-06-10-generic-git-url-source-design.md:3-6`), while
an Accepted ADR and `GitRepo` implementation exist
(`docs/adr/0011-generic-git-url-source.md:3,20-34`; `src/source.rs:15-27`). Its
plan also cites a nonexistent WIP path (`docs/superpowers/plans/
2026-06-10-generic-git-url-source.md:5`). This is an archive candidate requiring
completion evidence, not a declaration that every acceptance criterion passed.

Authority needs review independently of movement: `docs/format-spec.md:3-5`
says Draft, and ADR-0009 remains Proposed despite dependency implementation and
tests (`docs/adr/0009-coupling-tiers-and-dependencies.md:3`;
`tests/cli_requires.rs:25,100,153`). Migration must not silently ratify either.
AGENTS' ADR inventory stops at 0005 although the index reaches 0012
(`AGENTS.md:98`; `docs/adr/README.md:5-17`).

Upskill requires docs alongside implementation and `just fmt`/`just verify`
before PRs (`AGENTS.md:238-249`; `CONTRIBUTING.md:64-70`). However, the current
`verify` dependency chain has no book or link check (`justfile:12-33,49-55`).
The separate Pages workflow builds only on matching pushes to main, while the
PR workflow has no mdBook job (`.github/workflows/pages.yml:3-9,29-42`;
`.github/workflows/ci.yml:12-85`). These are pre-merge coverage gaps, not evidence
that today's book fails.

## Recommended destination map

Adopt RIDL's authority and lifecycle distinctions. Keep `docs/adr/` as the
decision home initially: renaming it to `decisions/` adds link churn without
changing authority. The following destinations are proposals, not current paths.

| Existing source                                                      | Proposed destination or treatment                                                                                                                                                                           |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/{intro,getting-started,commands,recipes,conventions}.md`       | `docs/book/` with the same basenames and published chapter URLs. Keep conventions explicitly informative.                                                                                                   |
| `docs/specification.md`                                              | `docs/specification/upskill-specification.md`, owning tool behavior. A thin `docs/book/specification.md` publishes it.                                                                                      |
| `docs/format-spec.md`                                                | `docs/specification/portable-format.md`, owning the on-disk contract. A thin `docs/book/format-spec.md` preserves its book URL. Resolve Draft status separately.                                            |
| `docs/adr/*.md`                                                      | Keep IDs, filenames, statuses and rationale. Expand the existing index to show status and amendments; reconcile implementation-dependent claims.                                                            |
| Architecture scattered through plans and ADR implementation sections | Consolidate current structure into `docs/design/generation-pipeline.md` and, only if substantial, `installation-lifecycle.md`. Retain decisions in ADRs and link them.                                      |
| Explanatory investigations with continuing value                     | `docs/technotes/<topic>.md`, explicitly informative; avoid another copy of the specifications.                                                                                                              |
| `docs/superpowers/specs/*.md`, `plans/*.md`                          | First inventory; then active material to `docs/wip/specs/` and `docs/wip/plans/`; evidenced completed material to matching subdirectories under `docs/archive/`. Preserve basenames and pair relationships. |
| `docs/research/2026-10-11-agent-customization/`                      | Eventually `docs/wip/research/2026-10-11-agent-customization/` as one active evidence pack. No move in this research task.                                                                                  |
| `docs/SUMMARY.md`, `book.toml`                                       | Move SUMMARY to `docs/book/`; change source root and edit-link template together, retaining the existing published URLs.                                                                                    |

Each populated lifecycle directory should have a short README defining
authority, navigation and transitions. A top-level `docs/README.md` should route
readers by purpose. Avoid filling empty directories merely for symmetry.

For the Superpowers backlog, keep unknown or partial work active and labelled.
Pair generic Git URL work with ADR-0011 for the first bounded reconciliation;
review coupling slices together with ADR-0009, and MCP work with ADR-0010.
The MCP v2 document explicitly scopes a documentation-only deliverable
(`docs/superpowers/specs/2026-06-03-mcp-v2-convention-design.md:9-11`), so its
evidence is the delivered recipe/skill and their validation, not an invented
Rust implementation requirement. A plan without its referenced design, such as
the plugin descriptor plan's missing spec (`2026-05-23-plugin-descriptor-modes.md:11`),
needs provenance recovery or an explicit gap, not a fabricated replacement.

An archive disposition should name the delivered or superseding commit/PR,
relevant implementation and tests, reconciled durable records, residual work
and its owner/tracker. Preserve raw reasoning and record link repairs or dated
corrections in the archive index. Extract unfinished scope into active WIP with
provenance before archiving its completed predecessor. Neither age, approval,
checkboxes nor an existing ADR substitutes for this evidence.

## The new research pack

The six target reports, standards comparison, design inputs and schema
snapshots are research evidence for the detailed design still to be decided.
The new Accepted ADR-0013 already records the six target families, intended
Codex implementation and deferred Cursor/Gemini CLI implementation
(`docs/adr/0013-agent-customization-targets.md:3,24-33`). Preserve that decision
in the ADR home; do not demote it to WIP or reopen it during migration.
The research pack's current headers
already distinguish research from supported behavior (`design-inputs.md:3-13`;
`schemas/README.md:3-9`). Under RIDL's pattern, WIP research is the appropriate
home while decisions are open. The installed gardening skill's feature-pair
workflow does not authorize automatically archiving this independent directory.

Keep reports, `schemas/manifest.json`, schema bytes and licenses together so
relative links and provenance remain meaningful. Vendor contracts are external
evidence, not upskill's normative format or generated output. Later, approved
choices should become ADRs and specification amendments; implemented support
should update design records and book guidance. A maintained comparison may
become a dated technote. Retire the research snapshot only after its evidence
has been consumed or explicitly superseded, retaining the whole pack and a
successor link. Freshness review remains necessary for external client facts.

## Guidance and migration sequence

Proposed AGENTS guidance would explain where a task starts: specifications and
accepted ADRs for contracts, code/tests plus design records for implementation,
WIP for unresolved decisions, and the book for supported user behavior. It would
describe feature-scoped gardening after validation and require explicit handling
of residual work. This is a description of future guidance, not an applied
rule or a request to empty WIP. README would provide a compact documentation map;
CONTRIBUTING would explain transitions and the actual documentation checks.

1. **Inventory before movement.** Add the documentation map and a disposition
   ledger with source, status, evidence, owner and intended durable record.
   Record uncertain statuses and missing references. Do not ratify drafts as
   part of tidying paths.
2. **Protect publication.** Add a temporary-copy book check and relative-link
   check to local verification and PR CI before broad moves. Exercise missing
   chapter/include and broken-link failures. Preserve the current published
   page/anchor inventory as migration acceptance criteria.
3. **Separate the book and references.** Move the five guide pages; introduce
   canonical specification sources and thin book wrappers; update SUMMARY,
   `book.toml`, edit links, repository links, source citations and skill
   references in one change. Verify included-source links after rendering.
   Preserve old GitHub entry paths with small forwarding pages where useful;
   preserve published filenames and anchors, adding explicit redirects for any
   unavoidable URL changes.
4. **Garden one feature at a time.** Reconcile each evidenced spec/plan group,
   write only necessary durable records, then archive raw sources with indexed
   provenance. Uncertain work and the active research pack remain active. Move
   the pack only in the agreed migration, preserving schema bytes and digests.
5. **Close the verification gap.** Run `just fmt` and `just verify`, plus the
   new book/link gates. Inspect rendered navigation, reference links, old URLs,
   and edit links; ensure research and archive assets are outside the published
   output. Add bare-path checking if needed, with historical exceptions made
   explicit. Copy RIDL's story-name gate only if upskill develops that problem.

The first deliverable should therefore be an indexed lifecycle and a working
pre-merge documentation gate. Bulk archival and ADR renaming are unnecessary
prerequisites.
