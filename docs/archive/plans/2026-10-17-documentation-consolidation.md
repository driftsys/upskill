# Documentation consolidation implementation plan

> Execute inline using the executing-plans skill; review the complete migration.

**Goal:** Give readers one canonical reference per concern.

**Architecture:** Move existing requirements and architecture into eight
numbered topics, retain compatibility navigation at old paths, and publish a
coherent reading sequence. Rename technical notes without changing evidence
assets.

**Spec:**
[Approved consolidation](../specs/2026-10-17-documentation-consolidation.md).

## Constraints and review focus

Preserve contract text, Draft status, activation limits, ADR IDs, historical
anchors, schema and evidence bytes. Check moved relative links and generated
book anchors. Existing WIP remains unresolved.

## Tasks

- [ ] Create `docs/topics/0001-content-format.md` through
      `0008-execution-and-recovery.md`, moving contract and architecture
      sections.
- [ ] Replace old design and format paths with compatibility pages; retain old
      tool-specification section anchors as concern links.
- [ ] Rename technotes to concern paths, update references, index validation
      records, and preserve asset hashes.
- [ ] Update documentation map, book reading order and contributor guidance.
- [ ] Run `just docs-check`, inspect migration coverage and rendered output,
      then `just fmt` and `just verify`; review and archive only this spec/plan
      pair.
