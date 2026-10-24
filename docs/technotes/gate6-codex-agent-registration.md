# Gate 6 Codex package-owned agent registration

Task [#393](https://github.com/driftsys/upskill/issues/393), Q7 of the
[implementation handoff](../wip/specs/2026-10-23-customization-delivery-implementation-spec.md),
retains bounded native results on Codex CLI 0.164.0, macOS arm64, 2026-10-24. An
explicitly registered package-owned TOML agent runs as a real child and inherits
the parent's read-only execution boundary. The identical TOML inside an
installed plugin is not discovered without external registration. Original
user-configuration byte restoration initially fails after normal workspace
admission. A later owned notice-counter correction recovers the exact original
hash; both results are retained. Production remains schema 1.

## Fixture and native method

The [dated campaign](gate6-plugin-profile-inventory.md) pins Codex 0.164.0. Each
model request checks the native version. Claude 2.1.308, Copilot 1.0.103 and
OpenCode 2.1.4 run no controls in this packet.

The existing `scripts/qualify-plugin-rules.py` creates two distinct disposable
Git roots and a neutral managed delivery root. The positive project's native
`.codex/config.toml` contains a fresh instruction marker. The candidate project
references a package-owned `agent.toml` through `agents.<name>.config_file`.
That file sets its name, description and developer instructions, with no sandbox
or approval override. The response marker exists only in a project resource. No
`CLAUDE.md`, `AGENTS.md`, nested equivalent or symlink is created.

The child is instructed to read that resource, attempt one harmless write to a
new `denied.txt`, and report the actual result. It must not retry, escalate,
delegate or change permissions. Both parent requests explicitly use read-only
sandboxing and approvals set to `never`. The normally provisioned isolated
profile is reused with inherited overrides stripped; credentials are not copied.

Normal native folder trust is accepted only for these two reviewed generated
projects. Admission creates no model prompt or hook definition. It is separate
from the user's `/hooks` review; no hook trust is added, edited or bypassed. An
initial zero-size terminal attempt is retained, then corrected to a fixed
terminal size before admission.

## Results

| Control                         | Pin           | Result                                                                                                                       |
| ------------------------------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Native instruction positive     | Codex 0.164.0 | PASS: exact native marker, completed turn, exit 0                                                                            |
| Explicit package-owned role     | Codex 0.164.0 | PASS: exact custom role spawns a real child; resource read succeeds                                                          |
| Inherited execution boundary    | Codex 0.164.0 | PASS: parent and child native contexts show read-only/never; child write exits 1 with “Operation not permitted”; file absent |
| Child response                  | Codex 0.164.0 | PASS: exact JSON object containing the resource marker and denied result                                                     |
| Installed plugin-only agent     | Codex 0.164.0 | PASS: explicit role absent from calibrated native spawn schema after external registration removal                           |
| Final pre-UI user configuration | Codex 0.164.0 | PASS after correction: exact original hash and mode; initial restoration failure retained                                    |

The
[boundary evidence](evidence/2026-10-24-codex-agent-registration/child-boundary-evidence.json)
correlates the child session's native parent ID and role with the parent
request. Native child tool-call IDs bind the resource read and denied command to
their results. The child transcript includes inherited history; the assessment
selects its explicit subagent session metadata rather than assuming the last
metadata record identifies the child. Final prose alone is not treated as a
denial.

## Plugin-only discovery

The local marketplace plugin contains the same TOML bytes under
`agents/probe.toml`, without hooks. Native marketplace add and plugin add
succeed and the installed path is a native cache. The harness never writes that
cache directly. The explicit registration is removed only from the new fixture's
own project config for this separate control.

Native `debug prompt-input` exposes the `spawn_agent` schema without inference.
With explicit registration, its `agent_type` description lists the exact custom
role. With only the installed plugin, the spawn API remains present but the
custom-role argument is absent. The
[catalog assessment](evidence/2026-10-24-codex-agent-registration/catalog-role-assessment.json)
retains those separate calibrated observations. Mere absence of the role name
from arbitrary prompt text is not the classification method.

The first unfiltered plugin inventory fails because a pre-existing marketplace
source lacks a supported manifest. A second native query targets only the new
marketplace and succeeds. That existing discovery source is preserved. Native
remove and marketplace remove succeed, and the original fixture project config
is restored. These results do not qualify other plugin layouts or registration
schemas.

The
[official configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)
and
[subagent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents)
inform the fixture; the pinned native run establishes this bounded result.

## Initial preservation failure and final correction

The
[plugin preservation receipt](evidence/2026-10-24-codex-agent-registration/retry-plugin-preservation.json)
shows native plugin management restoring the immediate post-admission user
configuration baseline exactly. Its own project config also returns to its
original bytes and mode.

The earlier, pre-UI configuration is different. Its starting hash and mode were
retained, but its original bytes were not held across admission. The helper
removes the two exact newly granted fixture trust entries after all controls.
Attempts to reconstruct the original bytes do not match the starting hash, so no
guessed replacement is written. Remaining native configuration bytes are
preserved; their difference is not explained or dismissed as formatting. The
[restoration record](evidence/2026-10-24-codex-agent-registration/trust-restoration.json)
keeps both the failed exact-restoration result and the owned-entry cleanup.

The initial restoration is a failure, independent of the passed child/sandbox
and catalog cells. Later inspection identifies the native availability-notice
counter advancing from 3 to 4. Restoring its earlier value and original trailing
whitespace reconstructs bytes matching the complete pre-UI hash. Only that
matching candidate is written, after the subsequent control releases the
profile. The
[final correction](evidence/2026-10-24-codex-agent-registration/final-notice-restoration.json)
records exact original hash and mode restoration; the initial failed receipt is
unchanged. Its retained helper is explicitly an equivalent guarded source saved
after the original inline execution.

Future controls hold original configuration bytes only in memory through
admission, execution and cleanup, and validate unrelated values before restoring
them in place. No credential export follows from that decision. Session logs,
native cache effects and other profile state are not whole-profile restoration
claims.

## Offline validation and consequence

Quick review strengthens child-marker identity, the forbidden operation and
restrictions, failure classifications and package metadata. Seven adverse
mutations are rejected by the latest nine Q7 tests. Resolved native-home
rejection is checked before writes; final full verification passes 67 offline
tests.

Seven offline tests are written first. `just lint` fails on missing helpers,
then passes all 65 offline tests. They pin the neutral layout, identical agent
bytes, explicit role and local plugin registration, marker separation, inherited
native denial evidence, exact responses and calibrated plugin-only discovery.
They never invoke a client or qualify native execution.

Raw transcripts, private test model selection, native prompt inputs and source
remain outside the repository. The
[source index](evidence/2026-10-24-codex-agent-registration/private-source-index.json)
and hashes bind the selected receipts and private sources. Public role/schema
assessments omit test model identifiers and unrelated catalog content.

The affected proposal row is
[Subagents, Codex](../wip/specs/2026-10-23-customization-delivery-proposal.md#subagents).
Explicit package-owned registration and inherited denial are observed on this
trusted-project pin; automatic plugin-contained TOML discovery is not observed.
No durable adoption is made before step 7. The initial preservation failure and
final correction stay visible. Linux, IDE/hosted surfaces, arbitrary user scope,
MCP inheritance and integrated lifecycle remain unqualified. This packet does
not promote Codex rules while
[#379](https://github.com/driftsys/upskill/issues/379) is open.
