# Gate 6 Claude unscoped rule hook controls

Task [#390](https://github.com/driftsys/upskill/issues/390), Q4 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains passing project-scope activation controls on Claude Code 2.1.308, macOS
arm64, 2026-10-24. One package-owned unscoped rule is observed on fresh startup
and after native compaction. Resume retains a callback and exact response, but
no fresh context attachment. Disabled and uninstalled controls return the exact
fallback without a hook event
([summary](evidence/2026-10-24-claude-rule-hooks/summary.json),
[provenance](evidence/2026-10-24-claude-rule-hooks/control-provenance.json)).
This is bounded Q4 evidence, not complete Gate 6 qualification. Production
remains schema 1.

## Method and pin

The row uses the [2026-10-24 campaign](gate6-plugin-profile-inventory.md):
Claude 2.1.308. Codex 0.164.0, Copilot 1.0.103 and OpenCode 2.1.4 do not run in
this packet. Model selection remains private.

The existing `scripts/qualify-plugin-rules.py` builds a neutral managed source
outside the repository and documented discovery roots. It contains a local
marketplace and a self-contained package with one `SessionStart` shell hook. The
hook reads one filename from its package-owned `rules/index`, accepts only its
declared payload basename and emits the precompiled JSON additional context. It
records native input in a private JSON sequence outside any client cache. No
team guides, symlinks or direct cache writes are created.

A separate Git project has one known native rule positive. The package candidate
project has no native rule file; its initial settings hold an unrelated manual
environment value, mode `0640`. Their empty Git root commits are distinct. Rule,
native and fallback markers are unpredictable and absent from each other's
sources. The request contains the fallback, but not the successful rule marker.

The native helper reuses the normally authenticated isolated profile, strips
inherited Git/client overrides and disables automatic updates. It checks the
installed version, then runs the known native positive before registration.
Management uses ordinary marketplace add, project install, disable, uninstall
and removal verbs. Requests expose no model tools; print-mode init confirms an
empty tool list. The native
[hook interface](https://code.claude.com/docs/en/hooks) and
[plugin layout](https://code.claude.com/docs/en/plugins-reference) inform the
fixture; the pinned run, not those pages, establishes these observations.

The installed print request persists a fresh session. That same session is
reopened in the native UI, answers the same question, runs native `/compact`,
and answers the question again. The ordinary UI setup selects dark theme,
acknowledges the security notice and admits only the generated fixture through
the normal workspace-trust prompt. These decisions and native side effects are
retained; no trust bypass flag or discovery override is used. UI exit is 0.

## Observations

| Control                 | Pin            | Result                                                                     |
| ----------------------- | -------------- | -------------------------------------------------------------------------- |
| Native rule positive    | Claude 2.1.308 | PASS: exact independent native marker                                      |
| Installed hook positive | Claude 2.1.308 | PASS: exact hook marker; one startup event                                 |
| Resume                  | Claude 2.1.308 | LIMITED: callback and exact response; fresh context unobserved             |
| Native compaction       | Claude 2.1.308 | PASS: compact boundary, one compact event and exact subsequent hook marker |
| Disabled                | Claude 2.1.308 | PASS: exact fallback; zero own hook events                                 |
| Uninstalled             | Claude 2.1.308 | PASS: exact fallback; zero own hook events                                 |

The [summary](evidence/2026-10-24-claude-rule-hooks/summary.json) retains each
request and its pin. The
[UI assessment](evidence/2026-10-24-claude-rule-hooks/ui-assessment.json) links
exact user prompts, assistant UUIDs and native turn-completion records. The
[native selection](evidence/2026-10-24-claude-rule-hooks/ui-native-selection.json)
retains the manual compact boundary and completion positions. Native hook
responses complete with exit 0. The retained startup and compact UI attachments
contain successful hook output and its exact single additional-context body;
resume is evidenced by its callback trace and exact response. The marker already
exists in that resumed history, so this response does not establish fresh
context injection on resume
([UI assessment](evidence/2026-10-24-claude-rule-hooks/ui-assessment.json)).
Startup, resume and compact events have the same session identity. No model tool
call is observed.

Callback counts and exact answers are separate observations. This packet does
not claim that a callback count proves a single provider-effective developer
body, or qualify all compaction modes. The tested rule is unscoped; no file-glob
activation claim is made.

## Preservation and native setup residue

The initial
[preservation record](evidence/2026-10-24-claude-rule-hooks/preservation.json)
records the automatic restoration boundary. Project removal leaves only empty
native registration objects; exact project settings bytes and mode are restored.
The automatic user-settings guard correctly stops because native UI onboarding
created a dark-theme preference beyond that allowed empty-object residue.

After all model controls, inspection finds that the user settings file was
originally absent and now contains only the theme selected during this test. The
[owned preference cleanup](evidence/2026-10-24-claude-rule-hooks/owned-ui-preference-restoration.json)
records its hash and restores the original absence. This does not rewrite the
initial automatic guard failure or manufacture a negative control.

UI onboarding also auto-registers the official marketplace in an originally
empty native registry. The
[native cleanup receipt](evidence/2026-10-24-claude-rule-hooks/onboarding-market-restoration.json)
retains the initial difference and both cleanup attempts: user-scoped removal
fails because the implicit marketplace is not declared in user settings; the
ordinary unscoped native removal succeeds. It removes only that newly added
name, after every model control. The registry then matches its original bytes
and mode. Installed-plugin registration bytes and mode are unchanged. Final
measured project/user settings and both registration files match their
baselines. No credential store is copied or exported.

The
[cache observation](evidence/2026-10-24-claude-rule-hooks/post-removal-cache.json)
records native post-removal state separately. Native UI trust/onboarding,
preferences outside the measured settings files, session/log state and cache
lifecycle are not whole-profile restoration evidence. No direct native-cache
cleanup is performed by the helper.

## Provenance, offline checks and consequence

Raw transcripts, hook input, source files, UI input/output and helpers remain
outside the repository. The
[source index](evidence/2026-10-24-claude-rule-hooks/private-source-index.json)
binds the private manifest; retained hashes bind sanitized selections. Model
identifiers, thinking and unrelated native transcript content are omitted.

The retained private validation logs record tests before implementation:
`just lint` failed six missing-helper tests, then passed all 56 offline tests.
They execute the prepared shell hook offline, check its exact independent
payload and callback sequence, reject escaping or multiple index entries, verify
separate Git identities and require exact positive/absence responses, one event
per active boundary, stable session identity, correct pin and native compaction
completion. Offline tests do not invoke Claude or any model.

Under the
[adopted rules direction](../wip/specs/2026-10-23-customization-delivery-proposal.md#rules),
the packet supplies bounded startup, absence and compaction results for the
retained hook fallback. Resume provides callback and response evidence only
([summary](evidence/2026-10-24-claude-rule-hooks/summary.json)). Durable
amendments wait for step 7. User scope, Linux, IDE/hosted surfaces, unintended
readers, effective-input uniqueness and integrated upskill ownership/recovery
remain unqualified. No production adapter is activated here.

Further regressions pin the native-positive file bytes, absent native rules in
the hook project, fresh role markers, protected repository/home roots, missing
evidence and matching invalid session identities. Further packet review tests
require explicit absence traces, no observed tool calls, the correct project,
valid callback objects and replay of the retained summary. Current verification
counts and logs are in the
[validation record](evidence/2026-10-24-claude-rule-hooks/validation.json); the
shell fixture also passes ShellCheck.

## Project identity and historical capture limits

The
[provenance selection](evidence/2026-10-24-claude-rule-hooks/control-provenance.json)
recovers cwd, session, tool calls and the tested package's catalog entry from
the original print streams. Both absence runs name the candidate project and
omit this package. UI user and assistant records name that same project. The
[project identities](evidence/2026-10-24-claude-rule-hooks/project-identities.json)
retain its distinct Git root. These are offline selections, with no native
request repeated.

Management cwd follows the original capture helper's call arguments. It was not
independently recorded in a process receipt. No settings bytes were captured
immediately after disable, so that state cannot be reconstructed from this
packet. The native catalog and fallback observation support the bounded absence
result; they do not establish exact post-disable settings preservation
([provenance](evidence/2026-10-24-claude-rule-hooks/control-provenance.json),
[preservation](evidence/2026-10-24-claude-rule-hooks/preservation.json)).

Remaining fixture and capture maintenance is tracked in
[debt #403](https://github.com/driftsys/upskill/issues/403).
