//! ATDD tests for the same-source `requires` transitive closure.
//!
//! Installing an item pulls in the items it requires from the SAME source.
//! The lockfile records `required_by` provenance for each pulled-in
//! dependency. Cross-source `{ name, source }` requires entries are not yet
//! resolved and must error clearly.

mod common;

use std::fs;
use std::path::Path;

/// Write an item entrypoint of `kind` (SKILL/AGENT) named `name` into a
/// `<root>/<name>/` directory with the given extra frontmatter lines.
fn write_item(root: &Path, entrypoint: &str, name: &str, frontmatter_extra: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nschema: 1\nname: {name}\ndescription: test {name}\n{frontmatter_extra}---\n# {name}\n"
    );
    fs::write(dir.join(entrypoint), body).unwrap();
}

#[test]
fn add_pulls_same_source_requires_and_records_provenance() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    // Fake git repo so upskill uses project scope (lockfile in cwd).
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

    let lock_path = tmp.path().join(".upskill-lock.json");
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    let items = lock["items"].as_array().unwrap();

    let has = |kind: &str, name: &str| items.iter().any(|i| i["kind"] == kind && i["name"] == name);
    assert!(has("agent", "code-review"), "lockfile: {lock}");
    assert!(has("skill", "sarif"), "lockfile: {lock}");

    let sarif = items
        .iter()
        .find(|i| i["kind"] == "skill" && i["name"] == "sarif")
        .unwrap();
    let required_by = sarif["required_by"].as_array().unwrap();
    assert!(
        required_by.iter().any(|r| r == "agent:code-review"),
        "sarif required_by must contain agent:code-review: {sarif}"
    );
}

#[test]
fn add_pulls_preload_skills_as_requires() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg = tmp.path().join("reg");
    write_item(&reg, "AGENT.md", "code-review", "preload-skills: [sarif]\n");
    write_item(&reg, "SKILL.md", "sarif", "");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "code-review"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();
    let sarif = items
        .iter()
        .find(|i| i["kind"] == "skill" && i["name"] == "sarif");
    assert!(sarif.is_some(), "preload-skills must pull sarif: {lock}");
    let required_by = sarif.unwrap()["required_by"].as_array().unwrap();
    assert!(
        required_by.iter().any(|r| r == "agent:code-review"),
        "preloaded sarif required_by must contain agent:code-review: {lock}"
    );
}

#[test]
fn add_errors_on_cross_source_requires() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg = tmp.path().join("reg");
    write_item(
        &reg,
        "AGENT.md",
        "code-review",
        "requires:\n  skills: [{ name: sarif, source: org/repo }]\n",
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "code-review"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cross-source"));
}
