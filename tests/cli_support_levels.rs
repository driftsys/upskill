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

/// Runs `doctor` with `args` and returns its stdout.
fn doctor_text(args: &[&str]) -> String {
    let (_tmp, home, repo) = empty_repo();
    let out = common::upskill_cmd(&home)
        .current_dir(&repo)
        .args(args)
        .assert()
        .success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

fn doctor_json(args: &[&str]) -> serde_json::Value {
    serde_json::from_str(&doctor_text(args)).unwrap()
}

#[test]
fn doctor_prints_one_level_for_each_client_and_kind() {
    let text = doctor_text(&["doctor"]);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "Support levels:")
        .unwrap_or_else(|| panic!("no section in:\n{text}"));
    let rows: Vec<&str> = lines[at + 1..]
        .iter()
        .copied()
        .take_while(|l| l.starts_with("  "))
        .collect();
    assert_eq!(rows.len(), 12, "{text}");
    let order = [
        ("claude", "rule", "experimental"),
        ("claude", "skill", "experimental"),
        ("claude", "agent", "experimental"),
        ("copilot", "rule", "experimental"),
        ("copilot", "skill", "experimental"),
        ("copilot", "agent", "experimental"),
        ("opencode", "rule", "experimental"),
        ("opencode", "skill", "experimental"),
        ("opencode", "agent", "experimental"),
        ("codex", "rule", "unsupported"),
        ("codex", "skill", "unsupported"),
        ("codex", "agent", "unsupported"),
    ];
    for (row, (client, kind, level)) in rows.iter().zip(order) {
        assert_eq!(
            row.split_whitespace().collect::<Vec<_>>(),
            [client, kind, level],
            "{text}"
        );
    }
    for (row, cell) in rows.iter().zip(SUPPORT) {
        assert_eq!(
            row.split_whitespace().collect::<Vec<_>>(),
            [cell.client, &cell.kind.to_string(), &cell.level.to_string()],
            "{text}"
        );
    }
    let last_scope_report = lines
        .iter()
        .rposition(|l| l.starts_with("doctor:"))
        .unwrap();
    assert!(last_scope_report < at, "{text}");
}

#[test]
fn doctor_prints_the_support_levels_after_the_reports_of_an_unclean_repository() {
    let (tmp, home, repo) = empty_repo();
    let source = tmp.path().join("source/rules/r1");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("RULE.md"),
        "---\nschema: 1\nname: r1\ndescription: Use when testing\n---\n\nBody.\n",
    )
    .unwrap();
    common::upskill_cmd(&home)
        .current_dir(&repo)
        .args([
            "add",
            tmp.path().join("source").to_str().unwrap(),
            "--claude",
            "--project",
        ])
        .assert()
        .success();
    std::fs::remove_file(repo.join(".claude/rules/r1.md")).unwrap();
    let out = common::upskill_cmd(&home)
        .current_dir(&repo)
        .arg("doctor")
        .assert()
        .code(1);
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let problem = text.find("missing per-client outputs").unwrap();
    let global = text.find("global (").unwrap();
    let heading = text.find("Support levels:").unwrap();
    assert!(problem < global && global < heading, "{text}");
    assert_eq!(text.matches("Support levels:").count(), 1, "{text}");
}

#[test]
fn doctor_prints_the_support_levels_after_the_scope_reports() {
    let text = doctor_text(&["doctor"]);
    let heading = text.find("Support levels:").unwrap();
    let project = text.find("project (").unwrap();
    let global = text.find("global (").unwrap();
    assert!(project < global && global < heading, "{text}");
}

#[test]
fn doctor_with_a_global_flag_prints_the_support_levels() {
    let text = doctor_text(&["doctor", "--global"]);
    assert!(text.contains("Support levels:"), "{text}");
}

#[test]
fn doctor_json_with_a_scope_flag_holds_the_support_table() {
    for flag in ["--project", "--global"] {
        let v = doctor_json(&["doctor", "--json", flag]);
        assert_eq!(v["support"].as_array().unwrap().len(), 12, "{flag}: {v}");
    }
}

#[test]
fn quiet_doctor_does_not_print_the_support_levels() {
    let text = doctor_text(&["--quiet", "doctor"]);
    assert!(!text.contains("Support levels:"), "{text}");
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
    // The rows of the cell table: five cells, not the header, not the separator.
    let page_rows: Vec<String> = page
        .lines()
        .map(collapse)
        .filter(|l| l.starts_with('|') && l.matches('|').count() == 6)
        .filter(|l| !l.starts_with("| Client |") && !l.starts_with("| -"))
        .collect();
    let compiled: Vec<String> = SUPPORT
        .iter()
        .map(|cell| {
            format!(
                "| {} | {} | {} | {} | {} |",
                cell.client,
                cell.kind,
                cell.level,
                cell.client_version.unwrap_or("-"),
                cell.checked.unwrap_or("-")
            )
        })
        .collect();
    assert_eq!(page_rows.len(), SUPPORT.len(), "{page_rows:#?}");
    assert_eq!(page_rows, compiled);
}
