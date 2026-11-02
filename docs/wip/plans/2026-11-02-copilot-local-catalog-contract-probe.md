# Copilot local catalog contract probe

Status: input prepared offline, 2026-11-02. No native command has run. Execution
remains blocked on preparing the bounded non-secret collector described below
and obtaining explicit approval for the concrete run. This is a contract probe,
not activation qualification or production admission.

The
[approved design](../specs/2026-10-31-four-client-skills-resources-design.md)
states: "Native activation, schema-2 production admission and qualification
waivers are not authorized by design approval." The completed Claude probe is
not authorization for this Copilot probe or for repeating Claude commands.

## Prepared input

The [fixture generator](../../../examples/native_adapter_fixture.rs) now accepts
`--copilot ABSENT_ABSOLUTE_ROOT`. Its original one-root syntax still prepares
Claude inputs. The Copilot path uses the actual selector, package compiler and
candidate user-command planner. Preparation invokes no native client.

Prepared plan:
`/private/tmp/upskill-copilot-contract-bgww20dj/packet/plan.json`. SHA-256:
`98586264ca21389533f9db3b2f0d6135587bfeb9ec34038c2962f7fc2610fe8f`. Preparation
receipt: `/private/tmp/upskill-copilot-contract-bgww20dj/preparation.json`, one
directory above the packet. It records generated-file digests/modes, OS/CPU and
static launcher/dependency identities. These temporary inputs are retained.

The packet contains nine files: captured source and resource; generated plugin,
catalog, skill and resource; a known non-secret project settings sentinel; a
manual home sentinel; and the plan. All files are mode `0600`, directories
`0700`. Copilot state under `home/.copilot` remains absent. The disposable home
already contains managed exports and the manual sentinel; it is not empty.

The generated plugin and marketplace share the full 72-character native name:
`upskill-924fc392d4b1c98352a84aacc1cc7605c2f6edd6603dce6a80e328a75f49786d`. The
package digest is
`ba87ed46d232ac86721a12e4a69882a9c573ac47c41e5cb5c3b04da3f36e9027`. Exports live
under the packet's **user** root `home/.upskill/exports/copilot/`; commands run
from its separate `project/` directory. This is no project-scope fallback and
says nothing about normal user-profile isolation.

The selected launcher is `/opt/homebrew/bin/copilot`, resolving to
`/opt/homebrew/lib/node_modules/@github/copilot/npm-loader.js`, SHA-256
`8ba289084c305e25ea9ae09da860c31614c7eb32f6e7ec099aa71f74f2d4f130`. Static
wrapper/platform package metadata says `1.0.4`; it does not establish the
previously captured runtime `1.0.105`. Revalidate the selected/resolved
launcher, Node and platform-entrypoint identities from `preparation.json`. The
first native command must match the complete retained
[version output](../../../tests/fixtures/native-adapters/2026-11-01-help/copilot-version.stdout).
A mismatch stops before inventories or mutations: no update, alternate binary,
version substitution or fallback route.

## Proposed execution envelope

Run only the eleven argument arrays in `plan.json`, once and in order. Every
array starts with `--no-auto-update`; arguments are passed separately without
shell evaluation. Maximum twenty seconds and 1 MiB combined output per command,
zero model/provider requests and zero authorized spending. No skill invocation,
chat, prompt, native update, trust override, force removal or broad cleanup is
included.

The sequence is: exact version; plugin and marketplace baseline JSON lists;
register the local catalog; install its exact plugin-at-marketplace selector;
list plugins and marketplaces; uninstall that selector; remove its marketplace;
list plugins and marketplaces. Mutation argv comes from the existing candidate
planner; list JSON flags come from the
[captured help](../../technotes/2026-11-01-native-adapter-command-contracts.md).
Do not infer CLI inventory or registry schemas from help.

Use closed stdin, the packet project as qualified CWD and an explicit minimal
environment. Set HOME to its `home/`, COPILOT_HOME to `home/.copilot`,
COPILOT_CACHE_HOME to `home/cache/copilot`, all XDG roots and TMPDIR below that
home, a fixed system PATH including the pinned Node and a fixed locale. Inherit
no credentials, client options, Node options or Git environment. Initialize the
fixture project offline with global/system Git configuration, templates and
hooks disabled. No normal profile is an input or requested write target.

Current
[GitHub configuration documentation](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference),
checked on 2026-11-02, distinguishes the configuration directory from the cache
directory and documents COPILOT_CACHE_HOME separately. Those documents do not
qualify these environment overrides on 1.0.105. Effective configuration/cache
roots, system policy, keychain access and delegated runtime remain unqualified;
an outside-packet source or unexpected authentication/trust requirement stops
the sequence without a workaround.

## Collector required before approval

Prepare and review a runner using the existing bounded subprocess API. It must
bind the plan digest, all fixture-file preconditions, launcher/dependency
identities and qualified root bindings before effects. Store chronological argv,
transport outcomes, bounded output lengths and non-secret observations in
private `0700`/`0600` evidence. Keep raw stdout/stderr only in bounded memory.
Unknown schema, duplicate keys or unexpected secret-bearing diagnostics stop the
run. Persist only sanitized structure and allowlisted fixture values; do not
dump unknown strings, dynamic map keys, outside paths or raw authentication
output.

Snapshot exact presence/bytes/modes only for the nine prepared non-secret files.
For native-created files, record bounded presence/type/mode observations; do not
snapshot, hash or journal native `config.json`, credentials, logs, sessions,
provider files or arbitrary generated configuration. The
[preservation contract](../specs/2026-10-25-customization-preservation-contract.md)
still applies. A later producer needing credential-bearing snapshots requires a
separate approved capture/storage contract. Preparing this input supplies none.

After any refusal, timeout, cancellation, incomplete output, unresolved process
group, changed binding, conflicting inventory or otherwise uncertain/partial
effect, stop. No further native inventory or cleanup command may run without an
independent witness that all relevant native workers have stopped. Reaping the
direct child and observing an empty process group do not establish
escaped-worker quiescence; no additional witness is admitted here. Retain the
packet and report Unknown. Never repair settings, delete caches or retry to make
the result pass.

On the fully completed transport path, only the planned exact-selector uninstall
and exact-marketplace removal may run within the eleven-command cap. Subsequent
inventories and file samples are bounded observations, not stable cleanup or
recovery proof. Leave all inputs and receipts intact.

## Decision evidence and limits

Current
[GitHub plugin documentation](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference)
caps legacy plugin names, marketplace names and plugin-entry names at 64
characters. The packet intentionally retains the approved full 72-character
name. Record an affirmative native refusal without assuming no partial effects;
do not truncate the name, weaken validation or try another encoding in this run.
Acceptance would inform this exact pin's contract, not override the other gates.

The useful outputs are pinned name/version acceptance or reported refusal,
local-directory inventory structure, fixture-file preservation observations and
native-created-file presence/mode residue. No result qualifies project scope,
manual authenticated configuration, cross-client isolation, skill/resource
activation, refresh, live/resumable retention, native-worker quiescence or the
complete adapter. Claude's retained preservation failure and every production
admission gate remain unchanged. No execution approval has been requested yet.
