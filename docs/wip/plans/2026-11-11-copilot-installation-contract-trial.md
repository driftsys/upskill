# Copilot installation contract trial with 64-character names

Status: prepared for approval, not executed. This packet replaces the packet of
2026-11-10, which stopped at sequence 4 because Copilot CLI 1.0.107 limits
marketplace and plugin names to 64 characters. Preparing and validating this
packet authorizes no native mutation.

## Purpose and exact input

Test registration of the generated local marketplace, installation of its exact
plugin selector, matching inventories, then planned removal if every preceding
step is accepted. This is a bounded contract experiment. It does not test
activation, isolation or production admission.

- Packet: `/private/tmp/upskill-copilot-install-trial-zvmlkujh/packet`.
- Plan SHA-256:
  `0cb3d581ff421a73653d00c8e487452a70f4aa92eea801d2f0a9d0a0d00eb604`.
- Preparation SHA-256:
  `cf709761292057c6a20700e4aef0774ca7e6e221b864545321ec6c7417134470`, for
  `/private/tmp/upskill-copilot-install-trial-zvmlkujh/preparation.json`, which
  is outside the packet root.
- Native plugin and marketplace name: `upskill-contract-probe-7b1e4a48bd2a2045`
  (39 characters, `upskill-<skill>-<hash16>`).
- Driver SHA-256:
  `82fe04e201e6db15672af7340f20292bb12917eedc570e813d61ae32160012bb`.

The packet was produced by `scripts/prepare_copilot_install_trial.py` with the
fixture compiler built at commit `7f57439`. The full serial test suite passed at
that commit: 118 targets, 1,757 passed, 0 failed, 7 ignored. The
[test summary](../../technotes/evidence/2026-11-11-copilot-installation-trial-preparation/test-suite-summary.txt)
keeps the per-target result lines.

The
[proposed trial manifest](../../technotes/evidence/2026-11-11-copilot-installation-trial-preparation/proposed-trial.json)
contains every exact argv array. The
[readiness receipt](../../technotes/evidence/2026-11-11-copilot-installation-trial-preparation/readiness.json)
binds the driver executable and eight source digests at commit `7f57439`. The
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

## Stop conditions

Stop permanently on any refusal, nonempty stderr, timeout, cancellation, output
limit, changed digest or unexpected inventory. Do not retry and do not run any
cleanup command after a stop without a separate decision. A stop leaves a
partial record that must be retained.

## Decision

Approve only this root, manifest and driver digest. Use the `--execute`
entrypoint once, with the exact argv in the readiness receipt. All release and
production qualification gates remain closed regardless of the outcome.
