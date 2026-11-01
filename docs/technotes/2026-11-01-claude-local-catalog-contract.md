# Claude local-catalog contract observations

Status: bounded native contract observations, 2026-11-01. Exact settings
preservation failed. No activation, isolation, resource-retention, stable
cleanup or production profile is qualified by this probe.

The user explicitly approved the twelve-command
[prepared probe](../wip/plans/2026-11-01-claude-local-catalog-contract-probe.md)
after its scope was presented. The probe ran once, with zero model/provider
requests and zero spending. All twelve recorded commands completed with exit
status zero, complete output, unchanged launcher/scope observations and an empty
observed process group. Those facts establish bounded command transport, not
absence of escaped native workers or effective skill activation.

## Provenance

The
[sanitized receipt summary](evidence/2026-11-01-claude-local-catalog-contract/summary.json)
contains ordered timestamps, exact argument arrays with the disposable packet
root replaced by `$PACKET`, parsed non-secret native JSON, original output
digests and protected-file samples. Private exact outputs and non-secret file
snapshots remain at
`/private/tmp/upskill-claude-contract-ebiihhzh/packet/execution-2026-11-01`. The
fixture and evidence were retained without manual repair or cache deletion.

Repository head was `559404b`; unrelated uncommitted test changes were not used
or modified. A temporary command driver linked the existing built library and
called `native_delivery::process::run_native_process` for each recorded command.
Driver source, executable and linked-library digests are in the summary; driver
sources were also retained with the private receipts. This was a probe driver,
not production adapter execution or a coordinator transaction.

The pinned launcher was `/opt/homebrew/Caskroom/claude-code/2.1.308/claude`,
SHA-256 `ca923c86ac84b30574a82331a05b6e066c27477a926ceaffe46a845a33e4dd37`. Its
digest matched before and after every command, and the first command reported
exactly `2.1.308 (Claude Code)`. The summary records macOS and CPU identity.
Launcher identity does not identify every delegated runtime.

Each command used the planned project directory, closed stdin, a twenty-second
runner deadline and a 1 MiB combined-output limit. HOME, Claude configuration,
XDG roots and temporary files were isolated under the previously empty fixture
home. The environment contained only those paths, a fixed system PATH and
locale. Git initialization used disabled global/system configuration, an empty
template and disabled hooks. No credentials or client options were inherited;
keychain/system-policy isolation was not established. No retries, alternate
commands, trust overrides or additional native inventory calls ran.

## Observations

1. Baseline plugin and marketplace list commands returned empty arrays.
2. Validation accepted the compiler-generated catalog with its full 72-character
   owner-derived name and content-derived version. It emitted two metadata
   warnings: missing marketplace description and plugin author.
3. Project-scoped registration and installation reported success. The following
   plugin inventory contained one exact selector, expected version,
   `scope: project`, `enabled: true`, matching `projectPath` and
   `projectEnabled: true`. Its `readFromFolder` was the immutable managed
   revision, while `installPath` named a separate native cache location.
4. Marketplace inventory reported `source: directory`, with `path` and
   `installLocation` pointing to the generated catalog. The captured private
   registry used a nested `source` object with the same directory/path fields;
   installed-plugin registry schema was version 2, with a project entry under
   the exact plugin-at-marketplace selector. These are metadata shapes, not
   ownership or effective binding receipts.
5. The planned scoped uninstall and marketplace removal reported success. Final
   list commands again returned empty arrays. Sampled registries remained
   present as `{}` and `{"version": 2, "plugins": {}}`, respectively, both mode
   `0600`. Their absence was not restored. No native cache-retention or
   cache-deletion conclusion follows from these registry samples.
6. Generated export files remained byte-identical at all twelve samples. Sampled
   user settings and project local settings remained absent.

## Preservation failure

The pre-existing project `.claude/settings.json` retained mode `0600` and its
sentinel value, but its exact bytes changed. Claude moved `enabledPlugins` and
`extraKnownMarketplaces` ahead of `companyAnnouncements`. Those empty containers
already existed in the baseline, so an allowance for operation-created empty
containers cannot explain away this lexical rewrite.

Baseline SHA-256:
`f0a5030131f8fde8553118744f223053a4f9e425e0c54de9a93a088bb2629bba`. Final
sampled SHA-256:
`9c8b0be20591a47975e878c2fa08efcafab15271791ccff41c686d62bfc6628e`. The summary
retains both non-secret fixture texts. No whole-file restoration was attempted.
This result does not approve a serializer, inverse or preservation workaround.

## Handoff and limits

The name/version and local-directory registry samples can inform offline
capture/decoder fixtures for this exact pin. They do not resolve Copilot's
documented name limit, another client's packaging acceptance or different Claude
versions. Exact preservation remains a blocker for this candidate native command
sequence; resolving it requires a separately reviewed owned configuration
contract and passing preservation controls, not manual repair of these results.

The probe supplied no qualified independent native-worker quiescence witness.
Even with complete transport and empty final inventories, stable cleanup and
native recovery remain unproven. The fixture and receipts remain intact.
Q11/Q11b isolation failures, effective skill/resource activation, live/resumable
session retention, complete adapters, state compatibility and all production
admission gates remain unchanged. No provider request, skill invocation,
cross-client observer or runtime refresh was attempted.
