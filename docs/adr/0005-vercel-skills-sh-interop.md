# Compatibility with the Vercel/skills.sh ecosystem

**Status**: Proposed (2026-05-01)

## Context

Vercel's `npx skills` CLI ([skills.sh](https://skills.sh)) is the de facto
distribution tool for the open Agent Skills ecosystem. It supports 40+
agents and uses `SKILL.md` as the standard format on the same
[agentskills.io](https://agentskills.io) base spec we adopted in
[ADR-0002](./0002-portable-content-format.md).

Org-internal tooling that competes with this would fragment the
ecosystem and force developers to choose. Interoperating preserves both
worlds: developers get one tool that works with our internal SSOT
content **and** with anything published to skills.sh.

This ADR is one of four child ADRs of [ADR-0001](./0001-v0.2-architectural-reset.md).
It is the cross-cutting strategic alignment decision; the impact lands
in [ADR-0002](./0002-portable-content-format.md) (format compatibility),
[ADR-0003](./0003-generation-pipeline.md) (no behavioural surprises),
and [ADR-0004](./0004-cli-surface.md) (source-format parity).

## Decision

### We participate, we don't compete

upskill interoperates with skills.sh content; it does not replace it.
Where the open standard already covers a concern (skill schema, source
formats, on-disk paths), upskill follows it.

### Source format parity for `add`

`upskill add <source>` accepts the same source forms as `npx skills add`:

- `owner/repo` (GitHub shorthand)
- `owner/repo@ref` (pinned ref)
- Full git URLs (HTTPS, SSH)
- Local paths (`./...`, `../...`, absolute, `~/...`)

Developers running both tools in the same repo see source resolution
behave the same way.

### Generated SKILL.md is agentskills.io-compliant

For skill output:

- `name` and `description` pass through unchanged.
- Agent Skills extended fields (`allowed-tools`, `when_to_use`,
  `disable-model-invocation`, `context`, `agent`, `argument-hint`,
  `paths`) pass through unchanged when present in SSOT.
- No upskill-specific fields appear in generated `SKILL.md`. Schema
  versioning, metadata block, and passthrough blocks are SSOT-side
  concerns; they are stripped or merged at generation time per
  [ADR-0003](./0003-generation-pipeline.md).

Any agentskills.io-compatible client consumes upskill-generated skills
unchanged.

### Discovery convention parity

upskill discovers `RULE.md`, `SKILL.md`, and `AGENT.md` files in the
same well-known directories as skills.sh:

- `.agents/{rules,skills,agents}/<name>/`
- `.claude/{rules,skills,agents}/<name>/` (Claude Code convention)
- `.github/{instructions,skills,agents}/` (Copilot convention)
- `.opencode/agents/<name>.md` (opencode convention)

### Bundle format is additive

The `<name>.bundle.md` manifest is an upskill-specific extension layered
on top of the open standard. It does not replace, redefine, or break
any existing standard field. Tools that don't understand `.bundle.md`
(including `npx skills`) simply ignore them — bundles never appear in
their output, and nothing else degrades.

Bundle authors who want their bundles consumable by skills.sh need a
separate publishing path (skills.sh doesn't currently understand the
manifest format). This is acceptable: bundles are for org curation, not
public distribution.

## Deliberate divergence: copy, not symlink

skills.sh defaults to symlink installation. upskill uses copy
**everywhere**, documented in
[ADR-0003](./0003-generation-pipeline.md). This is the one place we
deliberately diverge from the open ecosystem.

**Reason.** Symlink installation requires Developer Mode +
`core.symlinks=true` on Windows, which can't be guaranteed across the
range of developer machines we support. Copy-only eliminates this class
of support issues at the cost of one cosmetic difference.

**Practical impact.** Cosmetic. Installed content behaves identically.
A user running both `upskill add` and `npx skills add` in the same repo
sees the same content reach the same paths via different mechanisms.
The only observable difference is whether the destination file is a
symlink or a copy — invisible to the consuming agent tool.

**Surfacing.** This divergence is documented in v0.2.0 release notes
and `upskill --help` for `add`/`update`/`remove`.

## Consequences

**Positive.** Developers use one tool. upskill SSOT content reaches any
agentskills.io-compatible client without modification. Org investment
in upskill doesn't trap users into a closed system. The interop story
makes upskill cheap to adopt for teams already using `npx skills`.

**Negative.** Constrains schema evolution: we can't change required
fields in the open standard's frontmatter without breaking interop.
Bundle authors who want public distribution need a separate path.
Maintaining source-format parity means tracking changes to
`npx skills`'s accepted source forms.

## Alternatives considered

**(a) Reimplement Vercel's distribution model, ignore interop.**
Rejected: forces users to choose, fragments ecosystem, doesn't help
skills.sh-distributed content reach our developers.

**(b) Build only on top of skills.sh as a thin layer.** Rejected:
doesn't handle rules or agents, doesn't support SSOT-with-generation,
doesn't integrate with org-internal sources or governance.

**(c) Match skills.sh symlink default.** Rejected: Windows portability
requires guarantees we can't make. The cosmetic divergence is worth the
support-cost reduction.

## References

- Parent ADR: [ADR-0001](./0001-v0.2-architectural-reset.md)
- Sibling ADRs: [ADR-0002](./0002-portable-content-format.md),
  [ADR-0003](./0003-generation-pipeline.md),
  [ADR-0004](./0004-cli-surface.md)
- Agent Skills open standard: <https://agentskills.io>
- Vercel skills CLI: <https://github.com/vercel-labs/skills>
- skills.sh directory: <https://skills.sh>
