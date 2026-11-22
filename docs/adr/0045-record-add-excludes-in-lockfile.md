# AD-0045 — Record the excludes of `add` in the lockfile

Status: **accepted**. Resolves bullet 4 of
[#570](https://github.com/driftsys/upskill/issues/570). Related:
[#598](https://github.com/driftsys/upskill/issues/598). The behaviour is
specified in
[the specification, §2.2](../specification/specification.md#22-update) and
described for users in [`upskill add`](../book/commands.md).

## Context

`upskill add <source> --exclude X` leaves X out. The lockfile did not record
this, so a later `update` of the same source planned X again. Three paths bring
X back: a bundle-file source (`update` resolves the whole bundle again), a
directory source with recorded bundles (`update` resolves those bundles again),
and a plain directory source (`update` expands the `requires:` closure of the
recorded items again, so an excluded dependency, also one from another source,
returns). Planning X again can install it, replace the same item that another
source installed, or make `update` fail.

## Options

Duration of an exclude:

- A: it lasts until a later `add` undoes it (chosen).
- B: `remove` also records an exclude.
- C: the exclude ends when the requiring item changes.

Storage:

- One set of names for the lockfile, so for the install scope (chosen):
  `--exclude` matches by item name across every source of the closure, so the
  set has the same key.
- A record per source: see "Alternatives considered".
- A list copied onto each item entry of the `add`: the list is duplicated, two
  `add` commands of one source with different excludes are ambiguous, and an
  excluded dependency has no entry of its own.
- Excludes on the bundle record and on the requiring item: one concept in two
  places, and a plain directory source is not covered.

How a later `add` changes the set:

- Excludes accumulate, and naming an item undoes its exclude (chosen).
- Replace the set at each `add`: an unrelated `add` would silently undo an
  exclude.
- A separate `--include` option: naming the item already says the same thing.

## Decision

1. The lockfile has an optional top-level `excludes` array of item names. The
   array is sorted and without duplicates, it has no source key, it is omitted
   when empty, the schema number does not change, and a lockfile without
   excludes keeps its bytes. One lockfile is one install scope, so the set is
   per scope.
2. `add` and `update` compute their excludes as the recorded names, minus the
   names installed by name, plus the `--exclude` names of the command. `add`
   passes the item names of its command line as the names installed by name;
   `update` passes its item filter (the installed names), so an exclude never
   removes an item that was installed by name. `update` never removes a name
   from the recorded set; only a positional name of `add` does.
3. The resolvers skip an excluded dependency or bundle member before they link a
   `required_by` entry to it, and they do not pull its own `requires:` items. A
   whole-source `add` skips excluded items too. This holds in the bundle,
   directory-with-bundles and plain directory paths, also for a dependency from
   another source.
4. After a successful commit, `add` writes the set: the recorded names plus its
   `--exclude` names, minus the item names that it was given and does not also
   exclude. An `add` without item names and without `--exclude` keeps the set.
   `add <source> <name>` installs `<name>` and removes it from the set.
5. `add <source> --exclude d` removes an installed item `d` of the same source:
   that item only, with no member of a co-located group, no bundle record and no
   plugin entry. It deletes each `required_by` link that names `d` and prints
   `removed — excluded <kind> <name>` on stdout. `update` has no such removal
   and no `removed — excluded` status.
6. An `--exclude` name of an `add` that matches no item of the entry source and
   no item that the resolver skipped is a usage error (exit code 2), and the
   lockfile is unchanged. The recorded names are not checked, because an item
   can leave its source.
7. `add` and `update` print `excluded <kind> <name>` on stderr for each skipped
   item, once; an item that the same run removes is reported only as removed,
   not also as excluded; `--quiet` hides the line, and `update` prints it with
   `--dry-run`. `list` prints `excluded: <names>` for each scope, also for a
   scope with no items, and `list --json` has an `excludes` key for each scope.
8. `remove` records no exclude and prunes none. `doctor` has no check for
   excludes.

## Alternatives considered

A top-level record per source, `{ source, names }`, with `source` the canonical
label of the `add`. It was built first and dropped, because the label did not
match the name-based `--exclude`, which applies across the whole closure:

- A cross-source item returned: an exclude recorded for one source label did not
  apply to a dependency that another source supplied.
- A different spelling of the same source gave another label, so a later `add`
  or `update` did not find the record and lost the excludes.
- The undo failed: `add <other-source> <name>` could not remove a name that was
  recorded under the first source.

The per-source design also needed record pruning (delete the record when the
source had no item entry and no bundle record) and a `doctor` check for orphan
records (`orphan_excludes`). The per-scope set needs neither.

## Consequences

- A removed dependency still comes back through its requirer; `remove` records
  nothing (option B not taken).
- An `--exclude` name that matched an item when `add` ran stays in the set until
  an `add` names it, also after the item leaves its source.
- `remove` does not change the set, so a later `add` keeps the formerly excluded
  items out until an `add` names them.
- `update --check` does not need the set, because it plans no items.
- A name applies to every source and every item kind in the scope, so a later,
  unrelated item with the same name is skipped too. The skip line on stderr and
  the `list` output make this visible. Excluding a name for one source only is
  not possible; this loss is accepted.
- The project lockfile is committed, so an exclude applies to every user of the
  repository, as every other lockfile entry does.
- `required_by` links that earlier lockfiles already hold wrongly are not
  repaired.
- The ways to undo an exclude are `add <source> <name>` and editing the
  lockfile.

Satisfies: #570. Code: `src/lockfile.rs`, `src/pipeline/prepared.rs`,
`src/pipeline/lifecycle.rs`, `src/pipeline/mod.rs`, `src/pipeline/report.rs`,
`src/main.rs`. Tests: `tests/cli_update_excludes.rs`.
