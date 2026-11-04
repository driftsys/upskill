# Copilot local catalog contract probe

Status: one contract attempt stopped at the baseline plugin inventory on
2026-11-02. The version matched 1.0.105; a source string failed the packet-only
precondition. No registration, installation or removal was attempted. The used
packet and diagnostic evidence are retained. This is not activation
qualification or production admission.

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

At preparation, the packet contained nine approved fixture files: captured
source and resource; generated plugin, catalog, skill and resource; a known
non-secret project settings sentinel; a manual home sentinel; and the plan. All
files are mode `0600`, directories `0700`. Copilot state under `home/.copilot`
was absent at preparation. The stopped run subsequently prepared its environment
and invoked the version and plugin-list commands. The disposable home contains
managed exports and the manual sentinel; it was not empty before execution.
Native-created content is not included in the nine approved snapshots.

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
first Copilot command must match the complete retained
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
model request ran; the prepared eleven-command packet was still unused at that
point.

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
- [x] Exercise chronological pending/completed receipts and fail-stop execution
      through the existing bounded runner with fake executables. Test unknown
      schemas, changed bindings, stderr, nonzero exit, timeout, cancellation and
      incomplete output. Prove no subsequent command or cleanup runs after
      failure. Review the complete collector before requesting approval for the
      concrete eleven-command native sequence.

## Maintainer collector

The research executable is
[`copilot_contract_probe`](../../../examples/copilot_contract_probe.rs). Its
syntax is
`copilot_contract_probe [--check|--execute|--execute-marketplace-check] ROOT PLAN_SHA256
PREPARATION_SHA256`.
The default and `--check` modes only validate captured inputs: they do not
create evidence, initialize Git or invoke Copilot. Explicit `--execute`
publishes initial evidence and attempts the bounded sequence once. The retained
real packet has now been consumed by one stopped attempt; do not reopen or
replay it. This adds no ordinary upskill command or admitted production profile.

Collection publishes and syncs `step-NN.pending.json` before each external
command, then exclusively writes its sanitized terminal result. Pending binds
the previous result digest (or initial header); each result binds its Pending.
Any uncertainty stops the handle permanently. Existing or partial records are
retained without reopening, replay or cleanup. Output stays in bounded memory;
version must match the complete retained bytes, lists must match the observed
strict schema and expected fixture claims, and mutation output must be empty.
Unexpected diagnostics, conflicting claims or outside-packet sources stop the
sequence. Filesystem source strings must be absolute paths under the packet and
contain no parent-directory components. The candidate revision counts the exact
`builtin` label on unrelated inventory rows without interpreting it as a
filesystem path or trusted origin. Owned namespace references never receive this
exemption. All other relative or ambiguous sources stop before mutations. This
lexical check does not resolve arbitrary native paths or prove filesystem source
binding. Plugin identity must match the exact expected name or selector as well
as its marketplace, version and enabled state. Successful transport and fixture
counts remain diagnostic facts. The initial evidence header records twelve
external commands in total: eleven client commands and one setup command.

Native file observations cover only `home/.copilot`,
`home/.copilot/config.json`, `home/cache/copilot` and `project/.git/config`.
Descriptor-relative, non-following metadata reads record presence, type and
mode. No native file contents, content hashes or dynamic paths enter evidence.
The approved nine fixture snapshots remain the only stored file contents.

`tests/native_copilot_collector.rs` exercises the real bounded runner with fake
client code, including exact argv, qualified CWD, closed environment, digest
chronology, timeout/cancellation, conflicting inventory, input/code/storage/root
drift and native configuration secret canaries. These local controls do not
establish the real client's behavior or authorize its execution. Collector and
review fixes at `5e2d9c2` passed full `just verify`, all-target Clippy and Rust
1.85 checks. Before execution, the retained packet separately passed the
executable's read-only check without creating evidence, Git state or Copilot
configuration.

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
admission gate remain unchanged. The stopped attempt authorizes no further
native observation or retry.

## Stopped contract attempt on 2026-11-02

The collector ran one offline Git setup and two Copilot commands: exact version,
then the baseline `plugin list --json`. All three exited zero with complete
bounded transport and zero stderr; executable, root and known-input checks
passed. Version output matched the retained 1.0.105 bytes. The plugin response
was 132 bytes. The collector rejected a source string under its packet-only
policy and recorded `outside_packet_source`. That reason includes relative and
otherwise ambiguous sources; it does not establish an actual outside-profile
read or the entry's origin. Neither the source value nor its lexical category
was retained.

The handle stopped permanently. The marketplace baseline and every registration,
installation, removal and final-inventory command are NOT RUN. Full-name and
hash-version acceptance therefore remain unmeasured. There were zero model
requests and zero registration/install attempts. Direct-child reaping and an
empty process group were observed; native-worker quiescence remains unqualified.
No cleanup, repair or retry ran.

The packet's `evidence/` contains sixteen private files: the initial header and
nine approved snapshots, then three Pending/result pairs with verified digest
chronology. Post-run verification found all nine fixture bytes and modes
unchanged. This samples those known non-secret inputs, not ordinary
authenticated configuration or unknown native-created files. The separate run
receipt and verification are retained under
`/private/tmp/upskill-copilot-contract-run-ngjdd493/`. The packet remains
intact.

## Separate source classification check

Status: executed once after user approval on 2026-11-02; no model requests.

The private diagnostic root is retained at
`/private/tmp/upskill-copilot-source-check-3dtx0p4k/`. Its `source-check.rs` and
executable reuse the bounded runner and strict observed plugin-inventory
validator. Self-tests cover the five lexical categories, unknown fields, decoded
duplicate rejection and secret canaries. Five fake-client cases cover exact
argv, version mismatch, stderr, malformed inventory, credential environment
exclusion and cancellation after a positive readiness control. The approved
two-query run consumed this root once; it must not be replayed. `ready.json`
binds source, executable, runner and reviewed code-metadata digests. The
`execute.py --check` entrypoint passed without invoking a native executable. Its
explicit `--execute` mode exclusively creates a consumed-run marker before the
two queries; it is not authorized by preparing or checking the root.

The proposed envelope contains only two commands, once and in order:
`["--no-auto-update", "--version"]`, then
`["--no-auto-update", "plugin", "list", "--json"]`. Each has a twenty-second
limit; version output is capped at 512 bytes and inventory at 1 MiB. Use closed
stdin, explicit private HOME/Copilot/cache/XDG/temp roots, the captured Node
PATH and the original reviewed launcher, Node and platform code pins. Zero model
requests and zero spending are authorized. There is no registration,
installation, removal, cleanup, fallback or packet replay.

The projection retains only entry counts and counts of URI, parent-component,
relative, absolute lexically inside and absolute lexically outside sources. It
also counts exact comparisons with the fixed literals `builtin` and `built-in`;
those comparisons do not establish trusted builtin origin. Unknown names,
marketplaces, versions, source strings and paths remain discarded. The
projection opens no source paths and snapshots no native configuration.

The user approved this separate research envelope. It permitted one additional
non-stable read-only observation after the original run's stop. It did not
resume that run or supply a quiescence witness. Lifecycle/effect observations,
cleanup, qualification and production admission retain all their existing
requirements. The source check itself changed no collector guard.

### Observed result and candidate handling

Both queries exited zero with complete transport, zero stderr and unchanged
captured code identities. The version matched 1.0.105. The plugin inventory
contained one entry: its source was relative and matched one of the fixed
`builtin`/`built-in` literals. URI, parent-component and both absolute-source
counts were zero. The projector opened no source paths and retained no native
scalar values. This is a time-sampled classification, not trusted-origin,
effective-root, worker-quiescence or qualification evidence. The immutable
sanitized result is `summary.json` in the diagnostic root.

Static inspection of that runtime's `cli-main.js` found explicit handling of
`source === "builtin"` separately from non-builtin plugin records. Its listing
logic groups those labels apart from user-installed records. File identity and
digest provenance are retained in `runtime-code-inspection.json` beside the
source-check receipt; no native configuration was read. This explains a concrete
route to the earlier packet-path rejection without proving the identity or
origin of that earlier entry.

The candidate diagnostic revision passed its focused offline checks. It counts
the exact `builtin` label only on unrelated inventory rows, with no origin or
binding claim. Every owned name, plugin selector or marketplace reference still
requires the exact expected identity and private source path. Unknown relative
labels, URI sources, parent components and outside absolute paths still stop.
The hyphenated `built-in` spelling remains unsupported by this candidate; the
static code inspection establishes only `builtin` handling. Initial evidence
declares this interpretation policy and terminal observations count the
unrelated labels. No original packet replay or new native mutation is authorized
by this code change; any future contract trial requires a fresh reviewed packet
and execution decision. Production admission and Q11 remain unchanged.

The focused checks cover 38 collector, evidence, preflight and inventory
acceptance tests plus five probe unit tests. All-target Clippy, Rust 1.85
compatibility, lint and documentation checks passed. A deliberate mutation
removing the owned-reference exclusion compiled and failed the owned-reference
acceptance assertion; the source was then restored byte for byte. An additional
regression case requires the exact marketplace name rather than accepting its
plugin selector spelling. Full repository verification was retried after the
review follow-up below. These are fake-client checks only.

On 2026-11-03 the review follow-up extended ambiguous-source refusals to both
inventory kinds, added builtin-labelled marketplace selector rejection, and
asserted zero and multiple builtin entries mixed with unrelated packet-local
filesystem sources. All eleven collector tests passed. Three deliberate
mutations compiled and failed their acceptance assertions: admitting marketplace
`built-in`, exempting marketplace selector references, and counting multiple
entries as one. Production source was restored byte for byte after each
experiment. The two stale preservation/execution statements were corrected. The
previous full verification failed in two recovery tests with disk exhaustion;
that is not a passing result. After disk capacity was restored, full
`just verify` passed at `2f25137f2ff9eb487a093c222c168a0552b7c98c`. The receipt
is `/private/tmp/copilot-builtin-review-fixes-full-verify-result.json` and its
complete log is the adjacent `copilot-builtin-review-fixes-full-verify.log`. No
real native command was run by this review follow-up.

The required quick review of that follow-up found no documentation mismatch. Two
test-strengthening advisories remain: add a distinct persistence canary for
accepted unrelated filesystem source values, and assert the complete mixed
inventory totals in the collector receipts. These are unimplemented follow-ups,
not demonstrated runtime failures or qualification evidence. Hosted debt filing
remains pending because the command runner receives GitHub HTTP 401.

## Fresh contract retry prepared on 2026-11-03

Status: executed once after explicit user approval on 2026-11-04. Stopped at the
baseline marketplace inventory before registration or installation; see the
result below. The packet is consumed and must not be replayed.

The prepared packet was
`/private/tmp/upskill-copilot-contract-v2-be_l1eav/packet`. The actual fixture
compiler generated the new root-bound full 72-character name and payload; no
original packet name or hash was substituted. The plan SHA-256 is
`f376208e6a03e82dabd097e50fdb160f8bfd1ca46c3d8b0b8f83d08f864b31d6`. The
preparation receipt SHA-256 is
`7e7fa4397364276f1fe6936347538f83c91a8df74a30f17507616c78ac58107e`. At
preparation, the plan and preparation were retained as `packet/plan.json` and
`preparation.json` under that private root.

The maintainer diagnostic was rebuilt at `e4c793d`, whose full `just verify`
passed. Its driver SHA-256 is
`f480d3b99fcdda7918a1b0466412a4326e555c848edb8d2e3dd5cada2d25f6db`. The
`retry-preflight.json` receipt records all nine fixture files accepted,
unchanged captured launcher/Node/platform inputs and no existing native state or
evidence. This capture invokes no native client and qualifies no effective root.
`proposed-retry.json` records the proposed bounds beside it.

The proposed envelope is one fixed offline Git setup and at most eleven fixed
Copilot commands, once and in order, each bounded to twenty seconds and 1 MiB
combined output. It requests zero models and authorizes zero spending. It uses
the same version, inventories, local registration/install, detach and final
inventory sequence specified above, under this new disposable packet. The
candidate counts exact `builtin` labels only on unrelated inventory rows; these
are metadata counts, never trusted-origin or binding evidence. Owned names,
selectors and marketplace references still require exact identity and private
source paths. Every other non-packet or ambiguous source stops.

Execution requires a fresh explicit decision for this packet and interpretation
policy. Revalidate the input, code and root pins before starting, and retain the
first Pending/result sequence without replay, fallback or broad cleanup. The
original stopped packet stays untouched. This is a bounded contract experiment;
it does not waive preservation, quiescence, isolation or production admission.

## Approved retry result, 2026-11-04

The user explicitly approved the prepared packet and its one-setup/eleven-client
command envelope. The rebuilt driver matched the fresh read-only receipt
(`/private/tmp/upskill-gh-resume-preflight.json`) and source head `9c5f2e4`;
production source and example inputs were unchanged. The once-only collector ran
one offline Git setup and three Copilot commands: version, baseline plugin list
and baseline marketplace list. All four exited zero with complete bounded
transport, zero stderr and unchanged code/root/input observations.

The version matched 1.0.105. The plugin inventory passed with one unrelated
exact `builtin` label and no fixture-named entry. The marketplace response was
113 bytes and failed the packet-source policy with `outside_packet_source`. Its
scalar source value was not retained; this reason does not distinguish an
outside absolute path from a relative or otherwise ambiguous source and does not
prove an outside-profile read. No marketplace origin claim follows.

The collector stopped permanently before registration, installation, removal or
final inventories. No retry, fallback, cleanup, model request or spending ran.
The wrapper exited zero because it successfully reported an incomplete
experiment; its summary explicitly has `complete: false` and `qualified: false`.
Direct children were reaped and process groups observed empty, but native-worker
quiescence remains unqualified.

Read-only verification checked all four Pending/result digest links and all nine
known fixture files against their exact retained bytes and modes. The sanitized
[summary](../../technotes/evidence/2026-11-04-copilot-contract-retry/summary.json)
retains the result projections and terminal digest. Private inputs, snapshots,
Pending/result receipts, approval and execution wrapper result were recorded
under `/private/tmp/upskill-copilot-contract-v2-be_l1eav/`. The original stopped
packet is unchanged. Full-name acceptance, installation and every native
qualification/admission gate remain unresolved. Further native diagnostics need
a separate bounded decision; this failed run supplies no replay authority.

## Offline rejection diagnostics, 2026-11-04

The installed JavaScript copy at
`/opt/homebrew/lib/node_modules/@github/copilot/app.js` formats marketplace
source values with `GitHub:`, `Local:` or `URL:` prefixes. The
[static inspection receipt](../../technotes/evidence/2026-11-04-copilot-contract-retry/static-source-inspection.json)
records its digest and exact method. This copy is not proven to be the runtime
used by the trial: the private packet and prior inspection paths under
`/private/tmp` were no longer present when this follow-up began. Their absence
is not cleanup evidence. The tracked sanitized retry summary remains available.

The collector now retains `source_rejection_category` on a source rejection. The
fixed lexical labels are `display_github`, `display_local`, `display_url`,
`uri`, `parent_component`, `outside_absolute` and `relative`, checked in that
order. They describe string shape only, without retaining any suffix, path, name
or other native value. Other post-invocation failures emit a null category;
early pre-invocation refusals retain their existing receipt shape. The stop
reason remains `outside_packet_source`; the collection still stops immediately
and the failed observation stays null. No prefix is stripped, path opened or
source admitted through this diagnostic.

The first new fake-client acceptance run failed because the category was absent.
After implementation it passed all seven marketplace-source cases, including
exact stop order and evidence scans for private canaries. No native command ran
and the old rejected marketplace value cannot be reconstructed. A future fresh
diagnostic still needs its own bounded execution decision.

Independent scoped review found no functional or privacy defect. All 1,751 Rust
tests passed. The full verification command then stopped on formatting in the
new local review note; after formatting that note, lint, documentation checks
and the build each passed. The
[offline verification receipt](../../technotes/evidence/2026-11-04-copilot-contract-retry/offline-verification.json)
records the distinct command outcomes and source digests.

## Prepared marketplace source check

The [reduced source check](2026-11-04-copilot-marketplace-source-check.md)
selects only offline Git setup, the pinned version and baseline marketplace
inventory. It uses a fresh packet and cannot continue into registration. Its new
library and example-dispatch acceptance tests passed. A fresh explicit execution
decision remains required; the old consumed packet is not replayed.

## Marketplace display interpretation fix

The offline collector now distinguishes marketplace display metadata from plugin
filesystem sources. `Local:` is decoded only for marketplace rows, then
subjected to the existing absolute packet-path and parent-component checks. A
matching owned marketplace must have its exact native name, point to the
captured catalog rather than an arbitrary revision, and report
`isDefault: false`. Selector spellings, wrong directories, default-owned claims
and remote-owned claims reject. Plugin source handling is unchanged.

Unrelated marketplace rows reporting `isDefault: true` may contribute a count
when their source is a recognized `GitHub: owner/repository` or HTTP(S) `URL:`
display value. GitHub components must be nonempty ASCII repository-name
components and cannot be dot or parent components. URL displays require a
nonempty authority without user information and no whitespace or control
characters; this is lexical classification, not full URI validation. The
collector does not open or fetch either value. A default flag and recognized
display label are metadata, not trusted-origin, isolation or binding evidence.
Non-default remote entries, unsupported labels and malformed display values
still stop. No source is truncated, normalized into an owned binding or emitted
into retained evidence.

The header declares this interpretation policy. Results expose only
`unrelated_default_remote_display_entries`; no native names or display suffixes
are retained. New acceptance coverage includes the complete fake-client
registration/install/detach sequence with unrelated remote metadata and an exact
owned `Local:` catalog, private-value canaries, and negative ownership/scope
cases. Reduced-route tests now compare actual fake-client argv as well as
Pending records. The URI-before-parent precedence and full-route collection
discriminator are pinned too.

This corrects the diagnostic collector. The retained native failures and their
historical interpretation remain unchanged. No new native installation trial,
profile qualification or production admission is implied.

The source-policy fix passed `RUST_TEST_THREADS=1 just verify`. The serial
harness avoids concurrent test execution; individual limits remain unchanged.
Registered-inventory controls independently caught removal of exact-catalog
matching and removal of the non-default requirement in a separate source
snapshot. The snapshot was restored and matched the working source byte for
byte; current focused guard and actual-argv tests also passed. The
[verification receipt](../../technotes/evidence/2026-11-05-copilot-marketplace-display-fix/verification.json)
records these separate outcomes and source digests.

A fresh read-only baseline validation then ran one offline Git setup, the exact
1.0.105 version query and marketplace inventory, once. It completed successfully
with one unrelated default remote display entry, zero fixture-named entries,
complete transport and no source-format refusal. All nine fixture files and
three Pending/result digest links were verified. The sanitized
[result](../../technotes/evidence/2026-11-05-copilot-marketplace-display-fix/summary.json)
retains the declared policy and observations. No native registration,
installation or cleanup was selected, no model request ran, and no worker
quiescence, isolation or production profile was qualified.
