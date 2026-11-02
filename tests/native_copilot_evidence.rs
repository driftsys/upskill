//! Once-only diagnostic evidence, without any native command or admission.
#![cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/copilot_probe.rs"]
mod fixture;
use fixture::{Fixture, private};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use upskill::native_delivery::copilot::probe::CopilotEvidenceError;

fn snapshots(root: &std::path::Path) -> Vec<(String, Vec<u8>, u32)> {
    let mut files = fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            assert!(path.is_file());
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            (
                name,
                fs::read(&path).unwrap(),
                fs::metadata(path).unwrap().permissions().mode() & 0o7777,
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[test]
fn initial_evidence_publishes_only_known_private_snapshots_and_bindings() {
    let fixture = Fixture::new();
    let preflight = fixture.capture().unwrap();
    let mut evidence = preflight.begin_evidence().unwrap();
    let area = fixture.root.join("evidence");
    assert_eq!(
        fs::metadata(&area).unwrap().permissions().mode() & 0o7777,
        0o700
    );
    let header: Value =
        serde_json::from_slice(&fs::read(area.join("header.json")).unwrap()).unwrap();
    assert_eq!(header["schema"], 1);
    assert_eq!(header["plan_sha256"], fixture.request.plan_sha256);
    assert_eq!(
        header["preparation_sha256"],
        fixture.request.preparation_sha256
    );
    assert_eq!(header["maximum_native_commands"], 11);
    assert_eq!(header["model_calls"], 0);
    assert_eq!(header["maximum_authorized_spending"], 0);
    assert_eq!(header["qualified"], false);
    assert_eq!(header["roots"].as_array().unwrap().len(), 4);
    for path in [
        &fixture.root,
        &fixture.parent,
        &fixture.root.join("home"),
        &fixture.root.join("project"),
    ] {
        let matches = header["roots"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|root| root["binding"]["selected"] == serde_json::json!(path))
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        let metadata = fs::metadata(path).unwrap();
        assert_eq!(matches[0]["binding"]["device"], metadata.dev());
        assert_eq!(matches[0]["binding"]["inode"], metadata.ino());
    }
    assert_eq!(header["fixtures"].as_array().unwrap().len(), 9);
    let preparation: Value =
        serde_json::from_slice(&fs::read(fixture.parent.join("preparation.json")).unwrap())
            .unwrap();
    for (row, approved) in header["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .zip(preparation["fixture_files"].as_array().unwrap())
    {
        assert_eq!(row["source"], approved["path"]);
        assert_eq!(row["sha256"], approved["sha256"]);
        assert_eq!(row["bytes"], approved["bytes"]);
        assert_eq!(row["mode"], 0o600);
        let stored = area.join(row["snapshot"].as_str().unwrap());
        assert_eq!(
            fs::read(stored).unwrap(),
            fs::read(fixture.root.join(approved["path"].as_str().unwrap())).unwrap()
        );
    }
    let stored = snapshots(&area);
    assert_eq!(stored.len(), 10);
    assert!(stored.iter().all(|(_, _, mode)| *mode == 0o600));
    evidence.verify_unchanged().unwrap();
    fixture.no_native_calls();
    assert!(!fixture.root.join("home/.copilot").exists());
    drop(evidence);
    assert_eq!(snapshots(&area), stored);
}

#[test]
fn used_evidence_excludes_another_captured_candidate_even_after_drop() {
    let fixture = Fixture::new();
    let first = fixture.capture().unwrap();
    let second = fixture.capture().unwrap();
    let evidence = first.begin_evidence().unwrap();
    let before = snapshots(&fixture.root.join("evidence"));
    assert!(second.begin_evidence().is_err());
    drop(evidence);
    assert!(first.begin_evidence().is_err());
    assert!(second.begin_evidence().is_err());
    assert!(fixture.capture().is_err());
    assert_eq!(snapshots(&fixture.root.join("evidence")), before);
    fixture.no_native_calls();
}

#[test]
fn evidence_refuses_changed_or_no_longer_unused_inputs_before_writes() {
    for change in ["bytes", "native-state", "manual-evidence"] {
        let fixture = Fixture::new();
        let preflight = fixture.capture().unwrap();
        match change {
            "bytes" => private(&fixture.root.join("source/SKILL.md"), b"changed", 0o600),
            "native-state" => fs::create_dir(fixture.root.join("home/.copilot")).unwrap(),
            "manual-evidence" => {
                fs::create_dir(fixture.root.join("evidence")).unwrap();
                private(&fixture.root.join("evidence/manual"), b"manual", 0o600);
            }
            _ => unreachable!(),
        }
        assert!(preflight.begin_evidence().is_err(), "accepted {change}");
        if change == "manual-evidence" {
            assert_eq!(
                fs::read(fixture.root.join("evidence/manual")).unwrap(),
                b"manual"
            );
        } else {
            assert!(!fixture.root.join("evidence").exists());
        }
        fixture.no_native_calls();
    }
}

#[test]
fn evidence_drift_permanently_stops_the_handle_without_repair() {
    for change in ["bytes", "mode", "identity", "symlink", "extra", "directory"] {
        let fixture = Fixture::new();
        let preflight = fixture.capture().unwrap();
        let mut evidence = preflight.begin_evidence().unwrap();
        let area = fixture.root.join("evidence");
        let path = area.join("header.json");
        let before = fs::read(&path).unwrap();
        match change {
            "bytes" => private(&path, b"changed", 0o600),
            "mode" => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
            "identity" | "symlink" => {
                let retained = fixture.parent.join("retained");
                fs::rename(&path, &retained).unwrap();
                if change == "identity" {
                    fs::copy(retained, &path).unwrap();
                } else {
                    symlink(retained, &path).unwrap();
                }
            }
            "extra" => private(&area.join("secret-canary"), b"unapproved", 0o600),
            "directory" => {
                fs::rename(&area, fixture.parent.join("moved-evidence")).unwrap();
                fs::create_dir(&area).unwrap();
                fs::set_permissions(&area, fs::Permissions::from_mode(0o700)).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(evidence.verify_unchanged().is_err(), "accepted {change}");
        // Repairing a reversible mode drift must not revive the stopped handle.
        if change == "mode" {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert_eq!(
            evidence.verify_unchanged(),
            Err(CopilotEvidenceError::Stopped)
        );
        fixture.no_native_calls();
    }
}

#[test]
fn evidence_verification_ignores_native_configuration_and_preserves_input_guards() {
    let fixture = Fixture::new();
    let preflight = fixture.capture().unwrap();
    let mut evidence = preflight.begin_evidence().unwrap();
    let before = snapshots(&fixture.root.join("evidence"));
    let native = fixture.root.join("home/.copilot");
    fs::create_dir(&native).unwrap();
    assert_cmd::Command::new("/usr/bin/mkfifo")
        .arg(native.join("config.json"))
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success();
    evidence.verify_unchanged().unwrap();
    assert_eq!(snapshots(&fixture.root.join("evidence")), before);
    private(
        &fixture.root.join("home/manual-preservation.txt"),
        b"changed",
        0o600,
    );
    assert!(evidence.verify_unchanged().is_err());
    assert_eq!(
        evidence.verify_unchanged(),
        Err(CopilotEvidenceError::Stopped)
    );
    fixture.no_native_calls();
}

#[test]
fn evidence_holds_all_selected_root_guards_until_drop() {
    use upskill::filesystem::QualifiedScope;
    use upskill::ownership::{InstallationRoot, OwnershipScope};
    let fixture = Fixture::new();
    let preflight = fixture.capture().unwrap();
    let scopes = [
        fixture.root.clone(),
        fixture.parent.clone(),
        fixture.root.join("home"),
        fixture.root.join("project"),
    ]
    .into_iter()
    .map(|path| {
        QualifiedScope::open(&InstallationRoot {
            scope: OwnershipScope::Project,
            path,
        })
        .unwrap()
    })
    .collect::<Vec<_>>();
    for scope in &scopes {
        drop(scope.try_lock_writer().unwrap());
    }
    let evidence = preflight.begin_evidence().unwrap();
    for scope in &scopes {
        assert!(scope.try_lock_writer().is_err());
    }
    drop(evidence);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    for scope in &scopes {
        loop {
            match scope.try_lock_writer() {
                Ok(guard) => {
                    drop(guard);
                    break;
                }
                Err(error) => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "guard retained after drop: {error}"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }
    fixture.no_native_calls();
}
