# Design: Comment-Preserving `upskill fmt`

**Issue:** [#169](https://github.com/driftsys/upskill/issues/169)
**Date:** 2026-05-23
**Status:** Approved

## Problem

`upskill fmt` canonicalises YAML by round-tripping through serde
(`from_str → struct → to_string`). This silently destroys:

1. **Comments** in `.bundle.yaml` files and frontmatter
2. **Author formatting** — especially hand-wrapped `description` lines
   at 80 chars

The silent stripping causes empty commits in pre-commit hooks and data
loss without warning.

## Solution

Replace the serde round-trip with **line-level block reordering**:

1. Split the YAML into top-level key blocks (each key + preceding
   comments + indented children)
2. Reorder blocks by canonical struct field order
3. Validate the result by parsing through serde (safety net — no write)
4. Write only if the reordered text differs from the original

This preserves comments, inline formatting, and author wrapping while
still enforcing canonical key order.

## Canonical Key Orders

Derived from the Rust model struct field declarations:

**Bundle** (`schema → name → description → license → items → requires →
plugins → metadata → extras`):

```rust
const BUNDLE_KEY_ORDER: &[&str] = &[
    "schema", "name", "description", "license",
    "items", "requires", "plugins", "metadata",
];
```

**Skill** (`schema → name → description → audience → license →
metadata → claude → copilot → opencode → extras`):

```rust
const SKILL_KEY_ORDER: &[&str] = &[
    "schema", "name", "description", "audience", "license",
    "metadata", "claude", "copilot", "opencode",
];
```

**Rule** (`schema → name → description → audience → license → scope →
metadata → claude → copilot → opencode → extras`):

```rust
const RULE_KEY_ORDER: &[&str] = &[
    "schema", "name", "description", "audience", "license", "scope",
    "metadata", "claude", "copilot", "opencode",
];
```

**Agent** (`schema → name → description → audience → license → mode →
model → tools → preload-skills → metadata → claude → copilot →
opencode → extras`):

```rust
const AGENT_KEY_ORDER: &[&str] = &[
    "schema", "name", "description", "audience", "license",
    "mode", "model", "tools", "preload-skills",
    "metadata", "claude", "copilot", "opencode",
];
```

Keys not in the priority list (extras from `#[serde(flatten)]`) sort
alphabetically after all known keys.

## Algorithm: `reorder_yaml_keys`

```text
Input: raw YAML string (bundle file or extracted frontmatter)
Output: reordered YAML string with comments/formatting preserved

1. Split input into lines
2. Group lines into "blocks":
   - A block starts at a line matching `^[a-zA-Z_][a-zA-Z0-9_-]*:` (top-level key)
   - Preceding comment lines (`^#` or `^\s*$` between blocks) attach
     to the NEXT key block
   - Indented lines and inline content belong to the current block
3. Assign each block a sort key:
   - Known keys → index in the priority array
   - Unknown keys → (priority_len + alphabetical position)
4. Stable-sort blocks by sort key
5. Emit blocks in new order, joined by the existing inter-block spacing
6. Validate: parse result with serde into typed struct
   - If validation fails → return original unchanged + emit warning
```

**Edge cases:**

- **Preamble comments** (before the first key): stay at the top
- **Trailing comments** (after the last block): stay at the bottom
- **Blank lines between blocks**: preserved within each block; one
  blank line between blocks in output (normalize inter-block spacing)
- **File already in canonical order**: no change (idempotent)

## Frontmatter Handling

For items (SKILL.md, RULE.md, AGENT.md):

1. Split on `---` fences (existing `frontmatter::split`)
2. Apply `reorder_yaml_keys` to the extracted YAML string
3. Reassemble `---\n{reordered}\n---\n{body}`
4. Body preserved byte-for-byte (unchanged)

## dprint YAML Plugin

Add `dprint-plugin-yaml` to the external `dprint.json` config for
cosmetic YAML normalization (indentation, trailing whitespace) on
`.bundle.yaml` files. dprint preserves comments.

```json
{
  "yaml": {},
  "includes": ["**/*.md", "**/*.bundle.yaml"],
  "plugins": [
    "https://plugins.dprint.dev/markdown-0.17.8.wasm",
    "https://plugins.dprint.dev/g-plane/yaml-0.12.0.wasm"
  ]
}
```

`just fmt` runs `dprint fmt` (handles both Markdown and YAML) then
`upskill fmt` (handles key ordering). Order matters: dprint normalizes
cosmetics first, then upskill reorders keys.

## What Changes

| File                  | Change                                                                   |
| --------------------- | ------------------------------------------------------------------------ |
| `src/fmt.rs`          | Replace `roundtrip()` and bundle serde with `reorder_yaml_keys()`        |
| `src/fmt.rs`          | Add `reorder_yaml_keys(raw, key_order) -> String`                        |
| `src/fmt.rs`          | Keep serde parse as validation-only (don't serialize back)               |
| `dprint.json`         | Add yaml plugin + `*.bundle.yaml` include                                |
| `docs/format-spec.md` | Document canonicalisation policy: key order enforced, comments preserved |

## Testing

### Unit tests (in `fmt.rs`)

- `reorder_preserves_comments`: input with comments → comments survive
- `reorder_preserves_inline_comments`: `key: value # comment` intact
- `reorder_moves_blocks_to_canonical_order`: scrambled keys → correct
- `reorder_preserves_indented_children`: nested content travels with key
- `reorder_unknown_keys_sort_alphabetically_at_end`: extras
- `reorder_is_idempotent`: already-canonical stays unchanged
- `reorder_preamble_stays_at_top`: comments before first key
- `reorder_with_multiline_values`: literal/folded blocks preserved
- `reorder_validates_with_serde`: invalid YAML after reorder → skip

### Integration tests (existing `cli_fmt.rs`)

- `fmt_preserves_bundle_comments`: bundle with comments → no stripping
- `fmt_preserves_frontmatter_wrapping`: 80-char wrapped description
  survives

## Non-Goals

- Sub-key (nested) reordering — only top-level keys are reordered
- Comment insertion/creation — we only preserve existing comments
- Changing dprint's behavior for Markdown (stays as-is)

## Risks

- **Low:** Literal/folded block scalars with unindented continuation
  → mitigated by serde validation after reorder
- **Low:** Unusual YAML constructs (anchors, tags) → our format-spec
  doesn't use them; serde validation catches breakage
