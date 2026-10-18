# M1 Gate 6 native instruction discovery qualification

This qualification supports
[task #319](https://github.com/driftsys/upskill/issues/319) and investigates
[debt #316](https://github.com/driftsys/upskill/issues/316). It does not
activate schema 2 or the lifecycle CLI adapter.

## Method

`scripts/qualify-native-instructions.py` is an opt-in native harness. It creates
separate disposable Git repositories for positive and negative controls. The
positive contains one canonical instruction file. The negative contains only an
extensionless `.upskill-stage-<nonce>` sibling in the same directory. Distinct
unpredictable positive, staging and fallback markers prevent a response from
carrying evidence between controls. Current fixtures have distinct empty root
commits so native clients can identify separate Git projects; fixture-only
identity overrides disable Git hooks and commit signing. Each request starts a
new session. Only exact final responses qualify; failed positive controls,
nonmatching responses, authentication errors and timeouts never qualify a
negative. Requests without a successful, parseable version probe remain
`version_unqualified`. Timeout and interruption cleanup terminates and reaps the
owned process group before the harness returns or propagates cancellation.

Codex runs with ignored user configuration, ignored exec policy rules, ephemeral
sessions and a read-only sandbox. Its JSON events must contain a completed
assistant response and completed turn matching the final output file, with no
tool calls, unknown events or malformed records. Claude disables tools, session
persistence and external MCP configuration; it loads project settings. Copilot
disables available tools, built-in MCPs and automatic updates. OpenCode uses a
disposable project configuration denying all tool permissions with the v2
`permissions` rule array and JSON events; observed tool events disqualify the
run. The harness enables project discovery explicitly with both
`OPENCODE_CONFIG_PROJECT_DISABLE=0` and `OPENCODE_DISABLE_PROJECT_CONFIG=0`,
removes inline/file/directory config overrides from the child environment and
records these controlled settings. Existing native authentication is used
without reading, copying or printing authentication files. OpenCode retains its
normal data location for authentication; its state directory is isolated.

Evidence records platform, version, exact commands, configuration, sanitized
responses and SHA-256 hashes of captured stdout plus stderr. Arbitrary output is
discarded. Raw evidence must be outside the repository; sanitized summaries may
be retained as the evidence assets linked below. Fixture trees are removed by
their exact temporary directory ownership, never by staging prefix.

Reproduce with installed, authenticated clients:

```bash
python3 scripts/qualify-native-instructions.py --run \
  --timeout 40 --evidence /private/tmp/upskill-native-evidence
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s scripts/tests -p 'test_*.py'
```

The evidence directory must not already exist. Native requests can incur model
charges. Hosted CI runs only the offline fixture and classification tests,
through `just lint`.

## Offline checks and delivery boundary

The
[offline regression tests](../../scripts/tests/test_qualify_native_instructions.py)
exercise fixture-marker separation across all four clients, extensionless
staging siblings, independent staging detection, exact response matching,
rejection of failed controls and OpenCode text-event parsing with tool-event
rejection. `just lint` runs these tests without invoking native clients. They
validate harness mechanics; native discovery conclusions come only from the
paired observations below. Fourteen offline tests also pin exact canonical paths
and staging parents, distinct root commits, interrupt and timeout descendant
cleanup, version gating, state-aware orchestration, Codex completion evidence,
OpenCode v2 rule shape and controlled discovery settings. Mutation checks
confirm the tests reject altered canonical paths, discarded invocation states,
deleted tool-event rejection and a removed version gate. Every client flag and
all diagnostic classification paths remain outside this offline coverage.

The bounded implementation is recorded in local commit `26add38`, based on
`26ab826`. The dispatcher supplies passing baseline full-suite and Rust 1.85
checks; the original five offline examples passed during reconciliation. Review
fixes expanded the focused suite to fourteen passing tests. Final repository
verification, review, hosted CI and merge remain separate delivery evidence. No
production Rust code changes belong to this qualification slice.

The original
[qualification spec](../archive/specs/2026-10-18-gate6-native-qualification.md)
and [delivery plan](../archive/plans/2026-10-18-gate6-qualification.md) are
archived unchanged as provenance for this bounded harness and evidence record.
The plan also describes later delivery and activation work; archiving its raw
bytes does not mark those tasks complete. Existing M1 and foundation working
records retain their original paths and disposition. This note records observed
behavior and remaining qualification limits; it introduces no production
requirement or architecture decision.

## Observations on 2026-10-18

Platform: macOS 15.6.1, arm64. These are installed native CLI observations.

| Client      | Version | Canonical project instruction     | Positive                              | Extensionless staging negative              |
| ----------- | ------- | --------------------------------- | ------------------------------------- | ------------------------------------------- |
| Codex       | 0.163.0 | `AGENTS.md`                       | Exact positive marker; no tool events | Exact fallback; qualified default discovery |
| Claude Code | 2.1.292 | `.claude/rules/probe.md`          | Exact positive marker; tools disabled | Exact fallback; qualified default discovery |
| Copilot CLI | 1.0.98  | `.github/copilot-instructions.md` | Exact positive marker; tools disabled | Exact fallback; qualified default discovery |
| OpenCode    | 2.0.25  | `AGENTS.md`                       | See bounded supplemental evidence     | See bounded supplemental evidence           |

The three successful paired controls are preserved in
[project default evidence](agent-customization/native-qualification/2026-10-18-project-defaults.json),
byte-identical to `/private/tmp/upskill-gate6-native-qualified-v1/summary.json`.
SHA-256 `32d21ea5f11aa1f8e2a243ce6c1ac1587352cda5da052e42764991b2859fdc7b`. That
run deliberately isolated OpenCode data, causing failed controls; those OpenCode
failures establish no discovery result. A subsequent diagnostic retaining normal
OpenCode data exited successfully, so the earlier failure must not be described
as proof that the user's authentication was absent.

Historical OpenCode evidence from the first normal-data run is preserved in
[OpenCode v2 evidence](agent-customization/native-qualification/2026-10-18-opencode-v2.json),
byte-identical to `/private/tmp/upskill-gate6-native-opencode-v2/summary.json`.
SHA-256 `9ed7d7169aa70fe2b0cb708a43772574bc33343c1b70be8c2e977847ca9932a9`. With
normal authentication data retained, the default positive request exited 0 but
returned the fallback rather than its canonical marker. The default negative and
both configured-glob requests exited 1. No tool events were observed. Neither
default nor configured OpenCode discovery qualifies, and no configured staging
exposure was established by this run. This historical run used the v1
`permission` map with installed v2 and Git fixtures without root commits.
No-model diagnostics later showed that empty repositories shared the `global`
project identity and a canonical directory outside the fixture. These are
harness limitations, so the historical fallback is not evidence that OpenCode v2
ignores canonical instructions. V2 standalone mode starts a private server; it
does not itself disable instruction discovery.

The corrected default-only experiment is preserved in
[corrected OpenCode v2 evidence](agent-customization/native-qualification/2026-10-18-opencode-v2-corrected.json),
byte-identical to
`/private/tmp/upskill-gate6-native-opencode-v2-corrected/summary.json`. SHA-256
`a87210fee26f9cbba2aaf0d48dcecbf376d113e3eca02fb27ac205e6d96b5db1`. It uses the
v2 ordered deny-all rules, distinct root commits, existing native authentication
and explicit enabled project discovery. Prior no-model diagnostics confirmed v2
config-source loading and a deny-all agent rule; after root commits, generated
sessions had non-global project IDs and the requested fixture directory. Both
final model calls exited 1, with no tool events and `configuration_error` as the
generic sanitized category. The specific cause remains unknown. The positive
control failed, so OpenCode remains unqualified. The corrected configured-glob
case was not rerun and remains unqualified.

The three historical successful clients were not rerun during these fixes; their
recorded controls predate the stricter completion-event and root-identity
checks. Offline regressions do not upgrade those native observations to a new
version or configuration claim. No further model calls were made after the
bounded corrected OpenCode pair.

## Interpretation and remaining gates

Extensionless staging siblings avoid the tested default discovery paths for
Codex, Claude and Copilot. This conclusion is limited to the exact versions,
paths, configuration and successful controls above. Other canonical paths, user
scopes, IDE integrations and upgraded clients require their own evidence.
Incidental readers of `AGENTS.md` do not become additional owned-artifact
owners.

A filename cannot guarantee exclusion when explicit imports or configured
instruction globs select it. OpenCode has a separate configured case selecting
`AGENTS.md` and `.upskill-stage-*`; it must not be combined with default
discovery qualification. Broad configured globs may also select future staging
names. Neither a process kill nor a failed native request proves exclusion. A
killed writer can leave staging siblings before rename; recovery must use owned
transaction evidence, never prefix cleanup.

All user-root discovery remains explicitly unqualified: the harness never
modifies real personal instruction roots and isolated authenticated user-root
qualification has not been provisioned. OpenCode qualification is bounded by its
supplemental evidence above. Copilot CLI evidence does not qualify Copilot VS
Code Local. Linux hosted CI does not qualify native clients.

Gate 6 still requires native scope and client coverage, qualification of all
current content kinds, aggregate conflict handling, an authority decision for
external `CODEX_HOME`, and the next adapter's integration and recovery gates.
The main CLI and lockfile remain schema 1. Debt #316 should retain these limits
and the configured-discovery concern; this evidence alone does not close all
Gate 6 work.

## Discovery references

Official documentation read on 2026-10-18:

- [Codex instruction discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md):
  canonical names and configurable fallback filenames.
- [Claude project memory](https://code.claude.com/docs/en/memory): recursive
  Markdown rules and settings-source behavior.
- [Copilot CLI custom instructions](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions):
  repository, user, modular and additional-directory instructions.
- [OpenCode v1 rules](https://opencode.ai/docs/rules/): original historical
  reference; its permission shape was unsuitable for installed v2.
- [OpenCode v2 instructions](https://dev.opencode.ai/v2/docs/instructions/),
  [agents](https://dev.opencode.ai/v2/docs/agents/) and
  [CLI commands](https://dev.opencode.ai/v2/docs/cli/commands/): installed-v2
  discovery, permissions and private-server command guidance.
- [OpenCode configuration schema](https://opencode.ai/config.json): current
  ordered `permissions` rule shape, verified before the corrected experiment.
