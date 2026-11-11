# Fresh Copilot installation contract trial

Status: executed once on 2026-11-11 and stopped at sequence 4; see Outcome. This
packet replaces the packet of 2026-11-05, which was built for Copilot 1.0.105
and was never executed. Preparing and validating it authorizes no native
mutation. The full generated name remains unchanged at 72 characters.

## Purpose and exact input

Test registration of the generated local marketplace, installation of its exact
plugin selector, matching inventories, then planned removal if every preceding
step remains accepted. This is a bounded contract experiment, not activation,
resource/session retention, isolation, worker-quiescence or production
admission.

- Packet: `/private/tmp/upskill-copilot-install-trial-4bw_cs25/packet`.
- Plan SHA-256:
  `8a7a1bf16a31776ecf60899367ea078a1ed98f7bf7589f9135f9c59729fa4519`.
- Preparation SHA-256:
  `f3ea25c5de58a63caa6f1cd73aa14fa39d7e4ee818ef6d3a9e3bb3f68c111519`.
- Native plugin and marketplace name:
  `upskill-ce53747012a89f509314561dba2b381e44d3fe17c187a2cdc56d1f7f181dd827`.

The actual fixture compiler produced the package and plan. The nine non-secret
fixture files are bound by bytes, modes and digests. Directories are private
`0700`, fixtures `0600`. Selected/resolved launcher, Node and platform
entrypoint code pins are bound by the packet. Runtime must match the retained
1.0.107 version output at the first client command.

The client is a private, non-updating installation of `@github/copilot@1.0.107`
at `~/.local/share/upskill-trial/copilot-1.0.107`. It is used because the
wrapper and the runtime both report 1.0.107, and because every Copilot argv
starts with `--no-auto-update` and all package roots are inside the isolated
packet HOME. A code review found that the update route is skipped under that
flag; this has not been observed in a native run.

The ten `plugin ... --help` outputs of this installation, captured with an
isolated HOME and no model use, are byte-identical to
`tests/fixtures/native-adapters/2026-11-01-help/`; only `--version` differs.
This equality covers command and option text only. The JSON schema of the
inventory commands at 1.0.107 has not been observed, so the strict inventory
checks remain unconfirmed against this version.

The
[proposed trial manifest](../../technotes/evidence/2026-11-10-copilot-installation-trial-preparation/proposed-trial.json)
contains every exact argv array, generated package identities and bounds. The
[readiness receipt](../../technotes/evidence/2026-11-10-copilot-installation-trial-preparation/readiness.json)
binds the driver executable and seven source digests at commit `1b75a97`. The
source list is the same as on 2026-11-05 and does not cover every module that
runs on `--execute`; the driver digest pins the binary that runs. Revalidate all
pins and unused packet state again immediately before launch; drift stops before
effects, without recapturing a new profile as a fallback.

## Once-only command envelope

At most one fixed offline `/usr/bin/git` setup and eleven fixed Copilot
commands, once and in manifest order. Every Copilot argv starts with
`--no-auto-update`. Use closed stdin, the packet project as qualified CWD, the
packet HOME/Copilot/ cache/XDG/temp roots, disabled global/system Git
configuration, pinned Node PATH and fixed locale. Inherit no credentials or
client/Node options. Each command has a twenty-second deadline and 1 MiB
combined output cap. Zero model/provider requests and zero spending are
authorized by the proposed envelope.

The eleven client commands are: exact version; baseline plugin and marketplace
JSON lists; local catalog registration; exact-selector installation; plugin and
marketplace JSON lists; exact-selector uninstall; exact-marketplace removal;
final plugin and marketplace JSON lists. Only the unchanged candidate planner's
user-context route is selected. No project fallback, name truncation, native
update, chat, skill invocation, trust override or force removal is included.

A refusal or unexpected output may stop this trial before installation. Even a
zero-exit mutation with nonempty stdout is not accepted by the current strict
collector; it may already have effects and must be retained as incomplete. Do
not repair or retry merely to reach a later step. Full-name acceptance remains
unmeasured; any name refusal also stops rather than selecting a shorter name.

## Validation, privacy and stop conditions

The header declares the reviewed interpretation: default remote marketplace
GitHub/URL displays and unrelated builtin labels are metadata counts only.
Marketplace Local labels retain packet containment checks; owned rows require
the exact native name, captured catalog and non-default state. Plugin source,
version, marketplace and enabled checks remain unchanged. No display label or
default flag establishes trusted origin, effective roots or isolation.

Bind and durably publish each Pending record before its command, then its
sanitized result. Require complete transport, zero stderr, unchanged roots/code/
known inputs and strict inventory schema/identity checks before any later call.
Stop permanently on any refusal, ambiguity, timeout, cancellation, output bound,
changed precondition or partial/uncertain effect. No extra inventory or cleanup
may follow a stop without a separate execution decision and applicable witness.
Direct-child reaping and an empty process group do not prove escaped-worker
quiescence.

Snapshot only the nine prepared non-secret files. Native configuration,
credentials, logs and sessions must not be copied, hashed or dumped. Retain only
the collector's bounded fixed-path file metadata, known fixture claims and safe
counts/reasons. Leave all packet files and first Pending/result sequence intact.
The exact planned uninstall/removal pair is permitted only on the continuously
accepted path; its final samples do not prove stable cleanup or recovery.

## Decision

Approve only this root, manifest, driver pins and interpretation policy. The
previous once-only approvals and read-only observations are not replay
authority. After approval, use the fixed full `--execute` entrypoint once; never
substitute the reduced diagnostic mode or an additional native command. All
release and production qualification gates remain closed regardless of this
outcome.

At commit `1b75a97`, `cargo test --no-fail-fast -- --test-threads=1` ran 117
test targets: 1760 passed, 0 failed, 7 ignored. Preparing this packet ran only
read-only native commands: `--version` and ten `plugin ... --help` commands of
the private installation, with an isolated HOME and no model use. No inventory,
registration, installation or removal command ran.

## Outcome (2026-11-11)

The user approved one run. The run used the fixed `--execute` entrypoint once
and stopped with reason `transport` at sequence 4. The
[execution records](../../technotes/evidence/2026-11-11-copilot-installation-trial-execution/)
are copied from the packet evidence directory.

- Sequences 0 to 3 were accepted. The version output matched 1.0.107. The
  baseline plugin inventory had 2 unrelated builtin entries. The baseline
  marketplace inventory had 1 default remote entry. No entry used the fixture
  name. These are the first 1.0.107 inventory observations, and the strict
  collector accepted both schemas.
- Sequence 4, `plugin marketplace add`, exited with code 1 and wrote 225 bytes
  to stderr. Sequences 5 to 11 did not run. The run made zero model requests.
- Deviation: the driver that ran had SHA-256
  `c9cf4ab1067595b63518bbcdb6ea1ae8e8e09cd7d1688336693167675fbd6fdc`, not the
  `driver_sha256` in `readiness.json`. The driver was rebuilt after the
  readiness receipt was written, and `--check` did not compare the driver
  digest. This run is therefore evidence outside the approved bindings. The
  execution modes of `copilot_contract_probe` now require a fourth
  `DRIVER_SHA256` argument and stop before any effect when it does not match.
- A separate diagnosis repeated the same command against a copy of the export,
  in a throwaway home directory, with stderr kept. Copilot 1.0.107 reported:
  `Invalid marketplace.json: name: String must contain at most 64 character(s),
  plugins.0.name: String must contain at most 64 character(s)`.
  The generated 72-character name exceeds this limit for both the marketplace
  and the plugin. A relative path argument (`./mp`) was interpreted as a GitHub
  repository and caused a failed network clone; the trial uses absolute paths
  only.

The naming scheme needs a decision before a new packet is prepared. This packet
is consumed and must not be run again.
