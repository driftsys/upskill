# Claude local catalog contract probe

Status: prepared, awaiting explicit approval for native execution. This is a
bounded contract probe, not qualification or production admission.

The
[approved design](../specs/2026-10-31-four-client-skills-resources-design.md)
states: "Native activation, schema-2 production admission and qualification
waivers are not authorized by design approval."

## Prepared input

The offline [fixture generator](../../../examples/native_adapter_fixture.rs)
uses the actual skill compiler. It generates a resource-bearing Claude package
with the full 72-character native name and content-derived version. It creates
an absent absolute root and refuses to overwrite an existing root. No native
client is invoked by the generator.

The prepared packet is
`/private/tmp/upskill-claude-contract-ebiihhzh/packet/plan.json`. Its sibling
`project/` contains the generated catalog and a non-secret settings baseline
with a preservation sentinel. `home/` is disposable and empty. The packet is
local temporary evidence, not a published artifact.

Resolved launcher: `/opt/homebrew/Caskroom/claude-code/2.1.308/claude`. SHA-256:
`ca923c86ac84b30574a82331a05b6e066c27477a926ceaffe46a845a33e4dd37`. Revalidate
the launcher identity before execution and require exact version
`2.1.308 (Claude Code)`. A mismatch stops the probe without trying another pin.

## Execution envelope requiring approval

Run only the twelve argument arrays recorded in `plan.json`, in order, from its
`project/` directory, with closed stdin, an explicit environment and no shell
evaluation of arguments. Use the packet's empty HOME and separate
CLAUDE_CONFIG_DIR, XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME,
XDG_STATE_HOME and TMPDIR below that HOME. Preserve only a fixed system PATH and
locale; inherit no credentials, client options or Git environment. Initialize
the empty project Git repository offline with global/system Git configuration
disabled, an empty template and hooks disabled.

The sequence is: exact version; baseline plugin and marketplace lists; validate
the generated catalog; add that local catalog with explicit project scope;
install the exact plugin-at-marketplace selector with project scope; list
plugins and marketplaces; uninstall that selector with project scope; remove
that marketplace with project scope; list plugins and marketplaces. All paths
and selectors are literal values from the generated plan.

Maximum: twelve native commands, twenty seconds each, at most 1 MiB combined
output per command, zero model/provider requests and zero authorized spending.
There are no retries, trust overrides, alternate install routes, cache patches
or user-scope installation followed by disabling. Prompt requirements or an
unexpected command source stop the probe.

Capture baseline and subsequent presence, exact bytes and modes for protected
non-secret settings and local registry files. Keep bounded command results and
chronological receipts in this private packet. Do not persist unexpected
credential-bearing data; stop and retain only a non-secret diagnostic fact.
Record launcher identity and namespace stability around commands. HOME isolation
does not establish keychain or system-policy isolation.

If a command refuses before mutation, stop. After any timeout, cancellation,
incomplete transport, unresolved process group or otherwise uncertain/partial
mutation, stop the native sequence. Do not run further inventory or cleanup
commands until an independent witness establishes that all relevant native
workers have stopped. Reaping the direct child or finding its process group
empty does not establish quiescence of escaped or orphaned workers. No such
additional witness is admitted by this packet: unknown liveness means retain the
isolated packet and report Unknown, without inventory-based absence or cleanup
claims. Do not delete caches, roots or evidence to hide that state.

On the fully completed transport path, the planned cleanup commands may touch
only the exact fixture selector and catalog, in project scope and within the
twelve-command cap. Inventory and settings samples remain bounded observations;
do not claim stable cleanup without a qualified quiescence witness. A failed
cleanup leaves the packet intact and the outcome unresolved. Never modify the
normal user profile or infer that matching files alone establish native
recovery.

## Evidence and limits

The questions are whether the native CLI accepts the actual generated catalog,
full name and version; what local-directory registry records it writes; whether
project selectors are attributable; and whether removal restores the protected
settings presence, bytes and modes. An exit status alone does not answer these.
A settings rewrite is a preservation failure even if semantically equivalent.

No model command, skill invocation, competing-client observer, existing session
or provider input is tested. No result changes Q11/Q11b isolation results,
resource-retention status, profile admission or production dispatch. The
four-client study remains informative input on its separate branch.
