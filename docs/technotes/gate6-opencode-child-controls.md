# Gate 6 OpenCode child execution and permission observations

Task [#358](https://github.com/driftsys/upskill/issues/358) retains a bounded
[#337](https://github.com/driftsys/upskill/issues/337) native evidence slice.
The shared-root comparison remains unadopted, all 160 delivery cells remain
unqualified, and production stays schema 1. This records actual native child
execution; it does not implement an upskill agent adapter or expand M1 agent,
tool, permission or MCP management.

## Frozen context and accounting

OpenCode v2.0.32 ran on macOS arm64 in disposable project fixtures, using the
normally provisioned isolated OpenCode profile and explicit model
`github-copilot/gpt-6.1-sol`. Actual child exports identify provider
`github-copilot`, model `gpt-6.1-sol` and variant `default`. Native runs used
`--standalone --format json`, without `--auto`. The normal provider handled
authentication; no credentials or authentication-store contents were copied or
retained. An isolated authentication root does not qualify user-scope delivery.

The [summary](evidence/2026-10-22-gate6-opencode-child-controls/summary.json)
accounts for six native parents and six actual children, with distinct session
identities and matching parent/child links. Each parent made one completed
`subagent` call and returned its declared exact response. Each child reached a
native `idle` message with outcome `succeeded`; that means it completed its
instruction, including the expected denial response where applicable. It does
not mean every attempted tool operation succeeded.

There are 13 native tool calls: six parent delegations and seven child calls.
Twelve completed and one child patch received an actual native permission error.
The first two children used no tools; the remaining children made respectively
read/patch, read only, read/patch and read/patch calls.

## Bounded observations

| Control                  | Native observation                                                                                                                          | Boundary                                                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Configured child         | Parent launches the named project-configured child; child export confirms identity, model and successful completion                         | Actual configured delegation, not a file-catalog discovery result                                                 |
| File-backed child        | Parent launches the named `.opencode/agents/*.md` child; export confirms that child, model and successful completion                        | Actual file-backed execution; earlier empty `debug agents` output remains inconclusive                            |
| Narrow edit allow        | File-backed child reads and patches `sentinel.txt`; exact bytes and mode are captured after the successful operation                        | Independent narrow positive with deny-all followed by sentinel read/edit allowances                               |
| Edit unavailable         | Child reads the sentinel and returns the declared denial response; no patch call occurs and target stays unchanged                          | No attempted native edit rejection; a model response alone does not prove enforcement                             |
| Path experiment positive | Child reads and patches the sentinel successfully                                                                                           | Helper accidentally retained an additional `edit "*.txt"` allowance; broader policy than the following negative   |
| Path experiment negative | Child attempts a patch of `other.txt`; native tool returns `permission.rejected`, `Permission denied: edit`; protected file stays unchanged | Actual nonmatching-path rejection with only sentinel edit allowed; not an identical-policy positive/negative pair |

The last pair has an explicit policy mismatch. The independent narrow positive
supports allowed sentinel editing, but does not repair that pair
retrospectively. The
[path-policy audit](evidence/2026-10-22-gate6-opencode-child-controls/path-policy-audit.json)
retains both actual agent snapshots and the limitation. No single-policy scope
qualification follows from these observations.

Parent project permissions deny all actions except the named child launch;
children carry their own ordered native permission rules. These are manual
native fixtures, not outputs from the historical generator. Its legacy
permission map is not thereby qualified for this v2 runtime. No instruction-only
restriction substitutes for a native deny, and these cases do not establish MCP
inheritance, further-delegation limits or rule scoping inside child sessions.

## Resource identity, preservation and restoration

Native sanitized child exports redact tool inputs. Separate
[patch identity receipts](evidence/2026-10-22-gate6-opencode-child-controls/patch-resource-identities.json)
bind three patch call IDs to the actual owned fixture resources: narrow allow
and broader path positive target the sentinel; the rejected patch targets the
protected other file. The capture inspected only patch headers from exports in
memory and asserted paths remained within the disposable project. Full
unsanitized exports were not saved; provider state and authentication data were
not selected. The retained `owned_fixture_resources` field is a resource
binding, not filesystem-root qualification or an executor authority grant.

Before/after metadata retains manual team guidance, project configuration,
sentinel and unrelated-file bytes, hashes and modes. Successful fixture edits
were inspected before guarded restoration; the negative leaves the protected
file unchanged. The file-backed calibration restores the captured parent
configuration and preserves team guidance. These observations do not establish
upskill installation/update/removal, unrelated plugin ownership, atomicity or
crash recovery. No production user configuration was rewritten to force a pass.

## Parser correction and retained evidence

The first export-summary helper assumed legacy `messages[].info/parts` and
produced empty agent/model/tool arrays. Native v2 exports instead put
`agent/model/content` directly on assistant messages and `outcome` on an idle
message. The
[correction](evidence/2026-10-22-gate6-opencode-child-controls/export-parser-correction.json)
records this boundary. Those earlier empty arrays never establish absent agents
or tools. Actual child exports and terminal outcomes supply the retained facts.
Original private receipts and manifests remain unchanged.

The
[source index](evidence/2026-10-22-gate6-opencode-child-controls/private-source-index.json)
maps aliases to exact private disposable packet paths, original manifest hashes
and capture-helper hashes for independent local inspection. The packet retains
selected parent JSONL events, child identities/model/content/idle outcomes,
declarations and before/after snapshots. Provider state, accounting, snapshots
and child tool output bodies are omitted. Raw and selected/sanitized receipt
hashes differ intentionally. A separate
[retained manifest](evidence/2026-10-22-gate6-opencode-child-controls/receipt-hashes.json)
covers the repository evidence files. Private capture and retention helpers are
not shipped or tested qualification runners; this packet is informative data,
not executable product input.

## Remaining qualification

These cases establish bounded native child execution and one actual rejected
operation. They do not qualify four-client coexistence, shared `.agents/agents`
discovery, compatibility-directory exclusion, duplicate activation, automatic
selection, scoped rules, MCP inheritance, delegation limits, user roots, Linux
x86_64, IDE surfaces or integrated ownership and recovery. Gate 6 Task 2 and
CD-11 remain incomplete. No shared root or adapter is selected or adopted, and
schema-2 activation remains blocked.
