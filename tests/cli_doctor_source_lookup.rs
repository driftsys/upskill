//! ATDD tests for `doctor`'s local-source item-directory lookup (#208).
//!
//! On the SOURCE side an item lives in its FOLDER directory, not under its
//! consumer-side effective name. The lockfile records that folder as
//! `LockedItem.group` (and, when `--as`-aliased, the original effective name
//! as `source_name`). `doctor` must resolve the source item directory from the
//! folder key — group -> source_name -> name — not from the effective name.
//!
//! Two ways the consumer name can diverge from the source folder:
//!   (a) divergent-named rule/agent: folder `stuff/` holds `RULE.md` whose
//!       `name:` is `security-baseline` (relaxed naming, Slice 1);
//!   (b) `--as` alias: source skill `foo` installed as `bar`.
//! In both cases the pre-fix `ssot_root.join(&entry.name)` lookup misses,
//! producing a false `ItemMissingInSource` orphan and a non-zero exit.

mod common;

use predicates::prelude::*;
use std::fs;

/// Scenario (a): a folder whose name differs from the item's `name:`.
/// Pre-fix `doctor` looks for `<reg>/security-baseline/` (the effective name)
/// instead of `<reg>/stuff/` (the folder) and falsely flags it missing.
#[test]
fn doctor_clean_for_divergent_named_rule() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    // Fake git repo so upskill uses project scope (lockfile in cwd).
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Registry: folder `stuff/` holding a RULE.md named `security-baseline`.
    let reg = tmp.path().join("reg");
    let dir = reg.join("stuff");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("RULE.md"),
        "---\nschema: 1\nname: security-baseline\ndescription: baseline security rules\n---\n# Security baseline\n\nKeep secrets out of source control.\n",
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg"])
        .assert()
        .success();

    // doctor must be clean: no false "not in source" orphan.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("clean")
                .and(predicates::str::contains("not in source").not())
                .and(predicates::str::contains("no recoverable source").not()),
        );
}

/// Scenario (b): an `--as`-aliased item. Source skill `foo` installed as `bar`.
/// Pre-fix `doctor` looks for `<reg>/bar/` (the alias) instead of `<reg>/foo/`
/// and falsely flags it missing.
#[test]
fn doctor_clean_for_aliased_item() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Registry: a skill `foo`.
    let reg = tmp.path().join("reg");
    let dir = reg.join("foo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nschema: 1\nname: foo\ndescription: the foo skill\n---\n# foo\n",
    )
    .unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "--as", "foo=bar"])
        .assert()
        .success();

    // doctor must be clean: no false "not in source" orphan for the alias.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(
            predicates::str::contains("clean")
                .and(predicates::str::contains("not in source").not())
                .and(predicates::str::contains("no recoverable source").not()),
        );
}
