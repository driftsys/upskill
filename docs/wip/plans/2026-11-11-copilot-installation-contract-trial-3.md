# Third Copilot installation contract trial

Status: prepared for approval, not executed. This packet replaces the second
packet of 2026-11-11, which stopped at sequence 4 because the collector rejected
the success text of `plugin marketplace add`. Preparing and validating this
packet authorizes no native mutation.

## Purpose and exact input

Test registration of the generated local marketplace, installation of its exact
plugin selector, matching inventories, then planned removal if every preceding
step is accepted. This is a bounded contract experiment. It does not test
activation, isolation or production admission.

- Packet: `/private/tmp/upskill-copilot-install-trial-tvpjqnjn/packet`.
- Plan SHA-256:
  `ef6668a74b07172f70930cf599b699c35b353cfce69ef6be1adc1eb2f58562b3`.
- Preparation SHA-256:
  `b7c96506392ddf2c210c55c7ed255a40cbc7cc6863c688be15bdeb0d92dd8817`, for
  `/private/tmp/upskill-copilot-install-trial-tvpjqnjn/preparation.json`, which
  is outside the packet root.
- Native plugin and marketplace name: `upskill-contract-probe-f32d91261162a3c7`
  (39 characters).
- Driver SHA-256:
  `af6db5bf1a94026f8539e50918ec01ecd7f9978682e5d405849666cfe4fc8720`.

The packet was produced by `scripts/prepare_copilot_install_trial.py` with the
fixture compiler built at commit `b4bc38b`. The full serial test suite passed at
that commit: 118 targets, 1,760 passed, 0 failed, 7 ignored. The
[test summary](../../technotes/evidence/2026-11-11-copilot-installation-trial-preparation-3/test-suite-summary.txt)
keeps the per-target result lines.

The
[proposed trial manifest](../../technotes/evidence/2026-11-11-copilot-installation-trial-preparation-3/proposed-trial.json)
contains every exact argv array. The
[readiness receipt](../../technotes/evidence/2026-11-11-copilot-installation-trial-preparation-3/readiness.json)
binds the driver executable and nine source digests at commit `b4bc38b`. The
execution argv passes the driver digest as its fourth argument, and the driver
refuses to run when its own executable does not match that digest. Do not
rebuild the driver between this receipt and the run.

## Once-only command envelope

At most one offline `/usr/bin/git` setup and eleven Copilot commands, once and
in manifest order. Every Copilot argv starts with `--no-auto-update`. The client
is the private Copilot 1.0.107 installation. Each command has a twenty-second
deadline and a 1 MiB output limit. Zero model requests and zero spending are
authorized.

The eleven client commands are: version; baseline plugin and marketplace JSON
lists; local marketplace registration; exact-selector installation; plugin and
marketplace JSON lists; exact-selector uninstall; exact marketplace removal;
final plugin and marketplace JSON lists.

## Acceptance rules

- The baseline plugin and marketplace lists must contain no owned entry.
- The stdout of each of the four mutations must equal the exact Copilot 1.0.107
  success text, built from the native name, the marketplace path and the payload
  digest. The
  [mutation output diagnosis](../../technotes/evidence/2026-11-11-copilot-mutation-output-diagnosis/transcript.txt)
  records the measured text. Evidence records only whether the text matched.
- After installation, the plugin list must contain exactly one owned entry with
  `"source": "live"`, `"installedFrom"` equal to the marketplace path,
  `"enabled": true` and the planned version and marketplace. The marketplace
  list must contain the owned entry as `Local: <marketplace path>` with
  `"isDefault": false`.
- The final plugin and marketplace lists must contain no owned entry.

## Stop conditions

Stop permanently on any refusal, nonempty stderr, stdout that differs from the
expected text, timeout, cancellation, output limit, changed digest or unexpected
inventory. Do not retry and do not run any cleanup command after a stop without
a separate decision. A stop leaves a partial record that must be retained.

## Decision

Approve only this root, manifest and driver digest. Use the `--execute`
entrypoint once, with the exact argv in the readiness receipt. All release and
production qualification gates remain closed regardless of the outcome.
