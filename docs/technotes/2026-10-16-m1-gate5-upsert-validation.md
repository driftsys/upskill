# Gate 5 upsert validation — 2026-10-16

The public acceptance examples exercise the pure coordinator through complete
state and literal output expectations. The initial clean RED was an unresolved
upsert module, followed by 12 passing examples. A fixture initially attempted
`Default` on `LockedItem`; that compilation diagnostic was corrected before the
clean RED and is not assertion evidence.

Expanded examples exposed a known legacy destination collision with an
assertion-failing RED despite different installed aliases and a missing file.
Frozen-projection destination checks made it pass. A separate assertion RED
exposed an omitted new-root legacy blocker when prior selection already blocked;
collecting both probe results made it pass. One temporary-lifetime fixture error
was corrected before the destination RED and is excluded from evidence.

The resulting 20 acceptance examples cover new installs, changed source history,
empty output retention, exact graph retirement, manual aggregate bytes/modes,
selection authorization, complete consumer accounting, selected bundle closure,
legacy/native collisions, untouched history and conservative sharing rejection.
Additional examples pin new bundles referencing untouched installations and
rejection of invalid prior state or installation binding changes. They use
supplied observations; no native or filesystem qualification follows.

Eight independent source controls target empty history, authorization,
exclusions, untouched associations, no-op evidence, known legacy destinations,
complete selection and tagged-history retention. The scratch control harness
records assertion/panic failures and source restoration; compile errors are not
counted. Formatting, all-target sanity, full verification and locked Rust 1.85
all-target checking returned exit 0. Independent reviews and hosted CI remain
separate delivery gates. The successful outcome payload is boxed to satisfy the
zero-warning Clippy enum-size gate; two fixture style warnings were also
corrected. A full verification run passed its tests and then stopped on
formatting in concurrently gardened documentation; this diagnostic run is not
counted as a completed gate.

Production schema 1, CLI dispatch, existing mixed-planner multi-owner rejection
and native support claims remain unchanged. This slice does not complete Gate 5,
activate M1 or satisfy the updater prerequisite.

## Independent and quick review corrections

The isolated whole-branch review found no production defects. It identified a
small authorization coverage gap also reported by the quick tests review:
retirement rejection could mask missing reference authorization. Additional
quick findings concerned unselected bundle metadata, known legacy conflicts on
old affected paths, new-association ordering and blocker deduplication. The five
discriminating examples produce 25 passing public tests. Five additional
assertion controls target those exact gaps; combined with the original eight,
the cumulative count is 13. Production source remains unchanged. The design now
spells out diagnostic output order rather than using ambiguous “selector order”.
Final local checks and hosted/numbered reviews remain separate gates.
