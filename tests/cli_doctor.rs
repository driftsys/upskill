//! ATDD tests for `upskill doctor`.
//!
//! Verifies the item buckets:
//! - missing per-client output files (reinstall fixes)
//! - modified and unreadable recorded files
//! - SSOT hash drift on `local:` sources (update fixes)
//! - lockfile entries with no recoverable source (manual remove)
//!
//! Also verifies plugin reconciliation per issue #151 / ADR-0008:
//! - skipped plugins (warn-skip outcomes) surfaced as informational warnings
//! - installed plugins missing from the client surfaced as drift (exit 1)
//!
//! Doctor never fetches; remote-source drift detection is `update --dry-run`.
//!
//! The JSON tests pass `--project` and read `scopes[0]`: without a scope flag
//! `doctor` reports both scopes. `tests/cli_scope.rs` covers the both-scope
//! report.

mod common;

use std::fs;
use std::path::Path;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn stage_source(source: &Path) {
    let from = format!("{FIXTURES}/items");
    copy_dir_all(Path::new(&from), source).unwrap();
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let to_path = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &to_path)?;
        } else {
            fs::copy(entry.path(), &to_path)?;
        }
    }
    Ok(())
}

fn install(target: &Path, source: &Path, home: &Path) {
    common::upskill_cmd(home)
        .current_dir(target)
        .args(["add", source.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn doctor_clean_install_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains("doctor: clean"), "expected clean: {out}");
}

#[test]
fn doctor_empty_lockfile_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success();
}

#[test]
fn doctor_detects_missing_output_files() {
    // Install, then delete one rendered output behind upskill's back —
    // the bucket-1 path. Doctor must exit 1 and name the missing file.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let stolen = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    fs::remove_file(&stolen).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("missing per-client outputs") && out.contains("create-api-endpoint"),
        "expected missing-output report: {out}"
    );
    assert!(
        out.contains(".claude/skills/create-api-endpoint/SKILL.md"),
        "expected the missing path listed: {out}"
    );
}

#[test]
fn doctor_detects_ssot_hash_drift() {
    // Install from a local source, mutate the SSOT in place — the
    // bucket-2 path. Doctor must exit 1 and surface the stale-hash bucket.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let skill_md = source.join("create-api-endpoint/SKILL.md");
    let original = fs::read_to_string(&skill_md).unwrap();
    fs::write(&skill_md, format!("{original}\n<!-- mutation -->\n")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("SSOT hash drift") && out.contains("create-api-endpoint"),
        "expected stale-hash bucket: {out}"
    );
    assert!(
        out.contains("upskill update"),
        "should suggest the fix command: {out}"
    );
}

#[test]
fn doctor_detects_orphan_when_local_path_gone() {
    // Install from a local source, remove the source dir — the bucket-3
    // path. Doctor must exit 1 and surface every entry as orphan.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    fs::remove_dir_all(&source).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("no recoverable source"),
        "expected orphan bucket: {out}"
    );
    assert!(out.contains("local path gone"), "should explain why: {out}");
    assert!(
        out.contains("upskill remove"),
        "should suggest the fix command: {out}"
    );
}

#[test]
fn doctor_detects_orphan_when_item_removed_from_source() {
    // Install from a local source, remove just one item dir from the
    // SSOT (leaving the source root). Doctor must report that one item
    // as orphan with "item not in source", and other items stay clean.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    fs::remove_dir_all(source.join("create-api-endpoint")).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("create-api-endpoint") && out.contains("item not in source"),
        "expected orphan with reason: {out}"
    );
}

#[test]
fn doctor_json_clean_install_emits_empty_buckets() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor", "--json", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout was:\n{stdout}"));
    let v = &document["scopes"][0];
    for bucket in [
        "missing_outputs",
        "modified_files",
        "stale_hashes",
        "orphan_entries",
    ] {
        let arr = v[bucket].as_array().unwrap_or_else(|| {
            panic!("expected {bucket} to be an array, got: {v}");
        });
        assert!(arr.is_empty(), "expected empty {bucket}, got: {arr:?}");
    }
}

#[test]
fn doctor_json_orphan_entry_has_kebab_case_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    fs::remove_dir_all(&source).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor", "--json", "--project"])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout was:\n{stdout}"));
    let v = &document["scopes"][0];
    let orphans = v["orphan_entries"].as_array().unwrap();
    assert!(!orphans.is_empty(), "expected at least one orphan entry");
    for o in orphans {
        assert_eq!(o["reason"].as_str(), Some("local-path-gone"));
        assert!(!o["kind"].as_str().unwrap_or("").is_empty());
        assert!(o["name"].is_string());
        assert!(o["source"].as_str().unwrap().starts_with("local:"));
    }
}

#[test]
fn doctor_json_missing_output_lists_files() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let stolen = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    fs::remove_file(&stolen).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor", "--json", "--project"])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout was:\n{stdout}"));
    let v = &document["scopes"][0];
    let missing = v["missing_outputs"].as_array().unwrap();
    assert_eq!(missing.len(), 1);
    let entry = &missing[0];
    assert_eq!(entry["name"].as_str(), Some("create-api-endpoint"));
    assert_eq!(entry["kind"].as_str(), Some("skill"));
    let files: Vec<&str> = entry["missing_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(
        files.iter().any(|p| p.contains("create-api-endpoint")),
        "expected missing path entry, got: {files:?}"
    );
}

// ---------------------------------------------------------------------------
// Plugin reconciliation tests (issue #151 / ADR-0008)
// ---------------------------------------------------------------------------

/// Write a minimal lockfile JSON with one skipped plugin.
fn write_lockfile_with_skipped_plugin(target: &Path, plugin_name: &str, client: &str) {
    let content = serde_json::json!({
        "schema": 1,
        "items": [],
        "bundles": [],
        "plugins": [{
            "name": plugin_name,
            "client": client,
            "identifier": format!("{plugin_name}@some-source"),
            "scope": "project",
            "bundle": "test-bundle",
            "status": "skipped"
        }]
    });
    fs::write(
        target.join(".upskill-lock.json"),
        serde_json::to_string_pretty(&content).unwrap(),
    )
    .unwrap();
}

/// Write a minimal lockfile JSON with one installed plugin.
fn write_lockfile_with_installed_plugin(
    target: &Path,
    plugin_name: &str,
    client: &str,
    identifier: &str,
) {
    let content = serde_json::json!({
        "schema": 1,
        "items": [],
        "bundles": [],
        "plugins": [{
            "name": plugin_name,
            "client": client,
            "identifier": identifier,
            "bundle": "test-bundle",
            "status": "installed"
        }]
    });
    fs::write(
        target.join(".upskill-lock.json"),
        serde_json::to_string_pretty(&content).unwrap(),
    )
    .unwrap();
}

/// Create a fake CLI binary in `bin_dir` that prints `output` to stdout.
/// Cross-platform: writes a `#!/bin/sh` heredoc script on Unix and a
/// `.bat` file on Windows (so `Command::new(name)` resolves it via PATHEXT).
fn write_fake_cli(bin_dir: &Path, name: &str, output: &str) {
    #[cfg(unix)]
    {
        let script = bin_dir.join(name);
        // Use a heredoc so literal newlines inside `output` are preserved exactly.
        // `printf '%s\n' {output:?}` would debug-escape `\n` to a backslash-n pair.
        fs::write(
            &script,
            format!("#!/bin/sh\ncat <<'UPSKILLEOF'\n{output}\nUPSKILLEOF\n"),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
    }
    #[cfg(windows)]
    {
        // Windows: write a .bat file — Command::new("code") resolves code.bat via PATHEXT.
        let script = bin_dir.join(format!("{name}.bat"));
        let mut content = String::from("@echo off\r\n");
        for line in output.lines() {
            // `echo.` is the batch idiom for a blank line; otherwise `echo <text>`.
            if line.is_empty() {
                content.push_str("echo.\r\n");
            } else {
                content.push_str(&format!("echo {line}\r\n"));
            }
        }
        fs::write(&script, content).unwrap();
    }
}

/// Prepend `dir` to the current PATH using the platform-correct separator.
fn prepend_to_path(dir: &Path) -> std::ffi::OsString {
    let mut entries = vec![dir.to_owned()];
    if let Some(val) = std::env::var_os("PATH") {
        entries.extend(std::env::split_paths(&val));
    }
    std::env::join_paths(entries).expect("join paths")
}

#[test]
fn doctor_skipped_plugin_exits_zero_and_reports_it() {
    // A lockfile with a Skipped plugin should exit 0 (not drift) but still
    // surface the plugin in the output so it is not silently lost.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    write_lockfile_with_skipped_plugin(tmp.path(), "superpowers", "claude");

    let assert = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success(); // exit 0 — skipped plugins are warnings, not drift
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("superpowers"),
        "expected skipped plugin name in output: {out}"
    );
    assert!(
        out.contains("never installed") || out.contains("skipped"),
        "expected warn-skip context in output: {out}"
    );
}

#[test]
fn doctor_installed_plugin_missing_from_client_exits_one() {
    // A lockfile records a vscode plugin as Installed, but the fake `code`
    // binary returns an empty extension list.  Doctor must exit 1 and report
    // the plugin as missing from the client.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    let home = bin_dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    write_lockfile_with_installed_plugin(
        tmp.path(),
        "superpowers",
        "vscode",
        "anthropic.superpowers",
    );
    // Fake `code` that lists no extensions (empty stdout).
    write_fake_cli(bin_dir.path(), "code", "");

    let assert = common::upskill_cmd(&home)
        .env("PATH", prepend_to_path(bin_dir.path()))
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .failure()
        .code(1);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        out.contains("superpowers"),
        "expected missing plugin name in output: {out}"
    );
    assert!(
        out.contains("missing") || out.contains("not installed"),
        "expected missing-plugin context in output: {out}"
    );
}

#[test]
fn doctor_installed_plugin_present_in_client_exits_zero() {
    // A lockfile records a vscode plugin as Installed. The fake `code` binary
    // returns a list that includes the extension.  Doctor must exit 0.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let bin_dir = tempfile::tempdir().unwrap();
    let home = bin_dir.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    write_lockfile_with_installed_plugin(
        tmp.path(),
        "superpowers",
        "vscode",
        "anthropic.superpowers",
    );
    // Fake `code` that lists the extension.
    write_fake_cli(
        bin_dir.path(),
        "code",
        "anthropic.superpowers\nms-python.python",
    );

    common::upskill_cmd(&home)
        .env("PATH", prepend_to_path(bin_dir.path()))
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success();
}

#[test]
fn doctor_json_includes_skipped_plugins_bucket() {
    // --json output must include a skipped_plugins array even when empty.
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor", "--json", "--project"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout was:\n{stdout}"));
    let v = &document["scopes"][0];
    // New plugin buckets must always appear in JSON output.
    assert!(
        v.get("skipped_plugins").is_some(),
        "expected skipped_plugins key in JSON: {v}"
    );
    assert!(
        v.get("missing_plugins").is_some(),
        "expected missing_plugins key in JSON: {v}"
    );
}

#[test]
fn doctor_reports_modified_files() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);

    let edited = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    let mut bytes = fs::read(&edited).unwrap();
    bytes.extend_from_slice(b"local edit\n");
    fs::write(&edited, bytes).unwrap();

    let assert = common::upskill_cmd(&home)
        .current_dir(&target)
        .args(["doctor", "--json", "--project"])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let v = &document["scopes"][0];
    let modified = v["modified_files"].as_array().unwrap();
    assert_eq!(modified.len(), 1, "{stdout}");
    assert_eq!(modified[0]["name"], "create-api-endpoint");
    assert_eq!(modified[0]["kind"], "skill");
    assert_eq!(
        modified[0]["files"],
        serde_json::json!([".claude/skills/create-api-endpoint/SKILL.md"])
    );
    assert!(v["missing_outputs"].as_array().unwrap().is_empty());
}

/// A target with the fixture items installed for every client. Returns
/// `(tmp, home, target)`.
fn installed() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);
    (tmp, home, target)
}

fn doctor_json(target: &Path, home: &Path) -> (Option<i32>, serde_json::Value) {
    let out = common::upskill_cmd(home)
        .current_dir(target)
        .args(["doctor", "--json", "--project"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\nstdout: {stdout}\nstderr: {stderr}"));
    (out.status.code(), document["scopes"][0].clone())
}

fn bucket_paths(v: &serde_json::Value, bucket: &str, field: &str, name: &str) -> Vec<String> {
    v[bucket]
        .as_array()
        .unwrap_or_else(|| panic!("no {bucket} in {v}"))
        .iter()
        .filter(|e| e["name"] == name)
        .flat_map(|e| e[field].as_array().unwrap().clone())
        .map(|p| p.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn doctor_reports_a_recorded_file_under_a_non_directory_as_missing() {
    let (_tmp, home, target) = installed();
    let dir = target.join(".claude/skills/create-api-endpoint");
    fs::remove_dir_all(&dir).unwrap();
    fs::write(&dir, "not a directory\n").unwrap();

    let (code, v) = doctor_json(&target, &home);
    assert_eq!(code, Some(1), "{v}");
    assert!(
        bucket_paths(
            &v,
            "missing_outputs",
            "missing_files",
            "create-api-endpoint"
        )
        .contains(&".claude/skills/create-api-endpoint/SKILL.md".to_string()),
        "{v}"
    );
}

#[cfg(unix)]
#[test]
fn doctor_reports_an_unreadable_recorded_file_and_continues() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, home, target) = installed();
    let locked = target.join(".claude/skills/create-api-endpoint/SKILL.md");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&locked).is_ok() {
        // Running as root: the permission has no effect.
        return;
    }
    fs::remove_file(target.join(".claude/rules/api-conventions.md")).unwrap();

    let (code, v) = doctor_json(&target, &home);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        bucket_paths(&v, "unreadable_files", "files", "create-api-endpoint"),
        vec![".claude/skills/create-api-endpoint/SKILL.md".to_string()],
        "{v}"
    );
    assert!(
        bucket_paths(&v, "missing_outputs", "missing_files", "api-conventions")
            .contains(&".claude/rules/api-conventions.md".to_string()),
        "{v}"
    );
}

#[test]
fn doctor_reports_a_missing_recorded_resource_file() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    stage_source(&source);
    let skill = source.join("with-script");
    fs::create_dir_all(skill.join("scripts")).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nschema: 1\nname: with-script\ndescription: A skill with a script\n---\nRun scripts/run.sh.\n",
    )
    .unwrap();
    fs::write(skill.join("scripts/run.sh"), "#!/bin/sh\necho hi\n").unwrap();
    fs::create_dir_all(target.join(".git")).unwrap();
    install(&target, &source, &home);
    fs::remove_file(target.join(".claude/skills/with-script/scripts/run.sh")).unwrap();

    let (code, v) = doctor_json(&target, &home);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        bucket_paths(&v, "missing_outputs", "missing_files", "with-script"),
        vec![".claude/skills/with-script/scripts/run.sh".to_string()],
        "{v}"
    );
}

#[test]
fn doctor_uses_the_layout_check_for_an_item_without_a_record() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let lock = serde_json::json!({
        "schema": 2,
        "items": [{
            "kind": "rule",
            "name": "api-conventions",
            "source": "github:owner/repo",
            "clients": ["claude"]
        }],
        "bundles": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    let (code, v) = doctor_json(tmp.path(), &home);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        bucket_paths(&v, "missing_outputs", "missing_files", "api-conventions"),
        vec![".claude/rules/api-conventions.md".to_string()],
        "{v}"
    );
}

#[test]
fn doctor_does_not_adopt_a_schema_1_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let rule = tmp.path().join(".claude/rules/api-conventions.md");
    fs::create_dir_all(rule.parent().unwrap()).unwrap();
    fs::write(&rule, "hand-written rule\n").unwrap();
    let lock = serde_json::json!({
        "schema": 1,
        "items": [{
            "kind": "rule",
            "name": "api-conventions",
            "source": "github:owner/repo",
            "clients": ["claude"]
        }],
        "bundles": []
    });
    let bytes = serde_json::to_vec_pretty(&lock).unwrap();
    fs::write(tmp.path().join(".upskill-lock.json"), &bytes).unwrap();

    common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success();

    assert_eq!(
        fs::read(tmp.path().join(".upskill-lock.json")).unwrap(),
        bytes
    );
    assert!(!tmp.path().join(".upskill-lock.json.v1").exists());
}

#[test]
fn doctor_skips_a_recorded_path_that_remove_would_never_delete() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let lock = serde_json::json!({
        "schema": 2,
        "items": [{
            "kind": "rule",
            "name": "api-conventions",
            "source": "github:owner/repo",
            "clients": ["claude"],
            "files": [{
                "client": "claude",
                "path": "docs/api-conventions.md",
                "sha256": "00"
            }]
        }],
        "bundles": []
    });
    fs::write(
        tmp.path().join(".upskill-lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    let (code, v) = doctor_json(tmp.path(), &home);
    assert_eq!(code, Some(0), "{v}");
    assert!(v["missing_outputs"].as_array().unwrap().is_empty(), "{v}");
}
