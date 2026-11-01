//! Fake native CLI fixtures shared by integration and controlled-clock tests.
#![allow(dead_code)]
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use upskill::filesystem::{QualifiedScope, ScopeWriterGuard};
use upskill::native_delivery::process::{NativeCommandSpec, NativeExecutable};
use upskill::ownership::{InstallationRoot, OwnershipScope};
pub fn with_scope(run: impl FnOnce(&std::path::Path, &QualifiedScope, &ScopeWriterGuard<'_>)) {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("scope");
    std::fs::create_dir(&directory).unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: directory.clone(),
    };
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    struct StopDescendants<'a>(&'a std::path::Path);
    impl Drop for StopDescendants<'_> {
        fn drop(&mut self) {
            let _ = std::fs::write(self.0.join("stop-child"), b"stop");
        }
    }
    // This runs only after assertions (including on failure), so cooperative
    // fixture cleanup cannot masquerade as successful process-group cleanup.
    let _stop = StopDescendants(&directory);
    run(&directory, &scope, &guard);
}
pub fn script(root: &std::path::Path, body: &str) -> NativeCommandSpec {
    let path = root.join("native-fixture");
    std::fs::write(
        &path,
        format!("#!/bin/sh\nprintf '%s' \"$$\" > \"$LEADER_PID\"\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut environment = BTreeMap::from([(OsString::from("HOME"), root.as_os_str().into())]);
    environment.insert(
        "LEADER_PID".into(),
        root.join("leader-pid").into_os_string(),
    );
    NativeCommandSpec {
        executable: NativeExecutable::capture(&path).unwrap(),
        arguments: vec![],
        environment,
        timeout: Duration::from_secs(2),
        output_limit: 4096,
    }
}
pub fn descendants(root: &std::path::Path, parent_exit: bool) -> NativeCommandSpec {
    let mut spec = script(
        root,
        r#"/bin/sh -c 'printf ready > "$READY"; while [ -d "$HOME" ] && [ ! -f "$STOP_FILE" ]; do printf x >> "$HEARTBEAT"; /bin/sleep 0.01; done' &
printf '%s' "$!" > "$CHILD_PID"
printf 'spawned\n'
printf ready > "$PARENT_READY"
if [ "$PARENT_EXIT" = yes ]; then
  while [ ! -s "$HEARTBEAT" ]; do /bin/sleep 0.01; done
  exit 0
fi
wait"#,
    );
    for (name, file) in [
        ("READY", "ready"),
        ("PARENT_READY", "parent-ready"),
        ("HEARTBEAT", "heartbeat"),
        ("CHILD_PID", "child-pid"),
        ("STOP_FILE", "stop-child"),
    ] {
        spec.environment
            .insert(name.into(), root.join(file).into_os_string());
    }
    spec.environment.insert(
        "PARENT_EXIT".into(),
        if parent_exit { "yes" } else { "no" }.into(),
    );
    spec.timeout = Duration::from_secs(5);
    spec
}
pub fn stopped_heartbeat(root: &std::path::Path) {
    assert!(
        root.join("ready").exists(),
        "descendant never reached its positive control"
    );
    let before = std::fs::read(root.join("heartbeat")).unwrap();
    assert!(!before.is_empty(), "heartbeat positive control is empty");
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(std::fs::read(root.join("heartbeat")).unwrap(), before);
}
