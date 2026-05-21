# Source-registry layout

A **source registry** is the repository where you author SSOT items (rules,
skills, agents) and bundle manifests. The portable format itself
([format-spec §2.1](./format-spec.md#21-item-directory-structure))
intentionally lets you place items under any path — `content/`, `skills-src/`,
or anything else that fits your team.

The upskill project recommends a single concrete layout below. Following it
means:

- `upskill new <kind> <name>` drops items directly where they belong.
- Examples in this book — getting-started, recipes, fixtures — match what
  you see on disk.
- Readers cloning your registry know exactly where to look.

The recommendation is non-normative: the upskill CLI and any other
conforming tool MUST accept registries that deviate.

## Recommended layout

Place item directories and bundle files together under a single top-level
`skills/` directory, flat:

```text
<source-registry-root>/
└── skills/
    ├── platform-baseline.bundle.md     # bundles
    ├── android.bundle.md
    ├── license-awareness/               # rule item
    │   └── RULE.md
    ├── code-review/                     # skill item
    │   └── SKILL.md
    ├── security-reviewer/               # agent item
    │   └── AGENT.md
    └── api-handler/                     # co-located rule + skill
        ├── RULE.md
        └── SKILL.md
```

Why this shape:

- **Single discovery root.** Anyone cloning the registry sees SSOT content
  in one place; no frontmatter scan needed to find items.
- **Flat bundle placement.** Bundles sit next to the items they reference,
  so `requires:` and `items:` lists are visually adjacent to their targets.
- **Matches `upskill new`.** Running `upskill new <kind> <name>` from inside
  `skills/` produces `skills/<name>/<KIND>.md` with no follow-up moves.
- **Holds all three kinds.** Despite the directory name, `skills/` carries
  rules and agents as well. Kind is determined by the entrypoint filename
  ([format-spec §2.1](./format-spec.md#21-item-directory-structure)),
  not by the parent directory.

## When to use a `skills/bundles/` subdirectory

Once a registry holds enough bundles that `*.bundle.md` files start to
drown out the item directories in a single `ls`, move the bundles into a
sibling `bundles/` subdirectory:

```text
<source-registry-root>/
└── skills/
    ├── bundles/
    │   ├── platform-baseline.bundle.md
    │   ├── android.bundle.md
    │   └── rust-embedded.bundle.md
    ├── license-awareness/
    │   └── RULE.md
    ├── code-review/
    │   └── SKILL.md
    └── security-reviewer/
        └── AGENT.md
```

This trades visual adjacency between a bundle and the items it lists for a
clean separation between manifests and content. Rough heuristic: stick with
the flat layout until scanning `skills/` for an item becomes harder than
scanning it for a bundle.

`upskill` discovers bundles by scanning for the `.bundle.md` suffix
([format-spec §2.2](./format-spec.md#22-bundle-files)) and is indifferent
to which of these two sub-layouts you pick.

## What this page does not change

- **Portable format conformance.** `<item-root>` and `<bundle-root>` are
  MAY-level in the format spec. A registry that uses `content/` or
  `skills-src/` is still conforming.
- **Generation output.** Consumer-side paths (`.claude/skills/<name>/...`,
  `.github/skills/...`, `.agents/skills/...`) are unrelated to source-side
  layout. They are specified in
  [format-spec §7](./format-spec.md#7-generation-client-specific-output).
- **Avoid `.agents/` as a source root.** Source registries SHOULD NOT use
  `.agents/` as their item root, to prevent confusion with the
  consumer-side opencode canonical-store path
  ([format-spec §7.3](./format-spec.md#73-opencode)).

## Scaffolding a new item in this layout

```bash
mkdir -p skills && cd skills
upskill new skill code-review     # → skills/code-review/SKILL.md
upskill new rule license-checks   # → skills/license-checks/RULE.md
upskill new agent security-review # → skills/security-review/AGENT.md
```

Bundles are plain `*.bundle.md` files you create alongside the items —
there is no scaffolder for them; copy the
[format-spec §3.7](./format-spec.md#37-bundle-schema) example as a
starting point.
