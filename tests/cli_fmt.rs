//! ATDD tests for `upskill fmt`.
//!
//! Canonicalises YAML frontmatter in place. Body content is dprint's job;
//! this command does not touch it. Author command — refuses to run inside
//! a consumer project (`.upskill-lock.json` at the path's root) per
//! ADR-0004.

mod common;

use std::fs;
use std::path::Path;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn fmt_reorders_frontmatter_keys_canonically() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("scrambled/SKILL.md");
    // Canonical order is schema → name → description → audience → license →
    // metadata. Author wrote them in the wrong order.
    write(
        &item,
        concat!(
            "---\n",
            "name: scrambled\n",
            "license: proprietary\n",
            "schema: 1\n",
            "description: A skill written with shuffled keys.\n",
            "metadata:\n",
            "  version: \"1.0.0\"\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "Untouched.\n",
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();

    let after = fs::read_to_string(&item).unwrap();
    let yaml_end = after[4..].find("---").expect("frontmatter still ends");
    let yaml = &after[4..4 + yaml_end];
    let schema = yaml.find("schema:").unwrap();
    let name = yaml.find("name:").unwrap();
    let description = yaml.find("description:").unwrap();
    let license = yaml.find("license:").unwrap();
    let metadata = yaml.find("metadata:").unwrap();
    assert!(
        schema < name && name < description && description < license && license < metadata,
        "keys not in canonical order:\n{yaml}"
    );

    // Body must be byte-for-byte preserved.
    assert!(after.contains("\n\n## Body\n\nUntouched.\n"), "{after}");
}

#[test]
fn fmt_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("already-canonical/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: already-canonical\n",
            "description: Already in canonical key order.\n",
            "license: proprietary\n",
            "metadata:\n",
            "  version: \"1.0.0\"\n",
            "---\n",
            "\n",
            "## Body\n",
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();
    let pass1 = fs::read_to_string(&item).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();
    let pass2 = fs::read_to_string(&item).unwrap();

    assert_eq!(pass1, pass2, "fmt must be idempotent");
}

#[test]
fn fmt_does_not_touch_body() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("preserve-body/SKILL.md");
    let body = concat!(
        "\n",
        "## A heading\n",
        "\n",
        "Some prose with *italic* and **bold**.\n",
        "\n",
        "```rust\n",
        "fn main() {}\n",
        "```\n",
        "\n",
        "<!-- a comment -->\n",
    );
    write(
        &item,
        &format!(
            "---\nschema: 1\nname: preserve-body\ndescription: Body must be preserved verbatim.\n---\n{body}"
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();

    let after = fs::read_to_string(&item).unwrap();
    assert!(
        after.ends_with(body),
        "body changed:\n{after}\n\nexpected suffix:\n{body}"
    );
}

#[test]
fn fmt_refuses_to_run_inside_consumer_project() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        r#"{"schema":2,"items":[]}"#,
    )
    .unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("consumer project") || stderr.contains(".upskill-lock.json"),
        "expected refusal: {stderr}"
    );
}

#[test]
fn fmt_reports_files_changed_count() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("needs-fmt/SKILL.md"),
        concat!(
            "---\n",
            "name: needs-fmt\n",
            "schema: 1\n",
            "description: Out-of-order frontmatter.\n",
            "---\n",
            "## body\n",
        ),
    );
    write(
        &tmp.path().join("clean/SKILL.md"),
        concat!(
            "---\n",
            "schema: 1\n",
            "name: clean\n",
            "description: Already canonical.\n",
            "---\n",
            "## body\n",
        ),
    );

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["fmt"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("1 file") && out.contains("changed"),
        "expected change count: {out}"
    );
}
