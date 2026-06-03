//! ATDD tests for `doctor`'s orphaned-dependency advisory finding.
//!
//! An item pulled in only as a dependency (`required_by` non-empty) whose
//! every requirer is no longer installed is surfaced as an ADVISORY finding:
//! it does NOT affect the exit code (exactly like `skipped_plugins`). upskill
//! NEVER auto-removes (#196) — doctor just tells the user.

mod common;

use predicates::prelude::*;
use std::fs;
use std::path::Path;

/// Write an item entrypoint of `kind` (SKILL/AGENT) named `name` into its own
/// `<root>/<name>/` directory (so items are NOT co-located and removal won't
/// cascade across them).
fn write_item(root: &Path, entrypoint: &str, name: &str, frontmatter_extra: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nschema: 1\nname: {name}\ndescription: test {name}\n{frontmatter_extra}---\n# {name}\n"
    );
    fs::write(dir.join(entrypoint), body).unwrap();
}

#[test]
fn doctor_flags_orphaned_dependency_after_requirer_removed() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    // Fake git repo so upskill uses project scope (lockfile in cwd).
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg = tmp.path().join("reg");
    // Each item in its OWN folder — not co-located, so removing code-review
    // will not sweep sarif.
    write_item(
        &reg,
        "AGENT.md",
        "code-review",
        "requires:\n  skills: [sarif]\n",
    );
    write_item(&reg, "SKILL.md", "sarif", "");

    // Install: pulls in sarif with required_by: ["agent:code-review"].
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "code-review"])
        .assert()
        .success();

    // Remove only the requirer. sarif stays in the lockfile, now orphaned.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", "code-review"])
        .assert()
        .success();

    // doctor: advisory finding does NOT fail the exit code.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("orphaned")
                .and(predicates::str::contains("sarif"))
                .and(predicates::str::contains("upskill remove sarif")),
        );
}

#[test]
fn doctor_does_not_flag_dependency_while_requirer_installed() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg = tmp.path().join("reg");
    write_item(
        &reg,
        "AGENT.md",
        "code-review",
        "requires:\n  skills: [sarif]\n",
    );
    write_item(&reg, "SKILL.md", "sarif", "");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "code-review"])
        .assert()
        .success();

    // Requirer still installed — sarif is NOT orphaned.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicates::str::contains("orphaned").not());
}
