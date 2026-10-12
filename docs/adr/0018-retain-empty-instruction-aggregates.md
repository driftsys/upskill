# Retain aggregates after final-region removal

AD-0018

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

Satisfies: [Specification §4.2](../specification/specification.md#42-pure-owned-instruction-planning).
Related: [planner design](../design/owned-instruction-planning.md).
Rationale: [archived spec](../archive/specs/2026-10-12-owned-instruction-planning-design.md).
