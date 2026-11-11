# Copilot CLI 1.0.107 loads local marketplace plugins live

Status: informative. Observed on 2026-11-11 with the private Copilot CLI 1.0.107
installation, in a throwaway home directory, with zero model requests. The
[transcript](evidence/2026-11-11-copilot-mutation-output-diagnosis/transcript.txt)
records every command and output. Epic #283.

## Observations

1. `plugin marketplace add <absolute path>` registers a local marketplace. A
   relative path such as `./mp` is read as a GitHub repository name, and Copilot
   tries a network clone from `github.com`.
2. Marketplace and plugin names longer than 64 characters are rejected by
   `marketplace.json` validation.
3. `plugin install <name>@<name>` from a local marketplace does not copy the
   plugin. The output says: "It is loaded live from
   `<marketplace>/revisions/<payload>`, so edits take effect on the next session
   — nothing was copied." `plugin list --json` reports `"source": "live"` and
   `"installedFrom": "<marketplace path>"`.
4. `plugin uninstall <name>@<name>` only disables a live plugin. The output
   says: "It is still on disk at `<marketplace path>` — nothing was removed."
   `plugin list --json` still lists the plugin with `"enabled": false`.
5. `plugin marketplace remove <name>` removes the remaining plugin entry. The
   final `plugin list --json` listed only the two builtin plugins.
6. `plugin marketplace list --json` shows the registered local marketplace as
   `"source": "Local: <marketplace path>"` with `"isDefault": false`, next to
   the default `awesome-copilot` entry. The
   [marketplace list transcript](evidence/2026-11-11-copilot-mutation-output-diagnosis/marketplace-list-transcript.txt)
   records it.
7. All four mutations exit with code 0, write nothing to stderr and write a
   fixed success line to stdout.

## Consequences for delivery design

These are open questions for the delivery design. They are not decisions.

- The upskill export directory is the installed plugin. Any change to the
  revision directory that Copilot loads changes the plugin at the next session.
  Retention must keep that revision directory unchanged while the plugin is
  installed.
- Updating to a new revision cannot be done by replacing files in place without
  affecting the running state. Whether an update needs a new marketplace
  registration, a reinstall, or another step is not measured.
- Complete removal needs both `plugin uninstall` and
  `plugin marketplace remove`. Uninstall alone leaves a disabled entry.
- Deleting the export directory while the marketplace is registered is not
  measured.
