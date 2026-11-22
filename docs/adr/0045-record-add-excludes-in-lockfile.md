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

- A: it lasts until a later `add` of the same source undoes it (chosen).
- B: `remove` also records an exclude.
- C: the exclude ends when the requiring item changes.

Storage:

- A top-level record per source (chosen): one place and one meaning, matching
  the per-command scope of `--exclude`.
- A list copied onto each item entry of the `add`: the list is duplicated, two
  `add` commands of one source with different excludes are ambiguous, and an
  excluded dependency has no entry of its own.
- Excludes on the bundle record and on the requiring item: one concept in two
  places, and a plain directory source is not covered.

How a later `add` changes the record:

- Excludes accumulate, and naming an item undoes its exclude (chosen).
- Replace the record at each `add`: an unrelated `add` of the same source would
  silently undo an exclude.
- A separate `--include` option: naming the item already says the same thing.

## Decision

1. The lockfile has an optional top-level `excludes` array. A record is
   `{ source, names }`, where `source` is the canonical source label of the
   `add` and `names` is sorted and without duplicates. The array is omitted when
   empty, the schema number does not change, and a lockfile without excludes
   keeps its bytes.
2. After a successful commit, `add` sets the record of its entry source to the
   recorded names plus its `--exclude` names, minus the item names that it was
   given and does not also exclude. An empty result deletes the record. An `add`
   without item names and without `--exclude` keeps the record.
3. `update` passes the recorded names of a source as its excludes, in all three
   paths above. `update` never removes names from the record. An item excluded
   after it was installed stays installed until the next `update`, which removes
   it; the name stays recorded.
4. `remove` deletes the record of a source when no item entry and no bundle
   record of that source remain.
5. `doctor` reports a record whose source has no item entry and no bundle record
   (`orphan_excludes`); the report is not clean.
6. `--exclude` keeps matching by source name across every source of the closure.

## Consequences

- A removed dependency still comes back through its requirer; `remove` records
  nothing (option B not taken).
- An `--exclude` name that matches no item is recorded and stays until an `add`
  names it.
- `update --check` does not need the record, because it plans no items.
- The only way to undo an exclude is `add <source> <name>`, or editing the
  lockfile.

Satisfies: #570. Code: `src/lockfile.rs` (`record_excludes`, `excludes_for`,
`prune_excludes`), `src/pipeline/prepared.rs`, `src/pipeline/lifecycle.rs`.
Tests: `tests/cli_update_excludes.rs`, `tests/cli_doctor.rs`.
