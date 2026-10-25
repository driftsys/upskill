# Retain aggregates after final-region removal

AD-0018

Production-scope clarification (2026-10-25): this is an experimental library
mechanism, not authorization to place managed contributions in team instruction
files. Its pure planning and owner-evidence contracts remain reusable.
[AD-0040](0040-separate-customization-delivery-from-discovery.md) and the
[durable delivery inventory](../topics/0009-customization-delivery.md#amendment-inventory)
keep production rule delivery independent of team guidance.

## Context

Current contribution ownership does not establish whether an aggregate file
existed before installation or who created the containing file.

## Options

Delete the aggregate when its final owned region is removed, or retain the
resulting file. Retention is selected because file origin is unknown.

## Decision

Return resulting bytes and no file deletion action after removing the final
region. Preserve manual bytes; the retained file is empty only when no content
remains.

## Consequences

A later executor may retain an empty file. This avoids deleting a potentially
user-owned aggregate without evidence of file ownership.

Satisfies:
[Specification §4.2](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#42-pure-owned-instruction-planning).
Related:
[planner design](https://github.com/driftsys/upskill/blob/main/docs/topics/0006-artifact-planning.md#architecture-owned-instruction-planning).
Rationale:
[archived spec](../archive/specs/2026-10-12-owned-instruction-planning-design.md).
