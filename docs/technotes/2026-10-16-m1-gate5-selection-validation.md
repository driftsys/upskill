# Gate 5 selection validation — 2026-10-16

The 15 local public acceptance tests and full local verification passed. This
note records only observed checks; hosted checks and independent review remain
separate delivery gates.

The new pure selection boundary preserves all supplied state and returns no file
actions. Initial unresolved-interface RED preceded implementation. Nine initial
acceptance examples passed. A bundle descendant example exposed missing closure
with an assertion-failing RED after correcting an invalid cycle fixture. A
native bundle descendant example then exposed hidden historical plugin/MCP
associations with another assertion-failing RED. Subsequent examples cover
combined blockers and equal aliases across project/user scopes. A nonexistent
Global scope fixture was corrected to User before fault-control counting; its
compile failure is not an assertion-control result.

Production command dispatch and schema 1 remain unchanged. Source/scoped
selectors, CLI precedence, complete add/update, new-sharing compatibility,
backup/execution, recovery and native qualification remain later lifecycle work.
No completed Gate 5, M1 activation or updater prerequisite claim follows from
these checks.

## Assertion controls

Eight independent controls each failed an intended assertion (exit 101): ignored
legacy rows, hidden native bundle associations, ignored name ambiguity, ignored
unsupported kinds, omitted complete-state validation, reordered candidates,
omitted bundle closure and ignored unknown names. `controls.json` records each
focused test and restored SHA-256. Exact source bytes were restored after every
experiment; the earlier fixture compile failure is excluded from this count.

Additional acceptance examples cover populated unsupported kinds and a bundle
diamond selected together with an explicit member; complete candidates remain
unique in prior order. The final formatted suite and full verification passed.

## Local delivery checks

`just fmt`, rendered documentation checks, `just sanity` (all-target Clippy and
locked checking), `just verify`, and Rust1.85 locked all-target checking all
returned exit0. `delivery-receipts.json` records the command exits. An earlier
rendered-book failure was fixed by using the existing canonical GitHub
source-link convention for the unpublished design page; a separate earlier
formatter failure occurred while the archive index was being edited. Those runs
are diagnostic, not successful verification evidence. Independent
implementation/PR review and hosted checks remain separate gates. No native
qualification claim is made.
