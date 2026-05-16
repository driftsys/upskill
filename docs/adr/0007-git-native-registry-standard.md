# Git-native registry standard

**Status**: Accepted (2026-05-16)

## Context

v0.4 (#62) introduces custom registries. Issue #63 originally proposed a
CI-generated `.well-known/agent-skills/index.json` on GitHub/GitLab
Pages. Review (see the
[design spec](../superpowers/specs/2026-05-16-git-native-registry-standard-design.md))
rejected it: `.well-known` is RFC-8615-invalid under a project Pages
path, the index was skills-only, the layout predated ADR-0006 (flat
items), and Pages for private repos needs a paid tier.

## Decision

A registry is a git repo (public or private) holding multi-kind items
(flat layout, ADR-0006) plus bundles. The author runs
`upskill registry build`, which scans the tree and writes a committed
`.upskill-registry.json` (`schema: 1`, mirroring `.upskill-lock.json`).
CI verifies freshness with `upskill registry build --check` (the
`cargo fmt --check` / lockfile pattern). Consumers fetch that one file
via the git provider's raw/contents API; when absent, discovery falls
back to a live tree scan (a later story).

Registry identity lives in an authored `REGISTRY.md` (uppercase
entrypoint + YAML frontmatter, like `RULE.md`); `registry build` lifts
its frontmatter into the manifest header so identity has one source of
truth. The consumer-side named-registry map is `.upskill-registries.yaml`
(YAML, not TOML — no `toml` crate dependency; not `.upskill/…` — that
directory was dropped pre-1.0 (issue #110 / PR #120)).

No Pages, no `.well-known`. Publishing `npx skills`-compatible content
from a registry is explicitly future work.

## Consequences

Private registries work via existing token auth, no paid tier. One
predictable fetch path. A generated artifact must stay fresh, guarded by
CI exactly as the lockfile is. The manifest schema is owned by upskill
and versioned (`schema: 1`).

## Alternatives considered

**(a) CI-generated `.well-known/agent-skills/index.json` on Pages
(original #63).** Rejected: RFC-8615-invalid under a project Pages path,
skills-only, predated the flat-item layout, and private repos need a
paid Pages tier.

**(b) Live tree scan on every consumer fetch, no committed manifest.**
Rejected as the primary mechanism: N provider API calls or a full clone
per discovery, and unauthenticated GitHub is rate-limited (60 req/h).
Retained only as the fallback when no manifest is present.

## References

- Design spec:
  [2026-05-16-git-native-registry-standard-design.md](../superpowers/specs/2026-05-16-git-native-registry-standard-design.md)
- Epic #62, story #63
- Sibling: [ADR-0006](./0006-flat-item-layout.md) (flat item layout),
  [ADR-0003](./0003-generation-pipeline.md) (lockfile pattern)
