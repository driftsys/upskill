//! Real qualified private storage; no native adapter or transaction admission.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use upskill::filesystem::QualifiedScope;
use upskill::native_delivery::state::{EvidenceBlob, EvidenceKind, EvidenceReference};
use upskill::native_delivery::store::NativeEvidenceStore;
use upskill::ownership::{InstallationRoot, OwnershipScope};

fn root(path: &std::path::Path) -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::User,
        path: path.into(),
    }
}
fn blob() -> EvidenceBlob {
    // Independent protocol golden for three binary bytes, not a store-derived hash.
    EvidenceBlob {
        reference: EvidenceReference {
            kind: EvidenceKind::NativePrestate,
            digest: "5eadfd350ace3da982d31480c7aa2f9f5d3b9fda1838b35196e9068c83f969bc".into(),
            length: 3,
        },
        bytes: vec![0, 255, 1],
    }
}
const DIR: &str = ".upskill/native-evidence";
fn name() -> String {
    format!("native-prestate-{}", blob().reference.digest)
}

#[test]
fn private_store_round_trip_is_immutable_and_reopens_in_same_namespace() {
    let tmp = tempfile::tempdir().unwrap();
    let root = root(tmp.path());
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    assert!(
        NativeEvidenceStore::open(&root, &scope, &guard)
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
    let namespace = store.namespace().to_owned();
    use sha2::{Digest, Sha256};
    let owner = fs::read(tmp.path().join(DIR).join("owner.json")).unwrap();
    let mut framed = b"upskill:native-evidence-namespace:v1\0".to_vec();
    framed.extend_from_slice(&(owner.len() as u64).to_be_bytes());
    framed.extend_from_slice(&owner);
    let expected: String = Sha256::digest(&framed)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(namespace, expected);
    let independent = tempfile::tempdir().unwrap();
    let independent_root = self::root(independent.path());
    let independent_scope = QualifiedScope::open(&independent_root).unwrap();
    let independent_guard = independent_scope.try_lock_writer().unwrap();
    let independent_store =
        NativeEvidenceStore::initialize(&independent_root, &independent_scope, &independent_guard)
            .unwrap();
    assert_ne!(namespace, independent_store.namespace());
    assert!(store.read(&blob().reference).unwrap().is_none());
    store.put(&blob()).unwrap();
    store.put(&blob()).unwrap();
    assert_eq!(store.read(&blob().reference).unwrap(), Some(blob()));
    assert_eq!(
        fs::metadata(tmp.path().join(DIR))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o700
    );
    for file in fs::read_dir(tmp.path().join(DIR)).unwrap() {
        assert_eq!(
            file.unwrap().metadata().unwrap().permissions().mode() & 0o7777,
            0o600
        );
    }
    drop(store);
    let reopened = NativeEvidenceStore::open(&root, &scope, &guard)
        .unwrap()
        .unwrap();
    assert_eq!(reopened.namespace(), namespace);
    assert_eq!(reopened.read(&blob().reference).unwrap(), Some(blob()));
    assert!(!tmp.path().join(".upskill-lock.json").exists());
}

#[test]
fn invalid_references_and_corrupt_payloads_never_publish_or_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let root = root(tmp.path());
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
    for digest in [
        "../outside".to_owned(),
        String::new(),
        "ABCDEF".into(),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
    ] {
        let mut bad = blob();
        bad.reference.digest = digest;
        assert!(store.put(&bad).is_err());
        assert!(store.read(&bad.reference).is_err());
    }
    for field in ["kind", "length", "bytes"] {
        let mut bad = blob();
        match field {
            "kind" => bad.reference.kind = EvidenceKind::ConfigurationSnapshot,
            "length" => bad.reference.length += 1,
            _ => bad.bytes.push(3),
        }
        assert!(store.put(&bad).is_err());
    }
    assert_eq!(fs::read_dir(tmp.path().join(DIR)).unwrap().count(), 2);
    store.put(&blob()).unwrap();
    let path = tmp.path().join(DIR).join(name());
    fs::write(&path, b"bad").unwrap();
    assert!(store.read(&blob().reference).is_err());
    assert!(store.put(&blob()).is_err());
    assert_eq!(fs::read(path).unwrap(), b"bad");
}

#[test]
fn copied_store_and_replaced_namespace_are_rejected_without_adoption() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let ra = root(a.path());
    let rb = root(b.path());
    let sa = QualifiedScope::open(&ra).unwrap();
    let sb = QualifiedScope::open(&rb).unwrap();
    let ga = sa.try_lock_writer().unwrap();
    let gb = sb.try_lock_writer().unwrap();
    let store = NativeEvidenceStore::initialize(&ra, &sa, &ga).unwrap();
    store.put(&blob()).unwrap();
    fs::create_dir_all(b.path().join(DIR)).unwrap();
    fs::set_permissions(b.path().join(DIR), fs::Permissions::from_mode(0o700)).unwrap();
    for item in fs::read_dir(a.path().join(DIR)).unwrap() {
        let item = item.unwrap();
        fs::copy(item.path(), b.path().join(DIR).join(item.file_name())).unwrap();
    }
    assert!(NativeEvidenceStore::open(&rb, &sb, &gb).is_err());
    assert!(NativeEvidenceStore::initialize(&rb, &sb, &gb).is_err());
    fs::rename(a.path().join(DIR), a.path().join("retained")).unwrap();
    fs::create_dir(a.path().join(DIR)).unwrap();
    fs::set_permissions(a.path().join(DIR), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(store.read(&blob().reference).is_err());
    assert!(store.put(&blob()).is_err());
    assert_eq!(fs::read_dir(a.path().join(DIR)).unwrap().count(), 0);
}

#[test]
fn unsafe_storage_links_permissions_and_wrong_guard_reject() {
    for case in ["symlink", "hardlink", "mode", "ignore", "owner"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = root(tmp.path());
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        let other = tempfile::tempdir().unwrap();
        let other_scope = QualifiedScope::open(&self::root(other.path())).unwrap();
        let other_guard = other_scope.try_lock_writer().unwrap();
        assert!(NativeEvidenceStore::initialize(&root, &scope, &other_guard).is_err());
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
        let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
        store.put(&blob()).unwrap();
        let path = tmp.path().join(DIR).join(name());
        match case {
            "symlink" => {
                fs::remove_file(&path).unwrap();
                fs::write(other.path().join("manual"), &blob().bytes).unwrap();
                symlink(other.path().join("manual"), &path).unwrap();
            }
            "hardlink" => {
                fs::hard_link(&path, other.path().join("manual")).unwrap();
            }
            "mode" => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
            "ignore" => fs::write(tmp.path().join(DIR).join(".gitignore"), b"changed").unwrap(),
            _ => fs::write(tmp.path().join(DIR).join("owner.json"), b"{}").unwrap(),
        }
        assert!(store.read(&blob().reference).is_err(), "accepted {case}");
        assert!(store.put(&blob()).is_err(), "overwrote {case}");
    }
}

#[test]
fn project_scope_and_unowned_directories_are_not_initialized() {
    let tmp = tempfile::tempdir().unwrap();
    let mut root = root(tmp.path());
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    root.scope = OwnershipScope::Project;
    assert!(NativeEvidenceStore::initialize(&root, &scope, &guard).is_err());
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    root.scope = OwnershipScope::User;
    fs::create_dir_all(tmp.path().join(DIR)).unwrap();
    fs::set_permissions(tmp.path().join(DIR), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(tmp.path().join(DIR).join("manual"), b"keep").unwrap();
    assert!(NativeEvidenceStore::initialize(&root, &scope, &guard).is_err());
    assert_eq!(
        fs::read(tmp.path().join(DIR).join("manual")).unwrap(),
        b"keep"
    );
    assert_eq!(fs::read_dir(tmp.path().join(DIR)).unwrap().count(), 1);
}

#[test]
fn ignored_private_store_rejects_force_tracked_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let mut command = std::process::Command::new("git");
        command.current_dir(tmp.path()).args(args);
        for (key, _) in
            std::env::vars_os().filter(|(key, _)| key.as_encoded_bytes().starts_with(b"GIT_"))
        {
            command.env_remove(key);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    git(&["init", "-q"]);
    let root = root(tmp.path());
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
    store.put(&blob()).unwrap();
    let ignored = git(&["ls-files", "--others", "--exclude-standard", "--", DIR]);
    assert!(ignored.is_empty());
    git(&["-c", "core.hooksPath=/dev/null", "add", "-f", "--", DIR]);
    assert!(!git(&["ls-files", "--", DIR]).is_empty());
    assert!(store.read(&blob().reference).is_err());
    assert!(store.put(&blob()).is_err());
    assert!(NativeEvidenceStore::open(&root, &scope, &guard).is_err());
}

#[test]
fn tracked_evidence_is_detected_through_an_ancestor_alias() {
    let repo = tempfile::tempdir().unwrap();
    let aliases = tempfile::tempdir().unwrap();
    fs::create_dir_all(repo.path().join("container/user")).unwrap();
    symlink(repo.path().join("container"), aliases.path().join("alias")).unwrap();
    let git = |args: &[&str]| {
        let mut command = std::process::Command::new("git");
        command.current_dir(repo.path()).args(args);
        for (key, _) in
            std::env::vars_os().filter(|(key, _)| key.as_encoded_bytes().starts_with(b"GIT_"))
        {
            command.env_remove(key);
        }
        assert!(command.status().unwrap().success());
    };
    git(&["init", "-q"]);
    let root = root(&aliases.path().join("alias/user"));
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let store = NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
    store.put(&blob()).unwrap();
    git(&[
        "-c",
        "core.hooksPath=/dev/null",
        "add",
        "-f",
        "--",
        "container/user/.upskill/native-evidence",
    ]);
    assert!(
        store.read(&blob().reference).is_err(),
        "ancestor alias concealed tracked private evidence"
    );
}

#[test]
fn unsafe_parent_directories_reject_before_creating_private_entries() {
    for case in ["root-mode", "parent-mode", "parent-symlink"] {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("manual"), b"keep").unwrap();
        match case {
            "root-mode" => {
                fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o777)).unwrap()
            }
            "parent-mode" => {
                fs::create_dir(tmp.path().join(".upskill")).unwrap();
                fs::set_permissions(
                    tmp.path().join(".upskill"),
                    fs::Permissions::from_mode(0o777),
                )
                .unwrap();
            }
            _ => symlink(outside.path(), tmp.path().join(".upskill")).unwrap(),
        }
        let root = root(tmp.path());
        let scope = QualifiedScope::open(&root).unwrap();
        let guard = scope.try_lock_writer().unwrap();
        assert!(
            NativeEvidenceStore::initialize(&root, &scope, &guard).is_err(),
            "accepted {case}"
        );
        assert!(!tmp.path().join(DIR).exists());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 1);
        assert_eq!(fs::read(outside.path().join("manual")).unwrap(), b"keep");
    }
}

#[test]
fn different_selected_root_and_physical_alias_reject_before_creation() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let aliases = tempfile::tempdir().unwrap();
    fs::create_dir(a.path().join("child")).unwrap();
    symlink(a.path(), aliases.path().join("alias")).unwrap();
    let root = root(&a.path().join("child"));
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    for wrong in [
        self::root(b.path()),
        self::root(&aliases.path().join("alias/child")),
    ] {
        assert!(NativeEvidenceStore::initialize(&wrong, &scope, &guard).is_err());
        assert_eq!(fs::read_dir(&root.path).unwrap().count(), 0);
        assert_eq!(fs::read_dir(b.path()).unwrap().count(), 0);
    }
    NativeEvidenceStore::initialize(&root, &scope, &guard).unwrap();
}
