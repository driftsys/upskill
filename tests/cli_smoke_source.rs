//! The source that `scripts/qualify-native-instructions.py --smoke` installs
//! must be valid for the real `upskill add`. The script writes the source; this
//! test installs it for Claude Code and Copilot and checks the six outputs.
//! The test starts no AI client.

#![cfg(unix)]

mod common;

use std::path::Path;
use std::process::Command;

const NAME: &str = "upskill-smoke-test";
const MARKERS: [(&str, &str); 3] = [
    ("rule", "MARK_RULE_X"),
    ("skill", "MARK_SKILL_X"),
    ("agent", "MARK_AGENT_X"),
];

/// Asks the script to write the smoke source into `root`. Returns false when
/// `python3` is not on PATH.
fn write_source(root: &Path) -> bool {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/qualify-native-instructions.py");
    let code = "import importlib.util, sys\n\
        spec = importlib.util.spec_from_file_location('qualification', sys.argv[1])\n\
        q = importlib.util.module_from_spec(spec)\n\
        spec.loader.exec_module(q)\n\
        q.smoke_source(sys.argv[2], sys.argv[3], {'rule': 'MARK_RULE_X', 'skill': 'MARK_SKILL_X', 'agent': 'MARK_AGENT_X'})\n";
    match Command::new("python3")
        .args(["-I", "-c", code])
        .arg(&script)
        .arg(root)
        .arg(NAME)
        .output()
    {
        Ok(out) => {
            assert!(
                out.status.success(),
                "the script failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => panic!("cannot run python3: {e}"),
    }
}

#[test]
fn the_smoke_source_installs_each_kind_for_claude_and_copilot() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    if !write_source(&source) {
        eprintln!("skipped: python3 is not on PATH");
        return;
    }
    let home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(repo.join(".git")).unwrap();

    common::upskill_cmd(&home)
        .current_dir(&repo)
        .args([
            "add",
            source.to_str().unwrap(),
            "--claude",
            "--copilot",
            "--project",
        ])
        .assert()
        .success();

    let outputs = [
        (format!(".claude/rules/{NAME}.md"), "rule"),
        (
            format!(".github/instructions/{NAME}.instructions.md"),
            "rule",
        ),
        (format!(".claude/skills/{NAME}/SKILL.md"), "skill"),
        (format!(".github/skills/{NAME}/SKILL.md"), "skill"),
        (format!(".claude/agents/{NAME}.md"), "agent"),
        (format!(".github/agents/{NAME}.agent.md"), "agent"),
    ];
    for (path, kind) in &outputs {
        let text = std::fs::read_to_string(repo.join(path))
            .unwrap_or_else(|e| panic!("{path} was not written: {e}"));
        for (other, marker) in MARKERS {
            assert_eq!(text.contains(marker), other == *kind, "{path} and {other}");
        }
    }
}
