# Q11b registration isolation: failed Copilot calibration

Task [#428](https://github.com/driftsys/upskill/issues/428) attempted the
[user-approved concrete profile](../wip/specs/2026-10-27-q11b-project-profile.md)
on macOS arm64, 2026-10-29. Copilot's required positive failed, so Claude
calibration, owner registration, baseline, candidate and post-removal controls
were not run. This yields no registration-isolation result. Q11's retained
explicit-selector isolation FAIL and the existing slice-2 entry gate remain
unchanged.

## Native observation

The fresh
[campaign pins](evidence/2026-10-29-q11b-native-calibration/campaign-pins.json)
remain Claude 2.1.308, Copilot 1.0.103, Codex 0.164.0 and OpenCode 2.1.4. Only
Copilot executes a model request. Codex and OpenCode execute version probes
only. This is a new evidence attempt; no client version changed.

The single Copilot process uses its supported session `--plugin-dir` selector
for its own package in the neutral project. The prompt is the approved
namespaced request, without the content marker. Built-in MCP servers and child
execution are disabled; the native skill tool is the only available tool. The
process exits 0, without a timeout or malformed JSON stream.

The
[selected records](evidence/2026-10-29-q11b-native-calibration/native-selection.json)
show the full namespace in `session.skills_loaded`, with the expected entrypoint
path. The model calls `skill` once using the **bare skill name**. Its correlated
completion succeeds. There is **no namespaced tool attempt** in this attempt.
The final response contains two leading newline characters followed by the
content marker. Both the required namespaced load and exact untrimmed response
therefore fail. Successful bare-name loading does not pass the approved profile.
No slash transformation, prompt change or retry was performed.

The
[observation](evidence/2026-10-29-q11b-native-calibration/copilot-positive-observation.json)
is replayed through the existing `inspect_cross_client_positive` function. It
records zero namespaced activations, one content activation, an unexpected
bare-name tool call and a nonexact response. The
[summary](evidence/2026-10-29-q11b-native-calibration/summary.json) retains
`blocked_positive`, `qualified: false` and all dependent controls as `not_run`.
This failed positive is neither an isolation PASS nor an unintended-activation
FAIL: no foreign registration was exposed to the observer.

## Inventory, correlation and preservation

Both existing authenticated isolated profiles were locked exclusively before
native commands. Their configuration was inspected in memory. The
[inventory](evidence/2026-10-29-q11b-native-calibration/inventory.json) records
effective roots, configuration discovery facts and 162 protected paths. Existing
synchronized Claude content was fingerprinted; no team instructions, discovery
preferences, credentials or native registries were authored. The project's
unrelated sentinel settings file starts at mode 0640 with the exact approved
bytes. The matching owner and observer portable trees were prepared with
identical skill/resource bytes and modes. Their package envelopes differ by the
intended native manifest format. The unused marketplace fixture was prepared
outside the project; its native route was not exercised.

[Session correlation](evidence/2026-10-29-q11b-native-calibration/session-correlation.json)
binds the process's terminal session ID to the persisted native `session.start`,
client version, project cwd, event IDs, parent links and tool-call identity.
Every selected user, assistant and tool record has a corresponding native
history record. The persisted completion adds result detail; its identity,
parent, success and tool-call ID match stdout. The ephemeral catalog is bound
through the single process receipt and its native session-start parent ID; it is
not duplicated in persisted history. No broader session qualification is
inferred from this failed control.

[Before](evidence/2026-10-29-q11b-native-calibration/preservation-before.json)
and [after](evidence/2026-10-29-q11b-native-calibration/preservation-after.json)
receipts match for all 162 declared protected paths. Package envelopes and
portable trees also match their initial fingerprints. Copilot's native
session/log/cache/usage bookkeeping remains native-managed and outside this
protected-file equality claim. No whole-profile equality is claimed. No owner
registration was created, so no fixture uninstallation was required. The session
selector ended with the process. No native cache or registry was patched,
restored or deleted.

## Retention and next decision

Raw streams, model identifiers, native session history and capture helpers stay
in the private packet. The
[private-source index](evidence/2026-10-29-q11b-native-calibration/private-source-index.json)
retains their SHA-256 receipts without exporting auth-bearing configuration.
Public selected records replace markers with explicit symbolic values and paths
with root placeholders. Response whitespace is preserved through that
replacement. Quick review found stale owner-package metadata in the initial
observer fixture projection. The public observer fixture now consistently swaps
the paired owner/native and observer/native paths, manifests, intended clients
and fingerprints. The original private capture remains untouched; replay of the
actual native invocation and result is unchanged
([#435](https://github.com/driftsys/upskill/issues/435)).
[Receipt hashes](evidence/2026-10-29-q11b-native-calibration/receipt-hashes.json)
cover the public packet. The offline classifier and production code are
unchanged.

The next admissible attempt needs a successful positive under a reviewed profile
or evidence of a relevant client change. This packet does not approve changing
the namespace or exact-response contract. Q5 remains deferred, and no production
route, schema or replacement entry gate is adopted.
