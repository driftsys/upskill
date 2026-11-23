# Record the excludes of `add` so that `update` applies them

Satisfies: #570 (bullet 4). Related: #598 (review that widened the scope).

## Intent

`upskill add <source> --exclude X` leaves X out. The lockfile does not record
this, so a later `update` of that source plans X again. Three paths bring X
back:

- a bundle-file source: `update` resolves the whole bundle again;
- a directory source with recorded bundles: `update` resolves those bundles
  again;
- a plain directory source: `update` expands the `requires:` closure of the
  recorded items again, so an excluded dependency (also from another source)
  comes back.

Planning X again can install it, replace the same item that another source
installed, or make `update` fail.

## Goals

- `update` leaves out every name that the last `add` of the same source
  excluded, in all three paths above.
- A later `add` of the same source adds its `--exclude` names to the record. An
  `add` that names an excluded item explicitly (`add <source> d`) removes `d`
  from the record and installs it. This is the only way to undo an exclude. An
  `add` without item names and without `--exclude` keeps the record.

## Non-goals

- `remove` does not record anything new. A removed dependency still comes back
  through its requirer (the user chose option A, not B).
- No change to how `--exclude` matches: by source name, across every source of
  the closure.

## Anti-goals

- No schema bump, and no change to the bytes of a lockfile that has no excludes.

## Design

- `Lockfile` gets a top-level field `excludes: Vec<LockedExclude>`, with
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
  `LockedExclude { source: String, names: Vec<String> }`, where `source` is the
  canonical source label of the `add` (the entry source) and `names` is sorted
  and deduplicated.
- `add`: after a successful commit, the record of its entry source becomes
  (recorded names ∪ its `--exclude` names) minus the item names that it
  requested explicitly. An empty result deletes the record.
- `update`: when it builds the options for a source
  (`src/pipeline/lifecycle.rs`, today `excludes: vec![]`), use the recorded
  names of that source.
- `remove`: when no item entry and no bundle record of a source remains, delete
  its exclude record.
- `doctor`: report an exclude record whose source has no item entry and no
  bundle record.
- Docs: remove the "Current limitation (#570)" sentence from
  `docs/specification/specification.md` and `docs/book/commands.md`; state the
  record in the lockfile section of the specification and in format-spec if it
  describes the lockfile fields.

## Alternatives considered

1. **A top-level record per source (chosen).** One place and one meaning; it
   matches the per-command scope of `--exclude`. The cost is an upsert and
   removal path and one `doctor` check.
2. **An `excludes` list on each item entry of the `add`.** No new top-level
   structure, but the list is duplicated, two `add`s of one source with
   different excludes are ambiguous, and an excluded dependency has no entry of
   its own.
3. **`excludes` on the bundle record and on the requiring item.** Close to what
   each exclude affects, but one concept in two places, and a plain directory
   source with no requiring entry is not covered.

Duration of an exclude, decided by the user: option A (until a later `add` of
the same source undoes it). Options not taken: B (`remove` also records an
exclude) and C (the exclude ends when the requiring item changes).

## Left to the builder

- Acceptance tests first, one per path above, plus: an `add` that names an
  excluded item removes it from the record; `remove` of the last item deletes
  it; a lockfile without excludes stays byte-identical; `doctor` reports an
  orphan record.
- Whether `update --check` needs the record (it does not plan items; probably
  not).

How a later `add` changes the record, decided by the user after a review of this
design: excludes accumulate, and naming an excluded item undoes it (option B).
Not taken: replacing the record at each `add` (an unrelated `add` from the same
source would silently undo an exclude), and a separate `--include` option
(naming the item already says the same thing).
