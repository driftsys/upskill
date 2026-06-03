//! ATDD for co-location-aware `upskill remove` (Task 6).
//!
//! A single SSOT source folder may hold more than one item — e.g. a
//! `SKILL.md` plus a `RULE.md`. After Task 4 relaxed naming, those
//! co-located items may carry DIFFERENT effective names (the skill takes
//! the folder name; the rule takes its frontmatter `name:`). They are
//! nonetheless one logical unit and must travel together: removing any
//! named member removes the whole `(source, folder)` group.

mod common;

use std::fs;
use std::path::Path;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

/// Read the lockfile item count (`items` array length); 0 when absent.
fn lockfile_item_count(target: &Path) -> usize {
    let path = target.join(".upskill-lock.json");
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v.get("items")
        .and_then(|i| i.as_array())
        .map_or(0, Vec::len)
}

#[test]
fn remove_named_member_removes_the_whole_colocated_unit() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("reg");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // One folder, two co-located items with DIFFERENT effective names:
    //   skill effective name -> folder = "markspec-trace"
    //   rule effective name  -> frontmatter "markspec-trace-syntax"
    write(
        &source.join("markspec-trace/SKILL.md"),
        "---\nschema: 1\nname: markspec-trace\ndescription: trace skill for markspec.\n---\n\n## Body\n\nText.\n",
    );
    write(
        &source.join("markspec-trace/RULE.md"),
        "---\nschema: 1\nname: markspec-trace-syntax\ndescription: trace syntax rule for markspec.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    // Both items installed.
    assert!(
        target
            .join(".claude/skills/markspec-trace/SKILL.md")
            .exists(),
        "skill should be installed"
    );
    assert!(
        target
            .join(".claude/rules/markspec-trace-syntax.md")
            .exists(),
        "co-located rule should be installed"
    );
    assert_eq!(
        lockfile_item_count(&target),
        2,
        "lockfile should record both items"
    );

    // Removing the SKILL by name must also remove the co-located RULE.
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["remove", "markspec-trace"])
        .assert()
        .success();

    assert!(
        !target
            .join(".claude/skills/markspec-trace/SKILL.md")
            .exists(),
        "named skill must be gone"
    );
    assert!(
        !target
            .join(".claude/rules/markspec-trace-syntax.md")
            .exists(),
        "co-located rule must be gone too (the whole unit travels together)"
    );
    assert_eq!(
        lockfile_item_count(&target),
        0,
        "lockfile must have zero items after removing the unit"
    );
}

#[test]
fn remove_by_divergent_rule_name_removes_whole_unit() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("reg");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // One folder, two co-located items with DIFFERENT effective names:
    //   skill effective name -> folder = "markspec-trace"
    //   rule effective name  -> frontmatter "markspec-trace-syntax"
    write(
        &source.join("markspec-trace/SKILL.md"),
        "---\nschema: 1\nname: markspec-trace\ndescription: trace skill for markspec.\n---\n\n## Body\n\nText.\n",
    );
    write(
        &source.join("markspec-trace/RULE.md"),
        "---\nschema: 1\nname: markspec-trace-syntax\ndescription: trace syntax rule for markspec.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    // Both items installed.
    assert!(
        target
            .join(".claude/skills/markspec-trace/SKILL.md")
            .exists(),
        "skill should be installed"
    );
    assert!(
        target
            .join(".claude/rules/markspec-trace-syntax.md")
            .exists(),
        "co-located rule should be installed"
    );
    assert_eq!(
        lockfile_item_count(&target),
        2,
        "lockfile should record both items"
    );

    // Removing by the RULE's divergent name (NOT the folder/skill name)
    // must also remove the co-located SKILL: removal-by-unit is symmetric.
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["remove", "markspec-trace-syntax"])
        .assert()
        .success();

    assert!(
        !target
            .join(".claude/skills/markspec-trace/SKILL.md")
            .exists(),
        "co-located skill must be gone too (the whole unit travels together)"
    );
    assert!(
        !target
            .join(".claude/rules/markspec-trace-syntax.md")
            .exists(),
        "named rule must be gone"
    );
    assert_eq!(
        lockfile_item_count(&target),
        0,
        "lockfile must have zero items after removing the unit"
    );
}

#[test]
fn remove_solo_item_leaves_independent_items_in_other_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("reg");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Two items in SEPARATE folders -> distinct groups, no coupling.
    write(
        &source.join("alpha/SKILL.md"),
        "---\nschema: 1\nname: alpha\ndescription: alpha skill.\n---\n\n## Body\n\nText.\n",
    );
    write(
        &source.join("beta/SKILL.md"),
        "---\nschema: 1\nname: beta\ndescription: beta skill.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    assert!(target.join(".claude/skills/alpha/SKILL.md").exists());
    assert!(target.join(".claude/skills/beta/SKILL.md").exists());

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["remove", "alpha"])
        .assert()
        .success();

    assert!(
        !target.join(".claude/skills/alpha/SKILL.md").exists(),
        "alpha removed"
    );
    assert!(
        target.join(".claude/skills/beta/SKILL.md").exists(),
        "beta is in a different group and must survive"
    );
    assert_eq!(lockfile_item_count(&target), 1, "only beta should remain");
}

#[test]
fn remove_unknown_name_still_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("reg");
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    write(
        &source.join("alpha/SKILL.md"),
        "---\nschema: 1\nname: alpha\ndescription: alpha skill.\n---\n\n## Body\n\nText.\n",
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["remove", "does-not-exist"])
        .assert()
        .failure();
}
