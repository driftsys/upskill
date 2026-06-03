# `upskill fmt` formats the source body

**Date:** 2026-06-04
**Status:** Approved (brainstorm), pending implementation plan

## Problem

`upskill fmt` today has a deliberately narrow contract: it reorders YAML
frontmatter keys and preserves the markdown body **byte-for-byte**. Per
[ADR-0004](../../adr/0004-cli-surface.md), body formatting was considered
"dprint's job" and left out of scope. As a result, authored source bodies
can drift out of canonical form, and nothing fixes or flags this — even
though the generation pipeline already runs every emitted body through
dprint (`generate::format::format_markdown`).

We want `upskill fmt` to mean "fully canonicalize the source," body
included, and `upskill lint` to flag a non-canonical body so CI fails on
unformatted source.

## Decisions

1. **Default on (no flag).** `upskill fmt` always formats the body. One
   command, fully canonical source — matching how `cargo fmt` / `prettier`
   behave. No opt-out flag (consistent with the project's "no back-compat
   until 1.0" stance).
2. **Same dprint pass as generation.** Reuse
   `crate::generate::format::format_markdown` (default config,
   `text_wrap: Maintain` — prose is not re-wrapped). This keeps `fmt`
   idempotent: generation already formats bodies with the same function,
   so source body ≈ generated body.
3. **lint checks the body.** `upskill lint` emits a finding when an item's
   body is not dprint-clean. `fmt` is the fixer; `lint` is the checker — a
   symmetric pair, mirroring the project's "format before commit" rule.
4. **Items only.** Body formatting applies to item files (`SKILL.md`,
   `RULE.md`, `AGENT.md`). Bundles (`*.bundle.yaml`) are pure YAML with no
   markdown body (§2.2, ADR-0007) and are unaffected.

## Correctness note: directives survive

Conditional-content directives are HTML comments
(`<!-- @client:X -->` … `<!-- @endclient -->`, see
`generate::directives`). dprint-plugin-markdown leaves HTML comments
untouched, so formatting the source body does not mangle directive
markers. Directive processing happens at generation time on the
(now-formatted) source body; because dprint is idempotent, the
generation pipeline's own format pass is a no-op on already-formatted
bodies.

## Implementation

### `src/fmt.rs` — `canonicalise_item`

After splitting frontmatter/body and reordering the YAML keys, run the
body through the shared formatter before reassembling. Only the body is
formatted; the custom-reordered YAML frontmatter is left exactly as the
reorder produced it (dprint never touches frontmatter).

```rust
let reordered_yaml = reorder_yaml_keys(yaml_str, key_order);
serde_yaml_ng::from_str::<T>(&reordered_yaml)
    .with_context(|| format!("validate frontmatter {}", path.display()))?;

let formatted_body = crate::generate::format::format_markdown(body)
    .with_context(|| format!("format body {}", path.display()))?;

Ok(format!("---\n{reordered_yaml}---\n{formatted_body}"))
```

`format_markdown` is already `pub` under `pub mod generate` /
`pub mod format` — no visibility changes needed.

The existing `format_file` already skips files whose canonical form
equals the on-disk content, so already-formatted files cause no `mtime`
thrash.

### `src/lint.rs` — `check_body_format`

A new body check slotted beside the existing ones in `check_file`:

```rust
check_body_h1(file, body, out);
check_fence_lang(file, body, out);
check_directives(file, body, out);
check_body_format(file, body, out);   // new
```

`check_body_format` compares `body` to `format_markdown(body)`; on
mismatch it pushes a `Finding`:

- `rule_id: "body-format"`
- `severity: Severity::Warning` — matches the cosmetic `body-h1` /
  `fence-lang` checks; `--strict` promotes it to error in CI.
- `message`: `"body is not formatted — run`upskill fmt`"`

Bundles already return an empty body from `parse_kind`, so the check is a
natural no-op for them.

## Docs to update

- **[ADR-0004](../../adr/0004-cli-surface.md)** — remove the "body is
  preserved byte-for-byte / dprint's job" statement; record that `fmt`
  now canonicalizes the body and `lint` checks it via `body-format`.
- **`src/fmt.rs` module doc** — update the "What is preserved" list: the
  body is now formatted, not preserved verbatim. YAML comments,
  frontmatter author formatting, and nested content are still preserved.
- **lint rule table** (`src/lint.rs` header) — add the `body-format`
  (warning, §5) row.
- **[docs/format-spec.md](../../format-spec.md)** and
  **[docs/commands.md](../../commands.md)** — note the new `fmt`/`lint`
  behavior for bodies.

## Tests

- **Replace** `canonicalise_preserves_body_byte_for_byte` (now wrong)
  with `canonicalise_formats_body`: an unformatted body (e.g. `*` bullets
  that should become `-`, collapsed extra blank lines) comes out
  dprint-clean.
- **Add** an `fmt` body idempotence test (format twice → identical).
- **Add** a directive-survival test: a body containing
  `<!-- @client:claude -->` … `<!-- @endclient -->` keeps the markers
  after a format pass.
- **Add** lint tests: a dirty body yields a `body-format` warning; a
  clean body yields none; `--strict` promotes the warning to error.

## Out of scope

- No `--no-body` / `--body` flag (decision 1).
- No change to bundle handling.
- No change to the generation pipeline (it already formats bodies).
