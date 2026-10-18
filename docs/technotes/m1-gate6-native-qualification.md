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
carrying evidence between controls. Each request starts a new session. Only
exact final responses qualify; failed positive controls, nonmatching responses,
authentication errors and timeouts never qualify a negative. Timeouts kill the
process group.

Codex runs with ignored user configuration, ignored exec policy rules, ephemeral
sessions and a read-only sandbox. Its JSON events must contain no tool calls.
Claude disables tools, session persistence and external MCP configuration; it
loads project settings. Copilot disables available tools, built-in MCPs and
automatic updates. OpenCode uses a disposable project configuration denying all
tool permissions and JSON events; observed tool events disqualify the run.
Existing native authentication is used without reading, copying or printing
authentication files. OpenCode retains its normal data location for
authentication; its state directory is isolated.

Evidence records platform, version, exact commands, configuration, sanitized
responses and SHA-256 hashes of captured stdout plus stderr. Arbitrary output is
discarded. Evidence must be outside the repository; fixture trees are removed by
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

Supplemental OpenCode evidence is preserved in
[OpenCode v2 evidence](agent-customization/native-qualification/2026-10-18-opencode-v2.json),
byte-identical to `/private/tmp/upskill-gate6-native-opencode-v2/summary.json`.
SHA-256 `9ed7d7169aa70fe2b0cb708a43772574bc33343c1b70be8c2e977847ca9932a9`. With
normal authentication data retained, the default positive request exited 0 but
returned the fallback rather than its canonical marker. The default negative and
both configured-glob requests exited 1. No tool events were observed. Neither
default nor configured OpenCode discovery qualifies, and no configured staging
exposure was established by this run. The exact failure cause remains
unresolved; these results must not be attributed to absent authentication. The
installed v2 CLI may require a different qualification adapter than the
documented rule discovery flow.

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
supplemental evidence below. Copilot CLI evidence does not qualify Copilot VS
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
- [OpenCode rules](https://opencode.ai/docs/rules/): default instruction files
  and configured instruction globs.
