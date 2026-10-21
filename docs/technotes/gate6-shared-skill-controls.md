# Gate 6 shared-skill and exclusion observations

Task [#351](https://github.com/driftsys/upskill/issues/351) retains a bounded
native evidence slice for [#337](https://github.com/driftsys/upskill/issues/337)
and the [shared-root evaluation](shared-content-root-evaluation.md). Retained on
2026-10-21; original UTC event timestamps span 2026-10-20 to 2026-10-21.

These are Copilot 1.0.98 macOS arm64 project observations, not qualification of
a complete profile, a new upskill adapter or an ownership-aware skill lifecycle.
All 160 campaign cells remain unqualified; production stays schema 1.

## Native completion and scope

The user normally authenticated the isolated profile. No credential files are
included in this packet. Native requests used the existing isolated login;
credentials were not copied into a different profile. Project A contains the
shared-skill invocation/removal pair. Project B contains the resource,
Claude-compatible and unrelated manual-skill controls. Neither project is the
user's normal repository or personal instruction root.

[Receipt metadata](evidence/2026-10-21-gate6-shared-skill/summary.json) retains
nine distinct completed native sessions, commands, raw and sanitized hashes,
model/provider identity and the observed tool counts. Both sessions in the first
pair resolved the native default to `claude-sonnet-5.5` through `copilot`;
subsequent controls explicitly selected that model. This records an observed
route, not a supported model range.

Only `skill` was available for the invocation controls. Resource controls also
allowed `view`, disabled automatic temporary-directory access and retained the
exact read path. Built-in MCP servers were disabled. The retained native events
show nine skill calls, including one unsuccessful removed-skill invocation, and
two successful reads of the expected fixture resource. Each session has a native
`result` with exit code 0 and a final assistant response. Final responses are
taken from assistant events, not substring searches across echoed prompts.

## Observed controls

| Control                                                    | Native observation                                                            | Interpretation                                        |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------- |
| Shared `.agents/skills` item present                       | One successful skill call; exact unpredictable marker                         | Bounded explicit invocation positive                  |
| Same item temporarily removed, fresh session               | Skill call unsuccessful; exact absent fallback                                | Paired negative observed; fixture restored            |
| Shared skill with `references/marker.txt`                  | One skill call and one `view` of the adjacent resource; exact resource marker | Relative-resource positive only                       |
| Claude-compatible project skill present                    | One successful skill call; exact Claude marker                                | Same-client compatibility positive                    |
| Narrow `ignoredSkillsLocations` entry added                | Skill still loads successfully and returns its marker                         | Failed explicit-activation isolation control          |
| Unrelated manual Claude skill under that setting           | One successful skill call; exact manual marker                                | Surviving manual-skill positive                       |
| Shared resource skill under that setting                   | Expected resource read and marker                                             | Surviving shared-skill positive                       |
| Original user-settings state restored                      | Claude-compatible skill loads again                                           | Restored-baseline observation                         |
| Exclusion written/read back through native config commands | Readback matches the target path; a fresh invocation still loads the skill    | Repeated failed explicit-activation isolation control |

The two failed isolation controls remain failures. They do not qualify candidate
C for this selected explicit-activation condition. They also do not establish a
vendor bug, automatic-selection behavior, or behavior for every path, scope,
setting source or version. A skill-disable control from earlier local catalog
work is a different mechanism and cannot substitute for directory exclusion.
Rules and subagents receive no support claim from these skill observations.

## Preservation and capture limits

The temporary capture helpers reported equality of fixture files/team guidance
and restoration of the user-settings state. The authenticated profile's user
settings file was absent initially; a temporary exclusion file was created and
removed. This does not qualify preservation of a nonempty pre-existing user
settings file.
[Final fixture snapshots](evidence/2026-10-21-gate6-shared-skill/fixtures.json)
retain content, modes and hashes after the controls. Independent before/after
byte receipts were not captured, so the reported equality is explicitly weaker
than a retained lifecycle preservation proof.

The capture scripts are private experiment helpers, not a shipped or tested
qualification runner. Their hashes are recorded. The packet retains selected
original native JSONL events and native config output; it omits model request
bodies, deltas, usage and unrelated events. Paths are normalized to `$PROFILE`,
`$PROJECT_A` and `$PROJECT_B`. Hashes of complete private stdout/stderr streams
remain distinct from hashes of sanitized retained files. No authentication store
or opaque provider state is part of the retained data.

The first raw pair is under
`/private/tmp/upskill-gate6-copilot-shared-positive-7sn4x7wr`; resource and
native config confirmation streams are under
`/private/tmp/upskill-gate6-copilot-resource-exclusion-_m5z9ldx`. The retained
[hash manifest](evidence/2026-10-21-gate6-shared-skill/receipt-hashes.json)
covers all 13 receipt/metadata files. Native timestamps and session IDs are
preserved.

Timeout handling existed in the private helpers, but this packet contains no
interruption, crash or descendant-cleanup experiment. No locking, backup,
recovery or native registration lifecycle is qualified. Existing ownership,
preflight and recovery foundations remain requirements, not conclusions of this
packet.

## Consequences and remaining work

Candidate A remains the first shared-skill feasibility direction. Intentional
`.claude` sharing remains an alternative; selective exclusions remain optional
profile candidates, with the failed Copilot condition above explicitly blocked.
Do not install duplicate copies, widen permission semantics or suppress
unrelated manual content to make a control pass. This evidence selects no
production adapter or installed layout.

Required follow-up includes automatic selection, resource absence/relocation,
nonempty configuration preservation, direct/plugin and scope/alias duplicates,
configured/ancestor readers, native rule matching and actual subagent execution
with enforced restrictions. All four clients, both project/user scopes, macOS
arm64/Linux x86_64 and existing IDE obligations remain required. The new
owned-skill lifecycle still follows Gate 6; complete its integrated controls
before adopting that layout and releasing the combined MVP.
