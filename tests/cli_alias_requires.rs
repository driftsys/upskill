//! ATDD tests for `--as` alias interaction with `requires` provenance (#205).
//!
//! Slice 1 records same-source `requires` provenance as `required_by`
//! `"kind:name"` strings and `doctor` flags an item as an orphaned dependency
//! when none of its recorded requirers is still installed. `--as` renames an
//! item AFTER the closure provenance is computed, so without translation the
//! recorded requirer label (or dependency key) names the ORIGINAL identity and
//! `doctor` mismatches:
//!
//! - aliasing the REQUIRER (`--as code-review=cr`) leaves the dependency's
//!   `required_by` pointing at `"agent:code-review"` while the installed item
//!   is `"agent:cr"` -> false orphaned-dependency flag.
//! - aliasing the DEPENDENCY (`--as sarif=s`) drops the provenance entirely
//!   because the provenance key still names `sarif`, not `s`.

mod common;

use predicates::prelude::*;
use std::fs;
use std::path::Path;

/// Write an item entrypoint of `kind` (SKILL/AGENT) named `name` into its own
/// `<root>/<name>/` directory (so items are NOT co-located).
fn write_item(root: &Path, entrypoint: &str, name: &str, frontmatter_extra: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nschema: 1\nname: {name}\ndescription: test {name}\n{frontmatter_extra}---\n# {name}\n"
    );
    fs::write(dir.join(entrypoint), body).unwrap();
}

/// Read the installed lockfile and return the `required_by` Vec for the item
/// matching `(kind, name)`, or `None` if no such item is present.
fn required_by_of(lock_path: &Path, kind: &str, name: &str) -> Option<Vec<String>> {
    let raw = fs::read_to_string(lock_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for item in json["items"].as_array().unwrap() {
        if item["kind"] == kind && item["name"] == name {
            return Some(
                item["required_by"]
                    .as_array()
                    .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect())
                    .unwrap_or_default(),
            );
        }
    }
    None
}

#[test]
fn doctor_does_not_flag_dependency_when_requirer_aliased() {
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

    // Alias the REQUIRER: the agent installs as `cr`, pulling in `sarif`.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "code-review", "--as", "code-review=cr"])
        .assert()
        .success();

    // sarif's required_by must name the FINAL (aliased) requirer "agent:cr".
    let lock = tmp.path().join(".upskill-lock.json");
    assert_eq!(
        required_by_of(&lock, "skill", "sarif"),
        Some(vec!["agent:cr".to_string()]),
        "sarif required_by should reference the aliased requirer agent:cr"
    );

    // doctor: the requirer (cr) is still installed, so sarif is NOT flagged as
    // an orphaned dependency. (Exit code is not asserted: aliasing a local item
    // separately trips doctor's source lookup, which keys on the alias rather
    // than `source_name` — a distinct pre-existing bug, not #205.)
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .stdout(predicates::str::contains("orphaned").not());
}

#[test]
fn dependency_provenance_survives_dependency_alias() {
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

    // Alias the DEPENDENCY: the pulled skill installs as `s`.
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["add", "./reg", "code-review", "--as", "sarif=s"])
        .assert()
        .success();

    // The renamed dependency `s` must still carry its provenance.
    let lock = tmp.path().join(".upskill-lock.json");
    assert_eq!(
        required_by_of(&lock, "skill", "s"),
        Some(vec!["agent:code-review".to_string()]),
        "renamed dependency s should keep required_by agent:code-review"
    );

    // doctor: requirer code-review is still installed -> s is NOT flagged as an
    // orphaned dependency. (Exit code not asserted — see the sibling test.)
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .stdout(predicates::str::contains("orphaned").not());
}
