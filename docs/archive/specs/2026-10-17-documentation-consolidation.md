# Documentation consolidation

Approved scope: organize canonical contracts and implemented architecture into
eight numbered concern documents under `docs/topics/`: content format,
generation, sources, state and ownership, lifecycle planning, artifact planning,
filesystem access, and execution and recovery. Keep requirements distinct from
architecture within each topic. Preserve Draft and inactive-library boundaries.

Keep CLI contracts in the tool specification. Keep ADR IDs, book chapter URLs,
and historical section anchors stable through compatibility pages. Use date-free
technote paths; retain observation dates, revisions, schema bytes, licenses, and
evidence manifest hashes. Index every validation record by concern.

Do not classify unrelated WIP as complete or change product behavior. Verify
repository links, rendered anchors, publication isolation, and preserved assets,
then run `just fmt` and `just verify`.
