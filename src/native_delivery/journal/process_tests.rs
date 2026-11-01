use super::*;
use crate::filesystem::QualifiedScope;
use crate::native_delivery::roots::{NativeRoot, lock_native_roots};
use crate::ownership::OwnershipScope;
fn roots(base: &std::path::Path) -> Vec<InstallationRoot> {
    [
        ("project", OwnershipScope::Project),
        ("home", OwnershipScope::User),
        ("extra", OwnershipScope::Project),
    ]
    .into_iter()
    .map(|(name, scope)| InstallationRoot {
        scope,
        path: base.join(name),
    })
    .collect()
}
#[test]
#[ignore]
fn probe() {
    let Some(base) = std::env::var_os("UPSKILL_NATIVE_INTENT_ROOT") else {
        return;
    };
    let base = std::path::PathBuf::from(base);
    let target = std::env::var("UPSKILL_NATIVE_INTENT_CHECKPOINT").unwrap();
    let index: usize = std::env::var("UPSKILL_NATIVE_INTENT_INDEX")
        .unwrap()
        .parse()
        .unwrap();
    let records = roots(&base);
    let scopes: Vec<_> = records
        .iter()
        .map(|root| QualifiedScope::open(root).unwrap())
        .collect();
    let entries: Vec<_> = records
        .iter()
        .zip(&scopes)
        .map(|(root, scope)| NativeRoot {
            root: root.clone(),
            scope,
        })
        .collect();
    let locks = lock_native_roots(&entries).unwrap();
    let store = NativeOperationStore::initialize(&locks, &records[0], &records[1]).unwrap();
    TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |label, current, id| {
            if label == target && current == index {
                std::fs::write(base.join("control/id"), id)?;
                std::process::exit(73);
            }
            Ok(())
        }));
    });
    store.prepare(&NativeOperationCapture::default()).unwrap();
    panic!("requested native preparation checkpoint not reached");
}
#[test]
fn interrupted_intent_and_each_marker_are_inspected_in_a_fresh_process() {
    for (checkpoint, index) in [
        ("after_intent", 0),
        ("after_marker", 0),
        ("after_marker", 1),
        ("after_marker", 2),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let records = roots(tmp.path());
        for root in &records {
            std::fs::create_dir(&root.path).unwrap();
        }
        std::fs::create_dir(tmp.path().join("control")).unwrap();
        std::fs::create_dir(tmp.path().join("fake-home")).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "native_delivery::journal::process_tests::probe",
                "--nocapture",
            ])
            .env("UPSKILL_NATIVE_INTENT_ROOT", tmp.path())
            .env("UPSKILL_NATIVE_INTENT_CHECKPOINT", checkpoint)
            .env("UPSKILL_NATIVE_INTENT_INDEX", index.to_string())
            .env("HOME", tmp.path().join("fake-home"))
            .env("USERPROFILE", tmp.path().join("fake-home"))
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(73),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let id = std::fs::read_to_string(tmp.path().join("control/id")).unwrap();
        let inspection = {
            let scopes: Vec<_> = records
                .iter()
                .map(|root| QualifiedScope::open(root).unwrap())
                .collect();
            let entries: Vec<_> = records
                .iter()
                .zip(&scopes)
                .map(|(root, scope)| NativeRoot {
                    root: root.clone(),
                    scope,
                })
                .collect();
            let locks = lock_native_roots(&entries).unwrap();
            let store = NativeOperationStore::initialize(&locks, &records[0], &records[1]).unwrap();
            store.inspect(&id).unwrap()
        };
        assert_eq!(
            inspection.markers_complete,
            checkpoint == "after_marker" && index == 2
        );
        assert_eq!(inspection.journal.status, NativeJournalStatus::Prepared);
        assert!(!records[0].path.join(".upskill-lock.json").exists());
        if checkpoint == "after_marker" {
            // Only this marked root is shared with the next operation.
            let common = inspection.intent.roots[0].root.clone();
            let next = tempfile::tempdir().unwrap();
            let next_home = tempfile::tempdir().unwrap();
            let next_primary = InstallationRoot {
                scope: OwnershipScope::Project,
                path: next.path().into(),
            };
            let next_private = InstallationRoot {
                scope: OwnershipScope::User,
                path: next_home.path().into(),
            };
            let records = [next_primary.clone(), next_private.clone(), common];
            let scopes: Vec<_> = records
                .iter()
                .map(|root| QualifiedScope::open(root).unwrap())
                .collect();
            let entries: Vec<_> = records
                .iter()
                .zip(&scopes)
                .map(|(root, scope)| NativeRoot {
                    root: root.clone(),
                    scope,
                })
                .collect();
            let locks = lock_native_roots(&entries).unwrap();
            let store =
                NativeOperationStore::initialize(&locks, &next_primary, &next_private).unwrap();
            let error = store
                .prepare(&NativeOperationCapture::default())
                .unwrap_err();
            assert!(error.downcast_ref::<NativeRecoveryRequired>().is_some());
            assert!(format!("{error:#}").contains(&id));
            assert_eq!(std::fs::read_dir(next.path()).unwrap().count(), 0);
        }
    }
}
