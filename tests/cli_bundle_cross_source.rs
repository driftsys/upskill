//! ATDD tests for cross-source bundle `requires` (ADR-0009).
//!
//! A bundle MAY depend on a bundle living in another source via a
//! `{ name, source }` entry in `requires`. Installing the entry bundle pulls
//! in the required bundle's items from that other source; the lockfile records
//! each bundle and each item under its OWN source.

mod common;

use std::fs;
use std::path::Path;

/// Write a rule item at `<root>/<name>/RULE.md`.
fn write_rule(root: &Path, name: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let body =
        format!("---\nschema: 1\nname: {name}\ndescription: test rule {name}\n---\n# {name}\n");
    fs::write(dir.join("RULE.md"), body).unwrap();
}

/// Write a bundle manifest at `<root>/bundles/<name>.bundle.yaml`.
fn write_bundle(root: &Path, name: &str, body: &str) {
    let dir = root.join("bundles");
    fs::create_dir_all(&dir).unwrap();
    let content = format!("schema: 1\nname: {name}\ndescription: test bundle {name}\n{body}");
    fs::write(dir.join(format!("{name}.bundle.yaml")), content).unwrap();
}

#[test]
fn add_bundle_pulls_cross_source_required_bundle() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Source DEP: holds the leaf bundle `dep` and its item.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");

    // Source META: bundle `meta` requires `dep` from the DEP source by path.
    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    let bundle = reg_meta.join("bundles/meta.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();

    // The entry source label for a bundle-file `add` is the file path itself;
    // the cross-source dependency records its OWN (registry) source.
    let entry_src = format!("local:{}", bundle.display());
    let dep_src = format!("local:{}", reg_dep.display());

    let items = lock["items"].as_array().unwrap();
    let find = |name: &str| {
        items
            .iter()
            .find(|i| i["kind"] == "rule" && i["name"] == name)
            .unwrap_or_else(|| panic!("rule {name} must be installed: {lock}"))
    };
    assert_eq!(find("meta-rule")["source"], entry_src);
    assert_eq!(find("dep-rule")["source"], dep_src);

    // Both bundles recorded, each under its OWN source.
    let bundles = lock["bundles"].as_array().unwrap();
    let bundle_src = |name: &str| {
        bundles
            .iter()
            .find(|b| b["name"] == name)
            .unwrap_or_else(|| panic!("bundle {name} must be recorded: {lock}"))["source"]
            .clone()
    };
    assert_eq!(bundle_src("meta"), entry_src);
    assert_eq!(bundle_src("dep"), dep_src);
}

#[test]
fn add_bundle_by_name_pulls_cross_source_required_bundle() {
    // Same as above, but via name resolution (`add <source> meta`) rather
    // than a bundle-file path — the seed scenario.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_meta.to_str().unwrap(), "meta"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();
    let has = |name: &str| {
        items
            .iter()
            .any(|i| i["kind"] == "rule" && i["name"] == name)
    };
    assert!(has("meta-rule"), "entry bundle item installed: {lock}");
    assert!(
        has("dep-rule"),
        "cross-source required bundle item installed: {lock}"
    );
}

#[test]
fn add_bundle_cross_source_dep_with_same_source_subdeps() {
    // The seed/metapowers shape: a cross-source required bundle whose OWN
    // requires are bare (same-source) names within its own source.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // DEP source: bundle `dep` bare-requires sibling `leaf` (same source).
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_rule(&reg_dep, "leaf-rule");
    write_bundle(&reg_dep, "leaf", "items:\n  rules:\n  - leaf-rule\n");
    write_bundle(
        &reg_dep,
        "dep",
        "items:\n  rules:\n  - dep-rule\nrequires:\n  - { name: leaf }\n",
    );

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_meta.to_str().unwrap(), "meta"])
        .assert()
        .success();

    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".upskill-lock.json")).unwrap())
            .unwrap();
    let items = lock["items"].as_array().unwrap();
    let has = |name: &str| {
        items
            .iter()
            .any(|i| i["kind"] == "rule" && i["name"] == name)
    };
    assert!(has("meta-rule"), "entry item: {lock}");
    assert!(has("dep-rule"), "cross-source dep item: {lock}");
    assert!(
        has("leaf-rule"),
        "same-source sub-dependency of the cross-source bundle: {lock}"
    );
}

#[test]
fn add_bundle_cross_source_cycle_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_a = tmp.path().join("reg-a");
    let reg_b = tmp.path().join("reg-b");
    write_rule(&reg_a, "a-rule");
    write_rule(&reg_b, "b-rule");
    write_bundle(
        &reg_a,
        "a",
        &format!(
            "items:\n  rules:\n  - a-rule\nrequires:\n  - {{ name: b, source: {} }}\n",
            reg_b.display()
        ),
    );
    write_bundle(
        &reg_b,
        "b",
        &format!(
            "items:\n  rules:\n  - b-rule\nrequires:\n  - {{ name: a, source: {} }}\n",
            reg_a.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_a.to_str().unwrap(), "a"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cycle"));
}

#[test]
fn add_bundle_cross_source_item_conflict_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Both bundles provide a rule named `shared` from different sources.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "shared");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - shared\n");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "shared");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - shared\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            reg_meta.join("bundles/meta.bundle.yaml").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("conflict"));
}

#[test]
fn add_bundle_cross_source_bundle_name_conflict_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // Disjoint items isolate the bundle identity conflict from item conflicts.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_bundle(&reg_dep, "shared", "items:\n  rules:\n  - dep-rule\n");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "shared",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: shared, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", reg_meta.to_str().unwrap(), "shared"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(format!(
            "bundle `shared` is required from two different sources: `local:{}` and `local:{}`",
            reg_meta.display(),
            reg_dep.display()
        )));
}

#[test]
fn add_bundle_missing_cross_source_bundle_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();

    // DEP source exists but does NOT contain a bundle named `dep`.
    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "unrelated");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    let bundle = reg_meta.join("bundles/meta.bundle.yaml");
    common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap()])
        .assert()
        .failure();
}

/// Stage a project at `<tmp>/target` whose `.claude/rules` directory is the
/// DEP source: it holds the bundle `dep` and the rule `dep-rule` in the folder
/// `dep_folder`. The META source at `<tmp>/reg-meta` holds the rule
/// `meta-rule` and the bundle `meta`, which requires `dep` from DEP.
/// Returns `(tmp, home, target, meta bundle path, dep rule path)`.
fn stage_dep_source_in_claude_rules(
    dep_folder: &str,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_dep = target.join(".claude/rules");
    let dep_dir = reg_dep.join(dep_folder);
    fs::create_dir_all(&dep_dir).unwrap();
    let dep_rule = dep_dir.join("RULE.md");
    fs::write(
        &dep_rule,
        "---\nschema: 1\nname: dep-rule\ndescription: d\n---\n# dep-rule\n",
    )
    .unwrap();
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );
    let bundle = reg_meta.join("bundles/meta.bundle.yaml");
    (tmp, home, target, bundle, dep_rule)
}

#[test]
fn add_bundle_refuses_cross_source_dependency_inside_an_output_path() {
    // The dependency rule `dep-rule` lives at `.claude/rules/dep-rule/`, the
    // Claude resource base of `dep-rule`.
    let (_tmp, home, target, bundle, dep_rule) = stage_dep_source_in_claude_rules("dep-rule");
    let before = fs::read_to_string(&dep_rule).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap(), "--claude", "--project"])
        .assert()
        .failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("'dep-rule'") && stderr.contains("overlaps the output path"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(fs::read_to_string(&dep_rule).unwrap(), before);
    assert!(!target.join(".claude/rules/meta-rule.md").exists());
    assert!(!target.join(".upskill-lock.json").exists());
}

#[test]
fn add_bundle_refuses_dependency_item_inside_the_output_path_of_an_entry_item() {
    // The dependency rule `dep-rule` lives at `.claude/rules/meta-rule/dep-rule/`,
    // inside the Claude resource base of the entry rule `meta-rule`.
    let (_tmp, home, target, bundle, dep_rule) =
        stage_dep_source_in_claude_rules("meta-rule/dep-rule");
    let before = fs::read_to_string(&dep_rule).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["add", bundle.to_str().unwrap(), "--claude", "--project"])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("the source directory of rule 'dep-rule' (")
            && stderr.contains(") is inside the output path")
            && stderr.contains("of rule 'meta-rule'; installing would delete"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(fs::read_to_string(&dep_rule).unwrap(), before);
    assert!(!target.join(".claude/rules/meta-rule.md").exists());
    assert!(!target.join(".upskill-lock.json").exists());
}

#[test]
fn add_bundle_alias_does_not_apply_to_cross_source_dependency_items() {
    // The dependency rule `dep-rule` lives at `.claude/rules/x/`, the Claude
    // resource base of a rule named `x`. `--as dep-rule=x` names an entry
    // source alias only, so `dep-rule` keeps its name and is not refused.
    let (_tmp, home, target, bundle, dep_rule) = stage_dep_source_in_claude_rules("x");

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            bundle.to_str().unwrap(),
            "--as",
            "dep-rule=x",
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    assert!(target.join(".claude/rules/dep-rule.md").exists());
    assert!(dep_rule.exists());
}

/// Stage the entry source `<tmp>/target/.claude/skills`, which holds only the
/// bundle `meta` (it requires `dep` from another source) and the item folder
/// `q/y` that no bundle lists. The dependency source holds the skill `q`
/// with the resource `y/SKILL.md` and the rule `dep-rule`, both in `dep`.
/// Installing `q` would write `.claude/skills/q/y/SKILL.md`, the entrypoint
/// of the entry-source item `y`. Returns `(tmp, home, target, y, y content)`.
fn stage_entry_source_whose_bundle_selects_no_item() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    &'static str,
) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_dep = tmp.path().join("reg-dep");
    let q = reg_dep.join("q");
    fs::create_dir_all(q.join("y")).unwrap();
    fs::write(
        q.join("SKILL.md"),
        "---\nschema: 1\nname: q\ndescription: d\n---\n# q\n",
    )
    .unwrap();
    fs::write(q.join("y/SKILL.md"), "resource of q\n").unwrap();
    write_rule(&reg_dep, "dep-rule");
    write_bundle(
        &reg_dep,
        "dep",
        "items:\n  skills:\n  - q\n  rules:\n  - dep-rule\n",
    );

    let reg_entry = target.join(".claude/skills");
    let y = reg_entry.join("q/y/SKILL.md");
    fs::create_dir_all(y.parent().unwrap()).unwrap();
    let before = "---\nschema: 1\nname: y\ndescription: d\n---\n# y\n";
    fs::write(&y, before).unwrap();
    write_bundle(
        &reg_entry,
        "meta",
        &format!(
            "items: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );
    (tmp, home, target, y, before)
}

#[test]
fn add_bundle_refuses_overlap_with_an_item_of_an_entry_source_whose_bundle_selects_no_item() {
    let (_tmp, home, target, y, before) = stage_entry_source_whose_bundle_selects_no_item();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            "./.claude/skills/bundles/meta.bundle.yaml",
            "--claude",
            "--project",
            "--force",
        ])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("the source directory of skill 'y' (")
            && stderr.contains("of skill 'q'; installing would delete"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(fs::read_to_string(&y).unwrap(), before);
    assert!(!target.join(".claude/skills/q/SKILL.md").exists());
    assert!(!target.join(".claude/rules/dep-rule.md").exists());
    assert!(!target.join(".upskill-lock.json").exists());
}

#[test]
fn add_bundle_ignores_the_output_paths_of_an_excluded_dependency_item() {
    // `--exclude q` applies to the dependency source too, so nothing is
    // written for `q` and the source of `y` does not overlap the run.
    let (_tmp, home, target, y, before) = stage_entry_source_whose_bundle_selects_no_item();

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            "./.claude/skills/bundles/meta.bundle.yaml",
            "--exclude",
            "q",
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&y).unwrap(), before);
    assert!(!target.join(".claude/skills/q/SKILL.md").exists());
    assert!(target.join(".claude/rules/dep-rule.md").is_file());
}

#[test]
fn add_bundle_does_not_install_the_items_of_an_entry_source_whose_bundle_selects_no_item() {
    // The entry source `.claude/skills` holds the bundle `meta`, which selects
    // no item and requires `dep`, and the unlisted skill `x` at its own
    // Claude output path `.claude/skills/x`. `x` is not installed, so it has
    // no paths in the overlap check and the install succeeds.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_dep = tmp.path().join("reg-dep");
    write_rule(&reg_dep, "dep-rule");
    write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");

    let reg_entry = target.join(".claude/skills");
    let x = reg_entry.join("x/SKILL.md");
    fs::create_dir_all(x.parent().unwrap()).unwrap();
    let before = "---\nschema: 1\nname: x\ndescription: d\n---\n# x\n";
    fs::write(&x, before).unwrap();
    write_bundle(
        &reg_entry,
        "meta",
        &format!(
            "items: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            "./.claude/skills/bundles/meta.bundle.yaml",
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&x).unwrap(), before);
    assert!(target.join(".claude/rules/dep-rule.md").is_file());
}

#[test]
fn add_bundle_refuses_overlap_with_an_item_of_a_pass_through_dependency_source() {
    // Three sources: the entry bundle `meta` requires `mid` from the source
    // `.claude/skills`; `mid` selects no item and requires `dep` from a third
    // source. `.claude/skills` also holds the unlisted item folder `q/y`. The
    // dependency skill `q` would write `.claude/skills/q/y/SKILL.md`.
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg_dep = tmp.path().join("reg-dep");
    let q = reg_dep.join("q");
    fs::create_dir_all(q.join("y")).unwrap();
    fs::write(
        q.join("SKILL.md"),
        "---\nschema: 1\nname: q\ndescription: d\n---\n# q\n",
    )
    .unwrap();
    fs::write(q.join("y/SKILL.md"), "resource of q\n").unwrap();
    write_bundle(&reg_dep, "dep", "items:\n  skills:\n  - q\n");

    let reg_mid = target.join(".claude/skills");
    let y = reg_mid.join("q/y/SKILL.md");
    fs::create_dir_all(y.parent().unwrap()).unwrap();
    let before = "---\nschema: 1\nname: y\ndescription: d\n---\n# y\n";
    fs::write(&y, before).unwrap();
    write_bundle(
        &reg_mid,
        "mid",
        &format!(
            "items: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
            reg_dep.display()
        ),
    );

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: mid, source: {} }}\n",
            reg_mid.display()
        ),
    );
    let bundle = reg_meta.join("bundles/meta.bundle.yaml");

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            bundle.to_str().unwrap(),
            "--claude",
            "--project",
            "--force",
        ])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("the source directory of skill 'y' (")
            && stderr.contains("of skill 'q'; installing would delete"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(fs::read_to_string(&y).unwrap(), before);
    assert!(!target.join(".claude/skills/q/SKILL.md").exists());
    assert!(!target.join(".upskill-lock.json").exists());
}

/// Stage the source `<tmp>/target/.claude/skills` as a dependency source that
/// holds the unlisted skill `x` at its own Claude output path
/// `.claude/skills/x`. When `pass_through` is false, the source also holds
/// the bundle `dep` with the skill `q` (in the folder `grp/q`). When it is
/// true, the source holds the bundle `mid`, which selects no item and
/// requires `dep` (rule `dep-rule`) from a third source. The entry bundle
/// `meta` at `<tmp>/reg-meta` requires `dep` or `mid` from that source.
/// Returns `(tmp, home, target, meta bundle path, x path, x content)`.
fn stage_unselected_item_in_a_dependency_source(
    pass_through: bool,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    &'static str,
) {
    let tmp = tempfile::TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let target = tmp.path().join("target");
    fs::create_dir_all(target.join(".git")).unwrap();

    let reg = target.join(".claude/skills");
    let x = reg.join("x/SKILL.md");
    fs::create_dir_all(x.parent().unwrap()).unwrap();
    let before = "---\nschema: 1\nname: x\ndescription: d\n---\n# x\n";
    fs::write(&x, before).unwrap();
    let required = if pass_through {
        let reg_dep = tmp.path().join("reg-dep");
        write_rule(&reg_dep, "dep-rule");
        write_bundle(&reg_dep, "dep", "items:\n  rules:\n  - dep-rule\n");
        write_bundle(
            &reg,
            "mid",
            &format!(
                "items: {{}}\nrequires:\n  - {{ name: dep, source: {} }}\n",
                reg_dep.display()
            ),
        );
        "mid"
    } else {
        let q = reg.join("grp/q");
        fs::create_dir_all(&q).unwrap();
        fs::write(
            q.join("SKILL.md"),
            "---\nschema: 1\nname: q\ndescription: d\n---\n# q\n",
        )
        .unwrap();
        write_bundle(&reg, "dep", "items:\n  skills:\n  - q\n");
        "dep"
    };

    let reg_meta = tmp.path().join("reg-meta");
    write_rule(&reg_meta, "meta-rule");
    write_bundle(
        &reg_meta,
        "meta",
        &format!(
            "items:\n  rules:\n  - meta-rule\nrequires:\n  - {{ name: {required}, source: {} }}\n",
            reg.display()
        ),
    );
    let bundle = reg_meta.join("bundles/meta.bundle.yaml");
    (tmp, home, target, bundle, x, before)
}

#[test]
fn add_bundle_does_not_install_an_unselected_item_of_a_dependency_source() {
    // `x` is not selected by the `requires` closure, so it has no paths in
    // the overlap check, although it is at its own output path.
    for pass_through in [false, true] {
        let (_tmp, home, target, bundle, x, before) =
            stage_unselected_item_in_a_dependency_source(pass_through);

        common::upskill_cmd(&home)
            .current_dir(&target)
            .args(["add", bundle.to_str().unwrap(), "--claude", "--project"])
            .assert()
            .success();
        assert_eq!(fs::read_to_string(&x).unwrap(), before, "{pass_through}");
        assert!(
            target.join(".claude/rules/meta-rule.md").is_file(),
            "{pass_through}"
        );
        // The selected dependency item: `q` from the dependency source, or
        // `dep-rule` from the third source.
        let dependency = if pass_through {
            ".claude/rules/dep-rule.md"
        } else {
            ".claude/skills/q/SKILL.md"
        };
        assert!(target.join(dependency).is_file(), "{pass_through}");
    }
}

#[test]
fn add_bundle_refuses_a_selected_output_path_that_covers_an_unselected_item_of_a_dependency_source()
{
    // The dependency source `.claude/skills` supplies the selected skill `q`
    // (in the folder `grp/q`) and holds the unselected skill `u` at
    // `.claude/skills/q/u`, inside the Claude output path of `q`.
    let (_tmp, home, target, bundle, _x, _) = stage_unselected_item_in_a_dependency_source(false);
    let u = target.join(".claude/skills/q/u/SKILL.md");
    fs::create_dir_all(u.parent().unwrap()).unwrap();
    let before = "---\nschema: 1\nname: u\ndescription: d\n---\n# u\n";
    fs::write(&u, before).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args([
            "add",
            bundle.to_str().unwrap(),
            "--claude",
            "--project",
            "--force",
        ])
        .assert()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("the source directory of skill 'u' (")
            && stderr.contains("of skill 'q'; installing would delete"),
        "stderr does not explain the refusal: {stderr}"
    );
    assert_eq!(fs::read_to_string(&u).unwrap(), before);
    assert!(!target.join(".claude/skills/q/SKILL.md").exists());
    assert!(!target.join(".upskill-lock.json").exists());
}
