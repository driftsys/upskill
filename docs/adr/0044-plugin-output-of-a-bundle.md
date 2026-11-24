# AD-0044 — Plugin output of a bundle

Status: **accepted**; decision 7 amended on 2026-11-23 (see the end of this
record). Date: 2026-11-19. Resolves
[#563](https://github.com/driftsys/upskill/issues/563).

## Context

Bundle authors need to distribute skills and agents through Claude Code and
GitHub Copilot CLI marketplaces, while installing rules into each repository.
[ADR-0013](0013-agent-customization-targets.md) keeps a bundle and a native
plugin package as distinct concepts. A plugin directory is one output of a
bundle; it does not replace the bundle or change its schema.

## Decision

1. `upskill pack` reads a named bundle in the current source registry and builds
   its skills, resources and agents, including its `requires` closure. It reads
   no lockfile and writes only below `--out`. The required `--source` is
   validated and copied unchanged into the init command; it is not fetched.
2. Write separate `<bundle>-claude` and `<bundle>-copilot` directories,
   following
   [AD-0041](0041-separate-client-representations-as-design-baseline.md). The
   manifests, agent filenames, init client flags and hook paths differ. No
   client flag selects both; either flag selects its client, and both flags
   select both clients. `--force` replaces each selected output directory: every
   directory is first written in full to a staging directory, and only then is
   each target renamed aside and replaced by a rename. A failure while writing
   or renaming tries to undo the renames already made, so every output directory
   keeps its earlier content unless an undo also fails. No README or LICENSE
   file is generated.
3. upskill does not install, register or own the generated plugin, following
   [AD-0042, Decision 8](0042-resteer-customization-delivery.md). The author
   publishes it through a marketplace, and the user installs it with the client.
4. Rules are installed by `commands/init.md`, which instructs the model to run
   `upskill add <source> <bundle> --kind rule --project --claude` (or
   `--copilot`) and report the result. `--kind` skips plugin and MCP steps and
   their lockfile records. Init lists the client's declared third-party plugins
   with an `install_url` or manual `instructions_url` when present, otherwise by
   name, and the declared MCP servers from the closure for the user; it does not
   install them. Descriptors only for another client are omitted.
5. A client output with rules includes the existing notice hook unchanged.
   Without rules for that client, it has neither init nor the hook.
   `update --notice` creates `notice.lock` before reading the cache. A run
   encountering a lock younger than 60 seconds exits silently. Expired or
   future-dated locks are replaced after checking the observed ownership token.
   A changed token makes the run exit silently, as for a busy lock. Another lock
   error permits an unlocked run. Each run writes a unique token and removes its
   lock on completion only if that token still matches. An interrupted run can
   leave its lock to expire. This is not a writer lock for install, update or
   removal.
6. A lockfile bundle entry records the selected `kinds`; absent or empty means
   unrestricted, and empty is omitted on write. Recorded kinds are sorted
   without duplicates, and bundle entries are sorted by name. `update` restricts
   a source only when every recorded bundle of that source has recorded kinds,
   using their union. One unrestricted bundle makes the source unrestricted.
   Under a kind restriction, `update` skips both the plugin and MCP steps.
7. For a directory source with recorded bundles, `update` passes the recorded
   bundle names separately from the directly installed item names of that
   source. Bundle names resolve only as bundles, including cross-source
   `requires`. Direct items resolve their item `requires` closure, which is
   merged with the bundle closure. Before this decision, `update` after
   `add <dir> <bundle>` installed every item of the source. Every lockfile entry
   of that source whose kind is outside the restriction is left untouched and
   reported as up to date, regardless of bundle membership or `required_by`,
   unless its source is skipped. It then shares the source's skipped status,
   also when the source is skipped later because it requires an item of a
   skipped source. `add <source> <item>` updates it. A recorded bundle that is
   no longer in its source causes
   `bundle '<name>' is no longer in source <label>` before any write, even if an
   item has the same name.
8. `pack` resolves the entry bundle in the current directory. A `requires` entry
   with a `source` is fetched from that source. The init `--source` must parse
   as an install source and, like the bundle name, is limited to a safe
   character set, because both are written into a command line that a model
   runs. A plugin URL in the init text is written in angle brackets.

## Consequences

Plugin output is experimental until a smoke test with a hosted git marketplace
passes in both clients (the local-marketplace smoke test of 2026-11-19 passed).
`doctor` gains no row. Plugin updates belong to the client marketplace; project
rules remain managed by upskill.

The init command and hook require the upskill binary. Installing the same skills
through a plugin and `add` exposes duplicates. References by short skill name
are not verified to resolve to prefixed plugin names. Authors publishing both
outputs must check that Copilot does not also load the Claude output.

Ownership tokens prevent a completed run from removing a lock that has already
been replaced when it checks the token. The token check and removal are separate
operations: a replacement between them can still be deleted, so two runs
reclaiming an expired lock can both print. A run lasting at least 60 seconds can
also overlap its replacement. Sessions in different repositories share the cache
lock, so one can receive no notice until its next session. An unlocked run can
still race with another cache save.

## Alternatives considered

- Have upskill install the plugin: rejected by AD-0042 because native installs
  can rewrite user settings and Copilot can activate a Claude registration.
- Use one shared directory: rejected in favour of separate client
  representations and their different agent files and init text.
- Install plugins and MCP servers from init: rejected because installing project
  rules would also change client settings.
- Copy skills and agents from init: rejected because it duplicates plugin
  content.
- Ship an init script: unnecessary for one command, and Copilot did not expand
  the Claude plugin-root variable in the prototype.
- Depend on the separate notice plugin: rejected because each output must
  include its own notice hook.
- Use a permanent marker per notice: a killed run could suppress a notice
  forever.
- Skip the hook when the notice plugin is installed: client plugin stores
  differ.
- Deduplicate by session ID: not every client supplies one.
- Deliver rules through a session hook: deferred with general hook authoring.
- Update a directly installed item of another kind together with a restricted
  bundle: rejected because it needs two plans for one source.
- Introduce a separate plugin source schema: the bundle already supplies the
  metadata, content and dependencies.

## Amendment: update keeps an orphaned dependency of a directory source with recorded bundles (2026-11-23)

Decision 7 says that, for a directory source with recorded bundles, `update`
passes the recorded bundle names and the directly installed item names. The item
filter also passes each dependency of that source whose `required_by` is not
empty, names no requirer that the lockfile records (the `doctor` condition for
an orphaned dependency), and is listed by no recorded bundle of the source. A
member of a recorded bundle follows its bundle: when the bundle no longer lists
it in the source, `update` removes it as `no longer in source`. Without this,
`update` removed such a dependency as `no longer in source` after an exclude
removal that kept the `required_by` links. A dependency that still has an
installed requirer keeps the earlier behaviour. See the amendment of
[AD-0045](0045-record-add-excludes-in-lockfile.md) and
[#613](https://github.com/driftsys/upskill/issues/613).
