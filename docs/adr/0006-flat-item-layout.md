# Flat item layout — drop kind subdirectories

**Status**: Proposed (2026-05-13)

## Context

[ADR-0002](./0002-portable-content-format.md) established the SSOT
on-disk contract, with the illustrative layout:

```text
<item-root>/
├── rules/<name>/RULE.md
├── skills/<name>/SKILL.md
└── agents/<name>/AGENT.md
```

[`docs/format-spec.md` §2.1](../format-spec.md) and the lint discovery
walk ([`src/lint.rs:181`](../../src/lint.rs)) both treat the kind
subdirectories (`rules/`, `skills/`, `agents/`) as required. ADR-0002's
prose calls the diagram "illustrative, not normative," but the spec and
implementation enforce it as normative — a long-standing inconsistency.

Two observations make the kind subdirs net-negative:

1. **Kind is already encoded in the entrypoint filename.** `SKILL.md`,
   `RULE.md`, and `AGENT.md` are mutually exclusive within a directory.
   Storing the same information again in the parent directory name is
   redundancy without disambiguation value.
2. **The Agent Skills open standard ([agentskills.io](https://agentskills.io))
   requires only `<dir>/SKILL.md` with `name` matching the parent
   directory.** It explicitly allows "any additional files or
   directories" in a skill directory. Under the current layout, a
   skill is at `skills/<name>/SKILL.md` — compliant. But adding a
   sibling RULE.md or AGENT.md for the same capability is currently
   forbidden by spec, even though the standard would accept it as
   additional files.

The kind subdirs also force three near-empty parallel trees for
projects with a single dominant kind (most real registries are
skill-heavy or rule-heavy, not balanced).

A broader reorganization that grouped items by capability ("skillsets"
holding one rule + one skill + one agent) was rejected after a
thin-slice analysis: the MarkSpec planned inventory and a hypothetical
web-dev corpus both showed that natural cohesion groups rarely contain
exactly one of each kind, and the spec/code change surface was large.
This ADR adopts the narrower change: drop the kind subdirs, keep items
at one level, allow but do not require kind co-location within an item
directory.

## Decision

### Items at one level under `<item-root>`

```text
<item-root>/
├── license-awareness/
│   └── RULE.md
├── pr-summary/
│   └── SKILL.md
├── security-reviewer/
│   └── AGENT.md
└── markspec-trace/
    ├── SKILL.md         # name: markspec-trace
    └── AGENT.md         # name: markspec-trace (co-located, shared name)
```

Each item is a directory directly under `<item-root>`. Kind is
determined by the entrypoint filename (`RULE.md`, `SKILL.md`,
`AGENT.md`). An item directory MUST contain at least one entrypoint;
when it contains more than one, all `name:` fields in those entrypoints
MUST match the directory name.

### Implications for the contract

- **`name`-matches-parent-directory invariant preserved.** Every
  entrypoint's `name:` still equals its parent directory's name. When a
  directory holds multiple kinds, they share a name.
- **Agent Skills compatibility preserved.** A skill directory remains
  `<dir>/SKILL.md` with `name` matching the parent — compliant. Any
  sibling RULE.md or AGENT.md is "additional files" under the standard
  and ignored by agentskills.io-compatible tooling.
- **Per-client override files unchanged in mechanism.** `RULE.claude.md`,
  `SKILL.claude.md`, `AGENT.claude.md` can coexist in one directory,
  disambiguated by filename.
- **Supporting resources can be shared across kinds.** A `templates/`
  or `references/` subdirectory in an item directory is referenced by
  whichever entrypoints need it.
- **Bundle references unchanged.** `items.rules`, `items.skills`,
  `items.agents` continue to reference items by name; the on-disk
  layout under which a bundle's referenced items live is not part of
  the bundle contract.

### Co-location is allowed, never required

The MarkSpec planned inventory and the web-dev secondary corpus do not
exercise co-location: items rarely share names across kinds. The
allowance is a forward-compatible affordance for genuinely paired
content, not a structural expectation.

## Consequences

**Positive.**

- Smaller spec surface — §2.1, §2.3, §2.4, §3.1 simplify; §3.7 is
  unchanged.
- Smaller code surface — lint discovery collapses from three globs to
  one walk-by-entrypoint-filename; scaffold no longer chooses a kind
  subdir.
- Better alignment with the Agent Skills standard — items in
  `<item-root>/` look like an Agent Skills directory to compliant
  tooling.
- One less authoring decision — authors no longer ask "does my
  registry need three subdirs even though I only have skills?"
- Forward-compatible co-location for naturally-paired content.

**Negative.**

- Discovery is by-filename rather than by-folder — slight conceptual
  shift, but no scan-cost change in practice.
- The container directory holds mixed kinds. Visual "all rules
  together, all skills together" navigation goes away. Authors who
  want grouping use naming prefixes (`markspec-trace-*`).
- Authors with multi-kind items face a name-sharing constraint.
  Trivially satisfied since co-location is opt-in.
- One-time fixture migration in `tests/fixtures/` for the generation
  golden tests.

## Alternatives considered

**(a) Keep ADR-0002's layout as-is (do nothing).** Rejected: the spec
and implementation disagree with ADR-0002's "illustrative, not
normative" prose, and the kind subdirs encode information already
present in the entrypoint filename.

**(b) The broader skillset reorganization** — one directory per
capability, holding at most one of each kind. Rejected after thin-slice
analysis. MarkSpec's planned inventory does not fit the 1-of-each
shape: multiple skills naturally cluster (write-requirement,
write-test-entry, write-element, write-reference), as do rules
(5 syntax rules). Forcing them into per-skillset slots required
workarounds (sub-skillsets, filename variants) that re-introduced
per-item directories anyway. The cohesion benefit was available for
free via naming prefixes and existing bundle manifests.

**(c) Rename the container** (`items/` instead of leaving
`<item-root>` flexible). Rejected: ADR-0002 already declares
`<item-root>` author-choice. Mandating a name removes that flexibility
for no concrete benefit.

## Migration

Per [no back-compat until 1.0](../../AGENTS.md), the layout change is a
hard cut. v0.4 reads the new layout only; the kind-subdir layout is
removed from spec and code in one release. No translator, no fallback
discovery, no dual-support window.

Source registries that have authored content against v0.3 (the
`rules/skills/agents/` layout) move their item directories up one
level. This is a `git mv` per item.

## References

- Predecessor: [ADR-0002](./0002-portable-content-format.md) §
  "Directory-per-item layout, symmetric across kinds" — superseded by
  this ADR for the kind-subdir aspect; other ADR-0002 decisions
  (frontmatter shape, vocabulary, metadata, passthroughs, directives,
  bundle-as-manifest, schema versioning, body conventions) remain in
  force.
- Authoritative spec: [`docs/format-spec.md`](../format-spec.md)
- Epic: [#62](https://github.com/driftsys/upskill/issues/62) — v0.4
  Registry standard
- Agent Skills open standard: <https://agentskills.io>
