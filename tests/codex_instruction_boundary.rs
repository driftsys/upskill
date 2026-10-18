use sha2::{Digest, Sha256};
use upskill::instructions::plan::{DesiredInstructionContribution, InstructionLocation};
use upskill::lifecycle::contracts::{ConsumerContract, ContractContent};
use upskill::lifecycle::desired::codex::{
    CodexInstructionRequest, CompiledCodexInstructions, InstructionCompatibility,
    InstructionReaderRole, compile_codex_instructions,
};
use upskill::lifecycle::state::{Installation, SourceEvidence};
use upskill::operations::plan::DesiredArtifact;
use upskill::ownership::{ArtifactOwner, InstallationRoot, OwnershipScope};
use upskill::pipeline::ItemKind;
use upskill::source_manifest::{SourceManifest, capture_source};
use upskill::target::{
    CapabilityCategory, CapabilityError, TargetFamily, TargetProfile, TargetSurface,
};

fn captured(fields: &str, body: &str, extras: &[(&str, &[u8])]) -> SourceManifest {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("RULE.md"),
        format!("---\nschema: 1\nname: source\ndescription: demo\n{fields}---\n{body}"),
    )
    .unwrap();
    for (path, bytes) in extras {
        let path = tmp.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    capture_source(tmp.path()).unwrap()
}
fn installation(source: &SourceManifest, scope: OwnershipScope) -> Installation {
    Installation {
        owner: ArtifactOwner {
            installation_id: "rule-source".into(),
            source: "./captured-source".into(),
            source_revision: None,
            kind: ItemKind::Rule,
            source_name: "source".into(),
            installed_name: "alias".into(),
        },
        root: InstallationRoot {
            scope,
            path: std::env::temp_dir().join("codex-boundary-output"),
        },
        requested: vec![TargetSurface::CodexCli],
        source_evidence: SourceEvidence::Local {
            manifest_sha256: source.content_sha256.clone(),
        },
    }
}
fn location(item: &Installation) -> InstructionLocation {
    InstructionLocation {
        root: item.root.clone(),
        path: match item.root.scope {
            OwnershipScope::Project => "AGENTS.md",
            OwnershipScope::User => ".codex/AGENTS.md",
        }
        .into(),
    }
}
fn compile(
    item: &Installation,
    source: &SourceManifest,
) -> anyhow::Result<CompiledCodexInstructions> {
    compile_codex_instructions(CodexInstructionRequest {
        installation: item,
        source,
        folder_name: "folder",
        profile: TargetSurface::CodexCli.profile(),
        location: &location(item),
    })
}
fn contribution(result: &CompiledCodexInstructions) -> &DesiredInstructionContribution {
    let [DesiredArtifact::Instruction(value)] = result.desired.set.artifacts.as_slice() else {
        panic!("expected one instruction contribution")
    };
    value
}
fn reject_capability(item: &Installation, source: &SourceManifest) {
    let before = (item.clone(), source.clone());
    let error = compile(item, source).unwrap_err();
    assert!(
        error.downcast_ref::<CapabilityError>().is_some(),
        "{error:#}"
    );
    assert_eq!((item.clone(), source.clone()), before);
}

#[test]
fn generic_project_reports_five_literal_readers_but_owns_only_requested_codex_cli() {
    let source = captured("", "# Generic\n\nUse clear names.\n", &[]);
    let item = installation(&source, OwnershipScope::Project);
    let result = compile(&item, &source).unwrap();
    assert_eq!(result.desired.set.installation, item);
    assert_eq!(
        contribution(&result).markdown,
        "# Generic\n\nUse clear names.\n"
    );
    assert_eq!(
        contribution(&result).consumers,
        vec![TargetSurface::CodexCli]
    );
    assert_eq!(result.readers.location, location(&item));
    assert_eq!(result.readers.contract_revision, 1);
    assert_eq!(
        result.readers.compatibility,
        InstructionCompatibility::UnconditionalMarkdown
    );
    assert_eq!(
        result
            .readers
            .readers
            .iter()
            .map(|r| (
                r.profile.surface.name(),
                r.profile.contract_revision,
                r.role
            ))
            .collect::<Vec<_>>(),
        vec![
            ("codex.cli", 1, InstructionReaderRole::Requested),
            ("codex.ide", 1, InstructionReaderRole::PotentialIncidental),
            ("copilot.cli", 1, InstructionReaderRole::PotentialIncidental),
            (
                "copilot.vscode-local",
                1,
                InstructionReaderRole::PotentialIncidental
            ),
            (
                "opencode.cli",
                1,
                InstructionReaderRole::PotentialIncidental
            ),
        ]
    );
    let [contract] = result.desired.contracts.as_slice() else {
        panic!()
    };
    assert_eq!(
        contract.profiles,
        vec![ConsumerContract {
            surface: TargetSurface::CodexCli,
            contract_revision: 1
        }]
    );
    assert_eq!(contract.installation_id, item.owner.installation_id);
    assert_eq!(contract.source_manifest_sha256, source.content_sha256);
    assert_eq!(
        contract.contribution_id.as_deref(),
        Some(contribution(&result).id.as_str())
    );
    assert_eq!(contract.root, item.root);
    assert_eq!(contract.path, "AGENTS.md");
    assert_eq!(
        contract.content,
        ContractContent::Instruction {
            content_sha256: hash(contribution(&result).markdown.as_bytes())
        }
    );
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn project_rejects_equal_text_with_excluded_readers_and_family_divergent_directives() {
    for (fields, body) in [
        ("audience: [codex]\n", "Equal body\n"),
        (
            "",
            "<!-- @client:codex -->\nCodex only\n<!-- @endclient -->\n",
        ),
        ("audience: [codex, copilot]\n", "Equal body\n"),
        ("audience: [codex, opencode]\n", "Equal body\n"),
        (
            "",
            "Common\n<!-- @client:opencode -->\nOpenCode behavior\n<!-- @endclient -->\n",
        ),
        (
            "",
            "Common\n<!-- @client:copilot -->\nCopilot behavior\n<!-- @endclient -->\n",
        ),
        (
            "",
            "Common\n<!-- @client:codex -->\nCodex behavior\n<!-- @endclient -->\n",
        ),
    ] {
        let source = captured(fields, body, &[]);
        reject_capability(&installation(&source, OwnershipScope::Project), &source);
    }
}

#[test]
fn global_codex_only_accepts_two_readers_and_stable_framed_identity_across_updates() {
    let first = captured("audience: [codex]\n", "First\n", &[]);
    let item = installation(&first, OwnershipScope::User);
    let a = compile(&item, &first).unwrap();
    assert_eq!(
        a.readers
            .readers
            .iter()
            .map(|r| (r.profile.surface.name(), r.role))
            .collect::<Vec<_>>(),
        vec![
            ("codex.cli", InstructionReaderRole::Requested),
            ("codex.ide", InstructionReaderRole::PotentialIncidental),
        ]
    );
    assert_eq!(contribution(&a).location.root, item.root);
    assert_eq!(contribution(&a).location.root.scope, OwnershipScope::User);
    assert_eq!(contribution(&a).location.path, ".codex/AGENTS.md");
    assert_eq!(contribution(&a).consumers, vec![TargetSurface::CodexCli]);
    let [contract] = a.desired.contracts.as_slice() else {
        panic!("expected one global owner contract")
    };
    assert_eq!(contract.root, item.root);
    assert_eq!(contract.root.scope, OwnershipScope::User);
    assert_eq!(contract.path, ".codex/AGENTS.md");
    assert_eq!(
        contract.profiles,
        vec![ConsumerContract {
            surface: TargetSurface::CodexCli,
            contract_revision: 1,
        }]
    );
    let id = hash(br#"["upskill-codex-contribution-v1","rule-source"]"#);
    assert_eq!(contribution(&a).id, id);
    let second = captured("audience: [codex]\n", "Second\n", &[]);
    let mut updated = installation(&second, OwnershipScope::User);
    updated.owner.source_revision = Some("next-revision".into());
    updated.source_evidence = SourceEvidence::Git {
        resolved_revision: "next-revision".into(),
        manifest_sha256: second.content_sha256.clone(),
    };
    let b = compile(&updated, &second).unwrap();
    assert_eq!(contribution(&b).id, id);
    assert_ne!(
        a.desired.contracts[0].content,
        b.desired.contracts[0].content
    );
    updated.owner.installed_name = "another-alias".into();
    assert_eq!(contribution(&compile(&updated, &second).unwrap()).id, id);
    updated.owner.installation_id = "another-installation".into();
    assert_ne!(contribution(&compile(&updated, &second).unwrap()).id, id);
}

#[test]
fn excluded_codex_retains_history_without_artifacts_contracts_or_compatibility_claim() {
    let source = captured("audience: [claude]\n", "Claude\n", &[]);
    for (scope, path, readers) in [
        (
            OwnershipScope::Project,
            "AGENTS.md",
            vec![
                (
                    TargetFamily::Codex,
                    TargetSurface::CodexCli,
                    1,
                    InstructionReaderRole::Requested,
                ),
                (
                    TargetFamily::Codex,
                    TargetSurface::CodexIde,
                    1,
                    InstructionReaderRole::PotentialIncidental,
                ),
                (
                    TargetFamily::Copilot,
                    TargetSurface::CopilotCli,
                    1,
                    InstructionReaderRole::PotentialIncidental,
                ),
                (
                    TargetFamily::Copilot,
                    TargetSurface::CopilotVscodeLocal,
                    1,
                    InstructionReaderRole::PotentialIncidental,
                ),
                (
                    TargetFamily::OpenCode,
                    TargetSurface::OpenCodeCli,
                    1,
                    InstructionReaderRole::PotentialIncidental,
                ),
            ],
        ),
        (
            OwnershipScope::User,
            ".codex/AGENTS.md",
            vec![
                (
                    TargetFamily::Codex,
                    TargetSurface::CodexCli,
                    1,
                    InstructionReaderRole::Requested,
                ),
                (
                    TargetFamily::Codex,
                    TargetSurface::CodexIde,
                    1,
                    InstructionReaderRole::PotentialIncidental,
                ),
            ],
        ),
    ] {
        let item = installation(&source, scope);
        let result = compile(&item, &source).unwrap();
        assert_eq!(result.desired.set.installation, item);
        assert!(result.desired.set.artifacts.is_empty());
        assert!(result.desired.contracts.is_empty());
        assert_eq!(
            result.desired.set.audience_excluded,
            vec![TargetSurface::CodexCli]
        );
        assert_eq!(result.readers.location.root, item.root);
        assert_eq!(result.readers.location.path, path);
        assert_eq!(result.readers.contract_revision, 1);
        assert_eq!(
            result
                .readers
                .readers
                .iter()
                .map(|reader| (
                    reader.profile.family,
                    reader.profile.surface,
                    reader.profile.contract_revision,
                    reader.role,
                ))
                .collect::<Vec<_>>(),
            readers
        );
        assert_eq!(
            result.readers.compatibility,
            InstructionCompatibility::NotEvaluated
        );
    }
}

#[test]
fn rejects_effective_resources_scope_overrides_native_extensions_and_unknown_fields() {
    let source = captured(
        "ignore: [data.bin, nested/**]\n",
        "Generic\n",
        &[("data.bin", b"ignored"), ("nested/info.md", b"ignored")],
    );
    assert!(compile(&installation(&source, OwnershipScope::Project), &source).is_ok());
    for (fields, extras) in [
        ("", vec![("data.bin", b"effective".as_slice())]),
        ("scope:\n  paths: ['src/**']\n", vec![]),
        (
            "ignore: ['RULE.*.md']\n",
            vec![("RULE.codex.md", b"override".as_slice())],
        ),
        ("", vec![("RULE.copilot.md", b"reader override".as_slice())]),
        (
            "",
            vec![("RULE.opencode.md", b"reader override".as_slice())],
        ),
        ("codex: {}\n", vec![]),
        ("copilot: {}\n", vec![]),
        ("opencode: {}\n", vec![]),
        ("claude: {}\n", vec![]),
        ("future-behavior: true\n", vec![]),
    ] {
        let source = captured(fields, "Generic\n", &extras);
        reject_capability(&installation(&source, OwnershipScope::Project), &source);
    }
    let source = captured(
        "scope:\n  paths: []\nmetadata:\n  author: Test\nlicense: MIT\nrequires: {}\n",
        "Generic\n",
        &[],
    );
    assert!(compile(&installation(&source, OwnershipScope::Project), &source).is_ok());
}

#[test]
fn rejects_empty_unrepresentable_or_managed_marker_bodies_before_desired_output() {
    for body in [
        "",
        " \n\t\n",
        "\0\n",
        "<!-- upskill:begin injected -->\nBody\n",
        "<!-- @client:claude -->\nOnly Claude\n<!-- @endclient -->\n",
    ] {
        let source = captured("", body, &[]);
        assert!(
            compile(&installation(&source, OwnershipScope::User), &source).is_err(),
            "{body:?}"
        );
    }
}

#[test]
fn rejects_wrong_intent_profile_destination_and_malformed_binding() {
    let source = captured("", "Generic\n", &[]);
    for surfaces in [
        vec![],
        vec![TargetSurface::CodexIde],
        vec![TargetSurface::CodexCli, TargetSurface::CodexIde],
        vec![TargetSurface::ClaudeCli],
    ] {
        let mut item = installation(&source, OwnershipScope::Project);
        item.requested = surfaces;
        let error = compile(&item, &source).unwrap_err();
        let capability = error
            .downcast_ref::<CapabilityError>()
            .expect("selection must retain its typed capability error");
        assert_eq!(capability.surface, TargetSurface::CodexCli);
        assert_eq!(capability.key.category, CapabilityCategory::Instructions);
        assert_eq!(capability.key.name, "selection");
    }
    let item = installation(&source, OwnershipScope::Project);
    for profile in [
        TargetSurface::CodexIde.profile(),
        TargetProfile {
            contract_revision: 2,
            ..TargetSurface::CodexCli.profile()
        },
        TargetProfile {
            contract_revision: 0,
            ..TargetSurface::CodexCli.profile()
        },
        TargetProfile {
            family: TargetFamily::Claude,
            ..TargetSurface::CodexCli.profile()
        },
    ] {
        let error = compile_codex_instructions(CodexInstructionRequest {
            installation: &item,
            source: &source,
            folder_name: "folder",
            profile,
            location: &location(&item),
        })
        .unwrap_err();
        assert!(
            error.downcast_ref::<CapabilityError>().is_some(),
            "{error:#}"
        );
    }
    for scope in [OwnershipScope::Project, OwnershipScope::User] {
        let item = installation(&source, scope);
        for path in [
            "custom/AGENTS.md",
            "AGENTS.override.md",
            "../AGENTS.md",
            "/external/AGENTS.md",
            if scope == OwnershipScope::Project {
                ".codex/AGENTS.md"
            } else {
                "AGENTS.md"
            },
        ] {
            let mut destination = location(&item);
            destination.path = path.into();
            assert!(
                compile_codex_instructions(CodexInstructionRequest {
                    installation: &item,
                    source: &source,
                    folder_name: "folder",
                    profile: TargetSurface::CodexCli.profile(),
                    location: &destination
                })
                .is_err()
            );
        }
        let mut destination = location(&item);
        destination.root.path = std::env::temp_dir().join("external-codex-home");
        assert!(
            compile_codex_instructions(CodexInstructionRequest {
                installation: &item,
                source: &source,
                folder_name: "folder",
                profile: TargetSurface::CodexCli.profile(),
                location: &destination
            })
            .is_err()
        );
    }
    for field in ["source-name", "manifest", "alias", "kind"] {
        let mut item = item.clone();
        match field {
            "source-name" => item.owner.source_name = "different".into(),
            "manifest" => {
                item.source_evidence = SourceEvidence::Local {
                    manifest_sha256: "0".repeat(64),
                }
            }
            "alias" => item.owner.installed_name = "../invalid".into(),
            _ => item.owner.kind = ItemKind::Skill,
        }
        assert!(compile(&item, &source).is_err(), "{field}");
    }
    let mut invalid = source.clone();
    invalid.files[0].bytes.push(b'x');
    assert!(compile(&item, &invalid).is_err());
    assert!(
        compile_codex_instructions(CodexInstructionRequest {
            installation: &item,
            source: &source,
            folder_name: "../invalid",
            profile: TargetSurface::CodexCli.profile(),
            location: &location(&item)
        })
        .is_err()
    );
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn captured_lifecycle_keeps_manual_and_other_contribution_bytes_mode_and_contracts() {
    use std::os::unix::fs::PermissionsExt;
    use upskill::files::permissions::ResourceExecutionPolicy;
    use upskill::filesystem::{ExecutionOutcome, QualifiedScope};
    use upskill::lifecycle::commands::{
        CapturedUpsert, LifecycleSession, PrepareOutcome, SessionOutcome,
    };
    use upskill::lifecycle::selection::Selection;
    use upskill::lifecycle::state::{Reference, validate_state};
    use upskill::ownership::ArtifactContent;

    let tmp = tempfile::tempdir().unwrap();
    let root = InstallationRoot {
        scope: OwnershipScope::Project,
        path: tmp.path().join("output"),
    };
    std::fs::create_dir(&root.path).unwrap();
    let manual = b"# Manual\n\nKeep  these bytes.\n\n";
    let agents = root.path.join("AGENTS.md");
    std::fs::write(&agents, manual).unwrap();
    std::fs::set_permissions(&agents, std::fs::Permissions::from_mode(0o640)).unwrap();
    let roots: Vec<_> = ["first", "second"]
        .iter()
        .map(|name| {
            let source = InstallationRoot {
                path: tmp.path().join(name),
                ..root.clone()
            };
            std::fs::create_dir(&source.path).unwrap();
            std::fs::write(
                source.path.join("RULE.md"),
                format!("---\nschema: 1\nname: source\ndescription: demo\n---\n{name} body\n"),
            )
            .unwrap();
            source
        })
        .collect();
    let source_scopes: Vec<_> = roots
        .iter()
        .map(|root| QualifiedScope::open(root).unwrap())
        .collect();
    let scope = QualifiedScope::open(&root).unwrap();
    let guard = scope.try_lock_writer().unwrap();
    let mut second_bytes = Vec::new();
    let mut second_row = None;
    let mut second_contract = None;
    let mut first_id = String::new();
    for step in 0..4 {
        if step == 1 {
            std::fs::write(
                roots[0].path.join("RULE.md"),
                "---\nschema: 1\nname: source\ndescription: demo\n---\nUpdated first body\n",
            )
            .unwrap();
        }
        let snapshots: Vec<_> = source_scopes
            .iter()
            .map(|scope| scope.observe_source_tree().unwrap())
            .collect();
        let SessionOutcome::Ready(session) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        if step < 2 {
            let selection = Selection::Names(vec!["first".into()]);
            let sources: Vec<_> = if step == 0 {
                snapshots.iter().collect()
            } else {
                vec![&snapshots[0]]
            };
            let PrepareOutcome::Ready(prepared) = session
                .prepare_upsert(
                    if step == 0 { None } else { Some(&selection) },
                    &sources,
                    ResourceExecutionPolicy::Disabled,
                    |_| {
                        let mut request = CapturedUpsert::default();
                        for (index, snapshot) in
                            snapshots
                                .iter()
                                .enumerate()
                                .take(if step == 0 { 2 } else { 1 })
                        {
                            let name = ["first", "second"][index];
                            let mut item =
                                installation(snapshot.manifest(), OwnershipScope::Project);
                            item.root = root.clone();
                            item.owner.installation_id = name.into();
                            item.owner.installed_name = name.into();
                            item.owner.source = format!("./{name}");
                            let result = compile(&item, snapshot.manifest())?;
                            if index == 0 {
                                first_id = contribution(&result).id.clone();
                            }
                            if index == 1 {
                                let c = contribution(&result);
                                second_bytes = format!(
                                    "<!-- upskill:begin {} -->\n{}<!-- upskill:end {} -->\n",
                                    c.id, c.markdown, c.id
                                )
                                .into_bytes();
                            }
                            request.sets.push(result.desired.set);
                            request.contracts.extend(result.desired.contracts);
                            if step == 0 {
                                request.add_references.push(Reference::DirectInstallation {
                                    installation_id: name.into(),
                                });
                            }
                        }
                        Ok(request)
                    },
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(
                prepared.apply().unwrap().outcome,
                ExecutionOutcome::Complete
            );
        } else {
            let selection =
                Selection::Names(vec![if step == 2 { "first" } else { "second" }.into()]);
            let PrepareOutcome::Ready(prepared) = session.prepare_remove(&selection).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                prepared.apply().unwrap().outcome,
                ExecutionOutcome::Complete
            );
        }
        let bytes = std::fs::read(&agents).unwrap();
        assert!(bytes.starts_with(manual));
        assert_eq!(
            std::fs::metadata(&agents).unwrap().permissions().mode() & 0o7777,
            0o640
        );
        let SessionOutcome::Ready(current) =
            LifecycleSession::open(&scope, &guard, root.clone()).unwrap()
        else {
            panic!()
        };
        validate_state(current.state()).unwrap();
        let expected_count = [2, 2, 1, 0][step];
        assert_eq!(current.state().installations.len(), expected_count);
        assert_eq!(current.state().artifacts.len(), expected_count);
        assert_eq!(current.state().owner_contracts.len(), expected_count);
        assert_eq!(current.state().references.len(), expected_count);
        for association in &current.state().owner_consumers {
            assert_eq!(association.consumers, vec![TargetSurface::CodexCli]);
        }
        for row in &current.state().artifacts {
            assert_eq!(row.consumers, vec![TargetSurface::CodexCli]);
            assert_eq!(row.owners.len(), 1);
        }
        for contract in &current.state().owner_contracts {
            assert_eq!(
                contract.profiles,
                vec![ConsumerContract {
                    surface: TargetSurface::CodexCli,
                    contract_revision: 1
                }]
            );
        }
        if step < 3 {
            assert!(
                bytes
                    .windows(second_bytes.len())
                    .any(|window| window == second_bytes)
            );
            let row = current
                .state()
                .artifacts
                .iter()
                .find(|row| row.owners[0].installation_id == "second")
                .unwrap();
            let contract = current
                .state()
                .owner_contracts
                .iter()
                .find(|contract| contract.installation_id == "second")
                .unwrap();
            if step == 0 {
                second_row = Some(row.clone());
                second_contract = Some(contract.clone());
            }
            // Separator accounting may change when the preceding contribution is removed;
            // the second body's digest, identity, owners and contract must remain exact.
            let original = second_row.as_ref().unwrap();
            assert_eq!(row.root, original.root);
            assert_eq!(row.path, original.path);
            assert_eq!(row.owners, original.owners);
            let ArtifactContent::MarkdownContribution {
                contribution: retained,
            } = &row.content
            else {
                panic!()
            };
            let ArtifactContent::MarkdownContribution {
                contribution: original,
            } = &original.content
            else {
                panic!()
            };
            assert_eq!(retained.id, original.id);
            assert_eq!(retained.content_sha256, original.content_sha256);
            assert_eq!(contract, second_contract.as_ref().unwrap());
        }
        if step == 1 {
            assert!(String::from_utf8_lossy(&bytes).contains(&format!(
                "<!-- upskill:begin {first_id} -->\nUpdated first body\n"
            )));
        }
        if step == 2 {
            assert!(!String::from_utf8_lossy(&bytes).contains(&first_id));
        }
        if step == 3 {
            assert_eq!(bytes, manual);
            assert!(current.state().owner_consumers.is_empty());
        }
    }
}

#[test]
fn historical_captured_compiler_keeps_codex_audience_out_of_existing_clients() {
    use upskill::compile::item::{ItemCompileRequest, compile_item};
    use upskill::generate::Client;
    for (audience, expected) in [("codex", vec![]), ("codex, claude", vec![Client::Claude])] {
        let source = captured(&format!("audience: [{audience}]\n"), "Generic\n", &[]);
        let result = compile_item(
            &source,
            &ItemCompileRequest {
                kind: ItemKind::Rule,
                folder_name: "folder".into(),
                alias: None,
                clients: Client::ALL.to_vec(),
            },
        )
        .unwrap();
        assert_eq!(
            result
                .outputs
                .iter()
                .map(|output| output.client)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            result.audience_excluded,
            if expected.is_empty() {
                Client::ALL.to_vec()
            } else {
                vec![Client::Copilot, Client::OpenCode]
            }
        );
    }
}

#[test]
fn historical_local_install_writes_only_claude_for_mixed_codex_audience() {
    use upskill::generate::Client;
    use upskill::pipeline::install_from_local_path;
    for audience in ["codex", "codex, claude"] {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("source");
        let target = tmp.path().join("target");
        std::fs::create_dir_all(source.join("source")).unwrap();
        std::fs::write(source.join("source/RULE.md"), format!("---\nschema: 1\nname: source\ndescription: demo\naudience: [{audience}]\n---\nGeneric\n")).unwrap();
        let report = install_from_local_path(&source, &target, None).unwrap();
        assert_eq!(
            report
                .items
                .iter()
                .map(|item| item.client)
                .collect::<Vec<_>>(),
            if audience == "codex" {
                vec![]
            } else {
                vec![Client::Claude]
            }
        );
        assert_eq!(
            target.join(".claude/rules/source.md").exists(),
            audience != "codex"
        );
        for path in [
            "AGENTS.md",
            ".codex",
            ".github/instructions/source.instructions.md",
            ".agents/rules/source/RULE.md",
        ] {
            assert!(!target.join(path).exists(), "unexpected output {path}");
        }
    }
}

#[test]
fn impossible_captured_file_topology_rejects_before_resource_ignore_filtering() {
    use upskill::source_manifest::{CapturedSourceFile, validate_manifest};
    let mut source = captured("ignore: ['RULE.md/**']\n", "Generic\n", &[]);
    source.files.push(CapturedSourceFile {
        path: "RULE.md/ignored.bin".into(),
        bytes: b"ignored".to_vec(),
        content_sha256: hash(b"ignored"),
        mode: None,
    });
    // Independently encode the captured-manifest digest for a synthetic tree
    // that cannot be constructed through ordinary filesystem observation.
    let mut framed = Sha256::new();
    framed.update(b"upskill:source-manifest:v1\0");
    framed.update((source.files.len() as u64).to_be_bytes());
    for file in &source.files {
        framed.update((file.path.len() as u64).to_be_bytes());
        framed.update(file.path.as_bytes());
        framed.update((file.bytes.len() as u64).to_be_bytes());
        framed.update(&file.bytes);
        match file.mode {
            None => framed.update([0]),
            Some(mode) => {
                framed.update([1]);
                framed.update(mode.to_be_bytes());
            }
        }
    }
    source.content_sha256 = framed
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    validate_manifest(&source).unwrap();
    let error = compile(&installation(&source, OwnershipScope::Project), &source).unwrap_err();
    assert!(
        format!("{error:#}").contains("topology conflict"),
        "{error:#}"
    );
}

#[test]
fn rejects_scope_only_destination_mismatch_and_relative_installation_root() {
    let source = captured("", "Generic\n", &[]);
    let item = installation(&source, OwnershipScope::Project);
    let mut destination = location(&item);
    destination.root.scope = OwnershipScope::User;
    assert!(
        compile_codex_instructions(CodexInstructionRequest {
            installation: &item,
            source: &source,
            folder_name: "folder",
            profile: TargetSurface::CodexCli.profile(),
            location: &destination,
        })
        .is_err()
    );
    let mut invalid = item;
    invalid.root.path = "relative-output".into();
    assert!(compile(&invalid, &source).is_err());
}

#[test]
fn canonical_formatting_and_compatible_directives_bind_literal_output_digest() {
    for (scope, body, expected) in [
        (
            OwnershipScope::Project,
            "## Heading\nUse `clear` names.",
            "## Heading\n\nUse `clear` names.\n",
        ),
        (
            OwnershipScope::Project,
            "<!-- @client:codex,copilot,opencode -->\n# Shared\n\nShared text.\n<!-- @endclient -->\n<!-- @client:claude -->\nExcluded text.\n<!-- @endclient -->",
            "# Shared\n\nShared text.\n",
        ),
        (
            OwnershipScope::Project,
            "<!-- @client:codex -->\n## Shared\nText.\n<!-- @endclient -->\n<!-- @client:copilot -->\n## Shared\n\nText.\n<!-- @endclient -->\n<!-- @client:opencode -->\n## Shared\n\n\nText.\n<!-- @endclient -->",
            "## Shared\n\nText.\n",
        ),
        (
            OwnershipScope::User,
            "<!-- @client:codex -->\nGlobal Codex text.\n<!-- @endclient -->",
            "Global Codex text.\n",
        ),
    ] {
        let source = captured("", body, &[]);
        let result = compile(&installation(&source, scope), &source).unwrap();
        assert_eq!(contribution(&result).markdown, expected);
        assert_eq!(
            result.desired.contracts[0].content,
            ContractContent::Instruction {
                content_sha256: hash(expected.as_bytes())
            }
        );
        assert_eq!(
            result.readers.compatibility,
            InstructionCompatibility::UnconditionalMarkdown
        );
    }
}

#[test]
fn absent_declared_source_name_uses_captured_folder_binding() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("RULE.md"),
        "---\nschema: 1\ndescription: demo\n---\nGeneric\n",
    )
    .unwrap();
    let source = capture_source(tmp.path()).unwrap();
    let mut item = installation(&source, OwnershipScope::Project);
    item.owner.source_name = "folder".into();
    assert_eq!(
        compile(&item, &source)
            .unwrap()
            .desired
            .set
            .installation
            .owner
            .source_name,
        "folder"
    );
}

#[test]
fn absent_declared_source_name_rejects_a_different_captured_folder_binding() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("RULE.md"),
        "---\nschema: 1\ndescription: demo\n---\nGeneric\n",
    )
    .unwrap();
    let source = capture_source(tmp.path()).unwrap();
    let mut item = installation(&source, OwnershipScope::Project);
    item.owner.source_name = "different-folder".into();
    let error = compile(&item, &source).unwrap_err();
    assert!(
        error.downcast_ref::<CapabilityError>().is_none(),
        "{error:#}"
    );
    let context = format!("{error:#}");
    assert!(
        context.contains("compile Codex instructions rule-source"),
        "{context}"
    );
    assert!(
        context.contains("source name differs from installation binding"),
        "{context}"
    );
}

#[test]
fn excluded_sources_reject_unsupported_semantics_before_audience_exclusion() {
    for (fields, extras, capability_name) in [
        ("scope:\n  paths: ['src/**']\n", vec![], "scope"),
        ("codex: {}\n", vec![], "extensions"),
        ("copilot: {}\n", vec![], "extensions"),
        ("opencode: {}\n", vec![], "extensions"),
        ("claude: {}\n", vec![], "extensions"),
        ("future-behavior: true\n", vec![], "extensions"),
        (
            "ignore: ['RULE.*.md']\n",
            vec![("RULE.codex.md", b"required override".as_slice())],
            "override",
        ),
        (
            "",
            vec![("RULE.copilot.md", b"required override".as_slice())],
            "override",
        ),
        (
            "",
            vec![("data.bin", b"effective resource".as_slice())],
            "resources",
        ),
    ] {
        let source = captured(
            &format!("audience: [claude]\n{fields}"),
            "Excluded\n",
            &extras,
        );
        for scope in [OwnershipScope::Project, OwnershipScope::User] {
            let item = installation(&source, scope);
            let before = (item.clone(), source.clone());
            let error = compile(&item, &source).unwrap_err();
            let capability = error
                .downcast_ref::<CapabilityError>()
                .expect("unsupported excluded source must retain its typed capability error");
            assert_eq!(capability.surface, TargetSurface::CodexCli);
            assert_eq!(capability.key.category, CapabilityCategory::Instructions);
            assert_eq!(capability.key.name, capability_name);
            assert_eq!((item, source.clone()), before);
        }
    }
}
