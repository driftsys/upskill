# Bundle file format — YAML, not Markdown-with-frontmatter

**Status**: Proposed (2026-05-21)

## Context

[ADR-0002](./0002-portable-content-format.md) established the SSOT
on-disk contract, including bundle files as
[Markdown-with-frontmatter](../format-spec.md) named `<name>.bundle.md`.
The frontmatter carried the manifest (`schema`, `name`, `description`,
`items`, `requires`, `metadata`); the body carried human-readable
documentation (install examples, adoption path, caveats).

That shape mirrored `SKILL.md` / `RULE.md` / `AGENT.md` and gave bundle
authors one file to point at. In practice, the body never carried
information that influenced installation behavior — the parser reads
the frontmatter and discards the body. The Markdown body is purely
documentation.

[ADR-0006](./0006-flat-item-layout.md) tightened the item layout. This
ADR tightens the bundle layout for the same reason: keep the SSOT
contract as small as it needs to be, drop redundancy.

## Decision

### Manifest is a pure YAML file

Bundle manifests are pure YAML, with the filename suffix
`.bundle.yaml`:

```text
<bundle-root>/
├── platform-baseline.bundle.yaml
├── android.bundle.yaml
└── rust-embedded.bundle.yaml
```

The file body is the YAML manifest verbatim — no `---` delimiters, no
trailing Markdown. The schema is unchanged from ADR-0002 §3.7
(`schema`, `name`, `description`, `license`, `items`, `requires`,
`metadata`); only the file format changes.

```yaml
schema: 1
name: platform-baseline
description: Baseline rules, skills, and agents for all repositories
license: proprietary

items:
  rules:
    - license-awareness
    - security-baseline
  skills:
    - pr-summary
  agents:
    - security-reviewer

metadata:
  version: "1.2.0"
  author: platform-dx
```

### Optional sibling README

A bundle MAY have a sibling Markdown file at `<name>.bundle.md` (same
stem, `.md` suffix) carrying human-readable documentation — install
examples, adoption path, exclusions, maintenance notes. The parser
ignores `.bundle.md` files entirely; they exist for humans browsing the
registry.

### Discovery is suffix + schema-field gate

Discovery walks for `*.bundle.yaml`. The parser does a cheap
`schema:` field check first: if the top-level `schema:` key is absent
or not an integer, the file is silently skipped (it's not a bundle,
just a YAML file that happens to share the suffix).

Explicit-path operations (`upskill add ./foo.bundle.yaml`) are strict:
a file that doesn't parse as a valid bundle is a hard error. The user
pointed at it; silence would hide bugs.

## Consequences

**Positive.**

- One parser path for bundles, one for items. No frontmatter-extraction
  step on bundle files — direct `serde_yaml_ng::from_str`.
- Manifest validation is stricter — no Markdown body that can drift
  away from the frontmatter, no risk of an author editing the body and
  accidentally changing what looks like manifest text.
- Conventional shape — `Cargo.toml`, `package.json`, `action.yml`, and
  most other manifest files in the wider ecosystem are pure
  data-language files, not data-plus-prose.
- The README sibling is optional. Bundles that need no narrative
  documentation ship as one file. Bundles that benefit from narrative
  put it in a sibling, freely editable without touching the manifest.

**Negative.**

- Shape consistency with `SKILL.md` is lost. Items use
  frontmatter+body; bundles do not. The trade-off is judged worthwhile:
  bundles are mostly manifests, items are mostly content, and the
  current shared shape was forcing a body on a file that doesn't need
  one.
- One-time fixture migration in `tests/fixtures/bundles/` and the
  in-repo `skills/prompt-engineering.bundle.md`.
- Tutorials and existing registries authored against v0.5 (`.bundle.md`)
  must rename and strip the body. Per
  [no back-compat until 1.0](../../AGENTS.md), this is a hard cut.

## Alternatives considered

**(a) Keep `.bundle.md` with frontmatter+body.** Rejected: the body
adds no parser semantics and creates two places where an author can
write contradictory information about the same bundle.

**(b) TOML instead of YAML.** Rejected: every other SSOT file in this
project is YAML-frontmatter; introducing TOML for one file kind adds a
second config language without solving any problem the current YAML
shape has. TOML's strengths (flat key-value, strict scalar parsing) do
not match the bundle's structured `items.*` and `requires[]` shape.

**(c) Filename namespace — `<name>.upskill-bundle.yaml`.** Rejected:
bundle files live in tool-owned locations and are installed via
`upskill add` explicitly. The `schema:` field check is a stronger gate
than a filename prefix — random `.bundle.yaml` files outside our schema
are skipped silently in discovery.

**(d) Directory form `<name>.bundle/{bundle.yaml, README.md}`.**
Rejected: bundles do not carry sidecar assets the way items do. A
directory per bundle is overhead for nothing.

**(e) Allow qualified item names (`owner/repo:skill-name`) in
`items.*`.** Deferred. The simple shape — bare names resolving from
the bundle's host registry — covers every current use case. Adding
cross-registry composition at the bundle-file level adds resolver
complexity (multi-source cloning, name collisions, lockfile shape)
without a real consumer asking for it. Cross-registry composition
remains available at the `upskill add` level by installing multiple
bundles or sources.

## Migration

Per [no back-compat until 1.0](../../AGENTS.md), the format change is
a hard cut. v0.6 reads `.bundle.yaml` only; `.bundle.md` is removed
from spec and parser in one release. No translator, no fallback
discovery.

Registries authored against v0.5 (`.bundle.md`) must:

1. Rename `<name>.bundle.md` → `<name>.bundle.yaml`.
2. Strip the `---` delimiters and the Markdown body. If the body
   contained documentation worth keeping, move it to a sibling
   `<name>.bundle.md`. The new `.bundle.md` is documentation only —
   the parser ignores it.

A simple shell rename + body-strip script suffices; no schema changes
to the manifest itself.

## References

- Predecessor: [ADR-0002](./0002-portable-content-format.md) §
  "Bundle file shape" — superseded by this ADR for the file format;
  the bundle schema (fields, semantics, transitive resolution) is
  unchanged.
- Authoritative spec: [`docs/format-spec.md`](../format-spec.md) §2.2,
  §3.7.
- Related: [ADR-0006](./0006-flat-item-layout.md) (same "drop
  redundancy in the SSOT contract" rationale, applied to items).
