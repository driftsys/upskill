# `upskill fmt` formats the source body — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `upskill fmt` canonicalize the markdown body of item files via the shared dprint pass (in addition to reordering YAML keys), and make `upskill lint` flag a non-canonical body with a new `body-format` warning.

**Architecture:** A single shared helper `fmt::canonical_body(frontmatter, body)` formats the combined `---\n{yaml}---\n{body}` string with `generate::format::format_markdown` (dprint treats the `---…---` block as opaque frontmatter — YAML content, comments, key order, and wrapping are preserved byte-for-byte; only the body is formatted and the seam is normalized to one blank line) and strips the frontmatter prefix back off, returning the canonical body region including its leading blank-line separator. `fmt` uses it to write; `lint` uses it to detect drift — so checker and fixer can never disagree. Bundles are pure YAML with no body and are unaffected.

**Tech Stack:** Rust (edition 2024), `dprint-plugin-markdown` `=0.21.1`, `anyhow`, `serde_yaml_ng`; integration tests with `assert_cmd` + `tempfile`.

**Reference spec:** [docs/superpowers/specs/2026-06-04-fmt-format-body-design.md](../specs/2026-06-04-fmt-format-body-design.md)

---

## Pre-flight (already verified during design — do not re-investigate)

- `generate::assemble` does `body.trim_start_matches('\n')` then `---\n{fm}---\n\n{body}`. Feeding the combined string to `format_markdown` preserves the frontmatter region byte-for-byte and normalizes the seam to exactly one blank line.
- dprint default config: `*` bullets → `-`, collapses multiple blank lines, strips a leading blank line from an isolated body.
- All four fixture item bodies in `tests/fixtures/items/` are **already dprint-clean**, so `lint_clean_fixture_corpus_exits_zero` will keep passing.
- The existing `body-h1` and `fence-lang` ATDD fixtures in `tests/cli_lint.rs` are already dprint-clean, so they will not gain a `body-format` finding.
- `crate::generate::format::format_markdown` is already `pub` — no visibility change needed.

---

## File Structure

- `src/fmt.rs` — add `pub(crate) fn canonical_body`, call it from `canonicalise_item`, update module doc. Unit tests in the same file's `#[cfg(test)] mod tests`.
- `src/lint.rs` — add `fn check_body_format`, wire it into `check_file`, add the rule-table row in the module header. Unit tests in the same file.
- `tests/cli_fmt.rs` — add one ATDD test; update the file header doc comment.
- `tests/cli_lint.rs` — add ATDD tests for the warning and `--strict` promotion.
- `docs/adr/0004-cli-surface.md` — replace the "`fmt` is frontmatter only" section.
- `docs/commands.md` — update the `fmt` description, the `fmt` section prose, and the lint rule table.
- `docs/format-spec.md` — note body canonicalisation in §3.8.

---

## Task 1: `fmt::canonical_body` helper + `canonicalise_item`

**Files:**

- Modify: `src/fmt.rs` (function `canonicalise_item`; add `canonical_body`; replace one unit test)
- Test: `src/fmt.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Replace the now-wrong unit test with a failing one**

In `src/fmt.rs`, find the existing test `canonicalise_preserves_body_byte_for_byte` (it asserts the body is untouched — that contract is being removed) and replace the entire test function with:

```rust
#[test]
fn canonicalise_formats_body() {
    // `*` bullets become `-`, extra blank lines collapse, and the
    // single blank-line separator after the frontmatter is kept.
    let raw = concat!(
        "---\n",
        "schema: 1\n",
        "name: dirty\n",
        "description: body needs formatting.\n",
        "---\n",
        "\n",
        "## Body\n",
        "\n",
        "\n",
        "* one\n",
        "* two\n",
    );
    let out = canonicalise(raw, Path::new("dirty/SKILL.md")).unwrap();
    assert!(
        out.ends_with("---\n\n## Body\n\n- one\n- two\n"),
        "body not formatted canonically:\n{out}"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib fmt::tests::canonicalise_formats_body`
Expected: FAIL — the current `canonicalise_item` returns the body verbatim, so `out` ends with `* one\n* two\n`.

- [ ] **Step 3: Add the `canonical_body` helper**

In `src/fmt.rs`, immediately after the `canonicalise_item` function, add:

```rust
/// Canonical body region for an item file, given its on-disk
/// frontmatter string and body. Formats via dprint the same way the
/// generation pipeline does: dprint treats `---…---` as opaque
/// frontmatter, so the YAML (content, comments, key order, wrapping) is
/// preserved byte-for-byte and only the body is formatted, with the seam
/// normalised to a single blank line. Returns the region after the
/// closing `---`, including that leading blank-line separator. Empty when
/// the body is blank. Used by both `fmt` (to write) and `lint::check_body_format`
/// (to detect drift) so checker and fixer never disagree.
pub(crate) fn canonical_body(frontmatter: &str, body: &str) -> Result<String> {
    if body.trim().is_empty() {
        return Ok(String::new());
    }
    let prefix = format!("---\n{frontmatter}---\n");
    let combined = format!("{prefix}{body}");
    let formatted =
        crate::generate::format::format_markdown(&combined).context("format item body")?;
    formatted
        .strip_prefix(&prefix)
        .map(|region| region.to_string())
        .ok_or_else(|| anyhow!("dprint altered the frontmatter region while formatting the body"))
}
```

- [ ] **Step 4: Call the helper from `canonicalise_item`**

In `src/fmt.rs`, change the body of `canonicalise_item`. Replace this exact block:

```rust
    let reordered_yaml = reorder_yaml_keys(yaml_str, key_order);

    // Validate the reordered YAML parses correctly.
    serde_yaml_ng::from_str::<T>(&reordered_yaml)
        .with_context(|| format!("validate frontmatter {}", path.display()))?;

    Ok(format!("---\n{reordered_yaml}---\n{body}"))
```

with:

```rust
    let reordered_yaml = reorder_yaml_keys(yaml_str, key_order);

    // Validate the reordered YAML parses correctly.
    serde_yaml_ng::from_str::<T>(&reordered_yaml)
        .with_context(|| format!("validate frontmatter {}", path.display()))?;

    let body_region = canonical_body(&reordered_yaml, body)
        .with_context(|| format!("format body {}", path.display()))?;

    Ok(format!("---\n{reordered_yaml}---\n{body_region}"))
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test --lib fmt::tests::canonicalise_formats_body`
Expected: PASS.

- [ ] **Step 6: Run the whole fmt unit test module to catch regressions**

Run: `cargo test --lib fmt::tests`
Expected: PASS. (The existing `canonicalise_is_idempotent` and comment/wrapping tests must still pass — frontmatter handling is unchanged and their bodies are already clean.)

- [ ] **Step 7: Commit**

```bash
git add src/fmt.rs
git commit -m "feat(fmt): format item markdown body via shared canonical_body helper"
```

---

## Task 2: fmt unit tests — seam preservation, idempotence, directive survival

**Files:**

- Test: `src/fmt.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write three failing/guard tests**

In `src/fmt.rs` `mod tests`, after `canonicalise_formats_body`, add:

```rust
    #[test]
    fn canonicalise_preserves_clean_body_seam() {
        // An already-clean body with one blank line after `---` is left
        // exactly as-is — the separator is preserved, not stripped.
        let raw = concat!(
            "---\n",
            "schema: 1\n",
            "name: clean\n",
            "description: already canonical body.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "- one\n",
            "- two\n",
        );
        let out = canonicalise(raw, Path::new("clean/SKILL.md")).unwrap();
        assert_eq!(out, raw, "clean body must round-trip unchanged:\n{out}");
    }

    #[test]
    fn canonicalise_body_is_idempotent() {
        let raw = concat!(
            "---\n",
            "name: dirty\n",
            "schema: 1\n",
            "description: scrambled keys and bullets.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "* one\n",
            "* two\n",
        );
        let path = Path::new("dirty/SKILL.md");
        let pass1 = canonicalise(raw, path).unwrap();
        let pass2 = canonicalise(&pass1, path).unwrap();
        assert_eq!(pass1, pass2, "fmt must be idempotent over the body too");
    }

    #[test]
    fn canonicalise_preserves_directives() {
        let raw = concat!(
            "---\n",
            "schema: 1\n",
            "name: cond\n",
            "description: body has client directives.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "<!-- @client:claude -->\n",
            "Claude-only line.\n",
            "<!-- @endclient -->\n",
        );
        let out = canonicalise(raw, Path::new("cond/SKILL.md")).unwrap();
        assert!(
            out.contains("<!-- @client:claude -->"),
            "open directive lost:\n{out}"
        );
        assert!(
            out.contains("<!-- @endclient -->"),
            "close directive lost:\n{out}"
        );
    }
```

- [ ] **Step 2: Run them**

Run: `cargo test --lib fmt::tests`
Expected: PASS for all three (the helper from Task 1 already produces this behavior; these lock it in).

If `canonicalise_preserves_clean_body_seam` fails because dprint emits a slightly different clean form, fix the test's expected `raw` to match the canonical form rather than changing the implementation — dprint's output is the source of truth.

- [ ] **Step 3: Commit**

```bash
git add src/fmt.rs
git commit -m "test(fmt): cover body seam preservation, idempotence, directive survival"
```

---

## Task 3: fmt ATDD + doc-comment updates

**Files:**

- Test: `tests/cli_fmt.rs` (add one test)
- Modify: `tests/cli_fmt.rs` (header doc comment)
- Modify: `src/fmt.rs` (module doc + `fmt` fn doc)

- [ ] **Step 1: Write the failing ATDD test**

In `tests/cli_fmt.rs`, after the existing `fmt_reorders_frontmatter_keys_canonically` test, add:

```rust
#[test]
fn fmt_formats_markdown_body() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("dirty/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: dirty\n",
            "description: A skill whose body needs formatting.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "\n",
            "* one\n",
            "* two\n",
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();

    let after = fs::read_to_string(&item).unwrap();
    assert!(
        after.ends_with("---\n\n## Body\n\n- one\n- two\n"),
        "body not formatted:\n{after}"
    );
}
```

- [ ] **Step 2: Run it to verify it passes**

Run: `cargo test --test cli_fmt fmt_formats_markdown_body`
Expected: PASS (Task 1 already implemented the behavior; this is the acceptance-level proof through the real CLI).

- [ ] **Step 3: Update the `tests/cli_fmt.rs` header doc comment**

Replace this exact block at the top of `tests/cli_fmt.rs`:

```rust
//! ATDD tests for `upskill fmt`.
//!
//! Canonicalises YAML frontmatter in place. Body content is dprint's job;
//! this command does not touch it. Author command — refuses to run inside
//! a consumer project (`.upskill-lock.json` at the path's root) per
//! ADR-0004.
```

with:

```rust
//! ATDD tests for `upskill fmt`.
//!
//! Canonicalises YAML frontmatter in place and formats the markdown body
//! via the shared dprint pass (same formatter the generation pipeline
//! uses). Author command — refuses to run inside a consumer project
//! (`.upskill-lock.json` at the path's root) per ADR-0004.
```

- [ ] **Step 4: Update the `src/fmt.rs` module doc**

Replace this exact block at the top of `src/fmt.rs`:

```rust
//! `upskill fmt` — canonicalise YAML key order in SSOT files.
//!
//! Per [ADR-0004](../../docs/adr/0004-cli-surface.md), `fmt` and `lint`
//! are sibling author commands. `fmt` operates on YAML frontmatter and
//! bundle files; markdown body content is dprint's job and is preserved
//! byte-for-byte. Like `lint`, this command refuses to run inside a
//! consumer project (`.upskill-lock.json` at the path's root).
//!
//! What gets canonicalised:
//!
//! - **Key order** — fixed by priority tables derived from the
//!   [`crate::model`] struct field order.
//! - Unknown top-level keys (extras) sort alphabetically after all
//!   known keys.
//!
//! What is **preserved**:
//!
//! - YAML comments (both standalone and inline)
//! - Author formatting (indentation, line wrapping)
//! - Nested/indented content (travels with its parent key block)
```

with:

```rust
//! `upskill fmt` — canonicalise YAML key order and markdown body in SSOT files.
//!
//! Per [ADR-0004](../../docs/adr/0004-cli-surface.md), `fmt` and `lint`
//! are sibling author commands. `fmt` reorders YAML frontmatter keys and
//! formats the markdown body through the same dprint pass the generation
//! pipeline uses (see [`canonical_body`]); `lint` reports a `body-format`
//! finding when a body is not canonical. Like `lint`, this command
//! refuses to run inside a consumer project (`.upskill-lock.json` at the
//! path's root).
//!
//! What gets canonicalised:
//!
//! - **Key order** — fixed by priority tables derived from the
//!   [`crate::model`] struct field order.
//! - Unknown top-level keys (extras) sort alphabetically after all
//!   known keys.
//! - **Markdown body** — formatted via dprint; the frontmatter↔body seam
//!   is normalised to a single blank line.
//!
//! What is **preserved**:
//!
//! - YAML comments (both standalone and inline)
//! - Author frontmatter formatting (indentation, line wrapping)
//! - Nested/indented YAML content (travels with its parent key block)
//! - Prose wrapping inside the body (dprint `text_wrap: Maintain`)
//! - HTML-comment directives (`<!-- @client:X -->`)
```

- [ ] **Step 5: Update the `fmt` function doc comment**

In `src/fmt.rs`, find the doc comment on `pub fn fmt` and replace this exact line:

```rust
/// Files whose content was already canonical are left untouched
/// (no `mtime` thrash). Body content is preserved byte-for-byte.
/// Comments and author formatting are preserved.
```

with:

```rust
/// Files whose content was already canonical are left untouched
/// (no `mtime` thrash). The body is formatted via dprint; YAML comments
/// and author frontmatter formatting are preserved.
```

- [ ] **Step 6: Verify the existing key-order ATDD test still passes**

The `fmt_reorders_frontmatter_keys_canonically` test asserts `after.contains("\n\n## Body\n\nUntouched.\n")`. Its body is already clean and uses one blank line after `---`, so it stays valid.

Run: `cargo test --test cli_fmt`
Expected: PASS (all tests).

- [ ] **Step 7: Commit**

```bash
git add tests/cli_fmt.rs src/fmt.rs
git commit -m "test(fmt): ATDD for body formatting; update fmt docs"
```

---

## Task 4: lint `check_body_format`

**Files:**

- Modify: `src/lint.rs` (`check_file`; add `check_body_format`; rule-table header)
- Test: `src/lint.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write failing unit tests**

In `src/lint.rs` `mod tests`, add:

```rust
    #[test]
    fn lint_flags_unformatted_body() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("dirty/SKILL.md");
        write(&item, &skill("dirty", "\n## Body\n\n* one\n* two\n"));
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "body-format")
            .expect("expected a body-format finding");
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn lint_clean_body_has_no_body_format_finding() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("clean/SKILL.md");
        write(&item, &skill("clean", "\n## Body\n\n- one\n- two\n"));
        let report = lint(&[tmp.path().to_path_buf()], false).unwrap();
        assert!(
            report.findings.iter().all(|f| f.rule_id != "body-format"),
            "unexpected body-format finding: {:?}",
            report.findings
        );
    }

    #[test]
    fn lint_strict_promotes_body_format_to_error() {
        let tmp = tempfile::tempdir().unwrap();
        let item = tmp.path().join("dirty/SKILL.md");
        write(&item, &skill("dirty", "\n## Body\n\n* one\n* two\n"));
        let report = lint(&[tmp.path().to_path_buf()], true).unwrap();
        let f = report
            .findings
            .iter()
            .find(|f| f.rule_id == "body-format")
            .expect("expected a body-format finding");
        assert_eq!(f.severity, Severity::Error);
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib lint::tests::lint_flags_unformatted_body lint::tests::lint_strict_promotes_body_format_to_error`
Expected: FAIL — `body-format` does not exist yet, so `.expect(...)` panics.

- [ ] **Step 3: Add the `check_body_format` function**

In `src/lint.rs`, after `check_directives`, add:

```rust
/// Compare the body against its canonical dprint-formatted form. A
/// mismatch means the author has not run `upskill fmt`. Uses the same
/// `fmt::canonical_body` helper that `fmt` writes with, so checker and
/// fixer never disagree. The on-disk frontmatter is passed through dprint
/// as an opaque prefix, so this is independent of frontmatter key order
/// (which `lint` does not check — only `fmt` reorders).
fn check_body_format(file: &Path, frontmatter: &str, body: &str, out: &mut Vec<Finding>) {
    let Ok(canonical) = crate::fmt::canonical_body(frontmatter, body) else {
        // A dprint failure is rare and surfaced by `fmt` itself; don't
        // double-report it here.
        return;
    };
    if canonical != body {
        out.push(Finding {
            rule_id: "body-format",
            severity: Severity::Warning,
            path: file.to_path_buf(),
            line: None,
            message: "body is not formatted — run `upskill fmt`".into(),
        });
    }
}
```

- [ ] **Step 4: Wire it into `check_file`**

In `src/lint.rs`, find this block in `check_file`:

```rust
check_body_h1(file, body, out);
check_fence_lang(file, body, out);
check_directives(file, body, out);
Ok(())
```

and replace it with:

```rust
check_body_h1(file, body, out);
check_fence_lang(file, body, out);
check_directives(file, body, out);
let frontmatter = frontmatter::split(&raw).map(|(fm, _)| fm).unwrap_or("");
check_body_format(file, frontmatter, body, out);
Ok(())
```

(`raw` is the `String` already read at the top of `check_file`; `frontmatter::split` is already imported via `use crate::parse::frontmatter;`. For bundles, `body` is `""` so the check is a no-op regardless of `frontmatter`.)

- [ ] **Step 5: Run the unit tests to verify they pass**

Run: `cargo test --lib lint::tests`
Expected: PASS for the three new tests and all existing ones (existing `body-h1`/`fence-lang` test bodies are already dprint-clean).

- [ ] **Step 6: Add the rule-table row in the module header**

In `src/lint.rs`, find this table in the module doc comment:

```rust
//! | `body-h1`         | warning  | §5.1               |
//! | `fence-lang`      | warning  | §5.2               |
//! | `directive`       | error    | §6.3 (any imbalance, unknown client, nesting) |
//! | `name-collision`  | error    | cross-file (bundle vs item name) |
```

and add a `body-format` row after `fence-lang`:

```rust
//! | `body-h1`         | warning  | §5.1               |
//! | `fence-lang`      | warning  | §5.2               |
//! | `body-format`     | warning  | §3.8 (dprint canonical body) |
//! | `directive`       | error    | §6.3 (any imbalance, unknown client, nesting) |
//! | `name-collision`  | error    | cross-file (bundle vs item name) |
```

- [ ] **Step 7: Commit**

```bash
git add src/lint.rs
git commit -m "feat(lint): add body-format rule using shared canonical_body helper"
```

---

## Task 5: lint ATDD

**Files:**

- Test: `tests/cli_lint.rs` (add two tests)

- [ ] **Step 1: Write the ATDD tests**

In `tests/cli_lint.rs`, after `lint_flags_h1_in_body_as_warning`, add:

```rust
#[test]
fn lint_flags_unformatted_body_as_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("dirty/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: dirty\n",
            "description: Body uses asterisk bullets that dprint rewrites.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "* one\n",
            "* two\n",
        ),
    );

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .success(); // warnings only, exit 0 by default
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("body-format"), "expected body-format finding: {out}");
    assert!(out.contains("warning"), "expected warning level: {out}");
}

#[test]
fn lint_strict_fails_on_unformatted_body() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("dirty/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: dirty\n",
            "description: Body uses asterisk bullets that dprint rewrites.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "* one\n",
            "* two\n",
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .failure(); // --strict promotes the warning to an error → non-zero exit
}
```

- [ ] **Step 2: Run them**

Run: `cargo test --test cli_lint lint_flags_unformatted_body_as_warning lint_strict_fails_on_unformatted_body`
Expected: PASS.

- [ ] **Step 3: Run the whole lint ATDD suite to confirm no regressions**

Run: `cargo test --test cli_lint`
Expected: PASS — including `lint_clean_fixture_corpus_exits_zero` (fixture bodies are already dprint-clean).

- [ ] **Step 4: Commit**

```bash
git add tests/cli_lint.rs
git commit -m "test(lint): ATDD for body-format warning and --strict failure"
```

---

## Task 6: Documentation + fixture corpus guard

**Files:**

- Modify: `docs/adr/0004-cli-surface.md`
- Modify: `docs/commands.md`
- Modify: `docs/format-spec.md`

- [ ] **Step 1: Update ADR-0004**

In `docs/adr/0004-cli-surface.md`, replace this exact block:

```markdown
### `fmt` is frontmatter only

`upskill fmt` canonicalises YAML frontmatter (key ordering, version
quoting, indentation). Markdown body formatting is dprint's job. The two
tools don't overlap.
```

with:

```markdown
### `fmt` canonicalises frontmatter and body

`upskill fmt` canonicalises YAML frontmatter (key ordering, version
quoting, indentation) and formats the markdown body through the same
dprint pass the generation pipeline uses. `upskill lint` reports a
`body-format` warning when a body is not canonical, so the check/fix pair
stays symmetric and CI (`lint --strict`) fails on unformatted source.
(This supersedes the original "frontmatter only" decision: leaving bodies
unchecked let authored source drift out of canonical form.)
```

- [ ] **Step 2: Update `docs/commands.md` — fmt one-liner**

In `docs/commands.md`, replace this exact line:

```markdown
| `fmt` | Author | Canonicalise YAML frontmatter (key order, quoting). |
```

with:

```markdown
| `fmt` | Author | Canonicalise YAML frontmatter and format the markdown body. |
```

- [ ] **Step 3: Update `docs/commands.md` — lint rule table**

In `docs/commands.md`, replace this exact block:

```markdown
| `body-h1` | warning | format-spec §5.1 |
| `fence-lang` | warning | format-spec §5.2 |
| `directive` | error | format-spec §6.3 |
```

with:

```markdown
| `body-h1` | warning | format-spec §5.1 |
| `fence-lang` | warning | format-spec §5.2 |
| `body-format` | warning | format-spec §3.8 |
| `directive` | error | format-spec §6.3 |
```

- [ ] **Step 4: Update `docs/commands.md` — fmt section prose**

In `docs/commands.md`, replace this exact block:

```markdown
Canonicalise YAML frontmatter (key order, indentation, alphabetised
unknown keys). Markdown body formatting is left to dprint — the two
tools don't overlap.
```

with:

```markdown
Canonicalise YAML frontmatter (key order, indentation, alphabetised
unknown keys) and format the markdown body via dprint (the same formatter
the generation pipeline uses). The frontmatter↔body seam is normalised to
a single blank line; YAML comments and prose wrapping are preserved.
```

Then replace this exact line in the same section:

```markdown
Files whose frontmatter is already canonical are left untouched (no
`mtime` thrash).
```

with:

```markdown
Files whose frontmatter and body are already canonical are left untouched
(no `mtime` thrash).
```

- [ ] **Step 5: Update `docs/format-spec.md` §3.8**

In `docs/format-spec.md`, find the `### 3.8 Frontmatter canonicalisation` section and replace this exact paragraph:

```markdown
An implementation MAY provide a formatting command (e.g., `upskill fmt`) that canonicalises SSOT
frontmatter. Canonicalisation produces a deterministic key order, making diffs predictable and
reducing merge conflicts.
```

with:

```markdown
An implementation MAY provide a formatting command (e.g., `upskill fmt`) that canonicalises SSOT
frontmatter and the markdown body. Frontmatter canonicalisation produces a deterministic key
order, making diffs predictable and reducing merge conflicts. Body canonicalisation applies the
same markdown formatter used for generated output (§7.4) and normalises the frontmatter↔body seam
to a single blank line, so authored source matches generated output. A linter MAY report a
`body-format` finding when a body is not canonical.
```

- [ ] **Step 6: Verify fixtures are still clean (guard the corpus assertion)**

Build the binary and dry-run `fmt` against a copy of the fixtures to confirm nothing changes (they were verified clean during design; this guards against drift):

```bash
cargo build --quiet
TMP=$(mktemp -d)
cp -R tests/fixtures/items "$TMP/items"
./target/debug/upskill fmt "$TMP/items"
git --no-pager -C /dev/null diff --no-index tests/fixtures/items "$TMP/items" || true
```

Expected: no differences reported. If any fixture body changed, copy the formatted version back into `tests/fixtures/items/...`, then run `cargo test --test generate_skills --test generate_rules --test generate_agents` and confirm the generation golden tests still pass before committing.

- [ ] **Step 7: Commit**

```bash
git add docs/adr/0004-cli-surface.md docs/commands.md docs/format-spec.md
git commit -m "docs: fmt formats the body; add body-format lint rule"
```

---

## Task 7: Final verification + PR

**Files:** none (verification only)

- [ ] **Step 1: Format everything**

Run: `just fmt`
Expected: success; review any reformatting it applies (commit it if so).

- [ ] **Step 2: Full verification**

Run: `just verify`
Expected: PASS — clippy clean (zero warnings), all tests green, docs tooling clean.

- [ ] **Step 3: Commit any formatting changes**

```bash
git add -A
git commit -m "chore: just fmt" || true
```

- [ ] **Step 4: Push and open the PR**

```bash
git push -u origin feat/fmt-format-body
gh pr create --base main --title "feat: upskill fmt formats the source body" --body "$(cat <<'BODY'
## What

`upskill fmt` now formats the markdown body of item files (SKILL/RULE/AGENT)
through the same dprint pass the generation pipeline uses, in addition to
reordering YAML frontmatter keys. `upskill lint` gains a `body-format`
warning that fires when a body is not canonical (promoted to an error under
`--strict`), so the check/fix pair stays symmetric.

A shared `fmt::canonical_body(frontmatter, body)` helper is the single
source of truth used by both `fmt` (to write) and `lint` (to detect drift).
The frontmatter↔body seam is normalised to one blank line; YAML comments,
key order, frontmatter wrapping, prose wrapping, and HTML-comment
directives are preserved.

Reverses ADR-0004's original "fmt is frontmatter only" decision (recorded
in the ADR).

## Tests

- fmt unit: body formatting, seam preservation, idempotence, directive survival
- fmt ATDD: `upskill fmt` formats a dirty body end-to-end
- lint unit + ATDD: `body-format` warning, clean body has none, `--strict` fails

Spec: docs/superpowers/specs/2026-06-04-fmt-format-body-design.md

🤖 Generated with [Claude Code](https://claude.com/claude-code)
BODY
)"
```

Expected: PR created against `main`. Then fix any CI findings first, per AGENTS.md.
