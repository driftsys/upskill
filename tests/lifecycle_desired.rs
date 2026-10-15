use sha2::{Digest, Sha256};
use upskill::files::plan::DesiredFile;
use upskill::generate::Client;
use upskill::lifecycle::desired::{
    CompilerBinding, RuleDesiredSetRequest, compile_rule_desired_set,
};
use upskill::lifecycle::state::{Installation, SourceEvidence};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::{
    CapturedSourceFile, SourceManifest, capture_source, validate_manifest,
};
use upskill::target::TargetSurface;

const RULE: &[u8] = b"---\nschema: 1\nname: source\ndescription: demo\n---\n\n# Demo\n";

fn captured(files: &[(&str, &[u8])]) -> SourceManifest {
    let tmp = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let destination = tmp.path().join(path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(destination, bytes).unwrap();
    }
    capture_source(tmp.path()).unwrap()
}

// Independent input encoding of the documented framed manifest protocol.
fn synthetic(files: &[(&str, &[u8], Option<u32>)]) -> SourceManifest {
    let mut files: Vec<_> = files
        .iter()
        .map(|(path, bytes, mode)| CapturedSourceFile {
            path: (*path).into(),
            bytes: bytes.to_vec(),
            mode: *mode,
            content_sha256: Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hash = Sha256::new();
    hash.update(b"upskill:source-manifest:v1\0");
    hash.update((files.len() as u64).to_be_bytes());
    for file in &files {
        hash.update((file.path.len() as u64).to_be_bytes());
        hash.update(file.path.as_bytes());
        hash.update((file.bytes.len() as u64).to_be_bytes());
        hash.update(&file.bytes);
        match file.mode {
            None => hash.update([0]),
            Some(mode) => {
                hash.update([1]);
                hash.update(mode.to_be_bytes());
            }
        }
    }
    let source = SourceManifest {
        files,
        content_sha256: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
    };
    validate_manifest(&source).unwrap();
    source
}

fn installation(source: &SourceManifest, requested: &[TargetSurface]) -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: "rule-source".into(),
            source: "./captured-source-no-longer-present".into(),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "source".into(),
            installed_name: "source".into(),
        },
        root: InstallationRoot {
            scope: OwnershipScope::Project,
            path: std::env::temp_dir().join("upskill-desired-output"),
        },
        requested: requested.into(),
        source_evidence: SourceEvidence::Local {
            manifest_sha256: source.content_sha256.clone(),
        },
    }
}
fn binding(client: Client, consumers: &[TargetSurface]) -> CompilerBinding {
    CompilerBinding {
        client,
        consumers: consumers.into(),
    }
}
fn request<'a>(
    installation: &'a Installation,
    source: &'a SourceManifest,
    bindings: &'a [CompilerBinding],
) -> RuleDesiredSetRequest<'a> {
    RuleDesiredSetRequest {
        installation,
        source,
        folder_name: "folder",
        bindings,
    }
}
fn file(artifact: &DesiredArtifact) -> &DesiredFile {
    match artifact {
        DesiredArtifact::File(file) => file,
        _ => panic!("rule producer returned a contribution"),
    }
}
fn reject(installation: &Installation, source: &SourceManifest, bindings: &[CompilerBinding]) {
    let before = (installation.clone(), source.clone(), bindings.to_vec());
    assert!(compile_rule_desired_set(request(installation, source, bindings)).is_err());
    assert_eq!(
        (installation.clone(), source.clone(), bindings.to_vec()),
        before
    );
}

#[test]
fn captured_rule_projection_retains_exact_paths_bytes_and_consumers() {
    let source = captured(&[
        (
            "RULE.md",
            include_bytes!("fixtures/items/api-conventions/RULE.md"),
        ),
        ("data.bin", &[0, 255, 17]),
    ]);
    let mut item = installation(
        &source,
        &[
            TargetSurface::CopilotVscodeLocal,
            TargetSurface::ClaudeCli,
            TargetSurface::CopilotCli,
        ],
    );
    item.owner.source_name = "api-conventions".into();
    item.owner.installed_name = "api-conventions".into();
    let bindings = vec![
        binding(
            Client::Copilot,
            &[TargetSurface::CopilotVscodeLocal, TargetSurface::CopilotCli],
        ),
        binding(Client::Claude, &[TargetSurface::ClaudeCli]),
    ];
    let before = (item.clone(), source.clone(), bindings.clone());
    let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    assert_eq!(result.installation, item);
    assert!(result.audience_excluded.is_empty());
    let files: Vec<_> = result.artifacts.iter().map(file).collect();
    assert_eq!(
        files
            .iter()
            .map(|f| f.location.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            ".claude/rules/api-conventions.md",
            ".claude/rules/api-conventions/data.bin",
            ".github/instructions/api-conventions.instructions.md",
            ".github/instructions/api-conventions/data.bin"
        ]
    );
    for f in &files {
        assert_eq!(f.location.root.scope, item.root.scope);
        assert_eq!(f.location.root.path.as_os_str(), item.root.path.as_os_str());
    }
    assert_eq!(
        files[0].value.bytes,
        include_bytes!("fixtures/expected/claude/api-conventions.RULE.md")
    );
    assert_eq!(
        files[2].value.bytes,
        include_bytes!("fixtures/expected/copilot/api-conventions.RULE.md")
    );
    assert_eq!(files[1].value.bytes, [0, 255, 17]);
    assert_eq!(files[3].value.bytes, [0, 255, 17]);
    let mode = source
        .files
        .iter()
        .find(|f| f.path == "data.bin")
        .unwrap()
        .mode;
    assert_eq!(files[1].value.mode, mode);
    assert_eq!(files[3].value.mode, mode);
    assert_eq!(files[0].value.mode, None);
    assert_eq!(files[2].value.mode, None);
    assert_eq!(files[0].consumers, vec![TargetSurface::ClaudeCli]);
    assert_eq!(files[1].consumers, vec![TargetSurface::ClaudeCli]);
    assert_eq!(
        files[2].consumers,
        vec![TargetSurface::CopilotCli, TargetSurface::CopilotVscodeLocal]
    );
    assert_eq!(files[3].consumers, files[2].consumers);
    assert_eq!((item, source, bindings), before);
}

#[test]
fn source_and_alias_binding_mismatch_rejects() {
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let item = installation(&source, &[TargetSurface::ClaudeCli]);
    let bindings = vec![binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    for mutate in [
        |i: &mut Installation| {
            i.source_evidence = SourceEvidence::Local {
                manifest_sha256: "f".repeat(64),
            }
        },
        |i: &mut Installation| i.owner.source_name = "wrong-name".into(),
        |i: &mut Installation| i.owner.installed_name = "../bad".into(),
        |i: &mut Installation| i.owner.kind = ItemKind::Skill,
        |i: &mut Installation| i.owner.kind = ItemKind::Agent,
        |i: &mut Installation| i.owner.installation_id = "bad/id".into(),
        |i: &mut Installation| i.root.path = "relative".into(),
        |i: &mut Installation| i.owner.source.clear(),
    ] {
        let mut invalid = item.clone();
        mutate(&mut invalid);
        reject(&invalid, &source, &bindings);
    }
    let mut corrupt = source.clone();
    corrupt.files[0].bytes.push(b'x');
    reject(&item, &corrupt, &bindings);
    let mut corrupt = source.clone();
    corrupt.files[0].content_sha256 = "f".repeat(64);
    reject(&item, &corrupt, &bindings);
}

#[test]
fn git_revision_and_capture_evidence_must_agree() {
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let mut item = installation(&source, &[TargetSurface::ClaudeCli]);
    let bindings = vec![binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    item.source_evidence = SourceEvidence::Git {
        resolved_revision: "resolved-commit".into(),
        manifest_sha256: source.content_sha256.clone(),
    };
    reject(&item, &source, &bindings);
    item.owner.source_revision = Some("resolved-commit".into());
    assert_eq!(
        compile_rule_desired_set(request(&item, &source, &bindings))
            .unwrap()
            .installation,
        item
    );
    item.owner.source_revision = Some("different-commit".into());
    reject(&item, &source, &bindings);
    item.source_evidence = SourceEvidence::Local {
        manifest_sha256: source.content_sha256.clone(),
    };
    reject(&item, &source, &bindings);
}

#[test]
fn requested_surface_accounting_is_exact() {
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let mut item = installation(&source, &[TargetSurface::ClaudeCli]);
    reject(&item, &source, &[]);
    reject(&item, &source, &[binding(Client::Claude, &[])]);
    reject(
        &item,
        &source,
        &[binding(
            Client::Claude,
            &[TargetSurface::ClaudeCli, TargetSurface::ClaudeCli],
        )],
    );
    reject(
        &item,
        &source,
        &[binding(Client::Copilot, &[TargetSurface::ClaudeCli])],
    );
    reject(
        &item,
        &source,
        &[
            binding(Client::Claude, &[TargetSurface::ClaudeCli]),
            binding(Client::Copilot, &[TargetSurface::CopilotCli]),
        ],
    );
    item.requested.push(TargetSurface::CopilotCli);
    reject(
        &item,
        &source,
        &[binding(Client::Claude, &[TargetSurface::ClaudeCli])],
    );
    item.requested = vec![TargetSurface::CopilotCli, TargetSurface::CopilotVscodeLocal];
    reject(
        &item,
        &source,
        &[
            binding(Client::Copilot, &[TargetSurface::CopilotCli]),
            binding(Client::Copilot, &[TargetSurface::CopilotVscodeLocal]),
        ],
    );
    for surface in [TargetSurface::CodexCli, TargetSurface::CodexIde] {
        item.requested = vec![surface];
        let error = compile_rule_desired_set(request(
            &item,
            &source,
            &[binding(Client::Claude, &[surface])],
        ))
        .unwrap_err();
        assert!(format!("{error:#}").contains(surface.name()));
    }
    item.requested = vec![];
    reject(
        &item,
        &source,
        &[binding(Client::Claude, &[TargetSurface::ClaudeCli])],
    );
    item.requested = vec![TargetSurface::ClaudeCli, TargetSurface::ClaudeCli];
    reject(
        &item,
        &source,
        &[binding(Client::Claude, &[TargetSurface::ClaudeCli])],
    );
}

#[test]
fn copilot_consumers_are_explicit_and_binding_order_does_not_change_artifacts() {
    let source = synthetic(&[("RULE.md", RULE, None)]);
    for consumers in [
        vec![TargetSurface::CopilotCli],
        vec![TargetSurface::CopilotVscodeLocal],
        vec![TargetSurface::CopilotCli, TargetSurface::CopilotVscodeLocal],
    ] {
        let item = installation(&source, &consumers);
        let bindings = [binding(Client::Copilot, &consumers)];
        let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(file(&result.artifacts[0]).consumers, consumers);
        assert_eq!(
            file(&result.artifacts[0]).location.path,
            ".github/instructions/source.instructions.md"
        );
    }
    let item = installation(
        &source,
        &[
            TargetSurface::ClaudeCli,
            TargetSurface::CopilotCli,
            TargetSurface::CopilotVscodeLocal,
        ],
    );
    let mut bindings = vec![
        binding(Client::Claude, &[TargetSurface::ClaudeCli]),
        binding(
            Client::Copilot,
            &[TargetSurface::CopilotCli, TargetSurface::CopilotVscodeLocal],
        ),
    ];
    let first = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    bindings.reverse();
    bindings[0].consumers.reverse();
    assert_eq!(
        compile_rule_desired_set(request(&item, &source, &bindings)).unwrap(),
        first
    );
}

#[test]
fn audience_exclusion_keeps_installation_and_requested_selection() {
    let source = synthetic(&[(
        "RULE.md",
        b"---\nschema: 1\nname: source\ndescription: demo\naudience: [claude]\n---\n\n# Demo\n",
        None,
    )]);
    let item = installation(&source, &[TargetSurface::CopilotCli]);
    let bindings = [binding(Client::Copilot, &[TargetSurface::CopilotCli])];
    let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    assert!(result.artifacts.is_empty());
    assert_eq!(result.installation, item);
    assert_eq!(result.audience_excluded, vec![TargetSurface::CopilotCli]);
    let item = installation(
        &source,
        &[TargetSurface::CopilotCli, TargetSurface::ClaudeCli],
    );
    let bindings = [
        binding(Client::Copilot, &[TargetSurface::CopilotCli]),
        binding(Client::Claude, &[TargetSurface::ClaudeCli]),
    ];
    let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(
        file(&result.artifacts[0]).consumers,
        vec![TargetSurface::ClaudeCli]
    );
    assert_eq!(result.audience_excluded, vec![TargetSurface::CopilotCli]);
    assert_eq!(result.installation, item);
}

#[test]
fn resources_and_aliases_keep_binary_bytes_and_mode_evidence() {
    let source = synthetic(&[("RULE.md", b"---\nschema: 1\nname: source\ndescription: demo\nignore: ['ignored/**']\n---\n\n[ref](./refs/data.bin)\n", Some(0o7777)),
        ("refs/data.bin", &[0,255,17], Some(0o755)), ("refs/empty", b"", Some(0)), (".hidden", b"hidden", None),
        ("文/é.txt", "café 🌍".as_bytes(), Some(0o640)), ("ignored/drop", b"ignored", Some(0o600))]);
    let mut item = installation(&source, &[TargetSurface::ClaudeCli]);
    item.owner.installed_name = "alias".into();
    let bindings = [binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    let files: Vec<_> = result.artifacts.iter().map(file).collect();
    assert_eq!(
        files
            .iter()
            .map(|f| f.location.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            ".claude/rules/alias.md",
            ".claude/rules/alias/.hidden",
            ".claude/rules/alias/refs/data.bin",
            ".claude/rules/alias/refs/empty",
            ".claude/rules/alias/文/é.txt"
        ]
    );
    assert_eq!(
        files
            .iter()
            .map(|f| f.value.bytes.as_slice())
            .collect::<Vec<_>>(),
        vec![
            b"---\nname: source\ndescription: demo\n---\n\n[ref](./alias/refs/data.bin)\n"
                .as_slice(),
            b"hidden",
            &[0, 255, 17],
            b"",
            "café 🌍".as_bytes()
        ]
    );
    assert_eq!(
        files.iter().map(|f| f.value.mode).collect::<Vec<_>>(),
        vec![None, None, Some(0o755), Some(0), Some(0o640)]
    );
    assert_eq!(result.installation.owner.source_name, "source");
    assert_eq!(result.installation.owner.installed_name, "alias");
}

#[test]
fn active_opencode_scope_cannot_be_silently_dropped() {
    let source = synthetic(&[(
        "RULE.md",
        include_bytes!("fixtures/items/api-conventions/RULE.md"),
        None,
    )]);
    let mut item = installation(&source, &[TargetSurface::OpenCodeCli]);
    item.owner.source_name = "api-conventions".into();
    item.owner.installed_name = "api-conventions".into();
    reject(
        &item,
        &source,
        &[binding(Client::OpenCode, &[TargetSurface::OpenCodeCli])],
    );
    for rule in [
        RULE,
        b"---\nschema: 1\nname: source\ndescription: demo\nscope: {paths: []}\n---\n\n# Demo\n",
    ] {
        let source = synthetic(&[("RULE.md", rule, None)]);
        let item = installation(&source, &[TargetSurface::OpenCodeCli]);
        let result = compile_rule_desired_set(request(
            &item,
            &source,
            &[binding(Client::OpenCode, &[TargetSurface::OpenCodeCli])],
        ))
        .unwrap();
        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(
            file(&result.artifacts[0]).location.path,
            ".agents/rules/source/RULE.md"
        );
        assert_eq!(
            file(&result.artifacts[0]).value.bytes,
            b"---\nname: source\ndescription: demo\n---\n\n# Demo\n"
        );
        assert_eq!(
            file(&result.artifacts[0]).consumers,
            vec![TargetSurface::OpenCodeCli]
        );
    }
    let source = synthetic(&[("RULE.md", b"---\nschema: 1\nname: source\ndescription: demo\naudience: [claude]\nscope: {paths: ['src/**']}\n---\n\n# Demo\n", None)]);
    let item = installation(&source, &[TargetSurface::OpenCodeCli]);
    let bindings = [binding(Client::OpenCode, &[TargetSurface::OpenCodeCli])];
    let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    assert!(result.artifacts.is_empty());
    assert_eq!(result.audience_excluded, vec![TargetSurface::OpenCodeCli]);
    assert_eq!(result.installation, item);
}

#[test]
fn raw_root_scope_and_spelling_are_retained_without_writes() {
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let tmp = tempfile::tempdir().unwrap();
    let mut item = installation(&source, &[TargetSurface::ClaudeCli]);
    item.root.scope = OwnershipScope::User;
    item.root.path = tmp.path().join("absent").join(".");
    let result = compile_rule_desired_set(request(
        &item,
        &source,
        &[binding(Client::Claude, &[TargetSurface::ClaudeCli])],
    ))
    .unwrap();
    assert_eq!(
        file(&result.artifacts[0]).location.root.path.as_os_str(),
        item.root.path.as_os_str()
    );
    assert_eq!(
        file(&result.artifacts[0]).location.root.scope,
        OwnershipScope::User
    );
    assert!(!tmp.path().join("absent").exists());
}

fn blocked(
    outcome: upskill::lifecycle::desired::DesiredSetsOutcome,
) -> Vec<upskill::lifecycle::desired::DesiredSetDiagnostic> {
    match outcome {
        upskill::lifecycle::desired::DesiredSetsOutcome::Blocked(diagnostics) => diagnostics,
        other => panic!("expected complete batch rejection, got {other:?}"),
    }
}

#[test]
fn batch_reports_independent_conflicts_without_a_ready_subset() {
    use upskill::lifecycle::desired::compile_rule_desired_sets;
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let bindings = [binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    let good = installation(&source, &[TargetSurface::ClaudeCli]);
    let mut bad_binding = good.clone();
    bad_binding.owner.installation_id = "bad-binding".into();
    let mut bad_capture = source.clone();
    bad_capture.files[0].bytes.push(0);
    let mut bad_source = good.clone();
    bad_source.owner.installation_id = "bad-source".into();
    let before = (
        good.clone(),
        bad_binding.clone(),
        bad_source.clone(),
        bad_capture.clone(),
        bindings.clone(),
    );
    let requests = [
        request(&good, &source, &bindings),
        request(&bad_binding, &source, &[]),
        request(&bad_source, &bad_capture, &bindings),
    ];
    let diagnostics = blocked(compile_rule_desired_sets(&requests));
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].request_indices, [1]);
    assert_eq!(diagnostics[0].installation_ids, ["bad-binding"]);
    assert!(diagnostics[0].message.contains("consumers"));
    assert_eq!(diagnostics[1].request_indices, [2]);
    assert_eq!(diagnostics[1].installation_ids, ["bad-source"]);
    assert!(diagnostics[1].message.contains("captured source"));
    assert_eq!(diagnostics, blocked(compile_rule_desired_sets(&requests)));
    assert_eq!(
        (good, bad_binding, bad_source, bad_capture, bindings),
        before
    );
}

#[test]
fn batch_rejects_equal_byte_collisions_and_ancestor_topology() {
    use upskill::lifecycle::desired::compile_rule_desired_sets;
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let bindings = [binding(Client::OpenCode, &[TargetSurface::OpenCodeCli])];
    let first = installation(&source, &[TargetSurface::OpenCodeCli]);
    let mut second = first.clone();
    second.owner.installation_id = "second".into();
    let equal = blocked(compile_rule_desired_sets(&[
        request(&first, &source, &bindings),
        request(&second, &source, &bindings),
    ]));
    assert_eq!(equal.len(), 1);
    assert_eq!(equal[0].request_indices, [0, 1]);
    assert_eq!(equal[0].installation_ids, ["rule-source", "second"]);
    assert!(equal[0].message.contains(".agents/rules/source/RULE.md"));
    // A resource under alias source.md descends from the other Claude entrypoint.
    let descendant = synthetic(&[("RULE.md", RULE, None), ("child", b"resource", None)]);
    let bindings = [binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    let mut first = first.clone();
    first.requested = vec![TargetSurface::ClaudeCli];
    second.requested = vec![TargetSurface::ClaudeCli];
    second.owner.installed_name = "source.md".into();
    second.source_evidence = SourceEvidence::Local {
        manifest_sha256: descendant.content_sha256.clone(),
    };
    let topology = blocked(compile_rule_desired_sets(&[
        request(&first, &source, &bindings),
        request(&second, &descendant, &bindings),
    ]));
    assert!(
        topology
            .iter()
            .any(|diagnostic| diagnostic.message.contains("source.md/child"))
    );
    assert!(
        topology
            .iter()
            .all(|diagnostic| diagnostic.request_indices == [0, 1])
    );
}

#[test]
fn batch_preserves_disjoint_roots_and_scopes() {
    use upskill::lifecycle::desired::{DesiredSetsOutcome, compile_rule_desired_sets};
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let bindings = [binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    let first = installation(&source, &[TargetSurface::ClaudeCli]);
    let mut items = vec![first.clone(), first.clone(), first.clone(), first.clone()];
    for (index, item) in items.iter_mut().enumerate() {
        item.owner.installation_id = format!("item-{index}");
    }
    items[1].root.path.push(".");
    items[2].root.scope = OwnershipScope::User;
    items[3].owner.installed_name = "different".into();
    let requests: Vec<_> = items
        .iter()
        .map(|item| request(item, &source, &bindings))
        .collect();
    let DesiredSetsOutcome::Ready(sets) = compile_rule_desired_sets(&requests) else {
        panic!("disjoint lexical locations rejected")
    };
    assert_eq!(
        sets.iter()
            .map(|set| set.installation.clone())
            .collect::<Vec<_>>(),
        items
    );
    assert_ne!(
        sets[0].installation.root.path.as_os_str(),
        sets[1].installation.root.path.as_os_str()
    );
    assert!(
        matches!(compile_rule_desired_sets(&[]), DesiredSetsOutcome::Ready(sets) if sets.is_empty())
    );
}

#[test]
fn batch_rejects_duplicate_ids_before_returning_sets() {
    use upskill::lifecycle::desired::compile_rule_desired_sets;
    let source = synthetic(&[("RULE.md", RULE, None)]);
    let bindings = [binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    let first = installation(&source, &[TargetSurface::ClaudeCli]);
    let mut second = first.clone();
    second.root.path.push("other");
    let diagnostics = blocked(compile_rule_desired_sets(&[
        request(&first, &source, &bindings),
        request(&second, &source, &bindings),
    ]));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].request_indices, [0, 1]);
    assert_eq!(
        diagnostics[0].installation_ids,
        ["rule-source", "rule-source"]
    );
    assert!(diagnostics[0].message.contains("duplicate installation"));
}

#[test]
fn excluded_copilot_consumers_are_complete_and_sorted() {
    let source = synthetic(&[(
        "RULE.md",
        b"---\nschema: 1\nname: source\ndescription: demo\naudience: [claude]\n---\n\n# Demo\n",
        None,
    )]);
    for consumers in [
        vec![TargetSurface::CopilotVscodeLocal],
        vec![TargetSurface::CopilotVscodeLocal, TargetSurface::CopilotCli],
    ] {
        let item = installation(&source, &consumers);
        let bindings = [binding(Client::Copilot, &consumers)];
        let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
        let mut expected = consumers;
        expected.sort_by_key(|surface| surface.name());
        assert_eq!(result.audience_excluded, expected);
        assert!(result.artifacts.is_empty());
        assert_eq!(result.installation, item);
    }
}

#[test]
fn undeclared_rule_name_uses_supplied_folder() {
    let source = synthetic(&[(
        "RULE.md",
        b"---\nschema: 1\ndescription: demo\n---\n\n# Demo\n",
        None,
    )]);
    let mut item = installation(&source, &[TargetSurface::ClaudeCli]);
    item.owner.source_name = "fallback-name".into();
    let bindings = [binding(Client::Claude, &[TargetSurface::ClaudeCli])];
    let mut input = request(&item, &source, &bindings);
    input.folder_name = "fallback-name";
    let result = compile_rule_desired_set(input).unwrap();
    assert_eq!(
        file(&result.artifacts[0]).value.bytes,
        b"---\nname: fallback-name\ndescription: demo\n---\n\n# Demo\n"
    );
    assert_eq!(result.installation, item);
}

#[test]
fn opencode_resources_keep_exact_bytes_modes_and_consumers() {
    let source = synthetic(&[
        ("RULE.md", RULE, None),
        ("refs/data.bin", &[0, 255, 17], Some(0o755)),
    ]);
    let item = installation(&source, &[TargetSurface::OpenCodeCli]);
    let bindings = [binding(Client::OpenCode, &[TargetSurface::OpenCodeCli])];
    let result = compile_rule_desired_set(request(&item, &source, &bindings)).unwrap();
    assert_eq!(result.artifacts.len(), 2);
    let resource = file(&result.artifacts[1]);
    assert_eq!(resource.location.path, ".agents/rules/source/refs/data.bin");
    assert_eq!(resource.value.bytes, [0, 255, 17]);
    assert_eq!(resource.value.mode, Some(0o755));
    assert_eq!(resource.consumers, [TargetSurface::OpenCodeCli]);
}
