# Plugin installation — extend Bundle, shell out to client CLIs

**Status**: Accepted (2026-05-22)

## Context

[ADR-0001](0001-multi-kind-compiler-architecture.md) and
[ADR-0003](0003-generation-pipeline.md) established the SSOT-to-files contract:
upskill reads SSOT items, generates per-client output files, and never invokes
the host client at runtime. That contract is the right shape for rules, skills,
and agents — they are content that each client consumes by reading files from a
known path.

Plugins do not fit that mold. Claude Code plugins (e.g. `superpowers`), Copilot
CLI plugins, VS Code extensions, and opencode modules carry hooks, slash
commands, marketplace registration, and runtime state that the host client owns.
Re-emitting that surface from upskill SSOT duplicates the upstream plugin, can't
capture hooks and commands cleanly, and would have us shipping a second-class
copy of artifacts like `superpowers` that already exist as fully-formed plugins
maintained upstream.

Each of the four target clients exposes a CLI for plugin/extension management:

- `claude plugin marketplace add <source>` +
  `claude plugin install <plugin>@<marketplace> --scope <s>`
- `copilot plugin marketplace add <source>` +
  `copilot plugin install <plugin>@<marketplace>`
- `code --install-extension <ext-id>`
- `opencode plugin add <package>` (opencode 2 or later)

These CLIs already implement the lifecycle work (marketplace registration,
version pinning, scope semantics, idempotent updates) that upskill would
otherwise reinvent.

## Decision

### Bundle gains an optional `plugins:` map

[ADR-0007](0007-bundle-yaml-format.md) defined the Bundle YAML schema. This ADR
extends it with one new top-level key, `plugins:`, a map keyed by upskill-level
plugin name:

```yaml
schema: 1
name: superpowers-baseline
description: Superpowers plugin and supporting rules
items:
  rules:
    - license-awareness
plugins:
  superpowers:
    claude:
      source: anthropics/claude-plugins
      plugin: superpowers
      install_url: https://github.com/obra/superpowers#install
    copilot:
      source: obra/superpowers-marketplace
      plugin: superpowers
      install_url: https://github.com/obra/superpowers#install
    vscode:
      extension: anthropic.superpowers
      install_url: https://marketplace.visualstudio.com/items?itemName=anthropic.superpowers
    opencode:
      module: superpowers-opencode
      install_url: https://opencode.ai/plugins/superpowers
```

- The map key (`superpowers`) is the upskill-level identity — used in the
  lockfile, CLI output, and `upskill remove plugin superpowers`.
- Per-client blocks (`claude`, `copilot`, `vscode`, `opencode`) are each
  optional and typed to that client's actual install primitives. A plugin that
  only exists for Claude Code carries only a `claude:` block.
- Each client block MAY carry `install_url:` — a URL surfaced in the warn-skip
  message when that client's CLI is not on PATH. The field is optional; if
  absent, the warning fires without the URL line.

### Install shells out to the native CLI

When `upskill add` resolves a bundle that declares plugins and the user has the
matching client targeted, upskill shells out:

| Client   | Commands                                                                                                                                                                  |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| claude   | `claude plugin marketplace add <source>` (idempotent — checked via `claude plugin marketplace list`), then `claude plugin install <plugin>@<marketplace> --scope <scope>` |
| copilot  | `copilot plugin marketplace add <source>` (idempotent), then `copilot plugin install <plugin>@<marketplace>`                                                              |
| vscode   | `code --install-extension <extension>` (naturally idempotent)                                                                                                             |
| opencode | `opencode plugin add <module>`                                                                                                                                            |

Claude's `--scope` derives from upskill's existing project/global flag:

| upskill flag       | lockfile location          | `claude --scope` |
| ------------------ | -------------------------- | ---------------- |
| `--global`         | `$HOME/.upskill-lock.json` | `user`           |
| (default, project) | `<cwd>/.upskill-lock.json` | `project`        |

Claude's `local` scope (machine-local, not committed) is not exposed via
upskill; users who want it can call `claude plugin install` directly.

Copilot CLI does not support a `--scope` flag — plugins are installed globally
regardless of the upskill project/global context.

### CLI-missing policy: warn-skip

If a bundle declares a plugin for client X and X's CLI is not on PATH, upskill
prints a warning and continues with the rest of the install:

```text
warn: claude CLI not found; skipped plugin 'superpowers' for claude.
      manual install: https://github.com/obra/superpowers#install
```

The rules/skills/agents portion of the bundle still installs. Rationale: in
mixed-client teams, the bundle's primary value (SSOT-derived content) is
independent of whether every plugin lands. Failing the whole install for a
missing tertiary tool is hostile.

### opencode: version 2 or later only

upskill uses the plugin commands of opencode 2 or later only:
`opencode plugin add <package>` to install and `opencode plugin list` to check.
upskill runs no `opencode plugin remove`: `remove` deletes the plugin's entry
from the `plugin` and `plugins` arrays of `opencode.json` and runs no opencode
command. Before it runs any `opencode plugin` command, upskill runs
`opencode --version`. When the major version is below 2, or no version can be
read, upskill runs no `opencode plugin` command: opencode 1.x reads
`opencode plugin <word>` as an install of the module `<word>`, so
`opencode plugin list` would install a module named `list`. An install then
records the plugin as `skipped`, and `add` prints a warning that names
`opencode 2` and the version found. `doctor` treats the plugin as a plugin whose
state cannot be checked: it is not reported as missing.

A plugin declared with `plugin_uri` is installed by a write to the `plugins`
array of the `opencode.json` of the scope, not by a command. Its lockfile record
carries `"config_write": true` (omitted for every other install form; a record
without it reads as `false`). For a record with `config_write`, `doctor` and the
repair of `update` check the `opencode.json` that upskill writes for that scope
only: the identifier in the `plugin` or `plugins` array means installed, its
absence means missing, and no opencode command runs, with any opencode version
or none. The repair writes the entry again. For an opencode record without the
field (written before the field existed), `doctor` reads the file first and,
when the identifier is absent, checks with `opencode plugin list` under the
version check. When `update` applies or repairs such a plugin, it sets the
field.

### The opencode config file and the `plugins` key

The `opencode.json` of a scope is the file that opencode reads for that scope.
Trials with opencode 2.1.4 (issues #565 and #573) tested the following, with
`XDG_CONFIG_HOME` unset:

- In the global scope, opencode read `~/.config/opencode/opencode.json`. It
  ignored `~/opencode.json` and `~/.opencode/opencode.json`. `~/opencode.json`
  is the file that upskill wrote in the global scope before this decision.
- With a project `opencode.json` that held an entry under the legacy `plugin`
  key, `opencode debug config` printed only the key `plugins`, with the legacy
  entry listed under it. Only the listing was checked, not the loading of a
  plugin.
- In one trial (2026-11-20, opencode 2.1.4, `XDG_CONFIG_HOME` pointed at a copy
  of the global configuration, `opencode run` in a project directory; recorded
  in issue #581), a relative `instructions` path in the global configuration was
  resolved against the project directory: the marker file in
  `<project>/.agents/rules/` was loaded and the marker file in the config
  directory was not. The rules glob `.agents/rules/**/RULE.md` therefore does
  not reach `~/.agents/rules/` from the global configuration. The first runs
  with an absolute path stopped responding; the second trial below shows that
  the cause was the standard input of `opencode run`, not the path.
- In a second trial (2026-11-21, opencode 2.1.4, same method, recorded in issue
  #581), `opencode run` was started with standard input redirected from
  `/dev/null`. Without that redirection `opencode run` stops responding. In the
  global configuration these `instructions` values were not loaded: an absolute
  glob with `**`, the same glob under another directory, a `~/` glob with `**`,
  and an absolute glob with `*` in a directory segment. These values were
  loaded: a single absolute file path, a single `~/` file path, an absolute path
  with a wildcard in the file name only, and a `{file:...}` value whose content
  is an absolute file path. A file `$XDG_CONFIG_HOME/opencode/AGENTS.md` was
  also loaded; that file belongs to the user, so upskill does not write it. In
  the project configuration, the relative glob worked in the first trial.

upskill uses `$XDG_CONFIG_HOME/opencode/opencode.json` when `XDG_CONFIG_HOME` is
set and absolute. That location was not tested, except as the way the
relative-path trial above was run.

Decision:

- In the project scope the file is `<project>/opencode.json`. In the global
  scope it is `$XDG_CONFIG_HOME/opencode/opencode.json` when `XDG_CONFIG_HOME`
  is set and is an absolute path, and `$HOME/.config/opencode/opencode.json`
  otherwise. The plugin write, the listed check, the removal, the MCP write and
  removal, and the MCP check of `doctor` all use that file.
- upskill writes a `plugin_uri` plugin to the `plugins` array. The listed check
  and the removal accept an entry under `plugin` or `plugins`; the removal
  deletes the entry from both. An older entry under `plugin` stays in place and
  counts as present on write, so the write adds no second entry under `plugins`.
  There is no migration before version 1.0.
- upskill neither reads nor cleans the old `$HOME/opencode.json`.
- In the global scope upskill writes one `instructions` entry per rule that is
  installed for opencode, to the global configuration file: the absolute path of
  the generated file, `<home>/.agents/rules/<name>/RULE.md`. An absolute path
  works on every platform and does not depend on the home directory notation.
  upskill writes no glob there, because the global configuration does not expand
  `**`. The project scope keeps the single relative glob in
  `<project>/opencode.json`.
- The entries follow the lockfile. `add` and `update` write the entry of each
  rule that the lockfile records for opencode and remove an entry of that form
  whose rule is no longer recorded. `remove --global` deletes the entry of each
  removed rule; an error of that deletion is ignored, as for the plugin entry.
  `doctor --global` reports a recorded rule whose entry is missing, as drift
  with exit code 1, and `update --global` writes it again. Other entries and
  other keys of the file are kept. An `instructions` value that is not an array
  is an error.
- upskill does not read or write `$XDG_CONFIG_HOME/opencode/AGENTS.md`.

### Repair by `update`

The plugin step of an applied source attempts every plugin of the bundles that
the lockfile records for the source, with every client of each plugin. An
`update --yes` run applies every source, so it installs a missing or skipped
plugin through that step.

A terminal run does not apply a source whose preview changes nothing. For such a
source the repair runs instead: it installs again every plugin of the bundles
that the lockfile records for the source, when the lockfile records the plugin
as installed and the check of `doctor` reports it missing, or records it as
skipped. The repair installs only that client of the plugin. A skipped plugin
whose install succeeds is recorded as installed; when the client CLI is still
absent or opencode is still below 2, it stays skipped and the warning of `add`
is printed again. The repair does not run for a source that the answer did not
apply, with `--dry-run`, `--check` or `--notice`. A repair install that fails
prints a warning and does not change the exit code of `update`, as for `add`.

A source under a kind restriction (the lockfile records at least one bundle for
the source, and every such bundle has recorded kinds, `add --kind`) runs no
plugin step in `update`, and the repair does not run for it either. For a
missing or skipped plugin of such a source, `doctor` names the `upskill add`
command of the bundle on the plugin's line. That command has no `--kind`, so it
also removes the kind restriction of the bundle.

`update` runs over the sources of the items that the lockfile records. When no
recorded item comes from the source that the lockfile records for a plugin's
bundle (for example a bundle with `items: {}`), `update` does not reach that
plugin; for such a plugin `doctor` names the `upskill add` command of the bundle
on the plugin's line. The header of missing plugins says that
`upskill update --yes` installs them unless a line names another command. The
footer of skipped plugins says that `upskill update --yes` installs the plugins
skipped for a missing CLI or an opencode version below 2, unless a line names
another command, and that a plugin that needs a manual step (status
`instructions`, listed with the skipped plugins) keeps its instructions: the
repair never tries such a plugin again, and the plugin step of an applied source
prints its instructions again.

`update` prints the plugin results of the plugin step of each applied source and
of the repair with the printer of `add`; each plugin is attempted, and reported,
once per source of the run.

### Lockfile records the resolved triple

Each plugin install records `(client, marketplace, plugin-name,
scope)` (or the
client-equivalent identifier) in `.upskill-lock.json` so `remove` / `update` /
`doctor` can invoke the inverse CLI command. The lockfile schema bump is
documented in the format spec.

### Implementation shape

New module `src/plugin.rs`, peer to `src/fetch.rs` and `src/auth.rs`. Uses
`std::process::Command` directly — no new dependencies, no async, consistent
with the project's "shell out to git" stance from ADR-0001.

The module exposes typed functions per client operation
(`claude_marketplace_add`, `claude_plugin_install`, `copilot_marketplace_add`,
`copilot_plugin_install`, `vscode_extension_install`, `opencode_plugin_install`,
plus the inverse uninstall calls). All return `anyhow::Result<T>` and never
write to stdout/stderr — presentation lives in `main.rs` per existing project
conventions.

### Platform scope

Per PR #135 the supported Windows path is `install.sh` under WSL — `upskill`
runs as a Linux ELF binary regardless of host OS. Inside WSL all four target
CLIs are present on PATH as ordinary Linux executables (Claude Code, Copilot,
opencode, and VS Code each ship Linux builds; VS Code's WSL-remote workflow
additionally injects a `code` shim that proxies to Windows-side `code.exe` via
interop). Shellout is indistinguishable from native Linux — no `cfg!(windows)`
branches, no `PATHEXT` handling, no `.cmd` argument-escaping concerns.

Presence detection for the warn-skip policy uses `Command::spawn()` and matches
`ErrorKind::NotFound`, which is portable across every platform Rust targets — no
`which` crate dependency, consistent with ADR-0001 §3.

Since PR #157, `upskill` itself builds and runs on native Windows
(`cargo install upskill`, `USERPROFILE` fallback, Windows CI). However, plugin
CLI shellout on native Windows has known limitations: `.cmd`/`.bat` shim
resolution via `PATHEXT`, Rust 1.77 batch-argument escaping, and VS Code
Insiders discovery are not yet handled. WSL remains the primary supported path
for plugin shellout; native Windows plugin support would need its own ADR if
demand materialises.

## Consequences

**Positive.**

- Native lifecycle (marketplace updates, version pinning, scope semantics,
  idempotent upgrades) is delegated to each client, not reinvented in upskill.
- Consistent with the "shell out to git instead of `git2`" stance from
  [ADR-0001](0001-multi-kind-compiler-architecture.md) §3 — a proven pattern in
  the codebase.
- Bundle authors get a single SSOT entry that fans out to whichever clients the
  consumer has installed.
- Warn-skip keeps mixed-client teams unblocked: a Copilot-only user installing a
  bundle that includes a `claude:` plugin block sees a warning and gets the rest
  of the bundle.

**Negative.**

- Determinism loss: install outcome depends on which CLIs are present, which
  marketplaces they have configured, and which upstream plugin versions resolve
  at install time. Integration tests must shim the client CLIs on PATH (the
  pattern already used by `fetch.rs::tests` for `git`).
- Bundle schema growth: `plugins:` is a non-trivial sub-shape with three
  client-specific variants. Future clients add more variants.
- Warn-skip can mask a real misconfiguration (user expected the plugin to
  install but the CLI was missing). Mitigated by `doctor`, which surfaces every
  skipped plugin during reconciliation.

## Alternatives considered

**(a) Patch `~/.claude/settings.json` directly.** Rejected: depends on
undocumented internal schema, breaks across Claude Code versions, duplicates
registration logic the CLI already implements, and doesn't generalise to VS Code
or opencode (each has its own state file).

**(b) Re-emit plugin contents from SSOT.** Rejected: duplicates upstream plugins
like `superpowers`, can't represent hooks or slash commands in the current item
model, and forces upskill to track upstream plugin updates instead of letting
each client's CLI do it.

**(c) New top-level `Plugin` kind alongside rules/skills/agents.** Rejected:
plugins compose with rules/skills/agents in practice — a bundle like
`superpowers-baseline` wants to ship both the plugin and its supporting rules in
one unit. Keeping plugins on Bundle aligns with how authors actually distribute
them. A standalone Plugin kind would force users to install two artifacts to get
a coherent setup.

**(d) Hard-fail when the target client's CLI is missing.** Rejected: too hostile
to mixed-client teams. A user who runs
`upskill add
some-bundle --claude --vscode` (the client-selection flags
specified in [ADR-0012](0012-consumer-client-filtering.md)) and is missing
`code` would lose the entire install. Warn-skip lets the parts that can install
proceed, and `doctor` reports the gap so it's not silently lost.

**(e) Per-plugin `install_url` (single URL, applies to all clients).** Rejected:
install instructions are usually client-specific — "how to install superpowers
for Claude Code" differs from "how to install the VS Code extension." Per-client
`install_url:` is the natural home.

**(f) Make `install_url:` required.** Rejected: forces every bundle author to
track three URLs even when the warn-skip path is unlikely. Optional, with a
graceful degradation in the warning message when absent.

## Migration

None — additive. `plugins:` is optional on Bundle. Existing bundles without
plugin entries are unaffected. v0.6 introduces the schema extension; v0.5
bundles parse unchanged.

## References

- Predecessor: [ADR-0001](0001-multi-kind-compiler-architecture.md)
  (SSOT-to-files contract, dependency philosophy).
- Predecessor: [ADR-0003](0003-generation-pipeline.md) (generation pipeline this
  ADR explicitly _does not_ extend).
- Predecessor: [ADR-0007](0007-bundle-yaml-format.md) (Bundle YAML schema this
  ADR extends with `plugins:`).
- Authoritative spec:
  [`docs/format-spec.md`](https://github.com/driftsys/upskill/blob/main/docs/topics/0001-content-format.md)
  §3.7 — to be updated with the `plugins:` sub-shape.
- Claude Code plugin CLI: `claude plugin --help`,
  `claude plugin marketplace --help`.
- GitHub Copilot CLI plugin: `copilot plugin --help`,
  `copilot plugin marketplace --help`.
