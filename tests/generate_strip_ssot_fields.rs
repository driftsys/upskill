//! ATDD: SSOT-only frontmatter fields (`requires`, `ignore`) MUST NOT
//! appear in generated per-client output.
//!
//! Skills emit their `extra` pass-through map verbatim into output, so a
//! field that lands in `extra` leaks. Making `requires`/`ignore` typed
//! fields routes them away from `extra`; this test proves the strip.

mod common;

use std::fs;

#[test]
fn generated_skill_output_omits_requires_and_ignore() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    // Mark `target` as a project directory so auto-fallback picks project
    // scope (cwd) instead of global ($HOME).
    fs::create_dir_all(target.join(".git")).unwrap();

    // Skill that declares both SSOT-only fields. References `other` so the
    // on-disk form is realistic; resolution does not block install today.
    let code_review_dir = source.join("code-review");
    fs::create_dir_all(&code_review_dir).unwrap();
    fs::write(
        code_review_dir.join("SKILL.md"),
        concat!(
            "---\n",
            "schema: 1\n",
            "name: code-review\n",
            "description: Use when reviewing changes for correctness\n",
            "requires:\n",
            "  skills:\n",
            "    - other\n",
            "ignore:\n",
            "  - \"scripts/**\"\n",
            "---\n",
            "\n",
            "Review the diff carefully.\n",
        ),
    )
    .unwrap();

    // The required `other` skill so the source is well-formed.
    let other_dir = source.join("other");
    fs::create_dir_all(&other_dir).unwrap();
    fs::write(
        other_dir.join("SKILL.md"),
        concat!(
            "---\n",
            "schema: 1\n",
            "name: other\n",
            "description: Use when a dependency target is needed\n",
            "---\n",
            "\n",
            "Helper skill body.\n",
        ),
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    let generated = target.join(".claude/skills/code-review/SKILL.md");
    assert!(
        generated.exists(),
        "expected generated skill at {}",
        generated.display()
    );
    let contents = fs::read_to_string(&generated).unwrap();
    assert!(
        !contents.contains("requires"),
        "SSOT-only `requires` leaked into generated output:\n{contents}"
    );
    assert!(
        !contents.contains("ignore"),
        "SSOT-only `ignore` leaked into generated output:\n{contents}"
    );
}
