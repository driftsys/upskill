//! ATDD tests for `upskill lint`.
//!
//! Validates SSOT files against the format spec. Default mode emits
//! warnings and exits 0 unless an error rule fires; `--strict` promotes
//! warnings to errors. Author command (per ADR-0004): refuses to run
//! inside a consumer project (detects `.upskill-lock.json` at the path's
//! root).

mod common;

use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

const UNFORMATTED_SKILL_MD: &str = concat!(
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
);

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn lint_clean_fixture_corpus_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), &source).unwrap();

    let assert = common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("bad-h1/SKILL.md");
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

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .success(); // warnings only, exit 0 by default
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("body-h1"), "expected body-h1 finding: {out}");
    assert!(out.contains("warning"), "expected warning level: {out}");
}

#[test]
fn lint_flags_unformatted_body_as_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("dirty/SKILL.md");
    write(&item, UNFORMATTED_SKILL_MD);

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .success(); // warnings only, exit 0 by default
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("body-format"),
        "expected body-format finding: {out}"
    );
    assert!(out.contains("warning"), "expected warning level: {out}");
}

#[test]
fn lint_strict_fails_on_unformatted_body() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("dirty/SKILL.md");
    write(&item, UNFORMATTED_SKILL_MD);

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .failure(); // --strict promotes the warning to an error → non-zero exit
}

#[test]
fn lint_flags_fence_without_language_as_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("no-fence-lang/SKILL.md");
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

    let assert = common::upskill_cmd(&home)
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
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("strict-h1/SKILL.md");
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

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn lint_flags_name_directory_mismatch_as_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    // Directory named foo, frontmatter name is bar — error per §2.1.
    let item = tmp.path().join("foo/SKILL.md");
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

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn lint_flags_unbalanced_directive_as_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let item = tmp.path().join("unbalanced/SKILL.md");
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

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn lint_refuses_to_run_inside_consumer_project() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    // Marker file says "this is a consumer project, not a source tree".
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        r#"{"schema":2,"items":[]}"#,
    )
    .unwrap();

    let assert = common::upskill_cmd(&home)
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

#[test]
fn lint_flags_bundle_item_name_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let root = tmp.path();

    // Create a skill named "baseline"
    let skill_dir = root.join("baseline");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nschema: 1\nname: baseline\ndescription: a skill\n---\n# body\n",
    )
    .unwrap();

    // Create a bundle also named "baseline"
    let bundles_dir = root.join("bundles");
    fs::create_dir_all(&bundles_dir).unwrap();
    fs::write(
        bundles_dir.join("baseline.bundle.yaml"),
        "schema: 1\nname: baseline\ndescription: a bundle\nitems:\n  skills:\n    - baseline\n",
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(root)
        .args(["lint"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("name-collision"))
        .stdout(predicates::str::contains("baseline"));
}

#[test]
fn lint_excludes_a_command_only_directory_from_name_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("standup/COMMAND.md"),
        "---\nschema: 1\nname: standup\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    write(
        &tmp.path().join("standup.bundle.yaml"),
        "schema: 1\nname: standup\ndescription: a bundle\nitems:\n  commands:\n    - standup\n",
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .success()
        .stdout(predicates::str::contains("name-collision").not())
        .stdout(predicates::str::contains("0 findings"));
}

/// Counterpart of the test above: a skill directory that has a bundle's name
/// is still reported, so the exclusion is limited to command items.
#[test]
fn lint_still_reports_a_skill_directory_that_collides_with_a_bundle_next_to_a_command() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("standup/COMMAND.md"),
        "---\nschema: 1\nname: standup\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    write(
        &tmp.path().join("standup.bundle.yaml"),
        "schema: 1\nname: standup\ndescription: a bundle\nitems:\n  commands:\n    - standup\n",
    );
    write(
        &tmp.path().join("baseline/SKILL.md"),
        "---\nschema: 1\nname: baseline\ndescription: A skill\n---\n\nBody.\n",
    );
    write(
        &tmp.path().join("baseline.bundle.yaml"),
        "schema: 1\nname: baseline\ndescription: a bundle\nitems:\n  skills:\n    - baseline\n",
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("name-collision"))
        .stdout(predicates::str::contains("baseline"))
        .stdout(predicates::str::contains("standup.bundle.yaml").not());
}

#[test]
fn lint_accepts_items_without_schema_field() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let root = tmp.path().join("source");
    for (dir, entrypoint) in [
        ("plain-skill", "SKILL.md"),
        ("plain-rule", "RULE.md"),
        ("plain-agent", "AGENT.md"),
    ] {
        write(
            &root.join(dir).join(entrypoint),
            "---\ndescription: An item without schema.\n---\n\nBody.\n",
        );
    }

    let assert = common::upskill_cmd(&home)
        .current_dir(&root)
        .args(["lint", "--strict"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("3 file(s) checked, 0 findings"),
        "expected three clean files: {out}"
    );
}

#[test]
fn lint_rejects_bundle_without_schema_field() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let root = tmp.path().join("source");
    write(
        &root.join("bundles/meta.bundle.yaml"),
        "name: meta\ndescription: d\nitems: {}\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&root)
        .args(["lint"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("missing field `schema`"));
}

/// #539: a `SKILL.claude.md` file is an ordinary resource; `lint` reports
/// no finding for it.
#[test]
fn lint_accepts_client_named_body_file_without_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let root = tmp.path().join("source");
    write(
        &root.join("demo/SKILL.md"),
        "---\nschema: 1\nname: demo\ndescription: A demo skill.\n---\n\nBody.\n",
    );
    write(&root.join("demo/SKILL.claude.md"), "Claude-named file.\n");

    let assert = common::upskill_cmd(&home)
        .current_dir(&root)
        .args(["lint", "--strict"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("1 file(s) checked, 0 findings"),
        "expected one clean file: {out}"
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

#[test]
fn lint_accepts_a_command_with_argument_hint() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("release-notes/COMMAND.md"),
        "---\nschema: 1\nname: release-notes\ndescription: Write the release notes\nargument-hint: \"[version]\"\n---\n\n## Steps\n\nWrite the notes for $ARGUMENTS.\n",
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("1 file(s)"));
}

#[test]
fn lint_flags_a_command_without_name_as_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("release-notes/COMMAND.md"),
        "---\nschema: 1\ndescription: Write the release notes\n---\n\nWrite the notes.\n",
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "."])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("[frontmatter]"))
        .stdout(predicates::str::contains("missing field `name`"));
}

#[test]
fn lint_flags_a_command_name_that_differs_from_its_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("foo/COMMAND.md"),
        "---\nschema: 1\nname: bar\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "."])
        .assert()
        .code(1)
        .stdout(predicates::str::contains(
            "command `name: bar` must equal directory `foo`",
        ));
}

#[test]
fn lint_flags_a_command_name_mismatch_from_a_bare_relative_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("foo/COMMAND.md"),
        "---\nschema: 1\nname: bar\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    for arg in ["COMMAND.md", "."] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path().join("foo"))
            .args(["lint", "--strict", arg])
            .assert()
            .code(1)
            .stdout(predicates::str::contains(
                "command `name: bar` must equal directory `foo`",
            ));
    }
}

#[test]
fn lint_flags_a_skill_name_mismatch_from_a_bare_relative_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("foo/SKILL.md"),
        "---\nschema: 1\nname: bar\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    for arg in ["SKILL.md", "."] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path().join("foo"))
            .args(["lint", "--strict", arg])
            .assert()
            .code(1)
            .stdout(predicates::str::contains(
                "skill `name: bar` must equal directory `foo`",
            ));
    }
}

#[test]
fn lint_flags_a_skill_name_mismatch_through_a_parent_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("foo/SKILL.md"),
        "---\nschema: 1\nname: bar\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    fs::create_dir_all(tmp.path().join("foo/sub")).unwrap();
    for arg in ["..", "../SKILL.md"] {
        common::upskill_cmd(&home)
            .current_dir(tmp.path().join("foo/sub"))
            .args(["lint", "--strict", arg])
            .assert()
            .code(1)
            .stdout(predicates::str::contains(
                "skill `name: bar` must equal directory `foo`",
            ));
    }
}

#[test]
fn lint_flags_a_command_name_mismatch_through_a_parent_path() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("foo/COMMAND.md"),
        "---\nschema: 1\nname: bar\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    fs::create_dir_all(tmp.path().join("foo/sub")).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path().join("foo/sub"))
        .args(["lint", "--strict", "../COMMAND.md"])
        .assert()
        .code(1)
        .stdout(predicates::str::contains(
            "command `name: bar` must equal directory `foo`",
        ));
}

/// A symbolic link to a skill directory is compared by the name that the
/// path has as written, not by the name of the link target.
#[cfg(unix)]
#[test]
fn lint_compares_a_skill_through_a_symlinked_directory_by_the_link_name() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("store/target/SKILL.md"),
        "---\nschema: 1\nname: linked\ndescription: Write the notes\n---\n\nWrite the notes.\n",
    );
    fs::create_dir_all(tmp.path().join("skills")).unwrap();
    std::os::unix::fs::symlink(
        tmp.path().join("store/target"),
        tmp.path().join("skills/linked"),
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "skills/linked/SKILL.md"])
        .assert()
        .success()
        .stdout(predicates::str::contains("must equal directory").not());
    // The same directory under another link name does not match `linked`.
    std::os::unix::fs::symlink(
        tmp.path().join("store/target"),
        tmp.path().join("skills/other"),
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "skills/other/SKILL.md"])
        .assert()
        .code(1)
        .stdout(predicates::str::contains(
            "skill `name: linked` must equal directory `other`",
        ));
}

#[test]
fn lint_flags_a_command_and_a_skill_in_one_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    for entry in ["COMMAND.md", "SKILL.md"] {
        write(
            &tmp.path().join("both").join(entry),
            "---\nschema: 1\nname: both\ndescription: Write the notes\n---\n\nWrite the notes.\n",
        );
    }
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "."])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("[command-with-skill]"))
        .stdout(predicates::str::contains(
            "a COMMAND.md and a SKILL.md cannot share the directory `both`",
        ));
    // The finding is an error: it fails the run without `--strict` too.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "."])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("error"))
        .stdout(predicates::str::contains("[command-with-skill]"));
}

#[test]
fn lint_flags_an_unknown_command_field_as_error() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write(
        &tmp.path().join("release-notes/COMMAND.md"),
        "---\nschema: 1\nname: release-notes\ndescription: Write the notes\naudience: [claude]\n---\n\nWrite the notes.\n",
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", "."])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("unknown field `audience`"));
}
