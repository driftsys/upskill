# Customization delivery: implementation handoff specification

Date: 2026-10-23. Status: implementation handoff, slice 1 approved for
execution. Basis: the
[customization delivery proposal](2026-10-23-customization-delivery-proposal.md)
with its three amendments, accepted by the user on 2026-10-23 as the
implementation contract. The durable records named in that proposal's "Records
superseded or amended" table still carry their pre-amendment text; amending them
is part of this work (slice 1, step 7).

Audience: the implementing agent that takes over this branch. It knows the
repository conventions in `AGENTS.md`. It did not take part in the review
conversation, so every decision it needs is written here.

## Intent

Make the proposal implementable by first closing the native qualification gaps
that its per-client tables depend on, then packaging content on the routes that
passed. The proposal's tables cite pass and fail controls per row; several rows
still rest on documentation alone. No product code that changes production
routing is written until the controls that gate that code have run.

## Goals, non-goals, anti-goals

Goals:

- Every route in the proposal's rules, skills and subagents tables has a pinned
  pass, a pinned fail, or an explicit "unsupported" result before an adapter is
  written for it.
- Each control is retained the way the existing Gate 6 packets are retained, so
  the proposal's evidence section can cite it.
- The durable records listed in the proposal are amended once, after slice 1,
  from evidence rather than from documentation.

Non-goals for this handoff:

- The updater plugin, bundles as an authorable type, MCP management, hooks,
  tools and permissions as managed categories. They keep their roadmap place.
- Linux, IDE and hosted surfaces. Record them as unqualified; do not test them
  in slice 1.
- A new include or composition syntax for `RULE.md`, `SKILL.md`, `AGENT.md`.
- Lockfile schema 2 activation. Production routing stays schema 1 throughout.

Anti-goals, which make the work a failure even if tests pass:

- Any write to `CLAUDE.md`, `AGENTS.md` or a nested equivalent, including the
  shipped `@AGENTS.md` bridge creation.
- Any write into a client-owned plugin cache.
- A negative control made to pass by disabling or editing the user's or team's
  existing discovery settings.
- A test model identifier in any committed document.
- A documentation claim promoted to "qualified" without a retained pinned run.

## Constraints

- Single static Rust binary, no async runtime, no `git2`, dependency policy of
  ADR-0001. Client-side adapters may use the client's own supported runtime
  (shell for Claude, Codex and Copilot hooks; the OpenCode plugin API in
  JavaScript) and nothing else. No shared service or daemon.
- Installation is copy-only. No symlinks into or out of the managed root.
- The managed root must not be, or sit under, any documented client discovery
  path: `.claude/`, `.github/`, `.codex/`, `.opencode/`, `.agents/skills`,
  `.agents/plugins`, `AGENTS.md`, `CLAUDE.md` and their user-home equivalents.
  It must not match any `instructions`, `skills` or plugin glob already present
  in project or user client configuration.
- Each logical item has exactly one delivery route per intended client. A plan
  that selects both a native route and a package route for the same item on the
  same client is rejected.
- Reuse the existing library foundations without bypass: captured source
  evidence (AD-0019, AD-0020, AD-0030), complete selected-set preflight
  (AD-0027), qualified roots and locking (AD-0028), explicit file permissions
  (AD-0031), recovery checkpoints and journals (AD-0032, AD-0033), owner
  contracts (AD-0034). The 65 focused tests cited in the proposal's evidence
  assessment must keep passing.
- Repository workflow from `AGENTS.md`: own worktree per branch under
  `.claude/worktrees/<branch>`, acceptance tests before unit tests, Conventional
  Commits, `just fmt` then `just verify` before a PR, one PR per task, squash
  merge. Every task issue starts with `Epic: #283` and carries `task` plus
  `epic:<name>` labels.

## Slice 1: qualification of the gating routes

Each control below follows the method of the retained Gate 6 packets
(`docs/technotes/gate6-*.md`): disposable Git repositories with distinct root
commits, unpredictable positive, absent and fallback markers per control,
isolated client profiles with the user's normally provisioned authentication,
child environments stripped of inherited `GIT_*` and client override variables,
exact final-response matching, structured native event capture, and failed
positives retained as failures rather than counted as negative passes. Reuse
`scripts/qualify-native-instructions.py` and `scripts/qualify-plugin-rules.py`
patterns; extend them rather than writing a third harness where the fixture
shape allows. Offline fixture tests under `scripts/tests/` run in `just lint`
and never call a client.

Pins (my call): open a new campaign revision in
`docs/technotes/gate6-plugin-profile-inventory.md` that pins the currently
installed releases of all four clients before the first control runs. Every new
control runs on those pins. Retained cells from the 2026-10-19 campaign are not
re-run unless a step 7 decision depends on them. A table that mixes pins is
acceptable when every row names its pin; a row without a pin is not evidence.
Platform: macOS arm64 only in slice 1.

Harness scope (my call): slice 1 ships offline fixture and classification tests
per control under `scripts/tests/`, run by `just lint`. Native capture helpers
may stay private and unshipped, as PRs #347 to #380 did, provided the packet
retains receipts and hashes. A shipped automated native runner with timeout and
interruption acceptance tests is not part of slice 1.

Local installation verbs (my call): when a pinned CLI has no local-path install
verb, the control uses a local marketplace entry, as `src/plugin.rs` already
does for Claude and Copilot. For OpenCode the owned `plugins` array and the
explicit skill-source entry are the registration route; "no local install verb"
is recorded as a result, not as a failed control.

Sequencing: PR #381, including this document, merges to `main` before the first
Q issue opens. Until step 7, each technote names the proposal row it affects by
linking the proposal under `docs/wip/specs/`; step 7 repoints those links to the
amended durable records when it gardens the proposal.

| Id | Client and scope                         | Control                                                                                                                                                                                                                                                             | Passes when                                                                                                                                                 | Decision it unlocks in the proposal                                                                                               |
| -- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Q1 | Copilot CLI, project                     | Positive: marker in `.github/copilot-instructions.md`. Candidate: a different marker only in `.claude/rules/probe.md`, no `.github/instructions`. Also run with a `paths:` frontmatter block present.                                                               | Positive returns its marker. Candidate result is recorded either way; "reads" or "does not read" is the output.                                             | Rules section, `.claude/rules` gate: which of options (a), (b), (c) applies when Claude and Copilot share a project.              |
| Q2 | Copilot CLI, project                     | Claude-format agent at `.claude/agents/probe.md` with a restricted `tools` list and a `permissionMode`. Check the Copilot agent catalog, invoke it, and record which tools the child could use.                                                                     | Catalog and invocation outcomes are recorded; a tool outside the Claude `tools` list being used is a FAIL for the "exception route is safe" claim.          | Subagents section: whether the native-file exception for restricted Claude subagents is usable in projects that also use Copilot. |
| Q3 | Copilot CLI, project and user            | Local plugin with `com.github.copilot/rules/<name>.md` (unscoped and `applyTo` scoped) and `com.github.copilot/agents/<name>.agent.md`. Install from a local path, run positive, disable, uninstall, reinstall; matching and nonmatching file access for the scope. | Rule marker present after install, absent after disable and uninstall; scoped rule activates on matching access and not on nonmatching; agent is delegable. | Rules and subagents tables: promote the Copilot plugin components from candidate to supported, or record them unsupported.        |
| Q4 | Claude Code, project                     | Local marketplace plugin whose `SessionStart` hook injects one unscoped rule from a package-owned rule index. Positive, disable, uninstall, resume, one compaction.                                                                                                 | One rule event per fresh session and per resume; none after disable or uninstall; rule present after compaction.                                            | Rules table: the Claude hook exception route for unscoped rules.                                                                  |
| Q5 | OpenCode, current 2.1.x release, project | Re-run the 2.0.32 plugin context-hook control, the package skill directory registered as an explicit skill source with a resource read, and `plugin add` with a local directory.                                                                                    | Context hook: one event per rule, resume included. Skill: native invocation reads the package resource. `plugin add` behavior recorded either way.          | Every OpenCode row at the current pin; closes the "2.1.4 execution blocked" gap in the evidence assessment.                       |
| Q6 | Claude Code, project and user            | Managed root outside all discovery paths, exposed through a local marketplace. Install at project scope, then user scope. Record the actual load path, the namespaced skill name, update behavior after editing the package, and settings bytes after uninstall.    | Load path, namespace, reinstall requirement and byte residue are recorded; residue beyond empty objects is a FAIL for the preservation contract.            | Storage section: installation mechanics and the residue contract; Packaging section: namespacing.                                 |
| Q7 | Codex CLI, trusted project               | Package-owned agent TOML referenced from `.codex/config.toml` as `agents.<name>.config_file`. Invoke a real delegated child; attempt an operation the parent sandbox forbids. Separately place the same TOML inside an installed plugin and check discovery.        | Child runs, inherits the parent boundary (forbidden operation denied), plugin-contained TOML is not discovered unless registered.                           | Subagents table, Codex row.                                                                                                       |
| Q8 | Codex CLI, project                       | Local marketplace plugin with one skill and one resource. Install, invoke, edit the package, invoke again without reinstall, then `plugin upgrade` or remove and re-add, then invoke.                                                                               | Namespaced identity recorded; whether edits are visible without reinstall is recorded; reinstall restores the edited bytes.                                 | Storage section: update loop for cache-copying clients.                                                                           |
| Q9 | Codex CLI, project                       | Resolve [#379](https://github.com/driftsys/upskill/issues/379): resume plus two automatic compactions in one session with two plugin rules; count developer rule bodies, not callbacks.                                                                             | Exactly one developer rule body per logical rule after resume and after each compaction. Two bodies for one rule is a FAIL, whatever the callback count.    | Rules table, Codex row. While #379 is open the Codex rules row cannot be amended to supported in step 7.                          |

Order: Q1, Q2 and Q6 first, because they decide the shape of the rules and
subagents tables for the client pair most teams run together. Then Q9, because
#379 is K0 and blocks the Codex rules row. Then Q3, Q4, Q5. Then Q7, Q8.

Steps per control:

1. Open a task issue (`Epic: #283`, `task`, severity and effort labels) with the
   control's fixture, markers policy, pin and pass criteria from the table.
2. Write the offline fixture and classification tests first; `just lint` must
   fail before the harness change and pass after.
3. Run the native control. Retain the packet under
   `docs/technotes/evidence/<YYYY-MM-DD>-<slug>/` with `summary.json`
   (`qualified`, controls, counts, limits), `receipt-hashes.json` and
   `private-source-index.json`, following the existing packets. Raw transcripts
   and private source stay outside the repository.
4. Write `docs/technotes/gate6-<slug>.md`: method, pins, each control's PASS or
   FAIL, limits, and the exact proposal row it affects. Add it to
   `docs/technotes/README.md`.
5. Open one PR per packet ("docs: retain ..."), as PRs #347 to #380 did.
6. Restore the client profile to its pre-control configuration and verify the
   bytes; record any residue in the technote.
7. After Q1 to Q8 have results, open one PR that amends the durable records in
   the proposal's "Records superseded or amended" table: ADR-0003 output table,
   specification §3.4, AD-0016 to AD-0018 and AD-0039 status, Topic 0001 §7,
   ADR-0012 ancillary note, roadmap MVP table and implementation dependencies,
   AD-0040 and Topic 0009 amendment sections, the 2026-10-14 route mapping. Each
   amendment cites the packet that justifies it. Rows whose control failed are
   amended to "unsupported", not left open.

8. Open one code PR after step 7 and before slice 2 (user decision, 2026-10-23):
   delete `CLAUDE.md` bridge creation in `src/ancillary.rs` and the
   `instructions: [".agents/rules/**/RULE.md"]` write into `opencode.json`, with
   no fallback or migration (pre-1.0 policy). Update the unit and CLI tests,
   specification §3.4 and the user guide. Existing generated bridge files and
   glob entries are reported by `doctor`, never deleted automatically. The
   `.vscode/settings.json` write stays until the VS Code profile decision (my
   call; VS Code is deferred).

Slice 1 is complete when every table row above has a retained PASS, FAIL or
"unsupported", step 7 has merged and the step 8 PR has merged.

## Slice 2 outline: skills packaging on passed routes

Entry criteria: Q5, Q6 and Q8 retained. Scope: a pure package generator for
skills and resources, one managed root, local marketplace registration through
the client CLIs already shelled out to in `src/plugin.rs`, and lifecycle (add,
update, remove, recovery) over the existing inactive owned-artifact planning.
Clients: Codex and OpenCode first (passed routes), then Claude. Claude enters
slice 2 when Q6 leaves no residue beyond empty `enabledPlugins` and
`extraKnownMarketplaces` objects, and that residue is declared in the
preservation contract; any other residue blocks Claude until it is fixed or
contracted (my call). Copilot follows once Q3 passes. Production routing stays
schema 1; the package route is reached only through the existing opt-in
captured-source lifecycle entry points.

Acceptance for slice 2 (to be refined in its own plan):

- Generating a package twice from the same captured source yields identical
  bytes.
- Installing a package exposes exactly one native skill per logical item per
  client, verified by native catalog listing, not by file presence.
- Remove disables the registration before reclaiming files and leaves every byte
  outside the managed root and the owned registration entry unchanged.
- An interrupted install or remove is recoverable from the journal without
  inspecting the client cache.

Slices 3 (rules) and 4 (subagents) follow the same shape and are planned after
slice 1 decides their routes.

## Decisions already made

- Native rule routes stay the default for Claude and Copilot. Package hooks are
  the Codex and OpenCode route. The Claude hook and Copilot plugin component are
  exceptions or candidates until Q3 and Q4 pass.
- Self-contained per-client packages are the default for skills and subagents.
  `.agents/skills` is an optional qualified profile and must exclude Claude.
- Subagents are in the MVP. Models and effort resolve through ADR-0014.
- Nothing is written to `CLAUDE.md` or `AGENTS.md`. The shipped bridge file is
  retired by the step 7 amendment; existing generated bridges are left in place
  and reported, never deleted automatically.
- Hooks for Codex require the user's `/hooks` trust review. The implementation
  never bypasses it and documents it as a user step.
- Marketplace: one marketplace entry per client package. A single multi-manifest
  package is not installed into two clients unless a negative control shows the
  second client ignores it.

## Left to the builder

- Exact managed root path, subject to the exclusion list above.
- Package granularity (per item or per resolved bundle), as long as the planner
  deduplicates logical items before export.
- The deterministic rule index format inside a package.
- Harness extension versus new script, per control.
- Issue slicing inside slice 1 beyond one issue per control.

## Open questions for the user

None blocking slice 1. Decided on 2026-10-23: the `CLAUDE.md` bridge and the
`opencode.json` glob are removed in a separate code PR (step 8). Two questions
surface at step 7 and are flagged there:

- Whether existing installations that carry managed `AGENTS.md` regions from the
  experimental compiler get an explicit removal command or are only reported.
- Whether the roadmap moves the updater plugin after skills packaging or keeps
  it before, once slice 2's entry criteria are known.

## References

- [Proposal](2026-10-23-customization-delivery-proposal.md)
- [AD-0040](../../adr/0040-separate-customization-delivery-from-discovery.md)
  and [Topic 0009](../../topics/0009-customization-delivery.md)
- [Gate 6 pinned profiles](../../technotes/gate6-plugin-profile-inventory.md)
- [Gate 6 native qualification method](../../technotes/m1-gate6-native-qualification.md)
- [Evidence assessment](../../technotes/evidence/2026-10-23-delivery-choice/assessment.md)
- Issues: [epic #283](https://github.com/driftsys/upskill/issues/283),
  [task #337](https://github.com/driftsys/upskill/issues/337) (plugin rule
  delivery and isolation, still open; Q3 to Q5 extend it)
