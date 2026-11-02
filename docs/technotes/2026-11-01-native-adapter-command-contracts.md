# Native adapter command contracts

Status: command-syntax evidence, 2026-11-01. Concrete adapters, their native
observers and all production profile admissions remain incomplete. This note
records the next implementation constraints for Stage 6 of the
[approved delivery plan](../wip/plans/2026-10-31-four-client-skills-resources-delivery.md).
It does not qualify a route or authorize a native experiment.

## Captured boundary

The
[receipt set](../../tests/fixtures/native-adapters/2026-11-01-help/receipts.json)
retains 37 version/subcommand-help outputs on Darwin arm64. Each command exited
zero before its 20-second limit. Native commands received a disposable working
directory without project content, an unauthenticated temporary home, separate
client/XDG roots, closed stdin and a minimal environment inheriting only `PATH`.
Copilot commands included `--no-auto-update`. No config/inventory, registration,
trust, cache, server or model command ran. The temporary homes were removed
after capture.

Resolved launcher SHA-256 values were unchanged within each capture batch. A
launcher hash is not the identity of every delegated runtime; this is especially
relevant to Copilot's JavaScript loader. These receipts do not establish profile
authenticity, effective settings, provider availability or preservation of an
authenticated native profile. Codex emitted a warning that helper aliases could
not be created under the disposable temporary home. Its help/version commands
still exited zero. Stderr hashes and lengths are retained; raw stderr containing
temporary paths remains outside the repository.

Observed versions are Claude **2.1.308**, Copilot **1.0.105**, Codex **0.166.0**
and OpenCode **2.1.4**. Copilot/Codex differ from the earlier qualification
pins; results from those earlier profiles cannot be transferred to these
releases.

## Claude: explicit scope exists, preservation remains unresolved

The captured
[marketplace-add help](../../tests/fixtures/native-adapters/2026-11-01-help/claude-plugin-marketplace-add-help.stdout)
and
[install help](../../tests/fixtures/native-adapters/2026-11-01-help/claude-plugin-install-help.stdout)
expose project/user scope and JSON output. A future adapter can express the
approved scope directly, rather than relying on the native user default.
Uninstall, update, enable and disable also expose scope selection. Update help
states that applying the updated plugin requires a restart.

Implementation must still distinguish marketplace registration, installed cache,
enabled scope and effective source selection. A JSON command result cannot
replace those independent observations. Use only owned plugin/marketplace
selectors; no broad disable, prune or trust command belongs in an inverse.
Capture native cache paths separately from the immutable managed export paths.

The inactive
[Claude command planner](../../src/native_delivery/claude/commands.rs) now emits
fixed argv for catalog validation, registration, install, update, uninstall and
scoped marketplace removal. It checks the retained version, Claude command
route, full native digest name and matching managed catalog and revision paths.
These are consistency checks on supplied records; they do not establish
ownership, catalog contents, trust or qualification. Every mutation still needs
a durable invocation checkpoint and independent effect observation. No command
executes through this planner and the production registry stays empty.

The retained [Q11b result](gate6-project-registration-isolation.md) and its
preservation failures remain constraints. The
[Copilot-local exclusion proposal](copilot-project-plugin-overlay-profile.md)
remains a separate, unqualified configured profile. Scope syntax does not repair
unintended-reader activation, lexical rewrites or unknown old-resource
retention.

## Copilot: no project install switch was observed

The captured
[install help](../../tests/fixtures/native-adapters/2026-11-01-help/copilot-plugin-install-help.stdout)
accepts a marketplace selector or repository source. Its options expose neither
project scope nor installation in a disabled state. The captured enable/disable
commands also provide no project-scope switch. This is a bounded observation of
1.0.105 help, not proof that every configured project route is impossible.

The adapter must not invent a scope flag or use user installation followed by
disable to satisfy project admission. A configured repository route requires its
own qualified capture, writer and effective-observer contract. Separately
resolve the approved full-digest name acceptance question against the exact
native pin; this command capture performs no manifest/name validation and
changes no generated name. The inactive
[Copilot command planner](../../src/native_delivery/copilot.rs) now emits only
the captured user-context registration/install/update/uninstall/removal argv,
including `--no-auto-update`. It rejects every project operation before
returning argv and preserves the full native name. The declared user scope must
still be bound to the correct HOME and effective configuration by a qualified
executor; there is no native acceptance, activation or ownership proof here. The
earlier [component packet](gate6-copilot-plugin-components.md) does not qualify
these missing controls.

## Codex: marketplace management is distinct from project activation

The captured
[add help](../../tests/fixtures/native-adapters/2026-11-01-help/codex-plugin-add-help.stdout)
accepts a plugin/marketplace selector and supports JSON output. It exposes no
project-scope or install-disabled option. The inherited `--enable`/`--disable`
options control features; they are not plugin-scope switches. Marketplace add,
list, upgrade and remove are available command families in this capture.

The [official CLI reference](https://learn.chatgpt.com/docs/cli/reference) and
[packaging guide](https://developers.openai.com/plugins/build/plugins), fetched
on 2026-11-01, describe marketplace configuration separately from plugin
activation. Exact local help remains the syntax evidence for this installed pin.
A future adapter must establish a project-only admission sequence without a
transient user binding. Do not import the old remove/re-add sequence as a safe
project contract: [Q8](gate6-codex-skill-updates.md) observed visibility but did
not prove that intermediate scopes stayed inactive.

The inactive [Codex command planner](../../src/native_delivery/codex.rs) now
emits captured user-context local registration, add, remove and
marketplace-removal argv with JSON output. Every project operation rejects.
Local-package update also rejects: the captured marketplace upgrade command
refreshes Git snapshots, so it is not a local revision switch. The planner adds
no feature/configuration or scope override and never substitutes remove/re-add.
HOME/configuration binding, native effects, cache retention and qualification
remain open.

Native removal can invalidate cache-backed resources. Require observed native
retention or session clearance before refresh/removal, even while all managed
exports remain present. Preserve D4's exact configuration restoration boundary;
no notice/trust-table repair follows from permission to manage a plugin.

### Separate read-only inventory structure on 2026-11-02

Two diagnostics used fresh private HOME, CODEX_HOME, XDG and temporary roots
with qualified project directories. Together they ran five commands: two exact
version queries, two `plugin list --json` queries and one
`plugin marketplace list --json` query. All exited zero with complete bounded
transport, zero stderr and unchanged captured selected code; version bytes
matched the retained 0.166.0 capture. Each command had a twenty-second deadline;
list output was capped at 1 MiB. No registration, installation, removal or model
request ran.

The marketplace response was an object containing an empty `marketplaces` array,
with no unknown top-level fields. The first plugin projection discarded two
unrecognized field names. A separately capped follow-up projected the fixed
`installed` and `available` labels: both were empty arrays, with no unknown
top-level fields. All scalar values were discarded, and raw native output stayed
in memory. No native configuration snapshots were retained. These empty shapes
do not establish the nonempty entry contract, effective roots, source binding,
project admission, activation or retention.

Sanitized receipts are retained at
`/private/tmp/upskill-codex-inventory-maumd_vm/summary.json` and
`/private/tmp/upskill-codex-inventory-fields-16qq39jo/summary.json`. Each
retains driver source/binary and parser digests; the corresponding source,
executable and fake-client controls remain beside it. Before each diagnostic,
fake controls verified exact argv, version mismatch, stderr and
decoded-duplicate stops, closed-environment handling and omission of secret
canaries. These observations leave the production admission registry unchanged.

## OpenCode: file exposure needs an independently qualified observer

The captured
[debug help](../../tests/fixtures/native-adapters/2026-11-01-help/opencode-debug-help.stdout)
provides agents, configuration-source, heap and global-path commands. It exposes
no skill-catalog debug command. The
[configuration help](../../tests/fixtures/native-adapters/2026-11-01-help/opencode-debug-config-help.stdout)
describes source listing, not a proof of effective source selection. The
[API help](../../tests/fixtures/native-adapters/2026-11-01-help/opencode-api-help.stdout)
allows operation IDs or HTTP method/path requests, but this capture establishes
neither an inventory endpoint/schema nor its freshness or isolation semantics.
No server or API request was started.

The [current V2 skills guide](https://opencode.ai/v2/docs/skills) and
[configuration schema](https://opencode.ai/config.json), inspected on
2026-11-01, specify a top-level `skills` array of source strings. The schema
also accepts the deprecated object form with `paths` and `urls` arrays. These
web documents do not establish the installed 2.1.4 runtime contract.

The inactive [OpenCode source planner](../../src/native_delivery/opencode.rs)
now plans one owned configuration patch for a batch of source changes. New
contributions use the preferred array; an existing legacy object retains its
form and unrelated URL entries. Refresh replaces the exact owned absolute
selected-revision `skills/` source. Removal preserves surviving owners and can
restore exact original bytes, mode or file absence under the existing rollback
preconditions. Project overlays must remain ignored and untracked; external
user-config roots, duplicate exact sources and conflicting layers reject. Mixed
inherited representations may reject pending a qualified mapping.

This is a pure preview from caller-supplied approved non-secret captures. It
makes no native call and grants no effective-source, skill-ID uniqueness,
retention, quiescence or admission proof. The coordinator still gates exposure
and requires independent effective observations; no native observer is supplied
by this planner.

Implement the approved absolute selected-revision skill source as an owned
configuration exposure, with file-only action ordering in the existing
coordinator. Do not invent `debug skill`, introduce a runtime JavaScript plugin
or treat configuration-source presence as activation. Before admitting its
observer, establish effective overlay loading, layer precedence, unique source
identity, nested-CWD behavior and resource reads. The retained
[failed native positive](gate6-opencode-explicit-paths-retry.md) remains failed;
a catalog or help capture cannot replace it.

## Next implementation and qualification sequence

1. Implement concrete adapters against these pinned command families and the
   existing adapter trait. Keep the production registry empty until every
   required cell is qualified. No production CLI route follows from this note.
2. Pin resolved executable/runtime identity and exact invocation/environment,
   effective roots/settings/trust, competing readers and package identities.
   Missing executables, changed versions or unsupported command contracts must
   reject before capture/effects with actionable typed errors.
3. Bind non-secret capture, owned inverse and independent observation to the
   operation/profile/target/scope. Credential-bearing whole-document snapshots
   remain outside the approved D6 storage contract, including private journals.
4. Add fake-native executable acceptance tests for missing commands, malformed
   inventory, refusals, cache changes, duplicate readers, nested CWD and partial
   lifecycle effects. Independently log commands and verify preservation at each
   coordinator boundary. Keep legacy commands unchanged.
5. Prove owned native children are quiescent after timeout and parent death
   before an observer returns stable Before/After. Unknown liveness keeps the
   operation pending; killing only the direct child does not prove quiescence.
6. Prepare an exact profile's bounded campaign: calibrated positive and cleanup,
   selector-free baseline, owner registration, fresh owner and unintended-reader
   controls, unrelated-project control, update/resource-retention controls,
   native removal, fresh post-removal absence and final preservation witness.
   Pin fresh process/session identities and receipt order for every observation.
7. Before executing that campaign, verify the live account, allowed model,
   allowance/reset and finite cost/call cap with explicit reserve thresholds.
   Stop on a failed positive, drift, ambiguous attribution, unknown retention or
   reserve threshold. No paid call is scheduled or authorized by this note.

The help capture closes only the syntax-inspection prerequisite. It leaves
configuration/writer admission, native quiescence, runtime qualification and
four-client release admission open.
