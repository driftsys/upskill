//! ATDD tests for the `requires` transitive closure.
//!
//! Installing an item pulls in the items it requires from the same source
//! and from cross-source `{ name, source }` entries. The lockfile records
//! `required_by` provenance for each pulled-in dependency and the dependency
//! records its own source.

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
fn add_pulls_cross_source_requires_from_local_sibling() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Second source holding the required skill.
    let reg_b = tmp.path().join("reg-b");
    write_item(&reg_b, "SKILL.md", "sarif", "");

    // Entry source requires `sarif` from `reg-b` by absolute path.
    let reg_a = tmp.path().join("reg-a");
    write_item(
        &reg_a,
        "AGENT.md",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
            reg_b.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();

    let sarif = items
        .iter()
        .find(|i| i["kind"] == "skill" && i["name"] == "sarif")
        .unwrap_or_else(|| panic!("cross-source sarif must be installed: {lock}"));

    // The dependency records its OWN source (reg-b), not the entry source.
    let expected_source = format!("local:{}", reg_b.display());
    assert_eq!(
        sarif["source"], expected_source,
        "sarif must record its own cross-source: {sarif}"
    );

    let required_by = sarif["required_by"].as_array().unwrap();
    assert!(
        required_by.iter().any(|r| r == "agent:code-review"),
        "cross-source sarif required_by must contain agent:code-review: {lock}"
    );
}

#[test]
fn add_resolves_transitive_cross_source_closure() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg_c = tmp.path().join("reg-c");
    write_item(&reg_c, "RULE.md", "security-baseline", "");

    let reg_b = tmp.path().join("reg-b");
    write_item(
        &reg_b,
        "SKILL.md",
        "sarif",
        &format!(
            "requires:\n  rules: [{{ name: security-baseline, source: {} }}]\n",
            reg_c.display()
        ),
    );

    let reg_a = tmp.path().join("reg-a");
    write_item(
        &reg_a,
        "AGENT.md",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
            reg_b.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();
    let has = |kind: &str, name: &str| items.iter().any(|i| i["kind"] == kind && i["name"] == name);
    assert!(has("agent", "code-review"), "lockfile: {lock}");
    assert!(has("skill", "sarif"), "lockfile: {lock}");
    assert!(has("rule", "security-baseline"), "lockfile: {lock}");

    let baseline = items
        .iter()
        .find(|i| i["kind"] == "rule" && i["name"] == "security-baseline")
        .unwrap();
    assert_eq!(
        baseline["source"],
        format!("local:{}", reg_c.display()),
        "transitive rule must record its own source: {baseline}"
    );
    let required_by = baseline["required_by"].as_array().unwrap();
    assert!(
        required_by.iter().any(|r| r == "skill:sarif"),
        "security-baseline required_by must contain skill:sarif: {baseline}"
    );
}

#[test]
fn add_cross_source_conflict_with_existing_install_aborts() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // Source C provides sarif directly.
    let reg_c = tmp.path().join("reg-c");
    write_item(&reg_c, "SKILL.md", "sarif", "");
    // Source B also provides sarif (a different source).
    let reg_b = tmp.path().join("reg-b");
    write_item(&reg_b, "SKILL.md", "sarif", "");
    // Source A's agent requires sarif FROM B.
    let reg_a = tmp.path().join("reg-a");
    write_item(
        &reg_a,
        "AGENT.md",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
            reg_b.display()
        ),
    );

    // Install sarif from C first.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-c", "sarif"])
        .assert()
        .success();

    // Adding A pulls sarif from B — conflict with the C install.
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    assert!(
        String::from_utf8_lossy(&out).contains("already installed"),
        "expected conflict error; stderr was: {}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn add_cross_source_cycle_aborts() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg_a = tmp.path().join("reg-a");
    let reg_b = tmp.path().join("reg-b");
    fs::create_dir_all(&reg_a).unwrap();
    fs::create_dir_all(&reg_b).unwrap();
    write_item(
        &reg_a,
        "RULE.md",
        "alpha",
        &format!(
            "requires:\n  rules: [{{ name: beta, source: {} }}]\n",
            reg_b.display()
        ),
    );
    write_item(
        &reg_b,
        "RULE.md",
        "beta",
        &format!(
            "requires:\n  rules: [{{ name: alpha, source: {} }}]\n",
            reg_a.display()
        ),
    );

    // Add `alpha` by the SAME absolute locator that `beta`'s back-reference
    // uses, so the entry source label matches the cross-source label. A
    // relative `./reg-a` here would resolve to a different label than the
    // absolute back-reference and trip the "two different sources" identity
    // check before the cycle is ever traversed.
    let reg_a_arg = reg_a.display().to_string();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", &reg_a_arg, "alpha"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    assert!(
        String::from_utf8_lossy(&out).contains("circular"),
        "expected circular error; stderr was: {}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn removing_requirer_leaves_cross_source_dep_as_doctor_orphan() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    let reg_b = tmp.path().join("reg-b");
    write_item(&reg_b, "SKILL.md", "sarif", "");
    let reg_a = tmp.path().join("reg-a");
    write_item(
        &reg_a,
        "AGENT.md",
        "code-review",
        &format!(
            "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
            reg_b.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();

    // Remove the requirer; dependency must NOT cascade.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", "code-review"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap())
            .unwrap();
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["name"] == "sarif"),
        "removal must not cascade to the cross-source dependency: {lock}"
    );

    // doctor surfaces the orphaned dependency (advisory -> exit 0).
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .arg("doctor")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(
        String::from_utf8_lossy(&out).contains("sarif"),
        "doctor should surface orphaned dependency sarif; stdout was: {}",
        String::from_utf8_lossy(&out)
    );
}
