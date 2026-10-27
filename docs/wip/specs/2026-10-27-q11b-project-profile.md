# Q11b: Claude project registration observed by Copilot

Task: [#428](https://github.com/driftsys/upskill/issues/428). Status: proposed
concrete native-control profile; execution approval pending. This supplements
the [Q11b control design](2026-10-25-q11-registration-isolation-follow-up.md).
It does not approve a production adapter or replace Q11's failed entry control.

## Concrete execution boundary

- Platform: macOS arm64. Capture fresh Claude, Copilot, Codex and OpenCode
  version receipts before calibration. Current read-only probes report Claude
  2.1.308, Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4.
- Owner: Claude Code, project-scope local marketplace registration. Observer:
  Copilot CLI in that same disposable project, without `--plugin-dir` or any
  candidate registration during baseline, candidate and post-removal phases.
- Fixture: a new mode-0700 directory under `/private/tmp/upskill-q11b-*`.
  Marketplace source stays outside the project and native discovery roots.
  Generate unique plugin/skill identities and private content/absence markers.
  Claude and Copilot calibration packages contain identical skill/resource
  bytes. Content markers never appear in prompts. No team instruction files.
- Use the existing authenticated isolated profiles rooted at
  `/private/tmp/upskill-gate6-claude-auth-20261020` and
  `/private/tmp/upskill-gate6-copilot-auth-20261020`, through the existing
  `fixture_environment` allowlist. Do not use the personal profile, copy
  credentials, renew authentication or change discovery preferences. Acquire
  exclusive profile locks before any native command. A busy lock blocks the run.
  Specifically, call `fixture_environment` with each of the two roots above.
  Claude receives `HOME=<claude-root>/home` and
  `CLAUDE_CONFIG_DIR=<claude-root>/home/.claude`; Copilot receives
  `HOME=<copilot-root>/home` and `COPILOT_HOME=<copilot-root>/home/.copilot`.
  Keep `USER` for native authentication and disable client auto-update.
- Keep model choices and raw streams private. Limit tools to native skill
  invocation, with built-in MCP servers and child execution disabled. Preserve
  the exact untrimmed response rule. A failed positive stops dependent controls.

## Sequence and observations

1. Inventory the isolated user roots, project/ancestor layers, relevant nested
   discovery roots, configured imports/globs, compatibility readers and known
   IDE/cloud surfaces. Observe configuration in memory; retain only non-secret
   discovery facts and digests. Unknown applicable readers block admission.
2. Calibrate Copilot using its own session package selector. Calibrate Claude
   through the project marketplace route described below. Capture catalog
   identity, exact namespaced invocation, correlated native load and response.
   End both processes and remove Claude's calibration registration natively.
3. Start a fresh Copilot process/session after cleanup. Require catalog absence,
   explicit namespaced not-found evidence and no load. Exercise a known alias
   when the native catalog exposes one; an unresolved alias blocks the cell.
4. Register the marketplace with
   `claude plugin marketplace add <source>
   --scope project --json`, then
   `claude plugin install <plugin>@<market>
   --scope project --json`. Check
   the intended owner identity and invocation. Start a fresh Copilot
   process/session after install completion and repeat catalog and invocation
   controls without a foreign selector.
5. Use `claude plugin uninstall <plugin>@<market> --scope project` and
   `claude plugin marketplace remove <market> --scope project`. Start a third
   fresh Copilot process/session after removal completes and repeat the absence
   check. Retain distinct session IDs and operation/startup ordering receipts.

Copilot calibration supplies `--plugin-dir <copilot-native-package>` and the
prompt
`Invoke the skill named <namespace> and follow its instruction. Do not
use any other tools. Return only the required response.`
Require one successful native `skill` tool call whose `arguments.skill` equals
the full namespace, with a matching `toolCallId` completion and the exact
untrimmed content marker response. A bare-name fallback or slash-request
transformation does not pass this profile. Do not retry with a different prompt
after seeing its outcome. Observer absence prompts request the same namespaced
tool call, with the additional instruction
`If unavailable, return exactly <absence-marker>.` The private content marker is
absent from both templates. Claude uses the same positive request with the
native `Skill` tool and full namespace. Alias checks are separate observations
and never substitute for the namespaced calibration.

A failed positive still triggers safe native cleanup of fixture registrations
whose successful creation and ownership are established. An unknown operation
outcome stops for reconciliation, without blind uninstall or retries.

Every phase records actual cwd, client version, native session identity and
start/finish times. Catalog exposure without activation blocks PASS; any
unintended activation is FAIL. Missing freshness, ordering or outcome evidence
blocks classification. Earlier Q11 results remain unchanged.

## Preservation inventory and cleanup

Measure presence, SHA-256 and ordinary modes before and after native effects:

- Both isolated homes' `CLAUDE.md` and `AGENTS.md`, ancestor/project/nested
  equivalents, project `.claude/settings.json` and
  `.claude/settings.local.json`.
- Claude isolated home `.claude.json`, configuration-root `.claude.json`,
  `settings.json`, `settings.local.json`, `plugins/installed_plugins.json` and
  `plugins/known_marketplaces.json`; system managed settings where present.
- Copilot isolated root `config.json`, `settings.json`,
  `plugins/installed_plugins.json` and `plugins/known_marketplaces.json`.
- Every fixture package manifest, skill and resource, plus inventoried unrelated
  configuration/discovery files. Reject symlinks and unsupported metadata before
  effects; do not follow references outside the declared inventory silently.

The new project starts with a fixture-owned `.claude/settings.json` at mode 0640
containing an unrelated sentinel. Preserve its sentinel, meaningful order and
mode. Only this fixture's `enabledPlugins` and `extraKnownMarketplaces` entries
may be added through native management. Operation-created empty containers in
that pre-existing file may remain under approved policy D2. Any unrelated change
fails preservation; do not overwrite it with a snapshot.

The initial project settings bytes are
`{"env":{"UPSKILL_Q11B_SENTINEL":"preserve"}}` followed by one newline. The
exact eligible owned entries are `enabledPlugins["<plugin>@<market>"] = true`
and
`extraKnownMarketplaces["<market>"] = {"source":{"source":"directory","path":"<absolute-marketplace-root>"}}`.
The path is the freshly captured fixture marketplace root. No other JSON entry
is owned. A different native representation blocks this profile for review.
Native formatting changes are measured, not silently accepted as byte equality;
the approved empty-container residue allowance does not waive unrelated-byte
preservation. The sentinel file is a disposable fixture, never team settings.

Native registry/cache paths are observation targets only. Never write, restore
or delete them directly. No automatic removal of dependencies with surviving
references. Keep source packages for replay after native unregistration.
Authentication, trust, logs, sessions, caches and usage bookkeeping are reported
as separate native effects, without whole-profile equality claims. Original
shared/auth-bearing configuration bytes stay in memory and are never written to
a backup or automatically restored. Stop on unexpected configuration changes and
report them for reconciliation. No notice/trust repair or authentication renewal
is authorized by this profile.

## Approval and delivery

This requests approval only for the bounded native experiment and its stated
residue/preservation measurements. It does not approve a shared configuration
writer, production support, another scope or a narrower replacement entry gate.
Offline fixture/classification tests and independent profile review precede
execution. Retain a separate evidence packet, receipt hashes, private-source
index and technote even when a positive fails. Run `just fmt` and `just verify`
before the resulting PR.

## Offline classification boundary

`scripts/qualify-plugin-rules.py::classify_registration_isolation` consumes
captured phase rows, operation receipts, an inventory declaration and file
fingerprints. Its fixture binds owner/observer pins, project cwd, namespace,
aliases and the expected source path for each positive. Package envelopes must
contain exactly their declared portable skill trees, with valid file digests and
modes. Owner and observer skill/resource bytes must match.

Both owner positives require affirmative `candidate_registered` evidence for the
declared marketplace route. A valid, correlated candidate activation is
classified before later removal and post-removal observations; missing or failed
cleanup cannot erase an established activation FAIL. That result does not claim
cleanup or preservation succeeded.

The capture producer remains responsible for native record correlation, session
binding, disclosure detection and inventory collection. This function does not
run clients or verify a producer's assertions against raw streams. Its offline
tests use synthetic captures; they provide no native qualification.

`discovery_absence_observed` is a bounded isolation result only. Preservation is
reported separately: `unchanged` requires complete, valid, equal before/after
file receipts covering exactly the fixture's nonempty, duplicate-free
`protected_paths` list, captured before effects. Missing, substituted or extra
paths cannot establish preservation. This classifier conservatively reports
`preservation_failed` for differing receipts, including declared empty-container
residue; it does not infer approval of that residue. Accepting any residue needs
its separate captured preservation witness. No result from this classifier
replaces the existing Q11 entry gate or approves production routing.

Malformed top-level observations or fixture identities return `invalid_capture`
or `invalid_fixture`. Missing or malformed nested capture mappings block their
applicable control; malformed preservation mappings fail preservation
separately. These outcomes never supply positive evidence.

The
[offline guard validation](../../technotes/evidence/2026-10-27-q11b-offline-guards/validation.json)
records 29 focused tests and 23 independently rejected guard mutations. The
mutations run on temporary copies, including consistent malformed envelopes,
cross-client inequality, exact alias/response checks, completion and operation
state, protected-path coverage and activation ordering. This remains synthetic
evidence, with no native client execution or qualification.
