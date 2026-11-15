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

#[test]
fn update_does_not_pull_unrelated_siblings_of_a_cross_source_dependency() {
    // Regression for #211: a source that only contributes a cross-source
    // dependency must not have its unrelated siblings installed by `update`.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();

    // reg-b vends the required `sarif` skill AND an unrelated sibling.
    let reg_b = tmp.path().join("reg-b");
    write_item(&reg_b, "SKILL.md", "sarif", "");
    write_item(&reg_b, "SKILL.md", "unrelated-sibling", "");

    // reg-a's agent requires only `sarif` from reg-b.
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

    let sibling_out = tmp.path().join(".claude/skills/unrelated-sibling/SKILL.md");
    assert!(
        !sibling_out.exists(),
        "add must not install the unrelated sibling"
    );

    // The regression is in `update`: it groups by source and previously
    // reinstalled the WHOLE dependency source, pulling the sibling in.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .success();

    assert!(
        !sibling_out.exists(),
        "update must not pull the unrelated sibling of a cross-source dependency"
    );
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join(".upskill-lock.json")).unwrap())
            .unwrap();
    assert!(
        !lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["name"] == "unrelated-sibling"),
        "unrelated sibling must not be recorded after update: {lock}"
    );
    // The genuine dependency is still present.
    assert!(
        lock["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["name"] == "sarif"),
        "the cross-source dependency must survive update: {lock}"
    );
}

/// Read the lockfile of `root`.
fn read_lock(root: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(root.join(".upskill-lock.json")).unwrap()).unwrap()
}

/// The lockfile entry `kind`/`name` of `lock`.
fn entry<'a>(lock: &'a serde_json::Value, kind: &str, name: &str) -> &'a serde_json::Value {
    lock["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == kind && i["name"] == name)
        .unwrap_or_else(|| panic!("{kind}:{name} must be in the lockfile: {lock}"))
}

/// `required_by` of the entry `kind`/`name` (empty when absent).
fn required_by(lock: &serde_json::Value, kind: &str, name: &str) -> Vec<String> {
    entry(lock, kind, name)["required_by"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

/// A project with `reg-a` (agent `code-review`, which requires skill `sarif`
/// of `reg-b`) and `reg-b` (skill `sarif`). Returns `(tmp, home, reg_a, reg_b)`.
fn cross_source_project() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
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
    (tmp, home, reg_a, reg_b)
}

/// #504: the reinstall of the dependency's own source keeps the requirer
/// that another source recorded, for both orders of the two `add` commands.
#[test]
fn add_of_the_dependency_source_keeps_required_by_in_both_orders() {
    for dependency_first in [false, true] {
        let (tmp, home, reg_a, reg_b) = cross_source_project();
        let add_a = [
            "add".to_string(),
            reg_a.display().to_string(),
            "code-review".to_string(),
        ];
        let add_b = [
            "add".to_string(),
            reg_b.display().to_string(),
            "sarif".to_string(),
        ];
        let order = if dependency_first {
            [add_b.clone(), add_a.clone()]
        } else {
            [add_a.clone(), add_b.clone()]
        };
        for args in order {
            common::upskill_cmd(&home)
                .current_dir(tmp.path())
                .args(&args)
                .assert()
                .success();
        }
        let lock = read_lock(tmp.path());
        assert_eq!(
            required_by(&lock, "skill", "sarif"),
            vec!["agent:code-review".to_string()],
            "dependency_first={dependency_first}: {lock}"
        );
    }
}

/// #504: a bare `update` that changes both sources keeps the requirer in
/// `required_by`, for both orders of the two sources. `update` applies the
/// sources in the order of their labels, so the directory names select the
/// order: `dependency_last` names the requirer's source `a-req` and the
/// dependency's source `z-dep`, otherwise `z-req` and `a-dep`. The test
/// checks that the labels sort in the intended order.
#[test]
fn bare_update_keeps_required_by_for_both_source_orders() {
    for dependency_last in [true, false] {
        let tmp = tempfile::TempDir::new().unwrap();
        // The canonical path, so that the label of the source given on the
        // command line and the label of the `requires` source have one form.
        let root = fs::canonicalize(tmp.path()).unwrap();
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        let (req_dir, dep_dir) = if dependency_last {
            ("a-req", "z-dep")
        } else {
            ("z-req", "a-dep")
        };
        let reg_dep = root.join(dep_dir);
        write_item(&reg_dep, "SKILL.md", "sarif", "");
        let reg_req = root.join(req_dir);
        write_item(
            &reg_req,
            "AGENT.md",
            "code-review",
            &format!(
                "requires:\n  skills: [{{ name: sarif, source: {} }}]\n",
                reg_dep.display()
            ),
        );
        common::upskill_cmd(&home)
            .current_dir(&root)
            .args(["add", &reg_req.display().to_string(), "code-review"])
            .assert()
            .success();
        let lock = read_lock(&root);
        let dep_label = entry(&lock, "skill", "sarif")["source"]
            .as_str()
            .unwrap()
            .to_string();
        let req_label = entry(&lock, "agent", "code-review")["source"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(dep_label > req_label, dependency_last, "{lock}");

        // Change both sources, so that the update applies both of them.
        let agent = reg_req.join("code-review/AGENT.md");
        fs::write(&agent, fs::read_to_string(&agent).unwrap() + "\nchanged\n").unwrap();
        let skill = reg_dep.join("sarif/SKILL.md");
        fs::write(&skill, fs::read_to_string(&skill).unwrap() + "\nchanged\n").unwrap();

        common::upskill_cmd(&home)
            .current_dir(&root)
            .args(["update", "--yes"])
            .assert()
            .success();

        let lock = read_lock(&root);
        assert_eq!(
            required_by(&lock, "skill", "sarif"),
            vec!["agent:code-review".to_string()],
            "dependency_last={dependency_last}: {lock}"
        );
    }
}

/// #504: `update` refreshes `required_by` of an item that keeps its edited
/// files, without a change of its files or its hash, when the requirer of
/// the same source stops requiring it.
#[test]
fn update_refreshes_required_by_of_a_kept_item() {
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
        .args(["add", "./reg", "code-review", "--claude"])
        .assert()
        .success();
    let before = read_lock(tmp.path());
    assert_eq!(
        required_by(&before, "skill", "sarif"),
        vec!["agent:code-review".to_string()]
    );

    // Edit the installed file of `sarif`, so that the update keeps it.
    let installed = tmp.path().join(".claude/skills/sarif/SKILL.md");
    let edited = fs::read_to_string(&installed).unwrap() + "\nmy edit\n";
    fs::write(&installed, &edited).unwrap();
    // The requirer stops requiring `sarif`.
    write_item(&reg, "AGENT.md", "code-review", "");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .success();

    let after = read_lock(tmp.path());
    assert!(
        required_by(&after, "skill", "sarif").is_empty(),
        "required_by of the kept item must be refreshed: {after}"
    );
    let (old, new) = (
        entry(&before, "skill", "sarif"),
        entry(&after, "skill", "sarif"),
    );
    assert_eq!(old["hash"], new["hash"], "{after}");
    assert_eq!(old["files"], new["files"], "{after}");
    assert_eq!(fs::read_to_string(&installed).unwrap(), edited);
}

/// #504: when the requirer stops requiring a dependency of another source,
/// `update` removes the requirer from `required_by` of the dependency, and
/// after the removal of the requirer `doctor` reports no orphaned
/// dependency. The bare update also reinstalls the dependency's source
/// (after the requirer's source, in label order). The test passes on the
/// code before #504 too, because that reinstall replaced `required_by`
/// with an empty value; it pins that the run of the requirer's source
/// removes the requirer.
#[test]
fn update_refreshes_required_by_of_a_dependency_that_is_no_longer_required() {
    let (tmp, home, reg_a, _reg_b) = cross_source_project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();
    write_item(&reg_a, "AGENT.md", "code-review", "");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .success();

    let lock = read_lock(tmp.path());
    assert!(
        required_by(&lock, "skill", "sarif").is_empty(),
        "required_by of sarif must be refreshed: {lock}"
    );
    // With a stale `required_by: [agent:code-review]`, `doctor` would report
    // `sarif` as an orphaned dependency after this removal.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["remove", "code-review"])
        .assert()
        .success();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .arg("doctor")
        .assert()
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8_lossy(&out);
    assert!(
        !out.contains("orphaned dependency"),
        "doctor must report no orphaned dependency; stdout was: {out}"
    );
}

/// #504: an `update` of sources without changes restores a missing
/// requirer in `required_by` and reports no change.
#[test]
fn update_without_changes_refreshes_required_by_silently() {
    let (tmp, home, _reg_a, _reg_b) = cross_source_project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();
    // A lockfile written by an earlier version that dropped the requirer.
    let path = tmp.path().join(".upskill-lock.json");
    let mut lock = read_lock(tmp.path());
    for item in lock["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("required_by");
    }
    fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("updated") && !stdout.contains("sarif"),
        "the refresh must print no item line; stdout was: {stdout}"
    );

    let lock = read_lock(tmp.path());
    assert_eq!(
        required_by(&lock, "skill", "sarif"),
        vec!["agent:code-review".to_string()],
        "{lock}"
    );
}

/// #504: in a terminal, a source without changes is not applied; the
/// update still restores a missing requirer in `required_by`, and asks
/// nothing.
#[cfg(unix)]
#[test]
fn terminal_update_without_changes_refreshes_required_by_silently() {
    let (tmp, home, _reg_a, _reg_b) = cross_source_project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();
    let path = tmp.path().join(".upskill-lock.json");
    let mut lock = read_lock(tmp.path());
    for item in lock["items"].as_array_mut().unwrap() {
        item.as_object_mut().unwrap().remove("required_by");
    }
    fs::write(&path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

    let result = common::update_in_terminal(tmp.path(), &home, &[], "");
    assert_eq!(result["returncode"], 0, "{result}");
    for stream in ["stdout", "stderr"] {
        assert!(
            !result[stream].as_str().unwrap().contains("Apply"),
            "{result}"
        );
    }
    let lock = read_lock(tmp.path());
    assert_eq!(
        required_by(&lock, "skill", "sarif"),
        vec!["agent:code-review".to_string()],
        "{lock}"
    );
}

/// Installs `code-review` of `reg-a` (which records `sarif` as required by
/// it), adds the bundle `pack` (agent `code-review`) to `reg-a`, runs `add`
/// with `args`, and returns `required_by` of `sarif` after it.
fn required_by_after_a_second_add(args: &[&str]) -> Vec<String> {
    let (tmp, home, reg_a, _reg_b) = cross_source_project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review"])
        .assert()
        .success();
    fs::write(
        reg_a.join("pack.bundle.yaml"),
        "schema: 1\nname: pack\ndescription: d\nitems:\n  agents: [code-review]\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(args)
        .assert()
        .success();
    required_by(&read_lock(tmp.path()), "skill", "sarif")
}

/// #504 review: a bundle install reads no item `requires`, so it keeps the
/// recorded requirers.
#[test]
fn bundle_add_by_name_keeps_required_by() {
    assert_eq!(
        required_by_after_a_second_add(&["add", "./reg-a", "pack"]),
        vec!["agent:code-review".to_string()]
    );
}

/// #504 review: as above, for a bundle file source (another source label,
/// so `--force`).
#[test]
fn bundle_file_add_keeps_required_by() {
    assert_eq!(
        required_by_after_a_second_add(&["add", "./reg-a/pack.bundle.yaml", "--force"]),
        vec!["agent:code-review".to_string()]
    );
}

/// #504 review: an excluded dependency is still required by its requirer.
#[test]
fn add_with_an_excluded_dependency_keeps_required_by() {
    assert_eq!(
        required_by_after_a_second_add(&["add", "./reg-a", "code-review", "--exclude", "sarif"]),
        vec!["agent:code-review".to_string()]
    );
}

/// #504 review: a request that names a bundle and an item goes through
/// name resolution, without a closure; it keeps the recorded requirers.
#[test]
fn add_through_name_resolution_keeps_required_by() {
    assert_eq!(
        required_by_after_a_second_add(&["add", "./reg-a", "code-review", "pack"]),
        vec!["agent:code-review".to_string()]
    );
}

/// #504 review: `update` refreshes `required_by` of an item whose update is
/// skipped (its installed file was edited and its source changed).
#[test]
fn update_refreshes_required_by_of_a_skipped_item() {
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
        .args(["add", "./reg", "code-review", "--claude"])
        .assert()
        .success();
    let before = read_lock(tmp.path());

    let installed = tmp.path().join(".claude/skills/sarif/SKILL.md");
    let edited = fs::read_to_string(&installed).unwrap() + "\nmy edit\n";
    fs::write(&installed, &edited).unwrap();
    // The source of `sarif` changes too, so its update is skipped.
    write_item(&reg, "SKILL.md", "sarif", "version: changed\n");
    write_item(&reg, "AGENT.md", "code-review", "");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .code(1);

    let after = read_lock(tmp.path());
    assert!(
        required_by(&after, "skill", "sarif").is_empty(),
        "required_by of the skipped item must be refreshed: {after}"
    );
    assert_eq!(
        entry(&before, "skill", "sarif")["hash"],
        entry(&after, "skill", "sarif")["hash"],
        "{after}"
    );
    assert_eq!(fs::read_to_string(&installed).unwrap(), edited);
}

/// #504 review: the refresh matches a requirer by its installed (`--as`)
/// name.
#[test]
fn update_refreshes_required_by_of_an_aliased_requirer() {
    let (tmp, home, reg_a, _reg_b) = cross_source_project();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "code-review", "--as", "code-review=cr"])
        .assert()
        .success();
    assert_eq!(
        required_by(&read_lock(tmp.path()), "skill", "sarif"),
        vec!["agent:cr".to_string()]
    );
    write_item(&reg_a, "AGENT.md", "code-review", "");

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["update", "--yes"])
        .assert()
        .success();

    let lock = read_lock(tmp.path());
    assert!(
        required_by(&lock, "skill", "sarif").is_empty(),
        "required_by of sarif must be refreshed: {lock}"
    );
}

/// A chain across three sources: agent `a` of `reg-a` requires skill `x` of
/// `reg-b`, which requires skill `y` of `reg-c`. Installs `a`, then makes
/// `x` stop requiring `y`. Returns `(tmp, home)`.
fn chain_where_x_stops_requiring_y() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let reg_c = tmp.path().join("reg-c");
    write_item(&reg_c, "SKILL.md", "y", "");
    let reg_b = tmp.path().join("reg-b");
    let x_requires = format!(
        "requires:\n  skills: [{{ name: y, source: {} }}]\n",
        reg_c.display()
    );
    write_item(&reg_b, "SKILL.md", "x", &x_requires);
    let reg_a = tmp.path().join("reg-a");
    write_item(
        &reg_a,
        "AGENT.md",
        "a",
        &format!(
            "requires:\n  skills: [{{ name: x, source: {} }}]\n",
            reg_b.display()
        ),
    );
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "a"])
        .assert()
        .success();
    assert_eq!(
        required_by(&read_lock(tmp.path()), "skill", "y"),
        vec!["skill:x".to_string()]
    );
    write_item(&reg_b, "SKILL.md", "x", "");
    (tmp, home)
}

/// #504 review: a transitive dependency from another source, which the
/// run does not request directly, counts as an evaluated requirer: the
/// install of `a` alone removes `skill:x` from `required_by` of `y`.
#[test]
fn add_evaluates_a_transitive_cross_source_requirer() {
    let (tmp, home) = chain_where_x_stops_requiring_y();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "a"])
        .assert()
        .success();
    let lock = read_lock(tmp.path());
    assert!(required_by(&lock, "skill", "y").is_empty(), "{lock}");
}

/// #504 review: the `requires` of a dependency that `--exclude` leaves out
/// are still evaluated.
#[test]
fn add_evaluates_the_requires_of_an_excluded_dependency() {
    let (tmp, home) = chain_where_x_stops_requiring_y();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg-a", "a", "--exclude", "x"])
        .assert()
        .success();
    let lock = read_lock(tmp.path());
    assert!(required_by(&lock, "skill", "y").is_empty(), "{lock}");
}
