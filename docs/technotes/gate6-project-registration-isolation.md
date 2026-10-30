# Q11b: Copilot activates a Claude project registration

Task [#428](https://github.com/driftsys/upskill/issues/428) completes its first
bounded native cell on macOS arm64, 2026-10-30: **registration isolation FAIL**.
After both client positives and a clean selector-free Copilot baseline,
registering the local plugin only through Claude's project marketplace makes
Copilot catalog and activate that plugin without a foreign selector or observer
registration command. Native owner cleanup succeeds, and a fresh post-removal
Copilot process again reports the skill unavailable.

This is automatic discovery after intended-owner registration. It is distinct
from
[Q11's explicit foreign-selector FAIL](gate6-cross-client-package-isolation.md).
Neither result is relabelled, and the existing slice-2 entry gate remains
blocked. No production adapter or replacement gate is adopted.

## Controls and attribution

The [native-name positive](gate6-copilot-native-calibration.md) supplies the
retained Copilot prerequisite. Its client pin, neutral project, logical
identity, source bytes and profile were rechecked. The user explicitly
authorized Claude reauthentication in the isolated profile and completed browser
login before owner controls. Fresh four-client receipts define a
[continuation revision](evidence/2026-10-30-q11b-registration-isolation/campaign-revision.json):
Claude 2.1.308, Copilot 1.0.103, Codex 0.166.0 and OpenCode 2.1.4. Only the
nonparticipant Codex pin changed; it executed version probes only. This does not
qualify Codex, OpenCode, Linux, IDE, cloud or another scope.

Both isolated client profiles were locked. Model tools were limited to native
skill invocation, built-in MCP and child execution were disabled, and the
private content marker was absent from every request. Owner commands use only
native project-scope marketplace add/install/uninstall/remove. The copied local
marketplace package retains the original owner envelope and the exact portable
skill/resource bytes used by Copilot's positive.

| Phase             | Observation                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Copilot positive  | Retained native-name positive; exact response and source provenance pass.                                                                  |
| Claude positive   | Exact namespaced `Skill` invocation and marker response pass in the declared local source package.                                         |
| Copilot baseline  | Fresh selector-free process; catalog absent, native bare-name not-found and exact absence response; no activation.                         |
| Owner candidate   | Native Claude project registration and a fresh owner invocation pass.                                                                      |
| Copilot candidate | Fresh selector-free process catalogs the namespace, loads the bare name and returns the exact private marker.                              |
| Post-removal      | Native owner uninstall/remove complete; fresh Copilot process reports native not-found and the exact absence response, with no activation. |

The
[candidate attribution](evidence/2026-10-30-q11b-registration-isolation/candidate-attribution.json)
binds one `skill` call to its successful completion and the native
`skill.invoked` record. The record's name, plugin identity/version, path and
instruction body match the owner package declared before registration. The
loaded file is the declared owner source file, byte for byte. Its path also
matches the candidate catalog. The final exact response descends from that
native load in the same session. Copilot receives no `--plugin-dir` or other
foreign selector and executes no observer-owned registration command.

Selected
[candidate stdout](evidence/2026-10-30-q11b-registration-isolation/candidate-stdout-selection.json)
and
[native history](evidence/2026-10-30-q11b-registration-isolation/candidate-native-selection.json)
retain the tool and parent identities, source path and redacted body. Baseline,
candidate and post-removal have distinct native sessions and ordered receipts.
Owner installation finishes before the candidate starts; owner removal finishes
before the post-removal process starts. `candidate_registered: false` on
observer rows means no observer-owned registration command; the separately
recorded owner registration is active during the candidate.

The
[whole-cell replay](evidence/2026-10-30-q11b-registration-isolation/cell-classification.json)
returns `activation_observed` independently of later cleanup or preservation
problems. The regression also retains that FAIL if post-removal evidence is
missing or removal is reported failed. A contaminated baseline still blocks
admission; the counterexample does not waive prerequisite controls.

## Preservation and native residue

Preservation is reported separately as **`preservation_failed`** under the
strict whole-file witness. The original compact fixture settings are rewritten
by native management and retain only its operation-created empty
`enabledPlugins` and `extraKnownMarketplaces` containers. Its unrelated sentinel
value and mode 0640 survive. No settings snapshot was restored. The
[initial bytes](evidence/2026-10-30-q11b-registration-isolation/project-settings-initial.txt)
and
[final bytes](evidence/2026-10-30-q11b-registration-isolation/project-settings-final.txt)
make the formatting and residue visible; semantic empty-object residue is not
original-byte equality.

Native registration/cache metadata and Claude usage/plugin-sync bookkeeping also
changed. These are native-managed observations, not a claim of whole-profile
restoration. No cache or registry was patched, restored or deleted directly. All
source packages remain retained. The combined
[final file witness](evidence/2026-10-30-q11b-registration-isolation/final-witness-time.json)
was read after the completed sequence; no old receipt substitutes for an
unobserved final path. Directory metadata stays in each stage packet, outside
the classifier's whole-file receipt map. See
[preservation notes](evidence/2026-10-30-q11b-registration-isolation/preservation-notes.json)
for the boundary. Authentication changes were separately authorized and precede
this owner-control preservation baseline.

## Retention and execution limits

Five new model processes completed the remaining controls, with no automatic
paid retries. The earlier Copilot positive was reused after its participant pin
and fixture were revalidated. An initial native setup made no model request
because a private model-selector file had expired; native cleanup completed
before selecting the documented Haiku alias for the owner positive.

The initial owner parser expected the registry's cache path. Native directory
marketplaces load the local source package instead: native init matches the
source path and plugin key captured before effects, and source/cache bytes
match.
[Offline correction](evidence/2026-10-30-q11b-registration-isolation/owner-source-correction.json)
replays the unchanged native request successfully. The
[original summary](evidence/2026-10-30-q11b-registration-isolation/owner-initial-summary.json)
is retained. No owner request was repeated to correct attribution.

Independent review preceded the negative capture. Its cleanup gate and malformed
output handling were hardened with twelve offline checks: post-removal requires
affirmative native cleanup/settings validation, and later malformed diagnostics
cannot erase a bound content activation. Missing catalog evidence blocks clean
absence without erasing known native activation. Client-generated body delivery
is kept distinct from a marker supplied in the operator's request.

Public records replace private roots and markers, omit model IDs and reasoning,
and retain
[source hashes](evidence/2026-10-30-q11b-registration-isolation/private-source-index.json)
and
[public receipt hashes](evidence/2026-10-30-q11b-registration-isolation/receipt-hashes.json).
Raw streams, model choices and original fixture bytes remain private. No
auth-bearing configuration bytes were exported. The completed cell supports a
bounded FAIL for this owner/observer pair and route, not universal client
behavior.
