//! ATDD tests for `upskill lint`.
//!
//! Validates SSOT files against the format spec. Default mode emits
//! warnings and exits 0 unless an error rule fires; `--strict` promotes
//! warnings to errors. Author command (per ADR-0004): refuses to run
//! inside a consumer project (detects `.upskill-lock.json` at the path's
//! root).

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn lint_clean_fixture_corpus_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    for kind in ["skills", "rules", "agents"] {
        let from = format!("{FIXTURES}/{kind}");
        let to = source.join(kind);
        copy_dir_all(Path::new(&from), &to).unwrap();
    }

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(&source)
        .args(["lint"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("0 findings"), "expected clean: {out}");
}

#[test]
fn lint_flags_h1_in_body_as_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let item = tmp.path().join("skills/bad-h1/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: bad-h1\n",
            "description: H1 in body must be flagged.\n",
            "---\n",
            "\n",
            "# Forbidden heading\n",
            "\n",
            "## Body\n",
            "\n",
            "Some text.\n",
        ),
    );

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .success(); // warnings only, exit 0 by default
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("body-h1"), "expected body-h1 finding: {out}");
    assert!(out.contains("warning"), "expected warning level: {out}");
}

#[test]
fn lint_flags_fence_without_language_as_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let item = tmp.path().join("skills/no-fence-lang/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: no-fence-lang\n",
            "description: Fence without lang hint must be flagged.\n",
            "---\n",
            "\n",
            "## Example\n",
            "\n",
            "```\n",
            "echo hi\n",
            "```\n",
        ),
    );

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("fence-lang"),
        "expected fence-lang finding: {out}"
    );
}

#[test]
fn lint_strict_promotes_warnings_to_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let item = tmp.path().join("skills/strict-h1/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: strict-h1\n",
            "description: Body H1 promoted to error in --strict.\n",
            "---\n",
            "\n",
            "# Body H1 here\n",
        ),
    );

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn lint_flags_name_directory_mismatch_as_error() {
    let tmp = tempfile::tempdir().unwrap();
    // Directory named foo, frontmatter name is bar — error per §2.1.
    let item = tmp.path().join("skills/foo/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: bar\n",
            "description: Name mismatch with directory.\n",
            "---\n",
            "\n",
            "## Body\n",
        ),
    );

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn lint_flags_unbalanced_directive_as_error() {
    let tmp = tempfile::tempdir().unwrap();
    let item = tmp.path().join("skills/unbalanced/SKILL.md");
    write(
        &item,
        concat!(
            "---\n",
            "schema: 1\n",
            "name: unbalanced\n",
            "description: Open without close.\n",
            "---\n",
            "\n",
            "## Body\n",
            "\n",
            "<!-- @client:claude -->\n",
            "Never closed.\n",
        ),
    );

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn lint_refuses_to_run_inside_consumer_project() {
    let tmp = tempfile::tempdir().unwrap();
    // Marker file says "this is a consumer project, not a source tree".
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        r#"{"schema":2,"items":[]}"#,
    )
    .unwrap();

    let assert = Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .failure()
        .code(2);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("consumer project") || stderr.contains(".upskill-lock.json"),
        "expected refusal message: {stderr}"
    );
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let to_path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}
