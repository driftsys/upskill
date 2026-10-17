# Reject active captured overrides until precedence is implemented

AD-0025

## Context

The portable format defines client body overrides, but the existing installer
does not apply them. The captured producer must make this implementation gap
explicit without changing the installer or claiming format conformance for such
inputs.

## Options

The recorded delivery ruling considered rejecting active matching overrides,
silently ignoring them and implementing precedence in this slice. Silent
ignoring would conceal unsupported semantics; implementing precedence would
expand the bounded compatibility producer.

## Decision

Reject a canonical root override for the requested kind and an audience-included
requested client with a contextual unsupported-override error. Ignore patterns
cannot suppress this rejection. Overrides for other kinds or non-emitted clients
remain reserved source inputs, excluded from resources and included in source
identity. Unknown client suffixes retain the existing resource classification.

## Consequences

Callers receive an error rather than an apparently successful result with
omitted override semantics. The cost is a dedicated producer extension with
compatibility tests before such inputs can compile. Existing renderer directive
behavior and installer behavior remain unchanged.

Satisfies:
[Specification §4.5](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#45-captured-item-compilation).
Related:
[compiler design](https://github.com/driftsys/upskill/blob/main/docs/topics/0002-generation.md#architecture-captured-item-compilation),
[AD-0024](0024-pure-captured-item-compilation.md). Rationale:
[archived spec](../archive/specs/2026-10-12-captured-item-compilation-design.md).
