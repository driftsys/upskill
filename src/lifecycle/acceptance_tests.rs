//! Real process acceptance through the opt-in source command adapters.
use crate::files::permissions::ResourceExecutionPolicy;
use crate::filesystem::{ExecutionOutcome, QualifiedScope};
use crate::generate::Client;
use crate::lifecycle::desired::CompilerBinding;
use crate::lifecycle::orchestration::TargetSelection;
use crate::lifecycle::orchestration::{
    AddRule, CompilerContext, OrchestrationOutcome, SourceOptions, SourceRequest, with_prepared,
};
use crate::lifecycle::selection::Selection;
use crate::lifecycle::state::parse_state;
use crate::ownership::{InstallationRoot, OwnershipScope};
use crate::source::InstallSource;
use crate::target::TargetProfile;
use crate::target::TargetSurface;

fn context() -> CompilerContext {
    CompilerContext {
        bindings: vec![CompilerBinding {
            client: Client::Claude,
            consumers: vec![TargetSurface::ClaudeCli],
        }],
        profiles: vec![
            TargetProfile::new(
                TargetSurface::ClaudeCli.family(),
                TargetSurface::ClaudeCli,
                1,
            )
            .unwrap(),
        ],
        execution: ResourceExecutionPolicy::Disabled,
    }
}
fn root(path: &std::path::Path) -> InstallationRoot {
    InstallationRoot {
        scope: OwnershipScope::Project,
        path: path.into(),
    }
}
fn add(path: &std::path::Path) -> SourceRequest {
    SourceRequest::Add(vec![AddRule {
        source: InstallSource::LocalPath(path.into()),
        source_name: "source".into(),
        installed_name: "alias".into(),
    }])
}
fn write_rule(path: &std::path::Path, body: &str) {
    std::fs::write(
        path.join("RULE.md"),
        format!("---\nschema: 1\nname: source\ndescription: demo\n---\n\n# {body}\n"),
    )
    .unwrap();
}
fn run(root: &InstallationRoot, request: SourceRequest) -> OrchestrationOutcome {
    let scope = QualifiedScope::open(root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    with_prepared(
        &scope,
        &guard,
        root.clone(),
        request,
        SourceOptions {
            targets: TargetSelection::default(),
            add_defaults: &[TargetSurface::ClaudeCli],
            compiler: &context(),
        },
        |prepared| prepared.apply(),
    )
    .unwrap()
}
fn state(path: &std::path::Path) -> crate::lifecycle::state::CombinedState {
    parse_state(&std::fs::read_to_string(path.join(".upskill-lock.json")).unwrap()).unwrap()
}
use crate::filesystem::execution::TEST_HOOK;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type Image = BTreeMap<PathBuf, (Vec<u8>, u32)>;
fn image(path: &Path) -> Image {
    fn visit(base: &Path, at: &Path, result: &mut Image) {
        for entry in std::fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.strip_prefix(base).unwrap().starts_with(".upskill") {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                visit(base, &path, result);
            } else if entry.file_type().unwrap().is_file() {
                result.insert(
                    path.strip_prefix(base).unwrap().into(),
                    (
                        std::fs::read(&path).unwrap(),
                        entry.metadata().unwrap().permissions().mode() & 0o7777,
                    ),
                );
            }
        }
    }
    let mut result = Image::new();
    visit(path, path, &mut result);
    result
}
fn assert_image(path: &Path, expected: &Image) {
    let mut actual = image(path);
    actual.retain(|relative, (bytes, mode)| {
        if relative
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".upskill-stage-")
        {
            assert!(
                expected
                    .iter()
                    .any(
                        |(target, (payload, expected_mode))| target.parent() == relative.parent()
                            && payload == bytes
                            && expected_mode == mode
                    ),
                "unknown stage payload or parent: {relative:?}"
            );
            assert!(
                std::fs::symlink_metadata(path.join(relative))
                    .unwrap()
                    .file_type()
                    .is_file()
            );
            eprintln!("accounted staged-payload orphan: {relative:?}");
            false
        } else {
            true
        }
    });
    assert_eq!(actual, *expected);
}
fn restore(path: &Path, image: &Image) {
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            std::fs::remove_dir_all(entry.path()).unwrap();
        } else {
            std::fs::remove_file(entry.path()).unwrap();
        }
    }
    for (relative, (bytes, mode)) in image {
        let path = path.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(*mode)).unwrap();
    }
}
fn request(source: &Path, operation: &str) -> SourceRequest {
    match operation {
        "add" => add(source),
        "update" => SourceRequest::Update(Selection::Names(vec!["alias".into()])),
        "remove" => SourceRequest::Remove(Selection::Names(vec!["alias".into()])),
        _ => panic!("unknown operation"),
    }
}
#[test]
#[ignore = "subprocess entrypoint invoked explicitly by lifecycle process acceptance"]
fn probe() {
    let path = PathBuf::from(std::env::var_os("UPSKILL_GATE5_ROOT").unwrap());
    let source = PathBuf::from(std::env::var_os("UPSKILL_GATE5_SOURCE").unwrap());
    let operation = std::env::var("UPSKILL_GATE5_OPERATION").unwrap();
    let target = std::env::var("UPSKILL_GATE5_POINT").unwrap();
    let index: usize = std::env::var("UPSKILL_GATE5_INDEX")
        .unwrap()
        .parse()
        .unwrap();
    let mut stream = UnixStream::connect(path.join("probe.sock")).unwrap();
    TEST_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |point, current| {
            if point == target && current == index {
                writeln!(stream, "{point}:{current}")?;
                let mut byte = [0];
                stream.read_exact(&mut byte)?;
            }
            Ok(())
        }))
    });
    run(&root(&path), request(&source, &operation));
}
#[test]
fn every_observed_lifecycle_checkpoint_recovers_exact_captured_intent() {
    for migrated in [false, true] {
        for operation in ["add", "update", "remove"] {
            let temp = tempfile::Builder::new()
                .prefix("upskill-gate5-")
                .tempdir_in("/tmp")
                .unwrap();
            let path = temp.path().join("scope");
            let source = temp.path().join("source");
            std::fs::create_dir_all(&path).unwrap();
            std::fs::create_dir_all(&source).unwrap();
            write_rule(&source, "First");
            std::fs::create_dir(source.join("scripts")).unwrap();
            std::fs::write(source.join("scripts/check.sh"), b"resource").unwrap();
            if migrated {
                std::fs::write(path.join(".upskill-lock.json"), br#"{"schema":1,"items":[{"kind":"skill","name":"legacy","source":"unresolved","required_by":[],"clients":[]},{"kind":"skill","name":"legacy","source":"unresolved","required_by":[],"clients":[]}],"bundles":[]}"#).unwrap();
            }
            if operation != "add" {
                run(&root(&path), add(&source));
            }
            write_rule(&source, "Second");
            std::fs::write(source.join("scripts/check.sh"), b"second resource").unwrap();
            let before = image(&path);
            let events = Arc::new(Mutex::new(Vec::new()));
            let captured = events.clone();
            TEST_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |point, index| {
                    if [
                        "before_pending",
                        "after_pending",
                        "before_effect",
                        "after_effect",
                        "after_effect_sync",
                        "before_done",
                        "after_done",
                        "after_ack_sync",
                    ]
                    .contains(&point)
                    {
                        captured.lock().unwrap().push((point.to_string(), index));
                    }
                    Ok(())
                }))
            });
            assert!(
                matches!(run(&root(&path), request(&source, operation)), OrchestrationOutcome::Ready(report) if report.outcome == ExecutionOutcome::Complete)
            );
            TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
            let expected = image(&path);
            assert_eq!(state(&path).schema, 2);
            let events = events.lock().unwrap().clone();
            assert!(!events.is_empty());
            let labels: std::collections::BTreeSet<_> =
                events.iter().map(|(point, _)| point.as_str()).collect();
            for required in [
                "before_pending",
                "after_pending",
                "before_effect",
                "after_effect",
                "after_effect_sync",
                "before_done",
                "after_done",
                "after_ack_sync",
            ] {
                assert!(
                    labels.contains(required),
                    "missing {required} for {operation}"
                );
            }
            eprintln!(
                "gate5 migrated={migrated} operation={operation}: {} checkpoints",
                events.len()
            );
            let sabotage_index = events
                .iter()
                .find(|(point, _)| point == "after_effect")
                .map(|(_, index)| *index);
            let mut cases: Vec<_> = events
                .into_iter()
                .map(|(point, index)| (point, index, false))
                .collect();
            if operation == "update" {
                cases.push(("after_effect".into(), sabotage_index.unwrap(), true));
            }
            for (point, index, sabotage) in cases {
                restore(&path, &before);
                write_rule(&source, "Second");
                std::fs::write(source.join("scripts/check.sh"), b"second resource").unwrap();
                let listener = UnixListener::bind(path.join("probe.sock")).unwrap();
                listener.set_nonblocking(true).unwrap();
                let mut command = Command::new(std::env::current_exe().unwrap());
                for (key, _) in std::env::vars_os() {
                    if key.to_string_lossy().starts_with("GIT_") {
                        command.env_remove(key);
                    }
                }
                let mut child = command
                    .args([
                        "--exact",
                        "lifecycle::acceptance_tests::probe",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("UPSKILL_GATE5_ROOT", &path)
                    .env("UPSKILL_GATE5_SOURCE", &source)
                    .env("UPSKILL_GATE5_OPERATION", operation)
                    .env("UPSKILL_GATE5_POINT", &point)
                    .env("UPSKILL_GATE5_INDEX", index.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .unwrap();
                let start = Instant::now();
                let stream = loop {
                    if let Ok((stream, _)) = listener.accept() {
                        break stream;
                    }
                    assert!(
                        child.try_wait().unwrap().is_none(),
                        "child exited at {operation}:{point}:{index}"
                    );
                    if start.elapsed() > Duration::from_secs(30) {
                        let _ = child.kill();
                        let _ = child.wait();
                        panic!("checkpoint timeout {point}:{index}");
                    }
                    std::thread::sleep(Duration::from_millis(2));
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(30)))
                    .unwrap();
                let mut reader = BufReader::new(stream);
                let mut receipt = String::new();
                reader.read_line(&mut receipt).unwrap();
                assert_eq!(receipt, format!("{point}:{index}\n"));
                let competing = QualifiedScope::open(&root(&path)).unwrap();
                assert!(competing.try_lock_writer().is_err());
                child.kill().unwrap();
                child.wait().unwrap();
                drop(reader);
                drop(listener);
                std::fs::remove_file(path.join("probe.sock")).unwrap();
                write_rule(&source, "Changed after journal");
                let edited = if sabotage {
                    let old = parse_state(
                        std::str::from_utf8(&before[&PathBuf::from(".upskill-lock.json")].0)
                            .unwrap(),
                    )
                    .unwrap();
                    let output = path.join(&old.artifacts[0].path);
                    std::fs::write(&output, b"external edit").unwrap();
                    Some(output)
                } else {
                    None
                };
                let scope = QualifiedScope::open(&root(&path)).unwrap();
                let guard = scope.try_lock_writer().unwrap();
                let result =
                    crate::lifecycle::commands::LifecycleSession::open(&scope, &guard, root(&path))
                        .unwrap();
                match result {
                    crate::lifecycle::commands::SessionOutcome::Recovery(report) => {
                        if sabotage {
                            assert!(matches!(report.outcome, ExecutionOutcome::Reconcile { .. }));
                            assert_eq!(
                                std::fs::read(edited.as_ref().unwrap()).unwrap(),
                                b"external edit"
                            );
                            assert_eq!(
                                std::fs::read(path.join(".upskill-lock.json")).unwrap(),
                                before[&PathBuf::from(".upskill-lock.json")].0
                            );
                            assert!(scope.has_pending_operation().unwrap());
                            assert!(scope.recover_operation(&guard).unwrap().is_some());
                            continue;
                        }
                        assert_eq!(report.outcome, ExecutionOutcome::Complete);
                    }
                    crate::lifecycle::commands::SessionOutcome::Ready(_) => {
                        // Before the first pending receipt, no journal exists yet.
                        assert_eq!(point, "before_pending");
                        assert_eq!(index, 0);
                        drop(guard);
                        drop(scope);
                        write_rule(&source, "Second");
                        std::fs::write(source.join("scripts/check.sh"), b"second resource")
                            .unwrap();
                        run(&root(&path), request(&source, operation));
                        assert_image(&path, &expected);
                        continue;
                    }
                }
                assert_image(&path, &expected);
                assert!(!scope.has_pending_operation().unwrap());
                assert!(matches!(
                    crate::lifecycle::commands::LifecycleSession::open(&scope, &guard, root(&path))
                        .unwrap(),
                    crate::lifecycle::commands::SessionOutcome::Ready(_)
                ));
            }
        }
    }
}
