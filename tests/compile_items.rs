use std::fs;
use upskill::compile::item::{ItemCompileRequest, compile_item};
use upskill::generate::Client;
use upskill::pipeline::ItemKind;
use upskill::source_manifest::{SourceManifest, capture_source};

fn fixture(files: &[(&str, &[u8])]) -> SourceManifest {
    let directory = tempfile::tempdir().unwrap();
    for (path, bytes) in files {
        let destination = directory.path().join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(destination, bytes).unwrap();
    }
    capture_source(directory.path()).unwrap()
}

#[test]
fn captured_skill_emits_entrypoint_and_exact_binary_resource() {
    let source = fixture(&[
        (
            "SKILL.md",
            b"---\nschema: 1\ndescription: demo\n---\n\n# Demo\n",
        ),
        ("assets/data.bin", &[0, 255]),
    ]);
    let request = ItemCompileRequest {
        kind: ItemKind::Skill,
        folder_name: "demo".into(),
        alias: None,
        clients: vec![Client::Claude],
    };
    let before = source.clone();
    let result = compile_item(&source, &request).expect("compile captured skill");
    assert_eq!(result.outputs.len(), 1);
    let files = &result.outputs[0].files;
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, ".claude/skills/demo/SKILL.md");
    assert_eq!(
        files[0].bytes,
        b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n"
    );
    assert_eq!(files[0].mode, None);
    assert_eq!(files[1].path, ".claude/skills/demo/assets/data.bin");
    assert_eq!(files[1].bytes, [0, 255]);
    assert_eq!(files[1].mode, source.files[1].mode);
    assert_eq!(source, before);
}

#[test]
fn tampered_manifest_is_rejected() {
    let mut source = fixture(&[(
        "SKILL.md",
        b"---\nschema: 1\ndescription: demo\n---\n\n# Demo\n",
    )]);
    source.content_sha256 = "0".repeat(64);
    let request = ItemCompileRequest {
        kind: ItemKind::Skill,
        folder_name: "demo".into(),
        alias: None,
        clients: vec![Client::Claude],
    };
    let result = compile_item(&source, &request);
    assert!(
        result.is_err(),
        "a tampered source manifest must be rejected"
    );
}

use sha2::{Digest, Sha256};
use upskill::compile::item::{CompiledClientItem, CompiledItemFile};
use upskill::source_manifest::{CapturedSourceFile, validate_manifest};

const ENTRY: &[u8] = b"---\nschema: 1\ndescription: demo\n---\n\n# Demo\n";

fn request(kind: ItemKind) -> ItemCompileRequest {
    ItemCompileRequest {
        kind,
        folder_name: "demo".into(),
        alias: None,
        clients: Client::ALL.to_vec(),
    }
}

fn expected_file(path: &str, bytes: &[u8], mode: Option<u32>) -> CompiledItemFile {
    CompiledItemFile {
        path: path.into(),
        bytes: bytes.into(),
        mode,
    }
}

fn rejected(source: &SourceManifest, request: &ItemCompileRequest, context: &str) {
    let before_source = source.clone();
    let before_request = request.clone();
    let error =
        compile_item(source, request).expect_err("invalid input must return no compilation");
    let message = format!("{error:#}");
    assert!(
        message.contains(context),
        "expected {context:?} in {message:?}"
    );
    assert_eq!(source, &before_source);
    assert_eq!(request, &before_request);
}

// Independent test encoding of the documented source-manifest framed protocol.
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
    validate_manifest(&source).expect("synthetic manifest protocol must be valid");
    source
}

#[test]
fn all_nine_kind_client_pairs_match_checked_in_goldens_and_complete_resources() {
    let cases = [
        (
            ItemKind::Skill,
            "create-api-endpoint",
            include_bytes!("fixtures/items/create-api-endpoint/SKILL.md").as_slice(),
            [
                include_bytes!("fixtures/expected/claude/create-api-endpoint.SKILL.md").as_slice(),
                include_bytes!("fixtures/expected/copilot/create-api-endpoint.SKILL.md").as_slice(),
                include_bytes!("fixtures/expected/opencode/create-api-endpoint.SKILL.md")
                    .as_slice(),
            ],
            [
                ".claude/skills/create-api-endpoint/SKILL.md",
                ".github/skills/create-api-endpoint/SKILL.md",
                ".agents/skills/create-api-endpoint/SKILL.md",
            ],
            [
                ".claude/skills/create-api-endpoint/data.bin",
                ".github/skills/create-api-endpoint/data.bin",
                ".agents/skills/create-api-endpoint/data.bin",
            ],
        ),
        (
            ItemKind::Rule,
            "api-conventions",
            include_bytes!("fixtures/items/api-conventions/RULE.md").as_slice(),
            [
                include_bytes!("fixtures/expected/claude/api-conventions.RULE.md").as_slice(),
                include_bytes!("fixtures/expected/copilot/api-conventions.RULE.md").as_slice(),
                include_bytes!("fixtures/expected/opencode/api-conventions.RULE.md").as_slice(),
            ],
            [
                ".claude/rules/api-conventions.md",
                ".github/instructions/api-conventions.instructions.md",
                ".agents/rules/api-conventions/RULE.md",
            ],
            [
                ".claude/rules/api-conventions/data.bin",
                ".github/instructions/api-conventions/data.bin",
                ".agents/rules/api-conventions/data.bin",
            ],
        ),
        (
            ItemKind::Agent,
            "security-reviewer",
            include_bytes!("fixtures/items/security-reviewer/AGENT.md").as_slice(),
            [
                include_bytes!("fixtures/expected/claude/security-reviewer.AGENT.md").as_slice(),
                include_bytes!("fixtures/expected/copilot/security-reviewer.AGENT.md").as_slice(),
                include_bytes!("fixtures/expected/opencode/security-reviewer.AGENT.md").as_slice(),
            ],
            [
                ".claude/agents/security-reviewer.md",
                ".github/agents/security-reviewer.agent.md",
                ".opencode/agents/security-reviewer.md",
            ],
            [
                ".claude/agents/security-reviewer/data.bin",
                ".github/agents/security-reviewer/data.bin",
                ".opencode/agents/security-reviewer/data.bin",
            ],
        ),
    ];
    for (kind, name, input, goldens, entries, resources) in cases {
        let captured = fixture(&[
            (kind.entrypoint_filename(), input),
            ("data.bin", &[0, 255, 17]),
        ]);
        let captured_mode = captured
            .files
            .iter()
            .find(|file| file.path == "data.bin")
            .unwrap()
            .mode;
        let distinct_modes = synthetic(&[
            (kind.entrypoint_filename(), input, Some(0o700)),
            ("data.bin", &[0, 255, 17], Some(0o640)),
        ]);
        for (source, mode) in [(captured, captured_mode), (distinct_modes, Some(0o640))] {
            let before = source.clone();
            let req = ItemCompileRequest {
                folder_name: name.into(),
                ..request(kind)
            };
            let req_before = req.clone();
            let result = compile_item(&source, &req).unwrap();
            assert_eq!(result.kind, kind);
            assert_eq!(result.source_name, name);
            assert_eq!(result.installed_name, name);
            assert_eq!(result.source_manifest_sha256, source.content_sha256);
            assert_eq!(result.audience_excluded, []);
            assert_eq!(result.outputs.len(), 3);
            for (i, output) in result.outputs.iter().enumerate() {
                assert_eq!(output.client, Client::ALL[i]);
                let mut expected = vec![
                    expected_file(entries[i], goldens[i], None),
                    expected_file(resources[i], &[0, 255, 17], mode),
                ];
                expected.sort_by(|a, b| a.path.cmp(&b.path));
                assert_eq!(output.files, expected, "{kind} {:?}", output.client);
            }
            assert_eq!(source, before);
            assert_eq!(req, req_before);
        }
    }
}

#[test]
fn resource_vectors_preserve_empty_binary_unicode_hidden_files_and_independent_modes() {
    let source = synthetic(&[
        ("SKILL.md", ENTRY, Some(0o7777)),
        (".hidden", b"secret", None),
        ("a.bin", &[0, 255, 128], Some(0)),
        ("empty", b"", Some(0o755)),
        ("文/é.txt", "café 🌍".as_bytes(), Some(0o7777)),
    ]);
    let mut req = request(ItemKind::Skill);
    req.clients = vec![Client::Claude];
    let compiled = compile_item(&source, &req).unwrap();
    assert_eq!(
        compiled.outputs[0].files,
        vec![
            expected_file(".claude/skills/demo/.hidden", b"secret", None),
            expected_file(
                ".claude/skills/demo/SKILL.md",
                b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n",
                None
            ),
            expected_file(".claude/skills/demo/a.bin", &[0, 255, 128], Some(0)),
            expected_file(".claude/skills/demo/empty", b"", Some(0o755)),
            expected_file(
                ".claude/skills/demo/文/é.txt",
                "café 🌍".as_bytes(),
                Some(0o7777)
            ),
        ]
    );
}

#[test]
fn root_reserved_names_are_excluded_but_nested_equivalents_and_unknown_suffixes_survive() {
    let mut entries = vec![
        ("SKILL.md".to_owned(), ENTRY.to_vec()),
        ("RULE.md".into(), vec![255]),
        ("AGENT.md".into(), vec![255]),
    ];
    for kind in ItemKind::ALL {
        for client in Client::ALL {
            let stem = kind.entrypoint_filename().strip_suffix(".md").unwrap();
            let name = format!("{stem}.{}.md", client.name());
            if kind != ItemKind::Skill || client != Client::Claude {
                entries.push((name.clone(), vec![255]));
            }
            entries.push((format!("nested/{name}"), vec![0, 255]));
        }
        entries.push((
            format!("nested/{}", kind.entrypoint_filename()),
            vec![0, 255],
        ));
    }
    entries.push(("notes.claude.md".into(), b"ordinary".to_vec()));
    entries.push(("SKILL.future.md".into(), b"future".to_vec()));
    let source = fixture(
        &entries
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_slice()))
            .collect::<Vec<_>>(),
    );
    let req = ItemCompileRequest {
        clients: vec![Client::Claude],
        ..request(ItemKind::Skill)
    };
    let actual = compile_item(&source, &req).unwrap();
    let mut expected = vec![expected_file(
        ".claude/skills/demo/SKILL.md",
        b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n",
        None,
    )];
    for (path, bytes) in &entries {
        if path.starts_with("nested/") || path == "notes.claude.md" || path == "SKILL.future.md" {
            let mode = source.files.iter().find(|f| f.path == *path).unwrap().mode;
            expected.push(expected_file(
                &format!(".claude/skills/demo/{path}"),
                bytes,
                mode,
            ));
        }
    }
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(actual.outputs[0].files, expected);
}

#[test]
fn active_overrides_are_rejected_for_every_kind_client_even_when_ignored() {
    for kind in ItemKind::ALL {
        for client in Client::ALL {
            let filename = format!(
                "{}.{}.md",
                kind.entrypoint_filename().strip_suffix(".md").unwrap(),
                client.name()
            );
            let entry = format!(
                "---\nschema: 1\ndescription: demo\nignore: ['{filename}']\n---\n\n# Demo\n"
            );
            let source = fixture(&[
                (kind.entrypoint_filename(), entry.as_bytes()),
                (&filename, b"override"),
            ]);
            let req = ItemCompileRequest {
                clients: vec![client],
                ..request(kind)
            };
            rejected(&source, &req, "unsupported override");
            rejected(&source, &req, &filename);
        }
    }
}

#[test]
fn overrides_for_audience_excluded_clients_do_not_block_compilation() {
    let source = fixture(&[
        (
            "SKILL.md",
            b"---\nschema: 1\ndescription: demo\naudience: [copilot]\n---\n\n# Demo\n",
        ),
        ("SKILL.claude.md", &[255]),
        ("SKILL.opencode.md", &[255]),
    ]);
    let result = compile_item(&source, &request(ItemKind::Skill)).unwrap();
    assert_eq!(result.audience_excluded, [Client::Claude, Client::OpenCode]);
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.outputs[0].client, Client::Copilot);
    assert_eq!(
        result.outputs[0].files,
        [expected_file(
            ".github/skills/demo/SKILL.md",
            b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n",
            None
        )]
    );
}

#[test]
fn ignore_patterns_filter_resources_and_never_rewrite_ignored_links() {
    for kind in ItemKind::ALL {
        let entry = b"---\nschema: 1\ndescription: demo\nignore: ['*.log', 'scripts/**', 'a/*.txt', '?.md', 'fixtures']\n---\n\n[ignored](./drop.log) [kept](./a/b/x.txt)\n";
        let source = fixture(&[
            (kind.entrypoint_filename(), entry),
            ("drop.log", b"log"),
            ("nested/drop.log", b"log"),
            ("scripts/gate.sh", b"sh"),
            ("scripts/nested/tool", b"tool"),
            ("a/x.txt", b"text"),
            ("a/b/x.txt", b"keep"),
            ("x.md", b"md"),
            ("ab.md", b"keep"),
            ("fixtures/data", b"data"),
            ("nested/fixtures/data", b"data"),
        ]);
        let req = ItemCompileRequest {
            clients: vec![Client::Claude],
            ..request(kind)
        };
        let result = compile_item(&source, &req).unwrap();
        let (entrypath, base, header, link) = match kind {
            ItemKind::Skill => (
                ".claude/skills/demo/SKILL.md",
                ".claude/skills/demo",
                "---\nname: demo\ndescription: demo\n---\n\n",
                "./a/b/x.txt",
            ),
            ItemKind::Rule => (
                ".claude/rules/demo.md",
                ".claude/rules/demo",
                "---\nname: demo\ndescription: demo\n---\n\n",
                "./demo/a/b/x.txt",
            ),
            ItemKind::Agent => (
                ".claude/agents/demo.md",
                ".claude/agents/demo",
                "---\nname: demo\ndescription: demo\n---\n\n",
                "./demo/a/b/x.txt",
            ),
        };
        let expected = format!("{header}[ignored](./drop.log) [kept]({link})\n");
        let mut files = vec![expected_file(entrypath, expected.as_bytes(), None)];
        for path in ["a/b/x.txt", "ab.md"] {
            files.push(expected_file(
                &format!("{base}/{path}"),
                b"keep",
                source.files.iter().find(|f| f.path == path).unwrap().mode,
            ));
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(result.outputs[0].files, files, "{kind}");
    }
}

#[test]
fn alias_retains_source_identity_and_both_link_passes_for_flat_outputs() {
    let body = "[relative](./refs/a.md) ![image](refs/a.md) [old](./source/refs/a.md)\n\n[reference][r]\n\n[r]: ./refs/a.md\n\n[url](https://example.com/refs/a.md) [fragment](#part) [missing](./missing.md)\n\n`[code](./refs/a.md)`\n\n```md\n[fenced](./refs/a.md)\n```\n";
    let rewritten = "[relative](./alias/refs/a.md) ![image](alias/refs/a.md) [old](./alias/refs/a.md)\n\n[reference][r]\n\n[r]: ./alias/refs/a.md\n\n[url](https://example.com/refs/a.md) [fragment](#part) [missing](./missing.md)\n\n`[code](./refs/a.md)`\n\n```md\n[fenced](./refs/a.md)\n```\n";
    for kind in [ItemKind::Rule, ItemKind::Agent] {
        let entry = format!("---\nschema: 1\nname: source\ndescription: demo\n---\n\n{body}");
        let source = fixture(&[
            (kind.entrypoint_filename(), entry.as_bytes()),
            ("refs/a.md", b"reference"),
        ]);
        let req = ItemCompileRequest {
            folder_name: "folder".into(),
            alias: Some("alias".into()),
            ..request(kind)
        };
        let result = compile_item(&source, &req).unwrap();
        assert_eq!(result.source_name, "source");
        assert_eq!(result.installed_name, "alias");
        for (i, output) in result.outputs.iter().enumerate() {
            let (path, base, header, expected_body) = match (kind, i) {
                (ItemKind::Rule, 0) => (
                    ".claude/rules/alias.md",
                    ".claude/rules/alias",
                    "---\nname: source\ndescription: demo\n---\n\n",
                    rewritten,
                ),
                (ItemKind::Rule, 1) => (
                    ".github/instructions/alias.instructions.md",
                    ".github/instructions/alias",
                    "---\nname: source\ndescription: demo\napplyTo: '**'\n---\n\n",
                    rewritten,
                ),
                (ItemKind::Rule, 2) => (
                    ".agents/rules/alias/RULE.md",
                    ".agents/rules/alias",
                    "---\nname: source\ndescription: demo\n---\n\n",
                    body,
                ),
                (ItemKind::Agent, 0) => (
                    ".claude/agents/alias.md",
                    ".claude/agents/alias",
                    "---\nname: source\ndescription: demo\n---\n\n",
                    rewritten,
                ),
                (ItemKind::Agent, 1) => (
                    ".github/agents/alias.agent.md",
                    ".github/agents/alias",
                    "---\nname: source\ndescription: demo\n---\n\n",
                    rewritten,
                ),
                (ItemKind::Agent, 2) => (
                    ".opencode/agents/alias.md",
                    ".opencode/agents/alias",
                    "---\nname: source\ndescription: demo\nmode: subagent\n---\n\n",
                    rewritten,
                ),
                _ => unreachable!(),
            };
            let mut expected = vec![
                expected_file(path, format!("{header}{expected_body}").as_bytes(), None),
                expected_file(
                    &format!("{base}/refs/a.md"),
                    b"reference",
                    source
                        .files
                        .iter()
                        .find(|f| f.path == "refs/a.md")
                        .unwrap()
                        .mode,
                ),
            ];
            expected.sort_by(|a, b| a.path.cmp(&b.path));
            assert_eq!(output.files, expected, "{kind} {:?}", output.client);
        }
    }
}

#[test]
fn skill_folder_is_authoritative_and_alias_keeps_directory_links() {
    let source = fixture(&[
        (
            "SKILL.md",
            b"---\nschema: 1\nname: invalid/source\ndescription: demo\n---\n\n[resource](./x)\n",
        ),
        ("x", &[255]),
    ]);
    let req = ItemCompileRequest {
        folder_name: "文".into(),
        alias: Some("é".into()),
        clients: vec![Client::Claude],
        ..request(ItemKind::Skill)
    };
    let result = compile_item(&source, &req).unwrap();
    assert_eq!(result.source_name, "文");
    assert_eq!(result.installed_name, "é");
    assert_eq!(
        result.outputs[0].files,
        [
            expected_file(
                ".claude/skills/é/SKILL.md",
                "---\nname: 文\ndescription: demo\n---\n\n[resource](./x)\n".as_bytes(),
                None
            ),
            expected_file(".claude/skills/é/x", &[255], source.files[1].mode)
        ]
    );
}

#[test]
fn selection_order_and_audience_exclusions_follow_client_declaration_order() {
    for kind in ItemKind::ALL {
        let source = fixture(&[(
            kind.entrypoint_filename(),
            b"---\nschema: 1\ndescription: demo\naudience: [copilot]\n---\n\n# Demo\n",
        )]);
        let normal = request(kind);
        let reverse = ItemCompileRequest {
            clients: vec![Client::OpenCode, Client::Copilot, Client::Claude],
            ..normal.clone()
        };
        let result = compile_item(&source, &normal).unwrap();
        assert_eq!(result, compile_item(&source, &reverse).unwrap());
        assert_eq!(result.audience_excluded, [Client::Claude, Client::OpenCode]);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].client, Client::Copilot);
        let subset = ItemCompileRequest {
            clients: vec![Client::OpenCode, Client::Claude],
            ..normal
        };
        let excluded = compile_item(&source, &subset).unwrap();
        assert_eq!(excluded.outputs, []);
        assert_eq!(
            excluded.audience_excluded,
            [Client::Claude, Client::OpenCode]
        );
        let empty_audience = fixture(&[(
            kind.entrypoint_filename(),
            b"---\nschema: 1\ndescription: demo\naudience: []\n---\n",
        )]);
        let result = compile_item(&empty_audience, &request(kind)).unwrap();
        assert_eq!(result.outputs, []);
        assert_eq!(result.audience_excluded, Client::ALL);
    }
    let source = fixture(&[("SKILL.md", ENTRY)]);
    let normal = request(ItemKind::Skill);
    let reverse = ItemCompileRequest {
        clients: vec![Client::OpenCode, Client::Copilot, Client::Claude],
        ..normal.clone()
    };
    assert_eq!(
        compile_item(&source, &normal).unwrap(),
        compile_item(&source, &reverse).unwrap()
    );
    for (clients, context) in [
        (vec![], "nonempty"),
        (vec![Client::Claude, Client::Claude], "duplicate"),
    ] {
        rejected(
            &source,
            &ItemCompileRequest {
                clients,
                ..normal.clone()
            },
            context,
        );
    }
}

#[test]
fn invalid_names_fail_and_valid_unicode_names_survive() {
    for invalid in ["", ".", "..", "a/b", "a\\b", "a:b", "a\nb", "a\u{7f}b"] {
        let source = fixture(&[("SKILL.md", ENTRY)]);
        rejected(
            &source,
            &ItemCompileRequest {
                folder_name: invalid.into(),
                ..request(ItemKind::Skill)
            },
            "folder name",
        );
        rejected(
            &source,
            &ItemCompileRequest {
                alias: Some(invalid.into()),
                ..request(ItemKind::Skill)
            },
            "alias",
        );
        for kind in [ItemKind::Rule, ItemKind::Agent] {
            let name = serde_yaml_ng::to_string(invalid).unwrap();
            let entry = format!(
                "---\nschema: 1\nname: {}\ndescription: demo\n---\n",
                name.trim()
            );
            let source = fixture(&[(kind.entrypoint_filename(), entry.as_bytes())]);
            rejected(&source, &request(kind), "source name");
        }
    }
    for kind in [ItemKind::Rule, ItemKind::Agent] {
        let source = fixture(&[(
            kind.entrypoint_filename(),
            "---\nschema: 1\nname: 文-é\ndescription: demo\n---\n".as_bytes(),
        )]);
        let result = compile_item(&source, &request(kind)).unwrap();
        assert_eq!(result.source_name, "文-é");
        assert_eq!(result.installed_name, "文-é");
        assert!(
            result
                .outputs
                .iter()
                .all(|o| o.files[0].path.contains("文-é"))
        );
    }
}

#[test]
fn canonical_entrypoint_failures_are_contextual() {
    for kind in ItemKind::ALL {
        rejected(
            &fixture(&[("resource", b"content")]),
            &request(kind),
            kind.entrypoint_filename(),
        );
        for (bytes, context) in [
            (&[255][..], "UTF-8"),
            (&b"---\nschema: [\n---\n"[..], "parse"),
            (
                &b"---\nschema: 999\ndescription: demo\n---\n"[..],
                "schema version",
            ),
            (&b"no frontmatter"[..], "parse"),
        ] {
            rejected(
                &fixture(&[(kind.entrypoint_filename(), bytes)]),
                &request(kind),
                context,
            );
        }
    }
}

#[test]
fn complete_manifest_is_validated_including_ignored_files() {
    let valid = fixture(&[
        (
            "SKILL.md",
            b"---\nschema: 1\ndescription: demo\nignore: ['ignored']\n---\n",
        ),
        ("ignored", b"resource"),
    ]);
    for corruption in 0..7 {
        let mut source = valid.clone();
        match corruption {
            0 => source.files[1].path = "../escape".into(),
            1 => source.files.reverse(),
            2 => source.files[1].content_sha256 = "0".repeat(64),
            3 => source.content_sha256 = "0".repeat(64),
            4 => source.files[1].mode = Some(0o10000),
            5 => source.files[1].bytes.push(255),
            6 => source.files[1] = source.files[0].clone(),
            _ => unreachable!(),
        }
        rejected(&source, &request(ItemKind::Skill), "source manifest");
    }
}

#[test]
fn impossible_source_topology_is_rejected_even_when_ignored_and_nonadjacent() {
    let source = synthetic(&[
        (
            "SKILL.md",
            b"---\nschema: 1\ndescription: demo\nignore: ['a']\n---\n",
            None,
        ),
        ("a", b"file", None),
        ("a-b", b"sibling", None),
        ("a/b", b"descendant", None),
    ]);
    rejected(&source, &request(ItemKind::Skill), "topology");
}

#[test]
fn changing_and_removing_original_source_cannot_change_captured_output() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("SKILL.md"), ENTRY).unwrap();
    fs::write(directory.path().join("data"), [0, 255]).unwrap();
    let source = capture_source(directory.path()).unwrap();
    let before = source.clone();
    fs::write(directory.path().join("SKILL.md"), "invalid replacement").unwrap();
    fs::write(directory.path().join("data"), "different").unwrap();
    directory.close().unwrap();
    let req = ItemCompileRequest {
        clients: vec![Client::Claude],
        ..request(ItemKind::Skill)
    };
    let result = compile_item(&source, &req).unwrap();
    assert_eq!(result.source_manifest_sha256, before.content_sha256);
    assert_eq!(
        result.outputs[0].files,
        [
            expected_file(
                ".claude/skills/demo/SKILL.md",
                b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n",
                None
            ),
            expected_file(".claude/skills/demo/data", &[0, 255], source.files[1].mode)
        ]
    );
    assert_eq!(source, before);
}

#[test]
fn explicit_claude_and_opencode_audiences_emit_exact_outputs_for_every_kind() {
    for kind in ItemKind::ALL {
        for (audience, client, exclusions) in [
            (
                "claude",
                Client::Claude,
                [Client::Copilot, Client::OpenCode],
            ),
            (
                "opencode",
                Client::OpenCode,
                [Client::Claude, Client::Copilot],
            ),
        ] {
            let entry = format!(
                "---\nschema: 1\ndescription: demo\naudience: [{audience}]\n---\n\n# Demo\n"
            );
            let source = fixture(&[(kind.entrypoint_filename(), entry.as_bytes())]);
            let result = compile_item(&source, &request(kind)).unwrap();
            let (path, bytes) = match (kind, client) {
                (ItemKind::Skill, Client::Claude) => (
                    ".claude/skills/demo/SKILL.md",
                    b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n".as_slice(),
                ),
                (ItemKind::Skill, Client::OpenCode) => (
                    ".agents/skills/demo/SKILL.md",
                    b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n".as_slice(),
                ),
                (ItemKind::Rule, Client::Claude) => (
                    ".claude/rules/demo.md",
                    b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n".as_slice(),
                ),
                (ItemKind::Rule, Client::OpenCode) => (
                    ".agents/rules/demo/RULE.md",
                    b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n".as_slice(),
                ),
                (ItemKind::Agent, Client::Claude) => (
                    ".claude/agents/demo.md",
                    b"---\nname: demo\ndescription: demo\n---\n\n# Demo\n".as_slice(),
                ),
                (ItemKind::Agent, Client::OpenCode) => (
                    ".opencode/agents/demo.md",
                    b"---\nname: demo\ndescription: demo\nmode: subagent\n---\n\n# Demo\n"
                        .as_slice(),
                ),
                _ => unreachable!(),
            };
            assert_eq!(result.audience_excluded, exclusions, "{kind} {audience}");
            assert_eq!(
                result.outputs,
                [CompiledClientItem {
                    client,
                    files: vec![expected_file(path, bytes, None)]
                }],
                "{kind} {audience}"
            );
        }
    }
}

#[test]
fn wholly_audience_excluded_requests_still_validate_ignored_and_resource_files() {
    for kind in ItemKind::ALL {
        let source = fixture(&[
            (
                kind.entrypoint_filename(),
                b"---\nschema: 1\ndescription: demo\naudience: []\nignore: [ignored]\n---\n",
            ),
            ("ignored", b"ignored bytes"),
            ("resource", b"retained bytes"),
        ]);
        let req = request(kind);
        let valid = compile_item(&source, &req).unwrap();
        assert_eq!(valid.outputs, []);
        assert_eq!(valid.audience_excluded, Client::ALL);
        for path in ["ignored", "resource"] {
            let mut corrupt = source.clone();
            corrupt
                .files
                .iter_mut()
                .find(|file| file.path == path)
                .unwrap()
                .bytes
                .push(255);
            rejected(&corrupt, &req, "validate source manifest");
            rejected(&corrupt, &req, "source file digest mismatch");
            rejected(&corrupt, &req, path);
        }
    }
}

#[test]
fn changing_only_ignored_or_reserved_files_changes_exact_source_identity_not_outputs() {
    for kind in ItemKind::ALL {
        let entry = b"---\nschema: 1\ndescription: demo\nignore: [ignored]\n---\n\n# Demo\n";
        let reserved = ItemKind::ALL
            .into_iter()
            .find(|other| *other != kind)
            .unwrap()
            .entrypoint_filename();
        for changed in ["ignored", reserved] {
            let first = synthetic(&[
                (kind.entrypoint_filename(), entry, Some(0o700)),
                ("ignored", b"original ignored", Some(0o600)),
                (reserved, b"original reserved", Some(0o644)),
                ("resource", &[0, 255], Some(0o755)),
            ]);
            let mut values: Vec<_> = first
                .files
                .iter()
                .map(|file| (file.path.as_str(), file.bytes.as_slice(), file.mode))
                .collect();
            values
                .iter_mut()
                .find(|(path, _, _)| *path == changed)
                .unwrap()
                .1 = b"changed excluded bytes";
            let second = synthetic(&values);
            assert_ne!(first.content_sha256, second.content_sha256);
            let compiled_first = compile_item(&first, &request(kind)).unwrap();
            let compiled_second = compile_item(&second, &request(kind)).unwrap();
            assert_eq!(compiled_first.source_manifest_sha256, first.content_sha256);
            assert_eq!(
                compiled_second.source_manifest_sha256,
                second.content_sha256
            );
            assert_ne!(
                compiled_first.source_manifest_sha256,
                compiled_second.source_manifest_sha256
            );
            assert_eq!(
                compiled_first.outputs, compiled_second.outputs,
                "{kind} {changed}"
            );
            assert_eq!(
                compiled_first.audience_excluded,
                compiled_second.audience_excluded
            );
        }
    }
}
