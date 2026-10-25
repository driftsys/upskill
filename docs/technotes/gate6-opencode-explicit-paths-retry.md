# Gate 6 OpenCode explicit-path second retry: free usage limit

The user selected a different model for task
[#391](https://github.com/driftsys/upskill/issues/391). This separate 2026-10-25
attempt on macOS arm64 retains one failed native positive on OpenCode 2.1.4. The
built-in provider returns HTTP 429 `provider.free-limit` and a retry delay of
43909000 milliseconds, approximately 12 hours 12 minutes. No response completes
and no downstream control runs
([summary](evidence/2026-10-25-opencode-explicit-paths-retry/summary.json),
[events](evidence/2026-10-25-opencode-explicit-paths-retry/positive-events.json)).

All four installed versions are captured before the positive: Claude 2.1.308,
Codex 0.164.0, Copilot 1.0.103 and OpenCode 2.1.4. The additional
executing-client receipt confirms OpenCode 2.1.4. Model identifiers and raw
source remain private
([campaign](evidence/2026-10-25-opencode-explicit-paths-retry/campaign-pins.json),
[version](evidence/2026-10-25-opencode-explicit-paths-retry/positive-version-receipt.json)).

The native catalog contains the requested entry, and the exported session
confirms the requested model and session identity. The existing isolated profile
is reused. The three new disposable projects select that model; no custom
provider endpoint, team discovery setting, credential copy or hook trust bypass
is introduced. Tools are denied for the single-file instruction positive
([fixture](evidence/2026-10-25-opencode-explicit-paths-retry/fixture.json),
[audit](evidence/2026-10-25-opencode-explicit-paths-retry/positive-model-audit.json),
[request](evidence/2026-10-25-opencode-explicit-paths-retry/positive-receipt.json)).

| Control                                                | Pin            | Result                          |
| ------------------------------------------------------ | -------------- | ------------------------------- |
| Native explicit-file positive                          | OpenCode 2.1.4 | FAIL: HTTP 429 free usage limit |
| Install/update/remove/resume/reinstall/outside-project | OpenCode 2.1.4 | NOT RUN after failed positive   |
| Package skill/resource and fallback hook               | OpenCode 2.1.4 | NOT RUN after failed positive   |

The same test-driven instruction fixture and classifier from the
[earlier explicit-path packet](gate6-opencode-explicit-paths.md) are reused.
Seven focused offline tests pass before this native request. No harness behavior
changes. The result remains `positive_control_failed`; it establishes neither
instruction-path failure nor absence of a reader
([validation](evidence/2026-10-25-opencode-explicit-paths-retry/validation.json),
[controls](evidence/2026-10-25-opencode-explicit-paths-retry/controls.json)).

Measured project/user configuration, manual neighbors and rule bytes/modes
remain unchanged. Native session/cache/log/authentication effects remain outside
whole-profile equality. The provider delay is retained without another request
or an account/provider change to avoid the limit
([preservation](evidence/2026-10-25-opencode-explicit-paths-retry/preservation.json),
[source](evidence/2026-10-25-opencode-explicit-paths-retry/private-source-index.json)).

Earlier failures retain their original receipts. Q5 and the future OpenCode glob
removal stay gated on a successful explicit-path control. No production route,
schema-1 state or durable direction record changes
([handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
[summary](evidence/2026-10-25-opencode-explicit-paths-retry/summary.json)).
