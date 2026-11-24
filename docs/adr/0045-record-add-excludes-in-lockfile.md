# AD-0045 — Record the excludes of `add` in the lockfile

Status: **accepted**; decision 5 amended on 2026-11-23 (see the end of this
record). Resolves bullet 4 of
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
   when empty, and a lockfile without excludes keeps its bytes. One lockfile is
   one install scope, so the set is per scope.
2. `add` and `update` compute their excludes as the recorded names, minus the
   names installed by name, plus the `--exclude` names of the command. `add`
   passes the item names of its command line as the names installed by name;
   `update` passes the in-source names of every item that the lockfile records
   for the source, also a bundle member, so an exclude never removes an
   installed item. `update` never removes a name from the recorded set; only a
   positional name of `add` does.
3. The resolvers skip an excluded dependency or bundle member before they link a
   `required_by` entry to it, and they do not pull its own `requires:` items. A
   whole-source `add` skips excluded items too. This holds in the bundle,
   directory-with-bundles and plain directory paths, also for a dependency from
   another source.
4. After a successful commit, `add` writes the set: the recorded names plus its
   `--exclude` names, minus the item names that it was given and does not also
   exclude. An `add` without item names and without `--exclude` keeps the set.
   `add <source> <name>` installs `<name>` and removes it from the set.
5. `add <source> --exclude d` removes an installed item `d` of the same source,
   unless the same `add` also names `d`, within the kinds that `--kind` selects:
   that item only, with no member of a co-located group, no bundle record and no
   plugin entry. It deletes each `required_by` link that names `d` and prints
   `removed — excluded <kind> <name>` on stdout. An item with a modified
   recorded file is not removed: its files and lock entry stay and stderr gets
   `kept — excluded <kind> <name>: <path> was modified`. `update` has no such
   removal and no `removed — excluded` status.
6. An `--exclude` name of an `add` that matches no item of the entry source and
   no item that the resolver skipped is a usage error (exit code 2), and the
   lockfile is unchanged. The recorded names are not checked, because an item
   can leave its source.
7. `add` and `update` print `excluded <kind> <name>` on stderr for each skipped
   item, once; an item that the same run removes is reported only as removed,
   not also as excluded; `--quiet` hides the line, and `update` prints it with
   `--dry-run`. `list` prints `excluded: <names>` for each scope, also for a
   scope with no items, and `list --json` has an `excludes` key for each scope
   that has at least one exclude and omits the key when the list is empty.
8. `remove` records no exclude and prunes none. `doctor` has no check for
   excludes.
9. The lockfile schema number is 3. `load` reads `schema` 1, 2 and 3, and every
   save writes `schema: 3`; a `schema: 2` file has the same shape and is written
   back as `schema: 3` without other change. The reason: upskill v0.9.2 and
   earlier versions do not reject unknown lockfile fields, so when such a
   version rewrites a shared project lockfile it drops `excludes`, and the next
   `update` installs the excluded items again. These versions refuse a `schema`
   greater than 2 with an error that tells the user to upgrade upskill. Schema 3
   changes this silent data loss into that error.

## Alternatives considered

Keep `schema: 2` for a lockfile with `excludes`. This was the first decision,
because the field is optional and older versions parse the file. It was dropped:
an older version that saves the file drops the field without an error, so the
excludes are lost for every user of a shared project lockfile.

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
  repaired. This holds also for links that an earlier exclude removal deleted
  (see the amendment).
- An upskill version that predates schema 3 cannot read a lockfile that a
  current version saved, also one without excludes; the user must upgrade.
- The ways to undo an exclude are `add <source> <name>` and editing the
  lockfile.

Satisfies: #570, #613. Code: `src/lockfile.rs`, `src/pipeline/prepared.rs`,
`src/pipeline/install.rs`, `src/pipeline/bundle_plan.rs`,
`src/pipeline/lifecycle.rs`, `src/pipeline/mod.rs`, `src/pipeline/report.rs`,
`src/main.rs`. Tests: `tests/cli_update_excludes.rs`.

## Amendment: keep required_by links on exclude removal (2026-11-23)

Decision 5 said that `add <source> --exclude d` deletes each `required_by` link
that names the removed item `d`. This is replaced: the command keeps those
links, as `remove` does, and prints `removed — excluded <kind> <name>` on
stdout. Nothing else is removed, the lockfile schema is unchanged, and no field
is added.

Reason (#613): a dependency `e` that only `d` required had an empty
`required_by` after the deletion, which means "installed by name". `doctor` then
did not report `e` as an orphaned dependency, although it does after `remove d`
(AD-0009: removal never cascades and `doctor` reports the orphan).

Second change, found while testing: for a directory source with recorded
bundles, the `update` item filter took only entries with an empty `required_by`.
With the links kept, `e` was no longer in the filter, because no planned item
required it, and `update` removed it with the message `no longer in source`,
which is a cascade removal. The filter now also takes an entry whose
`required_by` is not empty, names no requirer that the lockfile records, and is
listed by no recorded bundle of the source. A member of a recorded bundle
follows its bundle: when the bundle no longer lists it in the source, `update`
removes it as `no longer in source`, as it did before. This is the `doctor`
condition for an orphaned dependency, shared as
`Lockfile::is_orphaned_dependency`. An entry that still has an installed
requirer keeps the earlier behaviour: when that requirer no longer requires the
entry in the source, `update` still removes the entry as `no longer in source`.

Known limit: the lockfile cannot tell an item installed by name before it was
required (`add e`, then `add d`) from a pure dependency. After the exclude of
`d`, or after `remove d`, `e` has `required_by: ["skill:d"]` and `doctor`
reports it as an orphaned dependency; the user decides what to do.

A lockfile from which an earlier `add --exclude` already deleted links is not
repaired: the deleted links do not return.
