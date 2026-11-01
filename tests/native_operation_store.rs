//! Durable intent and every-root exclusion; no application or native effects.
#![cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;
use upskill::filesystem::QualifiedScope;
use upskill::native_delivery::journal::{NativeOperationCapture, NativeOperationStore};
use upskill::native_delivery::roots::{NativeRoot, lock_native_roots};
use upskill::ownership::{InstallationRoot, OwnershipScope};

fn root(path: &std::path::Path, scope: OwnershipScope) -> InstallationRoot {
    InstallationRoot {
        scope,
        path: path.into(),
    }
}

#[test]
fn prepare_records_private_intent_and_every_root_marker_without_applying_state() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let extra = tempfile::tempdir().unwrap();
    let primary = root(project.path(), OwnershipScope::Project);
    let private = root(home.path(), OwnershipScope::User);
    let additional = root(extra.path(), OwnershipScope::Project);
    let ps = QualifiedScope::open(&primary).unwrap();
    let hs = QualifiedScope::open(&private).unwrap();
    let es = QualifiedScope::open(&additional).unwrap();
    let locks = lock_native_roots(&[
        NativeRoot {
            root: primary.clone(),
            scope: &ps,
        },
        NativeRoot {
            root: private.clone(),
            scope: &hs,
        },
        NativeRoot {
            root: additional,
            scope: &es,
        },
    ])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    let capture = NativeOperationCapture::default();
    let operation = store.prepare(&capture).unwrap();
    let inspection = store.inspect(operation.id()).unwrap();
    assert!(inspection.markers_complete);
    assert_eq!(inspection.intent.capture, capture);
    assert!(inspection.intent.prior_state.is_none());
    assert_eq!(inspection.intent.roots, locks.bindings());
    assert!(!project.path().join(".upskill-lock.json").exists());
    for path in [project.path(), home.path(), extra.path()] {
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(path.join(".upskill/native-pending/pending.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(marker["operation_id"], operation.id());
        let expected_root = locks
            .bindings()
            .iter()
            .find(|binding| binding.root.path == path)
            .unwrap();
        let actual_root: upskill::native_delivery::roots::NativeRootBinding =
            serde_json::from_value(marker["root"].clone()).unwrap();
        assert_eq!(&actual_root, expected_root);
        use sha2::{Digest, Sha256};
        let owner = fs::read(path.join(".upskill/native-pending/owner.json")).unwrap();
        let mut framed = b"upskill:native-marker-namespace:v1\0".to_vec();
        framed.extend_from_slice(&(owner.len() as u64).to_be_bytes());
        framed.extend_from_slice(&owner);
        let expected_namespace: String = Sha256::digest(&framed)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(marker["marker_namespace"], expected_namespace);
        assert_eq!(marker["namespace"], store.context().namespace);
        assert_eq!(
            marker["journal_name"],
            format!("journal-{}.json", operation.id())
        );
        assert_eq!(
            marker["private_root"]["path"],
            private.path.to_str().unwrap()
        );
        assert!(!marker.to_string().contains("captured_prestate"));
    }
    assert!(store.prepare(&capture).is_err());
    let intent_path = home
        .path()
        .join(".upskill/native-evidence")
        .join(format!("intent-{}.yaml", operation.id()));
    let mut bytes = fs::read(&intent_path).unwrap();
    bytes.push(b'\n');
    fs::write(&intent_path, &bytes).unwrap();
    let error = store.inspect(operation.id()).unwrap_err();
    assert!(format!("{error:#}").contains("intent digest changed"));
    assert_eq!(fs::read(intent_path).unwrap(), bytes);
}

#[test]
fn a_new_operation_sharing_only_a_secondary_root_requires_recovery_after_reopen() {
    let project = tempfile::tempdir().unwrap();
    let next = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    let private = root(home.path(), OwnershipScope::User);
    let common = root(shared.path(), OwnershipScope::Project);
    let primary = root(project.path(), OwnershipScope::Project);
    let id = {
        let ps = QualifiedScope::open(&primary).unwrap();
        let hs = QualifiedScope::open(&private).unwrap();
        let cs = QualifiedScope::open(&common).unwrap();
        let locks = lock_native_roots(&[
            NativeRoot {
                root: primary.clone(),
                scope: &ps,
            },
            NativeRoot {
                root: private.clone(),
                scope: &hs,
            },
            NativeRoot {
                root: common.clone(),
                scope: &cs,
            },
        ])
        .unwrap();
        let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
        store
            .prepare(&NativeOperationCapture::default())
            .unwrap()
            .id()
            .to_owned()
    };
    let different_home = tempfile::tempdir().unwrap();
    let next_private = root(different_home.path(), OwnershipScope::User);
    let next_primary = root(next.path(), OwnershipScope::Project);
    let ps = QualifiedScope::open(&next_primary).unwrap();
    let hs = QualifiedScope::open(&next_private).unwrap();
    let cs = QualifiedScope::open(&common).unwrap();
    let locks = lock_native_roots(&[
        NativeRoot {
            root: next_primary.clone(),
            scope: &ps,
        },
        NativeRoot {
            root: next_private.clone(),
            scope: &hs,
        },
        NativeRoot {
            root: common.clone(),
            scope: &cs,
        },
    ])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &next_primary, &next_private).unwrap();
    let error = store
        .prepare(&NativeOperationCapture::default())
        .unwrap_err();
    assert!(format!("{error:#}").contains(&id));
    assert_eq!(fs::read_dir(next.path()).unwrap().count(), 0);
    assert!(
        !different_home
            .path()
            .join(".upskill/native-evidence")
            .read_dir()
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("intent-"))
    );
    assert!(
        common
            .path
            .join(".upskill/native-pending/pending.json")
            .exists()
    );
}

#[test]
fn malformed_existing_marker_is_preserved_and_blocks_new_preparation() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let primary = root(project.path(), OwnershipScope::Project);
    let private = root(home.path(), OwnershipScope::User);
    let ps = QualifiedScope::open(&primary).unwrap();
    let hs = QualifiedScope::open(&private).unwrap();
    let locks = lock_native_roots(&[
        NativeRoot {
            root: primary.clone(),
            scope: &ps,
        },
        NativeRoot {
            root: private.clone(),
            scope: &hs,
        },
    ])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    store.prepare(&NativeOperationCapture::default()).unwrap();
    let path = project.path().join(".upskill/native-pending/pending.json");
    fs::write(&path, b"manual unknown marker").unwrap();
    assert!(store.prepare(&NativeOperationCapture::default()).is_err());
    assert_eq!(fs::read(path).unwrap(), b"manual unknown marker");
}

#[test]
fn same_scope_context_paths_use_logical_order_opposite_to_physical_lock_order() {
    use std::os::unix::fs::MetadataExt;
    let tmp = tempfile::tempdir().unwrap();
    for name in ["project", "home", "first", "second"] {
        fs::create_dir(tmp.path().join(name)).unwrap();
    }
    let physical = |name: &str| {
        let metadata = fs::metadata(tmp.path().join(name)).unwrap();
        (metadata.dev(), metadata.ino())
    };
    let (early, late) = if physical("first") < physical("second") {
        ("first", "second")
    } else {
        ("second", "first")
    };
    fs::rename(tmp.path().join(early), tmp.path().join("z")).unwrap();
    fs::rename(tmp.path().join(late), tmp.path().join("a")).unwrap();
    let primary = root(&tmp.path().join("project"), OwnershipScope::Project);
    let private = root(&tmp.path().join("home"), OwnershipScope::User);
    let a = root(&tmp.path().join("a"), OwnershipScope::Project);
    let z = root(&tmp.path().join("z"), OwnershipScope::Project);
    let roots = [primary.clone(), private.clone(), a.clone(), z.clone()];
    let scopes: Vec<_> = roots
        .iter()
        .map(|root| QualifiedScope::open(root).unwrap())
        .collect();
    let entries: Vec<_> = roots
        .iter()
        .zip(&scopes)
        .map(|(root, scope)| NativeRoot {
            root: root.clone(),
            scope,
        })
        .collect();
    let locks = lock_native_roots(&entries).unwrap();
    let physical: Vec<_> = locks
        .bindings()
        .iter()
        .filter(|binding| binding.root == a || binding.root == z)
        .map(|binding| binding.root.clone())
        .collect();
    assert_eq!(physical, vec![z.clone(), a.clone()]);
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    assert_eq!(store.context().additional_roots, vec![a, z]);
}

#[test]
fn consistent_hashes_do_not_make_a_different_preview_an_admitted_intent() {
    use sha2::{Digest, Sha256};
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let primary = root(project.path(), OwnershipScope::Project);
    let private = root(home.path(), OwnershipScope::User);
    let ps = QualifiedScope::open(&primary).unwrap();
    let hs = QualifiedScope::open(&private).unwrap();
    let locks = lock_native_roots(&[
        NativeRoot {
            root: primary.clone(),
            scope: &ps,
        },
        NativeRoot {
            root: private.clone(),
            scope: &hs,
        },
    ])
    .unwrap();
    let store = NativeOperationStore::initialize(&locks, &primary, &private).unwrap();
    let operation = store.prepare(&NativeOperationCapture::default()).unwrap();
    let mut intent = store.inspect(operation.id()).unwrap().intent;
    intent
        .plan
        .graph
        .removed_bundle_ids
        .push("imaginary-removal".into());
    let bytes = serde_yaml_ng::to_string(&intent).unwrap().into_bytes();
    let mut framed = b"upskill:native-intent:v1\0".to_vec();
    framed.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    framed.extend_from_slice(&bytes);
    let digest: String = Sha256::digest(&framed)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let intent_path = home
        .path()
        .join(".upskill/native-evidence")
        .join(format!("intent-{}.yaml", operation.id()));
    fs::write(&intent_path, &bytes).unwrap();
    let journal_path = home
        .path()
        .join(".upskill/native-evidence")
        .join(format!("journal-{}.json", operation.id()));
    let mut journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    journal["intent_sha256"] = digest.clone().into();
    fs::write(journal_path, serde_json::to_vec(&journal).unwrap()).unwrap();
    for root in [project.path(), home.path()] {
        let path = root.join(".upskill/native-pending/pending.json");
        let mut marker: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        marker["intent_sha256"] = digest.clone().into();
        fs::write(path, serde_json::to_vec(&marker).unwrap()).unwrap();
    }
    let error = store.inspect(operation.id()).unwrap_err();
    assert!(format!("{error:#}").contains("replayed captured plan"));
    assert_eq!(fs::read(intent_path).unwrap(), bytes);
    assert!(!project.path().join(".upskill-lock.json").exists());
}
