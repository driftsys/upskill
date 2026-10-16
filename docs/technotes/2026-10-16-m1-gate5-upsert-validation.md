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

## PR pass 1 coverage and documentation corrections

Pass 1 found no production defects. Four confirmed test gaps concerned
successful native-plugin history preservation, exact planned destinations,
added-reference authorization and per-artifact consumers. Minor findings
concerned preserved association positions and two older remaining-work
statements. All were fixed with examples and updated successor links. Additional
quick-review gaps pin opposing path/installation order, allowed selected bundle
metadata and merged diagnostic order. The suite now has 33 passing public
examples, including bundle/dependency retirement and mixed dedicated/instruction
empty-set retention.

Eight additional controls each failed an intended assertion or panic: skipped
addition authorization, association sorting by ID, rejected selected bundles,
globally reordered blockers, requested-consumer substitution, globally sorted
prior associations, cleared legacy plugins and corrupted file destinations.
Exact production source bytes were restored after each control. The cumulative
control count is 21. Formatting, all-target sanity, full verification and Rust
1.85 locked checks after these fixes returned exit 0. Numbered pass 2 and
exact-head hosted checks remain the next delivery gates.

## Final numbered-pass corrections

Pass 2 confirmed one coverage gap: the replacement association fixture kept
identical old and desired consumers. The strengthened example now changes the
selected output to Copilot while retaining its old position and an appended
Claude output. A separate quick-review fixture gap now uses distinct native rows
beside duplicates, in an order different from lexical sorting and reversal. The
full original snapshot is still asserted exactly.

Two further assertion controls fail when a replacement reuses its old payload or
retained plugin/MCP vectors are reversed. Exact source bytes were restored; 33
public examples pass and the cumulative control count is 23. Production source
is unchanged. Final local checks and exact-head hosted checks follow; there is
no third numbered review cycle.
