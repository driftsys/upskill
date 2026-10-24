# Customization delivery: implementation handoff specification

Date: 2026-10-23; rules direction adopted by the user on 2026-10-24 after Q1-Q9
review. Basis: the proposal's [native-first always-on rules decision][decision]
is decided, not an option. Other direction amendments remain unadopted. This WIP
records the required durable amendments but changes no code, durable record or
production route. [AD-0040][direction], [Topic 0009][contract] and the
[roadmap][roadmap] keep their current durable status until step 7.

Step 7 adoption owner: the user. At that PR review the user confirms the durable
wording for the decided native-first rules direction, the #379 K1 debt re-scope
and independent-of-team-guidance OpenCode route. Separately adopt or reject the
pending changes to package defaults, subagents in MVP, authorable bundles,
explicit OpenCode skill sources. Record the decided implementation order. A
native PASS cannot adopt those direction changes ([decision][decision],
[AD-0040 proposed amendments][direction], [roadmap][roadmap]).

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

- Every selected route has pinned evidence or an explicit pending/unsupported
  disposition. Always-on rules have priority; retain the accepted Codex
  limitation separately from Q9's former zero-duplicate FAIL. Failed positives
  leave downstream controls not run, not negative passes ([decision][decision],
  [CD-10][contract], [Q9][Q9], [Q5][Q5]).
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

Local package installation verbs: when a pinned CLI has no local-path install
verb, the control uses a local marketplace entry, as `src/plugin.rs` already
does for Claude and Copilot. For the OpenCode fallback the owned `plugins` array
and explicit skill-source entry remain candidate package registrations;
always-on rules instead use explicit instruction file paths after Q5 retry PASS
([decision][decision], [Q5][Q5]); "no local install verb" is recorded as a
result, not as a failed control.

Sequencing: PR #381, including this document, merges to `main` before the first
Q issue opens. Until step 7, each technote names the proposal row it affects by
linking the proposal under `docs/wip/specs/`; step 7 repoints those links to the
amended durable records when it gardens the proposal.

| Id | Client and scope                         | Control                                                                                                                                                                                                                                                                       | Passes when                                                                                                                                                                                                                                                                                                        | Decision it informs in the proposal                                                                                                                                                                                              |
| -- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1 | Copilot CLI, project                     | Positive: marker in `.github/copilot-instructions.md`. Candidate: a different marker only in `.claude/rules/probe.md`, no `.github/instructions`. Also run with a `paths:` frontmatter block present.                                                                         | Positive returns its marker. Candidate result is recorded either way; "reads" or "does not read" is the output.                                                                                                                                                                                                    | Establishes the duplicate vector. The decided route declares Copilot an intended reader of the one unscoped Claude file ([Q1][Q1], [summary][Q1 summary], [decision][decision]).                                                 |
| Q2 | Copilot CLI, project                     | Claude-format agent at `.claude/agents/probe.md` with a restricted `tools` list and a `permissionMode`. Check the Copilot agent catalog, invoke it, and record which tools the child could use.                                                                               | Catalog and invocation outcomes are recorded; a tool outside the Claude `tools` list being used is a FAIL for the "exception route is safe" claim.                                                                                                                                                                 | Subagents section: whether the native-file exception for restricted Claude subagents is usable in projects that also use Copilot.                                                                                                |
| Q3 | Copilot CLI, project and user            | Local plugin with `com.github.copilot/rules/<name>.md` (unscoped and `applyTo` scoped) and `com.github.copilot/agents/<name>.agent.md`. Install from a local path, run positive, disable, uninstall, reinstall; matching and nonmatching file access for the scope.           | Rule marker present after install, absent after disable and uninstall; scoped rule activates on matching access and not on nonmatching; agent is delegable.                                                                                                                                                        | Rules component FAIL at both scopes and retired at 1.0.103. Project plugin agent activation unverified; bounded user child does not qualify restrictions ([Q3][Q3], [summary][Q3 summary], [decision][decision]).                |
| Q4 | Claude Code, project                     | Local marketplace plugin whose `SessionStart` hook injects one unscoped rule from a package-owned rule index. Positive, disable, uninstall, resume, one compaction.                                                                                                           | One rule event per fresh session and per resume; none after disable or uninstall; rule present after compaction.                                                                                                                                                                                                   | Rules table: PASS, unscoped only; no scoped or effective-input uniqueness claim ([Q4][Q4], [summary][Q4 summary]).                                                                                                               |
| Q5 | OpenCode, current 2.1.x release, project | Retry explicit `instructions` file paths with no glob at 2.1.4, including install/update/remove, resume and resource identity; run native positive first. Then test package skill/resource loading and the fallback context hook separately ([decision][decision], [Q5][Q5]). | Explicit paths deliver the intended always-on rules with measured native identity, one activation per rule and preserved team/configuration bytes. Package skill/resource and fallback hook each need their own PASS; failed positive blocks downstream tests ([decision][decision], [CD-02 and CD-10][contract]). | Explicit-path rules PASS gates step 8 glob removal; skill PASS gates OpenCode slice 2. Prior positives FAIL and component tests not run ([Q5][Q5], [summary][Q5 summary], [decision][decision]).                                 |
| Q6 | Claude Code, project and user            | Managed root outside all discovery paths, exposed through a local marketplace. Install at project scope, then user scope. Record the actual load path, the namespaced skill name, update behavior after editing the package, and settings bytes after uninstall.              | Load path, namespace, update visibility and measured settings/registry residue recorded. Q6 PASS includes persistent native cache, not whole-profile equality ([Q6][Q6], [summary][Q6 summary]).                                                                                                                   | Storage section: installation mechanics and the residue contract; Packaging section: namespacing.                                                                                                                                |
| Q8 | Codex CLI, project                       | Local marketplace plugin with one skill and one resource. Install, invoke, edit the package, invoke again without reinstall, then `plugin upgrade` or remove and re-add, then invoke.                                                                                         | Namespaced identity recorded; whether edits are visible without reinstall is recorded; reinstall restores the edited bytes.                                                                                                                                                                                        | Storage section: update loop for cache-copying clients.                                                                                                                                                                          |
| Q7 | Codex CLI, trusted project               | Package-owned agent TOML referenced from `.codex/config.toml` as `agents.<name>.config_file`. Invoke a real delegated child; attempt an operation the parent sandbox forbids. Separately place the same TOML inside an installed plugin and check discovery.                  | Child runs, inherits the parent boundary (forbidden operation denied), plugin-contained TOML is not discovered unless registered.                                                                                                                                                                                  | Subagents table, Codex row.                                                                                                                                                                                                      |
| Q9 | Codex CLI, project                       | Retain Q9's reopen/compaction observations and one resume-only follow-up under #379: reopen a positive session, send a turn without compaction and count exact developer bodies per logical rule. Preserve ordinary trust ([Q9][Q9], [decision][decision]).                   | Record separate resume-only body counts and correlate native session/turn identity to attribute the duplicate. Old zero-duplicate criterion remains FAIL in Q9; the user accepts its declared transient limitation. Failed positive leaves attribution blocked ([Q9][Q9], [decision][decision]).                   | Step 7 records supported with declared limitation once resume-only evidence is retained; #379 becomes K1 debt, not a route blocker. Do not rewrite Q9's historical FAIL ([Q9][Q9], [summary][Q9 summary], [decision][decision]). |

Remaining slice-1 execution order: **Q5 retry, Codex resume-only control (Q9
follow-up), Q8, Q7**. Q5 must separately qualify explicit instruction paths and
skill/resource delivery; a rule PASS alone does not pass skills packaging
([decision][decision], [Q5][Q5], [Q9][Q9], [CD-10][contract]).

Q1b (`.claude/rules` with `paths:` plus matching-file access under Copilot) is
deferred, unscheduled, and not a gate for always-on delivery. Q3b, a Copilot
plugin session-start hook test using Q4's fixture shape, is also deferred and
unscheduled. Q3 tested the failed rules component, not this hook
([decision][decision], [Q1][Q1], [Q3][Q3], [Q4][Q4]).

Q10 and Q11 are slice-2 entry controls, not additional scheduled slice-1 work
([decision][decision], [CD-03 and CD-04][contract]):

| Id  | Entry control                                                                                               | PASS/FAIL                                                                                                                                                                                                                              |
| --- | ----------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q10 | Same package at user and project scope on one pinned client, in one workspace                               | PASS: one catalog entry and one activation per logical item. FAIL: duplicate exposure or activation; failed native positive blocks the result. Q6's separate projects do not prove overlap ([Q6][Q6], [CD-04][contract])               |
| Q11 | Another client pointed at a Claude or Copilot package, with relevant native catalog and explicit invocation | Require a successful positive on each client being tested as an unintended reader. PASS: no unintended activation; FAIL: activation. Do not edit existing discovery settings to manufacture the negative ([CD-03][contract], [Q1][Q1]) |

Current campaign pins are Claude 2.1.308, Copilot 1.0.103, Codex 0.164.0 and
OpenCode 2.1.4 on macOS arm64. Follow-ups name their actual pin and open a new
campaign revision if it changes. Q7/Q8 remain pending packet review here;
older-pin results do not qualify current execution ([Q6][Q6], [Q5][Q5],
[CD-10][contract]).

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
7. After Q1-Q9 have retained results, including the Q5 retry and Codex
   resume-only follow-up, open the separate durable-record PR. The user owns
   adoption at this PR review. List every amendment from the proposal's
   superseded-records table, including AD-0040/Topic 0009/roadmap's plugin-only
   wording, shared Claude/Copilot native files and declared readers, bounded
   Codex duplicate acceptance, #379 K1 debt, and the OpenCode explicit-path
   transition. Preserve the distinction between adopted direction, retained
   observation and qualified implementation ([decision][decision],
   [AD-0040][direction], [CD-10][contract]).

   The Codex outcome is **supported with declared limitation** once the
   resume-only control is retained. The user accepts the transient duplicate
   after resume followed by compaction until the next compaction; the
   implementing maintainer retains the attribution run and updates the debt
   record at step 7. #379 is re-scoped from K0 blocker to K1 debt, with that one
   remaining control. This docs-only decision does not claim the live issue
   labels or durable records have already changed, or turn Q9's original FAIL
   into a zero-duplicate PASS ([decision][decision], [Q9][Q9],
   [summary][Q9 summary]).

   Direction adoption is separate from evidence-driven route amendments. The
   user approves or rejects the remaining package-default, subagent/MVP,
   authorable-bundle and OpenCode skill-source amendments at the step 7 PR;
   always-on native-first rules and the stated implementation order are decided
   ([decision][decision], [AD-0040][direction], [roadmap][roadmap]).

8. After step 7 and a **retained Q5 retry PASS for explicit instruction file
   paths**, open the separate code PR removing `CLAUDE.md` bridge creation in
   `src/ancillary.rs` and the `opencode.json`
   `instructions: [".agents/rules/**/RULE.md"]` glob write. No fallback or
   migration code is added to that removal. The separately qualified OpenCode
   hook remains a delivery fallback category, not a migration mechanism
   ([decision][decision], [Q5][Q5],
   [ADR-0003](../../adr/0003-generation-pipeline.md)).

   Existing generated bridges and glob entries are reported by `doctor`, never
   automatically deleted. Update tests, specification §3.4 and usage in that
   future code PR. This is required future behavior, not a claim about the
   current doctor. Keep the VS Code ancillary write until its deferred profile
   decision ([decision][decision], [design ledger][ledger],
   [shipped ancillary contract](../../specification/specification.md)).

Slice 1 records a retained outcome for every scheduled row even when it FAILS.
Implementation completion additionally needs the scheduled retries, step 7
adoption and merged amendments, and the authorized step 8 code PR. Deferred
Q1b/Q3b do not block always-on work. Production remains schema 1
([decision][decision], [CD-10][contract]).

## Stack and merge order

The packet stack has two dependency chains, rather than independent PRs against
main ([design ledger][ledger]):

- [385](https://github.com/driftsys/upskill/pull/385) <-
  [386](https://github.com/driftsys/upskill/pull/386) <-
  [394](https://github.com/driftsys/upskill/pull/394): merge Q1, then Q2, then
  Q3.
- [385](https://github.com/driftsys/upskill/pull/385) <-
  [387](https://github.com/driftsys/upskill/pull/387) <-
  [389](https://github.com/driftsys/upskill/pull/389),
  [392](https://github.com/driftsys/upskill/pull/392): merge Q1, then Q6, then
  Q9 and Q4. Q5 [396](https://github.com/driftsys/upskill/pull/396) follows Q4
  ([Q5][Q5]).

After each squash merge, update dependent branches/base targets and rerun the
required checks before their merges. A change request on #385 can affect both
chains. A Q packet is **done** only when its packet is merged to main and its
technote is indexed, regardless of PASS or FAIL. This administrative done status
is separate from native route qualification, completion flags inside historical
packets and closure of #379 or a retry issue ([design ledger][ledger],
[CD-10][contract]).

## Slice order and required amendment

The user selects implementation order slice 2 skills, then slice 3 always-on
rules. It reverses the [roadmap][roadmap], [AD-0040][direction] and
[CD-11][contract] rules-before-new-owned-skills dependency. Reason: the native
Claude/Copilot generation already implements most always-on delivery; the
remaining rule work is the shared-reader planning, explicit OpenCode paths and
Codex hook package, while Claude Q6 supplies passed package feasibility. Scoped
rules are secondary and add no work in this slice ([decision][decision],
[ADR-0003](../../adr/0003-generation-pipeline.md), [Q6][Q6]).

The user records that explicit sequencing amendment at the step 7 durable-record
PR. The direction is decided here; no owned-skill implementation starts until
that PR reconciles the unchanged roadmap/CD-11 dependency ([decision][decision],
[roadmap][roadmap], [CD-11][contract]).

## Slice 2 outline: skills packaging on passed routes

Entry criteria: **retained PASS** for each client's applicable package,
invocation and resource controls, plus step 7 adoption and the governing
rules-first dependency unless the user explicitly amends it. Retained failures
do not satisfy entry ([design ledger][ledger], [CD-10 and CD-11][contract]).
Client order: Claude first on Q6 PASS with its bounded residue contract
approved, Codex when Q8 passes, and OpenCode when Q5 skill/resource controls
pass. Require retained PASS for Q10 and Q11 for each applicable package profile.
Copilot package skills require their own passed route; Q3's retired rules
component is not a gate for native always-on rules, and project plugin
activation remains unverified ([decision][decision], [Q6][Q6], [Q5][Q5],
[Q3][Q3], [CD-03 and CD-04][contract]).

Scope remains a pure package generator for skills/resources, one managed root,
local native registration and add/update/remove/recovery over the inactive
owned-artifact planning foundations. Production remains schema 1; no WIP outline
adopts a production route ([AD-0040][direction], [CD-11][contract]).

Required slice-2 tasks cover the proposal's previously unassigned musts:

| Task                             | Required result and acceptance evidence                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Preflight glob/import inventory  | Inventory project/user imports, globs, compatibility and nested roots, explicitly covering `.claude/rules` and `.github/instructions` as well as the managed root. Classify intended/unintended/unresolved readers and reject unsafe profiles before effects; preserve existing discovery settings ([Q1][Q1], [CD-01 and CD-03][contract])                                                                                                      |
| Collaborator project onboarding  | For each client package route, a fresh collaborator starts from committed project registration, performs ordinary install/trust and invokes the intended item once. Native rule files require no per-machine rule install under the decided route. Record offline/denied behavior, native catalog/registration and unrelated-project negative; committed registration alone is insufficient ([decision][decision], [Q6][Q6], [CD-08][contract]) |
| Per-client preservation contract | Specify exact bytes/modes/presence and owned keys, native pre/post registrations, allowed empty residue, persisting cache effects and recovery boundaries per client; reject unrelated changes. Obtain the user's explicit allowance before implementing it ([Q6][Q6], [Q3][Q3], [Q5][Q5], [CD-07][contract])                                                                                                                                   |
| Per-client update visibility     | Test source edit, fresh session, running/resumed session and native refresh separately; document what loads when. Preserve Claude's no-reinstall observation for a combined content-and-version edit and Copilot's live-source report without extrapolating to Codex/Q8 or OpenCode/Q5 ([Q6][Q6], [Q3][Q3], [Q5][Q5], [CD-10][contract])                                                                                                        |

Acceptance for slice 2 (to be refined in its own plan):

- Generating a package twice from the same captured source yields identical
  bytes.
- Installing a package exposes exactly one native skill per logical item per
  client, verified by native catalog listing, not by file presence.
- Remove disables registration before reclaiming files and preserves unrelated
  bytes/modes/keys under the approved per-client contract, recording declared
  native residue separately ([Q6][Q6], [Q3][Q3], [CD-07][contract]).
- An interrupted install or remove is recoverable from the journal plus observed
  native registration state. File-only success cannot prove native deactivation;
  unknown outcomes block replay ([CD-07][contract]).

Slice 3 covers **always-on rules only**: mostly existing native generation plus
shared Claude/Copilot declared-reader planning, explicit OpenCode paths and the
static Codex hook/index package. Retain the Q9 declared limitation. Scoped rules
are not widened or scheduled here. Slice 4 subagents requires its own passed
routes and scope adoption ([decision][decision], [Q1][Q1], [Q5][Q5], [Q9][Q9],
[ADR-0003](../../adr/0003-generation-pipeline.md)).

## Settled constraints and pending direction

- Native-first always-on rules are decided. Scoped rules remain secondary;
  unsupported scopes never widen to always-on ([decision][decision]).
- In a Claude+Copilot project, one unscoped `.claude/rules` file has both
  clients declared per item and no second Copilot native copy. Other surfaces
  remain unresolved readers ([decision][decision], [Q1][Q1], [CD-03][contract]).
- Preserve team guidance and schema-1 production routing. Lockfile ownership
  covers native files and packages; no team-guide fallback, direct cache writes
  or new production schema follows from this decision ([decision][decision],
  [CD-06][contract]).
- Codex uses a trusted hook and static index, never a runtime upskill binary
  call. Its accepted transient duplicate is recorded as a limitation, with
  resume-only attribution retained before step 7's supported-with-limitation
  status ([decision][decision], [Q9][Q9]).
- Decision required: other package defaults, MVP subagents, authorable bundles,
  OpenCode skill sources await user review at step 7; the decided sequencing is
  recorded there ([AD-0040][direction], [roadmap][roadmap]).

## Left to the builder

- Exact managed root path, subject to the exclusion list above.
- Package granularity (per item or per resolved bundle), as long as the planner
  deduplicates logical items before export.
- The deterministic rule index format inside a package.
- Harness extension versus new script, per control.
- Issue slicing inside slice 1 beyond one issue per control.

## Open questions for the user

Native-first always-on rules and the Codex limitation are decided. These
remaining questions go to the user at the step 7 PR, without reopening that
choice ([decision][decision]):

- Decision required: select the Codex distribution channel for hook-bearing
  packages. Under the reviewed contract the public directory does not accept
  them, and a metapowers entry does not itself supply hook trust
  ([design ledger][ledger], [Q9][Q9]).
- Decision required: determine whether changing only the static rule index
  triggers Codex re-trust. Q9 reused unchanged reviewed definitions. After
  normal user review, compare trust state before and after an index-only change,
  complete any native re-review and record activation. Do not bypass `/hooks` or
  assume a content update is a trusted definition change ([Q9][Q9],
  [CD-08][contract], [decision][decision]).
- Decision required: adopt the other direction/release-scope amendments and
  per-client preservation allowances; record the decided implementation order in
  the step 7 PR. Decide removal versus reporting for experimental managed
  regions and updater sequencing separately ([AD-0040][direction],
  [roadmap][roadmap], [CD-06 and CD-07][contract]).

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

## Retained campaign references

These fixed revisions cite the packet branches before merge. PASS and FAIL apply
only to the recorded macOS arm64 controls, not complete client qualification
([CD-10][contract]). Q7 and Q8 remain pending in this reconciliation; no local
unmerged result is promoted here ([CD-11][contract]).

[Q1]: https://github.com/driftsys/upskill/blob/a6166cf60d306f21c73b36688afdc49b57dbf5fd/docs/technotes/gate6-copilot-claude-rules-reader.md
[Q1 summary]: https://github.com/driftsys/upskill/blob/a6166cf60d306f21c73b36688afdc49b57dbf5fd/docs/technotes/evidence/2026-10-24-copilot-claude-rules-reader/summary.json
[Q2]: https://github.com/driftsys/upskill/blob/897876a1ce72bdb6faf8288b883e3dd793470ed6/docs/technotes/gate6-copilot-claude-agents-reader.md
[Q2 summary]: https://github.com/driftsys/upskill/blob/897876a1ce72bdb6faf8288b883e3dd793470ed6/docs/technotes/evidence/2026-10-24-copilot-claude-agents-reader/summary.json
[Q6]: https://github.com/driftsys/upskill/blob/0985dfb43c637996a8030c51bbd0e91fad2fe7eb/docs/technotes/gate6-claude-managed-root.md
[Q6 summary]: https://github.com/driftsys/upskill/blob/0985dfb43c637996a8030c51bbd0e91fad2fe7eb/docs/technotes/evidence/2026-10-24-claude-managed-root/summary.json
[Q9]: https://github.com/driftsys/upskill/blob/8ffde75620ca0b0993490b136f7015c869f7cc10/docs/technotes/gate6-codex-rule-lifecycle.md
[Q9 summary]: https://github.com/driftsys/upskill/blob/8ffde75620ca0b0993490b136f7015c869f7cc10/docs/technotes/evidence/2026-10-24-codex-rule-lifecycle/summary.json
[Q4]: https://github.com/driftsys/upskill/blob/43f7c26c3d64ad55aa5c1f8302ecb1a9573fba81/docs/technotes/gate6-claude-rule-hooks.md
[Q4 summary]: https://github.com/driftsys/upskill/blob/43f7c26c3d64ad55aa5c1f8302ecb1a9573fba81/docs/technotes/evidence/2026-10-24-claude-rule-hooks/summary.json
[Q3]: https://github.com/driftsys/upskill/blob/cd8c605674f34fea13c20233d648dcf16dd29cac/docs/technotes/gate6-copilot-plugin-components.md
[Q3 summary]: https://github.com/driftsys/upskill/blob/cd8c605674f34fea13c20233d648dcf16dd29cac/docs/technotes/evidence/2026-10-24-copilot-plugin-components/summary.json
[Q5]: https://github.com/driftsys/upskill/blob/6ec187f5422d72e9329c4c7f9cb706468e2ebb6b/docs/technotes/gate6-opencode-current-routes.md
[Q5 summary]: https://github.com/driftsys/upskill/blob/6ec187f5422d72e9329c4c7f9cb706468e2ebb6b/docs/technotes/evidence/2026-10-24-opencode-current-routes/summary.json
[contract]: ../../topics/0009-customization-delivery.md
[direction]: ../../adr/0040-separate-customization-delivery-from-discovery.md
[roadmap]: ../../roadmap.md
[ledger]: https://github.com/driftsys/upskill/pull/385#issuecomment-6384387650
[decision]: 2026-10-23-customization-delivery-proposal.md#user-decision-2026-10-24
