use upskill::legacy::{
    LegacyClient, LegacyDiagnostic, LegacyDiagnosticKind, LegacyIntegration, LegacyLocation,
    RequestedSelection, project_legacy,
};
use upskill::lockfile::{
    LockedBundle, LockedItem, LockedMcp, LockedPlugin, Lockfile, McpInstallStatus,
    PluginInstallStatus,
};
use upskill::pipeline::ItemKind;

fn item() -> LockedItem {
    LockedItem {
        kind: ItemKind::Skill,
        name: "installed alias".into(),
        source: "gitlab+gitlab.example.com:org/team/repo@v1:skills/tool".into(),
        git_ref: Some("v1".into()),
        hash: Some("a".repeat(64)),
        source_name: Some("tool".into()),
        required_by: vec![
            "rule:parent".into(),
            "rule:parent".into(),
            "agent:other".into(),
        ],
        group: Some("skills/tool".into()),
        clients: vec![],
    }
}

fn bundle() -> LockedBundle {
    LockedBundle {
        name: "bundle alias".into(),
        source: "github:org/registry@main:skills/base.bundle.yaml".into(),
        git_ref: Some("main".into()),
        items: vec![
            "installed alias".into(),
            "other".into(),
            "installed alias".into(),
        ],
    }
}

fn full_state() -> Lockfile {
    let mut lock = Lockfile::new();
    lock.items = vec![item(), item()];
    lock.items[1].source_name = None;
    lock.items[1].hash = None;
    lock.items[1].git_ref = None;
    lock.items[1].group = None;
    lock.bundles = vec![bundle(), bundle()];
    for status in [
        PluginInstallStatus::Instructions,
        PluginInstallStatus::Installed,
        PluginInstallStatus::Skipped,
    ] {
        lock.plugins.push(LockedPlugin {
            name: "plugin".into(),
            client: "future target".into(),
            identifier: "plugin@registry".into(),
            scope: Some("future scope".into()),
            bundle: "bundle alias".into(),
            status,
        });
    }
    lock.plugins.push(lock.plugins[0].clone());
    lock.plugins[1].scope = None;
    for status in [McpInstallStatus::Skipped, McpInstallStatus::Installed] {
        lock.mcps.push(LockedMcp {
            name: "server".into(),
            client: "unknown MCP client".into(),
            scope: Some("arbitrary scope".into()),
            bundle: "bundle alias".into(),
            status,
        });
    }
    lock.mcps.push(lock.mcps[0].clone());
    lock.mcps[1].scope = None;
    lock
}

fn expected_integrations(lock: &Lockfile) -> Vec<LegacyIntegration> {
    vec![
        LegacyIntegration::Plugin {
            index: 0,
            record: lock.plugins[0].clone(),
        },
        LegacyIntegration::Plugin {
            index: 1,
            record: lock.plugins[1].clone(),
        },
        LegacyIntegration::Plugin {
            index: 2,
            record: lock.plugins[2].clone(),
        },
        LegacyIntegration::Plugin {
            index: 3,
            record: lock.plugins[3].clone(),
        },
        LegacyIntegration::Mcp {
            index: 0,
            record: lock.mcps[0].clone(),
        },
        LegacyIntegration::Mcp {
            index: 1,
            record: lock.mcps[1].clone(),
        },
        LegacyIntegration::Mcp {
            index: 2,
            record: lock.mcps[2].clone(),
        },
    ]
}

#[test]
fn empty_clients_preserve_state_and_expand_only_historical_clients() {
    let mut lock = Lockfile::new();
    lock.items.push(item());
    let before = lock.clone();
    let projected = project_legacy(&lock);
    assert_eq!(lock, before);
    assert_eq!(projected.snapshot, before);
    assert_eq!(projected.items[0].index, 0);
    assert_eq!(projected.items[0].record, before.items[0]);
    assert_eq!(
        projected.items[0].requested_selection,
        RequestedSelection::Unresolved
    );
    assert_eq!(
        projected.items[0].clients,
        vec![
            LegacyClient::Claude,
            LegacyClient::Copilot,
            LegacyClient::OpenCode
        ]
    );
    assert_eq!(projected.items[0].candidates.len(), 3);
    assert_eq!(
        projected.items[0].candidates[0].entrypoint,
        ".claude/skills/installed alias/SKILL.md"
    );
    assert_eq!(
        projected.items[0].source.as_ref().unwrap().to_string(),
        "https://gitlab.example.com/org/team/repo@v1:skills/tool"
    );
    assert!(projected.diagnostics.is_empty());
}

#[test]
fn all_nine_paths_use_installed_alias_and_literal_historical_layout() {
    let cases = [
        (
            ItemKind::Skill,
            "claude",
            LegacyClient::Claude,
            ".claude/skills/x/SKILL.md",
            ".claude/skills/x",
        ),
        (
            ItemKind::Skill,
            "copilot",
            LegacyClient::Copilot,
            ".github/skills/x/SKILL.md",
            ".github/skills/x",
        ),
        (
            ItemKind::Skill,
            "opencode",
            LegacyClient::OpenCode,
            ".agents/skills/x/SKILL.md",
            ".agents/skills/x",
        ),
        (
            ItemKind::Rule,
            "claude",
            LegacyClient::Claude,
            ".claude/rules/x.md",
            ".claude/rules/x",
        ),
        (
            ItemKind::Rule,
            "copilot",
            LegacyClient::Copilot,
            ".github/instructions/x.instructions.md",
            ".github/instructions/x",
        ),
        (
            ItemKind::Rule,
            "opencode",
            LegacyClient::OpenCode,
            ".agents/rules/x/RULE.md",
            ".agents/rules/x",
        ),
        (
            ItemKind::Agent,
            "claude",
            LegacyClient::Claude,
            ".claude/agents/x.md",
            ".claude/agents/x",
        ),
        (
            ItemKind::Agent,
            "copilot",
            LegacyClient::Copilot,
            ".github/agents/x.agent.md",
            ".github/agents/x",
        ),
        (
            ItemKind::Agent,
            "opencode",
            LegacyClient::OpenCode,
            ".opencode/agents/x.md",
            ".opencode/agents/x",
        ),
    ];
    for (kind, raw_client, client, entrypoint, resource_search_base) in cases {
        let mut record = item();
        record.kind = kind;
        record.name = "x".into();
        record.clients = vec![raw_client.into()];
        let lock = Lockfile {
            items: vec![record],
            ..Lockfile::new()
        };
        let projection = project_legacy(&lock);
        let candidates = &projection.items[0].candidates;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].client, client);
        assert_eq!(candidates[0].entrypoint, entrypoint);
        assert_eq!(candidates[0].resource_search_base, resource_search_base);
        assert!(projection.diagnostics.is_empty());
    }
}

#[test]
fn singleton_clients_do_not_reconstruct_requested_selection() {
    for (raw, client) in [
        ("claude", LegacyClient::Claude),
        ("copilot", LegacyClient::Copilot),
    ] {
        let mut record = item();
        record.clients = vec![raw.into()];
        let lock = Lockfile {
            items: vec![record],
            ..Lockfile::new()
        };
        let projection = project_legacy(&lock);
        assert_eq!(
            projection.items[0].requested_selection,
            RequestedSelection::Unresolved
        );
        assert_eq!(projection.items[0].clients, vec![client]);
        assert_eq!(projection.items[0].candidates.len(), 1);
    }
}

#[test]
fn raw_clients_are_preserved_and_unknown_spellings_diagnosed() {
    let mut record = item();
    record.clients = vec![
        "claude".into(),
        "codex".into(),
        "claude".into(),
        "COPILOT".into(),
    ];
    let lock = Lockfile {
        items: vec![record],
        ..Lockfile::new()
    };
    let projection = project_legacy(&lock);
    assert_eq!(projection.snapshot, lock);
    assert_eq!(projection.items[0].record, lock.items[0]);
    assert_eq!(projection.items[0].clients, vec![LegacyClient::Claude]);
    assert_eq!(projection.items[0].candidates.len(), 1);
    assert_eq!(
        projection.diagnostics,
        vec![
            LegacyDiagnostic {
                location: LegacyLocation::Item(0),
                kind: LegacyDiagnosticKind::UnknownClient {
                    client: "codex".into()
                }
            },
            LegacyDiagnostic {
                location: LegacyLocation::Item(0),
                kind: LegacyDiagnosticKind::UnknownClient {
                    client: "COPILOT".into()
                }
            },
        ]
    );
}

#[test]
fn invalid_sources_retain_rows_and_known_candidate_facts() {
    let mut bad_item = item();
    bad_item.source = "gitlab:org/repo@".into();
    let mut bad_bundle = bundle();
    bad_bundle.source = "not a persisted label".into();
    let lock = Lockfile {
        items: vec![bad_item, item()],
        bundles: vec![bundle(), bad_bundle],
        ..Lockfile::new()
    };
    let projection = project_legacy(&lock);
    assert_eq!(projection.snapshot, lock);
    assert_eq!(projection.items[0].record, lock.items[0]);
    assert_eq!(projection.items[0].index, 0);
    assert!(projection.items[0].source.is_none());
    assert_eq!(projection.items[0].candidates.len(), 3);
    assert!(projection.items[1].source.is_some());
    assert_eq!(projection.bundles[1].record, lock.bundles[1]);
    assert_eq!(projection.bundles[1].index, 1);
    assert!(projection.bundles[1].source.is_none());
    assert!(projection.bundles[0].source.is_some());
    for (location, label) in [
        (LegacyLocation::Item(0), &lock.items[0].source),
        (LegacyLocation::Bundle(1), &lock.bundles[1].source),
    ] {
        let message = upskill::source::parse_install_source_label(label)
            .unwrap_err()
            .to_string();
        assert!(projection.diagnostics.contains(&LegacyDiagnostic {
            location,
            kind: LegacyDiagnosticKind::InvalidSource { message }
        }));
    }
}

#[test]
fn source_labels_decode_without_promoting_recorded_metadata() {
    for (label, canonical) in [
        ("gitlab:org/team/repo", "https://gitlab.com/org/team/repo"),
        (
            "gitlab+gitlab.example.com:org/team/repo@v1:skills/tool",
            "https://gitlab.example.com/org/team/repo@v1:skills/tool",
        ),
        (
            "https://git.example.com/org/repo@main:skills/tool",
            "https://git.example.com/org/repo@main:skills/tool",
        ),
        (
            "github:org/repo@branch:skills/tool",
            "github:org/repo@branch:skills/tool",
        ),
        ("local:./registry", "local:./registry"),
    ] {
        let mut record = item();
        record.source = label.into();
        let lock = Lockfile {
            items: vec![record],
            ..Lockfile::new()
        };
        let projection = project_legacy(&lock);
        assert_eq!(
            projection.items[0].source.as_ref().unwrap().to_string(),
            canonical
        );
        assert_eq!(projection.items[0].record, lock.items[0]);
        assert!(projection.diagnostics.is_empty());
    }
}

#[test]
fn invalid_names_suppress_candidates_but_retain_other_facts() {
    for name in ["", ".", "..", "a/b", "a\\b", "C:x", "a\n", "a\u{7f}"] {
        let mut record = item();
        record.name = name.into();
        let lock = Lockfile {
            items: vec![record],
            ..Lockfile::new()
        };
        let projection = project_legacy(&lock);
        assert_eq!(projection.snapshot, lock);
        assert_eq!(projection.items[0].record, lock.items[0]);
        assert!(projection.items[0].source.is_some());
        assert_eq!(projection.items[0].clients.len(), 3);
        assert!(projection.items[0].candidates.is_empty());
        assert_eq!(
            projection.diagnostics,
            vec![LegacyDiagnostic {
                location: LegacyLocation::Item(0),
                kind: LegacyDiagnosticKind::InvalidInstalledName { name: name.into() }
            }]
        );
    }
    for name in ["with spaces", "工具", "a.b"] {
        let mut record = item();
        record.name = name.into();
        let projection = project_legacy(&Lockfile {
            items: vec![record],
            ..Lockfile::new()
        });
        assert_eq!(projection.items[0].candidates.len(), 3);
        assert_eq!(
            projection.items[0].candidates[0].entrypoint,
            format!(".claude/skills/{name}/SKILL.md")
        );
        assert!(projection.diagnostics.is_empty());
    }
}

#[test]
fn duplicate_identity_and_entrypoint_diagnostics_include_every_row() {
    let mut record = item();
    record.clients = vec!["claude".into()];
    let lock = Lockfile {
        items: vec![record.clone(), record.clone(), record],
        ..Lockfile::new()
    };
    let projection = project_legacy(&lock);
    assert_eq!(projection.snapshot, lock);
    for (index, row) in projection.items.iter().enumerate() {
        assert_eq!(row.index, index);
        assert_eq!(row.record, lock.items[index]);
        assert_eq!(row.candidates.len(), 1);
    }
    assert_eq!(
        projection.diagnostics,
        vec![
            LegacyDiagnostic {
                location: LegacyLocation::Item(0),
                kind: LegacyDiagnosticKind::DuplicateItemIdentity {
                    kind: ItemKind::Skill,
                    name: "installed alias".into(),
                    indices: vec![0, 1, 2]
                }
            },
            LegacyDiagnostic {
                location: LegacyLocation::Item(0),
                kind: LegacyDiagnosticKind::IdenticalCandidateEntrypoint {
                    entrypoint: ".claude/skills/installed alias/SKILL.md".into(),
                    indices: vec![0, 1, 2]
                }
            },
        ]
    );
}

#[test]
fn disjoint_clients_collide_only_in_identity_and_kinds_remain_separate() {
    let mut first = item();
    first.clients = vec!["claude".into()];
    let mut second = first.clone();
    second.clients = vec!["copilot".into()];
    let mut third = first.clone();
    third.kind = ItemKind::Rule;
    let lock = Lockfile {
        items: vec![first, second, third],
        ..Lockfile::new()
    };
    let projection = project_legacy(&lock);
    assert_eq!(projection.items.len(), 3);
    assert_eq!(
        projection.diagnostics,
        vec![LegacyDiagnostic {
            location: LegacyLocation::Item(0),
            kind: LegacyDiagnosticKind::DuplicateItemIdentity {
                kind: ItemKind::Skill,
                name: "installed alias".into(),
                indices: vec![0, 1]
            },
        }]
    );
}

#[test]
fn typed_metadata_vector_order_and_tagged_integrations_survive() {
    let lock = full_state();
    let before = lock.clone();
    let projection = project_legacy(&lock);
    assert_eq!(lock, before);
    assert_eq!(projection.snapshot, before);
    assert_eq!(projection.items.len(), 2);
    assert_eq!(projection.bundles.len(), 2);
    for (index, row) in projection.items.iter().enumerate() {
        assert_eq!(row.index, index);
        assert_eq!(row.record, before.items[index]);
    }
    for (index, row) in projection.bundles.iter().enumerate() {
        assert_eq!(row.index, index);
        assert_eq!(row.record, before.bundles[index]);
    }
    let expected = expected_integrations(&before);
    assert_eq!(projection.integrations, expected);
}

#[test]
fn unsupported_schemas_preserve_indexed_records_without_interpretation() {
    for schema in [0, 2] {
        let mut lock = full_state();
        lock.schema = schema;
        lock.items[0].source = "invalid".into();
        lock.items[0].clients = vec!["codex".into()];
        let projection = project_legacy(&lock);
        assert_eq!(projection.snapshot, lock);
        assert_eq!(projection.items.len(), lock.items.len());
        assert_eq!(projection.bundles.len(), lock.bundles.len());
        for (index, row) in projection.items.iter().enumerate() {
            assert_eq!(row.index, index);
            assert_eq!(row.record, lock.items[index]);
            assert!(row.source.is_none());
            assert!(row.clients.is_empty());
            assert!(row.candidates.is_empty());
            assert_eq!(row.requested_selection, RequestedSelection::Unresolved);
        }
        for (index, row) in projection.bundles.iter().enumerate() {
            assert_eq!(row.index, index);
            assert_eq!(row.record, lock.bundles[index]);
            assert!(row.source.is_none());
        }
        assert_eq!(projection.integrations, expected_integrations(&lock));
        assert_eq!(
            projection.diagnostics,
            vec![LegacyDiagnostic {
                location: LegacyLocation::Global,
                kind: LegacyDiagnosticKind::UnsupportedSchema { schema }
            }]
        );
    }
}

#[test]
fn projection_paths_are_deterministic_and_scope_root_independent() {
    let lock = Lockfile {
        items: vec![item()],
        ..Lockfile::new()
    };
    let project_scope = project_legacy(&lock);
    let user_scope = project_legacy(&lock);
    assert_eq!(project_scope, user_scope);
    assert_eq!(
        project_scope.items[0].candidates[0].entrypoint,
        ".claude/skills/installed alias/SKILL.md"
    );
    assert_eq!(
        project_scope.items[0].candidates[0].resource_search_base,
        ".claude/skills/installed alias"
    );
}
