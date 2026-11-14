//! ATDD tests for the support level table: the `doctor` output, the
//! `doctor --json` key and the user-guide page.

mod common;

use upskill::support::SUPPORT;

fn empty_repo() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    (tmp, home, repo)
}

#[test]
fn doctor_prints_one_level_for_each_client_and_kind() {
    let (_tmp, home, repo) = empty_repo();
    let out = common::upskill_cmd(&home)
        .current_dir(&repo)
        .arg("doctor")
        .assert()
        .success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "Support levels:")
        .unwrap_or_else(|| panic!("no section in:\n{text}"));
    let rows = &lines[at + 1..];
    assert_eq!(rows.len(), 12, "{text}");
    assert_eq!(
        rows[0].split_whitespace().collect::<Vec<_>>(),
        ["claude", "rule", "experimental"]
    );
    assert_eq!(
        rows[11].split_whitespace().collect::<Vec<_>>(),
        ["codex", "agent", "unsupported"]
    );
}

#[test]
fn doctor_prints_the_support_levels_when_a_scope_flag_limits_the_report() {
    let (_tmp, home, repo) = empty_repo();
    let out = common::upskill_cmd(&home)
        .current_dir(&repo)
        .args(["doctor", "--project"])
        .assert()
        .success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(text.contains("Support levels:"), "{text}");
}

#[test]
fn doctor_json_holds_the_support_table() {
    let (_tmp, home, repo) = empty_repo();
    let out = common::upskill_cmd(&home)
        .current_dir(&repo)
        .args(["doctor", "--json"])
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    let support = v["support"].as_array().unwrap();
    assert_eq!(support.len(), 12, "{v}");
    for cell in support {
        let mut keys: Vec<&str> = cell
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            ["checked", "client", "client_version", "kind", "level"],
            "{cell}"
        );
    }
    assert_eq!(support[0]["client"], "claude");
    assert_eq!(support[0]["level"], "experimental");
    assert!(support[0]["checked"].is_null());
}

#[test]
fn the_user_guide_table_equals_the_compiled_table() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/book/support-levels.md");
    let page = std::fs::read_to_string(path).unwrap();
    let collapse = |s: &str| {
        s.split(' ')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let rows: Vec<String> = page.lines().map(collapse).collect();
    for cell in SUPPORT {
        let row = format!(
            "| {} | {} | {} | {} | {} |",
            cell.client,
            cell.kind,
            cell.level,
            cell.client_version.unwrap_or("-"),
            cell.checked.unwrap_or("-")
        );
        assert!(rows.contains(&row), "missing row: {row}");
    }
}
