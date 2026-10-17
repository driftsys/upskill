# Source-backed lifecycle orchestration

Authority: delegated Gate 5 completion lane. Production activation and native
Codex qualification remain Gate 6. This work owns the opt-in source-backed
library module, its integration examples, and this bounded record.

## Acceptance and sequence

1. RED: local source add, changed-source update, literal removal and target
   precedence examples require the absent orchestration API.
2. GREEN: open the guarded lifecycle session before all source work; complete
   update selection before resolution; retain resolved roots and captures
   through a scoped opaque prepared-operation callback. Revalidate full captures
   before apply. Compile only explicit rule roots through existing
   source/compiler APIs.
3. Extend examples for Git branch advancement, stable IDs, selection blocking,
   stale preparation, conflict preservation and configured target precedence.
4. Run focused examples, formatting, repository verification and MSRV; record
   actual outcomes. Full process interruption acceptance belongs to the parent
   completion lane and does not follow from these examples.

## Delegated rulings

- R-G5-ORCHESTRATION-1: Expose a scoped callback receiving an opaque prepared
  operation. The operation retains private captures and cannot accept
  substituted output or evidence. Application revalidates Git identity and
  source bytes.
- R-G5-ORCHESTRATION-2: Stable IDs hash a serialized tuple of scope, raw root,
  normalized requested source binding, kind, source name and installed alias.
  Actual Git commits remain evidence, outside the ID. Existing matching bindings
  reuse IDs; different bindings sharing an ID are rejected.
- R-G5-ORCHESTRATION-3: Empty target lists are absent overrides. Invocation,
  project configuration, global configuration, then recorded update targets or
  caller-provided add defaults determine requested targets. No implicit default
  or unresolved legacy selection is invented.
- R-G5-ORCHESTRATION-4: Only explicit selected rule roots are compiled here.
  Manual/shared instruction contributions remain on the captured command API.
  Removal delegates recorded reference semantics without source resolution.

## Status

Implementation and local validation complete: eleven integration examples, one
private identity example, full escalated `just verify`, `just fmt`, and explicit
Rust 1.85 `just msrv-check` pass. The fixture-isolation example was added during
full verification and run in a separate final focused suite. Its unsafe RED
attempt was rejected before execution; no failing-run evidence is claimed for
that example. Review/delivery and whole-lifecycle process checkpoint acceptance
remain with the completion lane. No Gate 5 completion or native qualification
claim.

- R-G5-ORCHESTRATION-5: Repeated add with the same full binding reuses the
  installation ID, selects its prior owner for complete upsert, and adds no
  duplicate direct reference. Different-binding alias reuse rejects before
  resolver fetch/capture, preventing ambiguous literal selection. Local alias
  preflight normalizes only ancestors, leaving the final entry for
  qualification. Existing bundle/owner reference graphs are preserved by update;
  this module does not discover registries or build bundle graphs. Supplied
  complete graphs and instruction sets remain the captured adapter's
  responsibility.
