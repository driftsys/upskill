# Pure captured item compilation with current generation compatibility

AD-0024

## Context

Lifecycle planning needs complete generated entrypoint and resource sets from
the same captured source bytes. Current rule, skill and agent rendering must
remain available before general lifecycle activation.

## Options

The working design considered invoking the filesystem installer into a temporary
directory, rewriting generation and resource handling as a new target adapter
framework, and compiling one captured item through existing pure renderers and
historical layouts. The installer would reread mutable sources, introduce
effects and inherit recursive cleanup. A new adapter framework would combine
rendering compatibility work with native target qualification.

## Decision

Provide a separate pure producer for one fully validated supplied manifest,
reusing current naming, resource classification, ignore filtering, typed
renderers, frozen layout and both flat-output resource link passes. Preserve
source-name rendering under installed aliases, complete per-client file sets,
canonical client ordering and explicit audience exclusions. Keep the current
installer on its existing path and use its generation `Client` vocabulary.

## Consequences

Captured values remain sufficient after the original directory changes or is
removed, and deterministic outputs can be reviewed before execution. The cost is
an inactive prerequisite: future coordination must supply roots, ownership,
consumers and destination evidence, and future adapters must qualify native
surfaces and required semantics. Source identity does not establish authenticity
or generated-content identity. Audience exclusion grants no deletion authority.
Changing native identity under aliases requires a later explicit adapter choice.

Satisfies:
[Specification §4.5](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#45-captured-item-compilation).
Related:
[compiler design](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#architecture-captured-item-compilation),
[AD-0019](0019-separate-strict-source-capture.md),
[AD-0021](0021-pure-file-planning.md). Rationale:
[archived spec](../archive/specs/2026-10-12-captured-item-compilation-design.md).
