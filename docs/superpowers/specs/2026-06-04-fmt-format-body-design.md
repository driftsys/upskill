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

## Correctness note: the frontmatter↔body seam

The body returned by `frontmatter::split` includes the blank-line
separator that follows the closing `---` (e.g. `"\n## Body\n…"`). dprint
**strips** a leading blank line when a body is formatted in isolation, so
naively reassembling `format_markdown(body)` would delete the
conventional separator and churn nearly every file.

Generation already solved this. `generate::assemble` does
`body.trim_start_matches('\n')` and re-inserts **exactly one** blank
line: `---\n{frontmatter}---\n\n{body}`. When the whole string is fed to
`format_markdown`, dprint treats `---…---` as **opaque frontmatter** (YAML
content, comments, wrapping, and key order are all preserved
byte-for-byte) and only formats the body, normalizing the seam to a
single blank line.

The canonical form of an item file is therefore
`---\n{yaml}---\n\n{formatted_body}`. `fmt` must reproduce exactly this,
and `lint` must compare against exactly this — so both go through one
shared helper.

## Implementation

### `src/fmt.rs` — shared `canonical_body` helper

A single, drift-proof helper produces the canonical body region for an
item. It formats the combined `frontmatter + body` string exactly as the
generation pipeline does (frontmatter opaque to dprint), then strips the
frontmatter prefix back off, returning the canonical body region
**including** its leading blank-line separator. Both `fmt` (to write) and
`lint` (to detect drift) call it, so they can never disagree.

```rust
/// Canonical body region for an item file, given its on-disk
/// frontmatter string and body. Formats via dprint the same way the
/// generation pipeline does — frontmatter is opaque to dprint, so the
/// body region is independent of frontmatter key order. Returns the
/// region after the closing `---`, including the single blank-line
/// separator (empty when the body is blank).
pub(crate) fn canonical_body(frontmatter: &str, body: &str) -> Result<String> {
    let prefix = format!("---\n{frontmatter}---\n");
    let combined = format!("{prefix}{body}");
    let formatted = crate::generate::format::format_markdown(&combined)
        .context("format item body")?;
    Ok(formatted
        .strip_prefix(&prefix)
        .unwrap_or(&formatted)
        .to_string())
}
```

`format_markdown` is already `pub` under `pub mod generate` /
`pub mod format` — no visibility changes needed.

### `src/fmt.rs` — `canonicalise_item`

After reordering and validating the YAML, build the body region with the
helper and reassemble:

```rust
let reordered_yaml = reorder_yaml_keys(yaml_str, key_order);
serde_yaml_ng::from_str::<T>(&reordered_yaml)
    .with_context(|| format!("validate frontmatter {}", path.display()))?;

let body_region = canonical_body(&reordered_yaml, body)
    .with_context(|| format!("format body {}", path.display()))?;

Ok(format!("---\n{reordered_yaml}---\n{body_region}"))
```

The existing `format_file` already skips files whose canonical form
equals the on-disk content, so already-formatted files cause no `mtime`
thrash.

### `src/lint.rs` — `check_body_format`

`check_file` already has the raw file content; it splits the frontmatter
once and passes it alongside the body to the new check, slotted beside
the existing body checks:

```rust
check_body_h1(file, body, out);
check_fence_lang(file, body, out);
check_directives(file, body, out);
check_body_format(file, frontmatter, body, out);   // new
```

`check_body_format` compares `body` to
`crate::fmt::canonical_body(frontmatter, body)`; on mismatch it pushes a
`Finding`:

- `rule_id: "body-format"`
- `severity: Severity::Warning` — matches the cosmetic `body-h1` /
  `fence-lang` checks; `--strict` promotes it to error in CI.
- message: "body is not formatted — run `upskill fmt`"

Bundles already return an empty body from `parse_kind` (and pass an empty
frontmatter), so the check is a natural no-op for them.

Because the helper passes the on-disk frontmatter through dprint as an
opaque prefix and strips it back, the body comparison is independent of
whether the author also got the key order right — key order stays
unchecked by `lint` (only `fmt` reorders), exactly as today.

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
- **Add** a seam-preservation test: an item whose body is already clean
  with a single blank line after `---` is left **unchanged** (the
  separator is preserved, not stripped), and `report.files_changed` is
  empty.
- **Add** an `fmt` body idempotence test (format twice → identical),
  covering both a dirty and an already-clean body.
- **Add** a directive-survival test: a body containing
  `<!-- @client:claude -->` … `<!-- @endclient -->` keeps the markers
  after a format pass.
- **Add** lint tests: a dirty body yields exactly one `body-format`
  warning; a clean body yields none; a bundle yields none; `--strict`
  promotes the warning to error.
- **Guard the fixture corpus:** the existing
  `lint_clean_fixture_corpus_exits_zero` test asserts "0 findings", so
  `tests/fixtures/items` bodies must be dprint-clean. Run `upskill fmt`
  over the fixtures (or hand-fix) and confirm the generation golden tests
  (`generate_*`) still pass before relying on that assertion.

## Out of scope

- No `--no-body` / `--body` flag (decision 1).
- No change to bundle handling.
- No change to the generation pipeline (it already formats bodies).
