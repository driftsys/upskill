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

    // Assert on parsed frontmatter keys, not whole-file substrings: the words
    // "requires"/"ignore" are ordinary English and may legitimately appear in
    // body or description prose. Only a leaked *key* is a defect.
    let frontmatter = extract_frontmatter(&contents);
    let map: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(&frontmatter)
        .unwrap_or_else(|e| panic!("frontmatter is not valid YAML mapping: {e}\n{frontmatter}"));
    assert!(
        !map.contains_key(serde_yaml_ng::Value::from("requires")),
        "SSOT-only `requires` key leaked into generated frontmatter:\n{frontmatter}"
    );
    assert!(
        !map.contains_key(serde_yaml_ng::Value::from("ignore")),
        "SSOT-only `ignore` key leaked into generated frontmatter:\n{frontmatter}"
    );
}

/// Extract the YAML frontmatter block — the text between the first `---` fence
/// line and the next `---` line. Generated output always opens with a `---`
/// fence.
fn extract_frontmatter(contents: &str) -> String {
    let mut lines = contents.lines();
    assert_eq!(
        lines.next().map(str::trim_end),
        Some("---"),
        "generated output does not open with a `---` frontmatter fence:\n{contents}"
    );
    let mut block = String::new();
    for line in lines {
        if line.trim_end() == "---" {
            return block;
        }
        block.push_str(line);
        block.push('\n');
    }
    panic!("no closing `---` fence found in generated output:\n{contents}")
}
