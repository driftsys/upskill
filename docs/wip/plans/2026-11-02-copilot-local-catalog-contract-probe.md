# Copilot local catalog contract probe

Status: input prepared offline, 2026-11-02. The prepared packet remains unused.
Separate read-only version and inventory diagnostics are recorded below. The
bounded non-secret collector is implemented locally; broader verification and
review remain open. Execution requires explicit approval for the concrete run.
This is a contract probe, not activation qualification or production admission.

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

## Separate read-only version observation

One ordinary read-only version query ran on 2026-11-02, outside the prepared
packet, from `/private/tmp/upskill-copilot-version-u_3_6t7_/project` with its
own fresh HOME, Copilot config/cache and XDG/temp roots. It used the bounded
runner, closed stdin, an explicit minimal environment, a twenty-second deadline
and a 512-byte combined-output cap. The only argv was
`["--no-auto-update", "--version"]`. No inventory, registration, installation,
removal or model request ran.

The receipt `/private/tmp/upskill-copilot-version-u_3_6t7_/summary.json` records
exit code zero, complete bounded transport, 71 stdout bytes, zero stderr bytes
and an exact match to the retained version output: runtime **1.0.105**. Captured
launcher, Node and platform-entrypoint identities matched before and after. The
wrapper's static `1.0.4` metadata therefore does not identify the reported CLI
version. Raw native output was processed in bounded memory, without persistent
dumps or native configuration snapshots. The summary retains sanitized facts and
driver source/binary digests. The driver source and executable remain beside the
receipt; expected version bytes came from the linked retained capture.

This is version discovery under the existing read-only inspection scope, not
approval for the eleven-command sequence or qualification of effective roots,
worker quiescence, credentials, model availability or billing. Design approval's
activation/admission/waiver restrictions remain unchanged.

## Separate read-only inventory structure observation

A second, separate diagnostic completed on 2026-11-02 under
`/private/tmp/upskill-copilot-inventory-dr6bva6w/`. Its private `summary.json`
records three commands: the exact version query, `plugin list --json` and
`plugin marketplace list --json`, each prefixed with `--no-auto-update`. All
three exited zero with complete bounded transport, zero stderr bytes and
unchanged captured launcher, Node and platform identities. The version output
matched the retained 1.0.105 pin. No registration, installation, removal or
model request ran; the prepared eleven-command packet remains unused.

The plugin list returned an array containing one object with string fields
`name`, `marketplace`, `source`, `version`, and a boolean `enabled` field. The
marketplace list returned an array containing one object with string fields
`name`, `source`, and a boolean `isDefault` field. Neither sampled object had
unknown keys. Their output lengths were 132 and 113 bytes respectively. All
scalar values were discarded; this evidence cannot identify either entry or
establish its origin, ownership, source binding or effective configuration root.
In particular, a fresh diagnostic home did not produce empty inventories.

The helper used the existing bounded subprocess runner, qualified disposable
CWD, closed stdin and explicit isolated HOME/config/cache/XDG/temp settings.
Each command had a twenty-second deadline; list output was capped at 1 MiB. Its
projection self-test passed before native calls, including decoded duplicate key
rejection and omission of secret canaries, unknown field names and scalar
values. Raw native output stayed in memory. No native configuration snapshots
were retained. The receipt retains source, executable and parser digests; source
and executable remain beside it. This is observed wire structure only, not a
complete inventory contract, isolation proof or admission evidence.

## Proposed execution envelope

Run one separately bounded offline Git setup, then only the eleven client
argument arrays in `plan.json`, once and in order. Every client array starts
with `--no-auto-update`; arguments are passed separately without shell
evaluation. Maximum twenty seconds and 1 MiB combined output per command, zero
model/provider requests and zero authorized spending. No skill invocation, chat,
prompt, native update, trust override, force removal or broad cleanup is
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
hooks disabled. The setup executable is exactly `/usr/bin/git`, captured before
execution with its resolved path, code digest, length and mode in Pending. Its
exact argv is
`["-c", "init.defaultBranch=main", "-c",
"core.hooksPath=/dev/null", "-c", "init.templateDir=", "init", "--quiet",
"--template=", "."]`.
It has the same twenty-second and 1 MiB bounds. The execution envelope therefore
permits at most **one setup command and eleven client commands**, with no
fallback. This separate setup does not change the retained plan or its
client-command cap. No normal profile is an input or requested write target.

Current
[GitHub configuration documentation](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference),
checked on 2026-11-02, distinguishes the configuration directory from the cache
directory and documents COPILOT_CACHE_HOME separately. Those documents do not
qualify these environment overrides on 1.0.105. Effective configuration/cache
roots, system policy, keychain access and delegated runtime remain unqualified;
an outside-packet source or unexpected authentication/trust requirement stops
the sequence without a workaround.

## Collector implementation sequence

Execute inline under the existing Stage 6 plan. This sequence implements the
execution envelope below; it does not authorize a native mutation run.

- [x] Add `src/native_delivery/copilot/probe.rs` with
      `summarize_inventory(version, kind, bytes)` returning a serializable count
      summary or a static typed error. Pin 1.0.105, the 1 MiB bound, decoded
      unique keys and the observed exact row field types. Reject unknown or
      missing fields; discard all scalar values. Acceptance:
      `tests/native_copilot_probe.rs` covers both lists, later malformed rows,
      version drift, decoded duplicates, unexpected diagnostics, scalar canaries
      and the exact byte boundary. This pure component neither invokes a client
      nor compares fixture selectors; it cannot prove ownership, source binding,
      absence or effective scope.
- [x] Add the bounded collector's input validation and once-only evidence
      orchestration. Capture and revalidation live in
      `src/native_delivery/copilot/probe/preflight.rs`, with separate contract
      validation and descriptor-relative file helpers. Its public
      `CopilotProbePreflight::capture(&CopilotProbeRequest)` accepts the packet
      root and separately reviewed plan/preparation SHA-256 values;
      `verify_unchanged()` rechecks retained files, directories and code assets.
      `tests/native_copilot_preflight.rs` uses the real offline fixture
      generator and positively controlled fake executable assets. Cover rehashed
      plan drift, unknown manifest paths, links, hard links, unsafe modes,
      existing evidence/native state and later root/file/code replacement.
      Capture and revalidation perform no commands or writes and are not
      once-only execution evidence. Rebuild expected packages from fixed
      non-secret source with the existing compiler: native names and payload
      hashes include the scope root. Do not substitute another root's literal
      hashes. Read only fixed fixture paths; bound plan reads to 16 KiB and
      preparation reads to 64 KiB. Require private directories, ordinary
      single-link `0600` fixture files and pinned code identities. Preservation
      rechecks sample known inputs and bindings while ignoring new native
      configuration content; they do not freeze namespaces or qualify ACLs or
      effective native roots. Before any native call, bind plan/preparation
      digests, exact fixed argv, known fixture bytes/modes, root identities and
      executable dependencies. Test changed bytes, symlinks, modes, roots and
      existing evidence against fake executables; none may invoke a client.
      Initial evidence publication lives in
      `src/native_delivery/copilot/probe/preflight/evidence.rs`:
      `CopilotProbePreflight::begin_evidence()` returns a borrowed
      `CopilotProbeEvidence` holding live root locks. Revalidate unused inputs,
      exclusively create `evidence/`, sync its parent, then exclusively write
      and sync a non-secret `header.json` and nine fixture snapshots at `0600`.
      Retain existing and partial evidence without replay or cleanup. Evidence
      verification must detect root, directory, file identity/content/mode and
      unexpected-entry drift; any error permanently stops that handle.
      `tests/native_copilot_evidence.rs` covers publication, two captured
      candidates, drop retention, initial drift, later evidence/input drift and
      exclusion of new native configuration. Private publication fault tests
      must retain partial evidence and reject reopening. This initial store
      publishes no command Pending or effect receipt and runs no native client.
- [ ] Exercise chronological pending/completed receipts and fail-stop execution
      through the existing bounded runner with fake executables. Test unknown
      schemas, changed bindings, stderr, nonzero exit, timeout, cancellation and
      incomplete output. Prove no subsequent command or cleanup runs after
      failure. Review the complete collector before requesting approval for the
      concrete eleven-command native sequence.

## Maintainer collector

The research executable is
[`copilot_contract_probe`](../../../examples/copilot_contract_probe.rs). Its
syntax is
`copilot_contract_probe [--check|--execute] ROOT PLAN_SHA256
PREPARATION_SHA256`.
The default and `--check` modes only validate captured inputs: they do not
create evidence, initialize Git or invoke Copilot. Explicit `--execute`
publishes initial evidence and attempts the bounded sequence once. Do not use it
on the retained real packet before execution approval. This adds no ordinary
upskill command or admitted production profile.

Collection publishes and syncs `step-NN.pending.json` before each external
command, then exclusively writes its sanitized terminal result. Pending binds
the previous result digest (or initial header); each result binds its Pending.
Any uncertainty stops the handle permanently. Existing or partial records are
retained without reopening, replay or cleanup. Output stays in bounded memory;
version must match the complete retained bytes, lists must match the observed
strict schema and expected fixture claims, and mutation output must be empty.
Unexpected diagnostics, conflicting claims or outside-packet sources stop the
sequence. Source strings must be absolute paths under the packet and contain no
parent-directory components; relative or otherwise ambiguous origins stop before
mutations. This lexical check does not resolve arbitrary native paths or prove
filesystem source binding. Plugin identity must match the exact expected name or
selector as well as its marketplace, version and enabled state. Successful
transport and fixture counts remain diagnostic facts. The initial evidence
header records twelve external commands in total: eleven client commands and one
setup command.

Native file observations cover only `home/.copilot`,
`home/.copilot/config.json`, `home/cache/copilot` and `project/.git/config`.
Descriptor-relative, non-following metadata reads record presence, type and
mode. No native file contents, content hashes or dynamic paths enter evidence.
The approved nine fixture snapshots remain the only stored file contents.

`tests/native_copilot_collector.rs` exercises the real bounded runner with fake
client code, including exact argv, qualified CWD, closed environment, digest
chronology, timeout/cancellation, conflicting inventory, input/code/storage/root
drift and native configuration secret canaries. These local controls do not
establish the real client's behavior or authorize its execution.

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
