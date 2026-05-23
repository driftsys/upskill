# Plugin Descriptor Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the bundle `plugins:` schema to support instructions-only and config-write descriptor variants alongside the existing auto-install mode.

**Architecture:** Each per-client plugin descriptor (`ClaudePluginDescriptor`, etc.) becomes a `#[serde(untagged)]` enum with variants discriminated by field presence. The pipeline gains match arms for the new variants. The lockfile gets a new `Instructions` status. opencode config-write reuses the `ancillary.rs` JSON-merge pattern.

**Tech Stack:** Rust (edition 2024), serde + serde_yaml_ng (untagged enums), serde_json (opencode.json manipulation), existing test infrastructure (tempfile, assert_cmd).

**Spec:** `docs/superpowers/specs/2026-05-23-plugin-descriptor-modes-design.md`

---

## File Map

| File                         | Action | Responsibility                                                                                                     |
| ---------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------ |
| `src/model/bundle.rs`        | Modify | Convert 4 descriptor structs to untagged enums                                                                     |
| `src/lockfile.rs`            | Modify | Add `Instructions` variant to `PluginInstallStatus`                                                                |
| `src/plugin.rs`              | Modify | Add `ManualInstructions` variant to `PluginOutcome`; update function signatures to accept enum refs                |
| `src/pipeline.rs`            | Modify | Update `install_plugins_from_bundles` to match on enum variants; add config-write logic; update lockfile recording |
| `src/ancillary.rs`           | Modify | Add `ensure_opencode_plugin_registered` and `remove_opencode_plugin` functions                                     |
| `src/main.rs`                | Modify | Update `print_plugin_results` to handle instructions-only and config-write output                                  |
| `src/parse/bundle.rs`        | Modify | Update tests for new descriptor variants                                                                           |
| `docs/format-spec.md`        | Modify | Document new descriptor forms, lockfile status, disambiguation rules                                               |
| `tests/pipeline_lockfile.rs` | Modify | Add tests for instructions and config-write lockfile recording                                                     |

---

### Task 1: Model — Convert Descriptors to Untagged Enums

**Files:**

- Modify: `src/model/bundle.rs`

- [ ] **Step 1: Write tests for new descriptor variants parsing**

Add tests at the bottom of `src/model/bundle.rs` (or in `src/parse/bundle.rs` where bundle tests live) that verify YAML parsing of all three forms:

```rust
#[test]
fn opencode_descriptor_parses_install_variant() {
    let yaml = "module: superpowers-opencode\ninstall_url: https://example.com\n";
    let desc: OpencodePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, OpencodePluginDescriptor::Install { .. }));
}

#[test]
fn opencode_descriptor_parses_config_write_variant() {
    let yaml = "plugin_uri: \"superpowers@git+https://github.com/obra/superpowers.git\"\ninstall_url: https://example.com\n";
    let desc: OpencodePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, OpencodePluginDescriptor::ConfigWrite { .. }));
}

#[test]
fn opencode_descriptor_parses_instructions_variant() {
    let yaml = "instructions_url: https://github.com/obra/superpowers/INSTALL.md\nsummary: Do the thing\n";
    let desc: OpencodePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, OpencodePluginDescriptor::Instructions { .. }));
}

#[test]
fn claude_descriptor_parses_install_variant() {
    let yaml = "source: anthropics/claude-plugins\nplugin: superpowers\n";
    let desc: ClaudePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, ClaudePluginDescriptor::Install { .. }));
}

#[test]
fn claude_descriptor_parses_instructions_variant() {
    let yaml = "instructions_url: https://example.com/install\nsummary: Install manually\n";
    let desc: ClaudePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, ClaudePluginDescriptor::Instructions { .. }));
}

#[test]
fn vscode_descriptor_parses_install_variant() {
    let yaml = "extension: anthropic.superpowers\n";
    let desc: VscodePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, VscodePluginDescriptor::Install { .. }));
}

#[test]
fn vscode_descriptor_parses_instructions_variant() {
    let yaml = "instructions_url: https://marketplace.example.com\n";
    let desc: VscodePluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, VscodePluginDescriptor::Instructions { .. }));
}

#[test]
fn copilot_descriptor_parses_install_variant() {
    let yaml = "source: obra/marketplace\nplugin: superpowers\n";
    let desc: CopilotPluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, CopilotPluginDescriptor::Install { .. }));
}

#[test]
fn copilot_descriptor_parses_instructions_variant() {
    let yaml = "instructions_url: https://example.com/manual\n";
    let desc: CopilotPluginDescriptor = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(matches!(desc, CopilotPluginDescriptor::Instructions { .. }));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib -- descriptor_parses`
Expected: FAIL — types are still structs, not enums.

- [ ] **Step 3: Convert descriptor structs to untagged enums**

Replace the four descriptor struct definitions in `src/model/bundle.rs`:

```rust
/// Install descriptor for Claude Code plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudePluginDescriptor {
    /// Auto-install via `claude plugin marketplace add` + `claude plugin install`.
    Install {
        source: String,
        plugin: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Instructions-only: no shellout, prints summary + URL.
    Instructions {
        instructions_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}

/// Install descriptor for GitHub Copilot CLI plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CopilotPluginDescriptor {
    /// Auto-install via copilot CLI.
    Install {
        source: String,
        plugin: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Instructions-only.
    Instructions {
        instructions_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}

/// Install descriptor for VS Code extensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VscodePluginDescriptor {
    /// Auto-install via `code --install-extension`.
    Install {
        extension: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Instructions-only.
    Instructions {
        instructions_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}

/// Install descriptor for opencode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpencodePluginDescriptor {
    /// Auto-install via `opencode plugin <module>`.
    Install {
        module: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Config-write: upskill adds `plugin_uri` to opencode.json.
    ConfigWrite {
        plugin_uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        install_url: Option<String>,
    },
    /// Instructions-only.
    Instructions {
        instructions_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
    },
}
```

- [ ] **Step 4: Run the new tests to verify they pass**

Run: `cargo test --lib -- descriptor_parses`
Expected: PASS — all 9 tests green.

- [ ] **Step 5: Commit**

```bash
git add src/model/bundle.rs
git commit -m "feat(model): convert plugin descriptors to untagged enums (#168)

Adds Instructions variant to all four client descriptors and
ConfigWrite variant to OpencodePluginDescriptor. Existing YAML
with source/plugin/module/extension fields still parses into the
Install variant (backward-compatible)."
```

---

### Task 2: Fix Compilation — Update plugin.rs Callers

**Files:**

- Modify: `src/plugin.rs`

After Task 1, the codebase won't compile because `plugin.rs` functions accept `&ClaudePluginDescriptor` (formerly a struct with `descriptor.source` etc.) and now it's an enum.

- [ ] **Step 1: Add `ManualInstructions` variant to `PluginOutcome`**

In `src/plugin.rs`, add the new variant:

```rust
pub enum PluginOutcome {
    /// Command executed successfully (exit code 0).
    Success,
    /// Client CLI not found on PATH — plugin skipped.
    CliNotFound,
    /// Client CLI found but command returned non-zero.
    Failed {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// Instructions-only descriptor — no CLI action taken.
    ManualInstructions,
}
```

Update `is_success()` and `is_cli_not_found()` to NOT match `ManualInstructions`:

```rust
impl PluginOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn is_cli_not_found(&self) -> bool {
        matches!(self, Self::CliNotFound)
    }

    /// True when this is an instructions-only outcome (no automated action).
    pub fn is_manual_instructions(&self) -> bool {
        matches!(self, Self::ManualInstructions)
    }
}
```

- [ ] **Step 2: Update install function signatures**

Change the install functions to accept the `Install` variant's fields directly (extracted by the caller in `pipeline.rs`). The function signatures stay the same since they already take `&ClaudePluginDescriptor` etc., but we need to document that callers must destructure the enum first. Actually, the cleanest path: keep functions taking the struct-like data via new helper structs or just match in pipeline.rs and pass the individual fields.

The install functions in `plugin.rs` already take the descriptor by reference. Since they now receive enums, update them to take the inner data. The simplest approach: keep the current function signatures but change them to accept the specific variant data. Since serde enums with named fields can't easily be borrowed as structs, convert the functions to accept field values directly:

```rust
/// Install a Claude Code plugin via CLI.
pub fn install_claude_plugin(
    source: &str,
    plugin: &str,
    scope: PluginScope,
) -> PluginOutcome {
    // Step 1: Add marketplace source (idempotent).
    let result = run_command("claude", &["plugin", "marketplace", "add", source]);
    match result {
        PluginOutcome::CliNotFound => return PluginOutcome::CliNotFound,
        PluginOutcome::Failed { .. } => return result,
        PluginOutcome::Success => {}
        PluginOutcome::ManualInstructions => unreachable!(),
    }

    // Step 2: Install plugin with scope.
    let install_ref = format!("{plugin}@{source}");
    run_command(
        "claude",
        &["plugin", "install", &install_ref, "--scope", scope.as_claude_flag()],
    )
}

/// Install a VS Code extension via `code --install-extension`.
pub fn install_vscode_extension(extension: &str) -> PluginOutcome {
    run_command("code", &["--install-extension", extension])
}

/// Install an opencode module via `opencode plugin`.
pub fn install_opencode_plugin(module: &str) -> PluginOutcome {
    run_command("opencode", &["plugin", module])
}

/// Install a GitHub Copilot CLI plugin via CLI.
pub fn install_copilot_plugin(source: &str, plugin: &str) -> PluginOutcome {
    let result = run_command("copilot", &["plugin", "marketplace", "add", source]);
    match result {
        PluginOutcome::CliNotFound => return PluginOutcome::CliNotFound,
        PluginOutcome::Failed { .. } => return result,
        PluginOutcome::Success => {}
        PluginOutcome::ManualInstructions => unreachable!(),
    }

    let install_ref = format!("{plugin}@{source}");
    run_command("copilot", &["plugin", "install", &install_ref])
}
```

Similarly update uninstall functions to take `&str` args instead of the old struct fields.

- [ ] **Step 3: Fix compilation errors**

Run: `cargo check`
Expected: Still fails in `pipeline.rs` (Task 3), `parse/bundle.rs` tests. That's expected — we'll fix those next.

- [ ] **Step 4: Run plugin.rs unit tests**

Run: `cargo test --lib plugin`
Expected: PASS — the plugin module's own tests should pass (they test `run_command` directly, not the descriptors).

- [ ] **Step 5: Commit**

```bash
git add src/plugin.rs
git commit -m "refactor(plugin): accept field values instead of descriptor refs (#168)

Prepares for the enum-based descriptors by making install functions
take individual field values (&str) rather than whole descriptor
references. Adds ManualInstructions variant to PluginOutcome."
```

---

### Task 3: Fix Compilation — Update pipeline.rs

**Files:**

- Modify: `src/pipeline.rs`

- [ ] **Step 1: Update `install_plugins_from_bundles` to match on enum variants**

Replace the current body of `install_plugins_from_bundles` with enum-matching logic:

```rust
fn install_plugins_from_bundles(
    bundles: &[crate::model::Bundle],
    scope: crate::plugin::PluginScope,
    target: &Path,
) -> Vec<PluginResult> {
    use crate::model::bundle::{
        ClaudePluginDescriptor, CopilotPluginDescriptor, OpencodePluginDescriptor,
        VscodePluginDescriptor,
    };

    let mut results = Vec::new();

    for bundle in bundles {
        for (plugin_name, entry) in &bundle.plugins {
            // Claude
            if let Some(claude) = &entry.claude {
                match claude {
                    ClaudePluginDescriptor::Install { source, plugin, install_url } => {
                        let outcome = crate::plugin::install_claude_plugin(source, plugin, scope);
                        let identifier = format!("{plugin}@{source}");
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "claude".into(),
                            outcome,
                            identifier,
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    ClaudePluginDescriptor::Instructions { instructions_url, summary } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "claude".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: instructions_url.clone(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }

            // Copilot
            if let Some(copilot) = &entry.copilot {
                match copilot {
                    CopilotPluginDescriptor::Install { source, plugin, install_url } => {
                        let outcome = crate::plugin::install_copilot_plugin(source, plugin);
                        let identifier = format!("{plugin}@{source}");
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "copilot".into(),
                            outcome,
                            identifier,
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    CopilotPluginDescriptor::Instructions { instructions_url, summary } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "copilot".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: instructions_url.clone(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }

            // VS Code
            if let Some(vscode) = &entry.vscode {
                match vscode {
                    VscodePluginDescriptor::Install { extension, install_url } => {
                        let outcome = crate::plugin::install_vscode_extension(extension);
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "vscode".into(),
                            outcome,
                            identifier: extension.clone(),
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    VscodePluginDescriptor::Instructions { instructions_url, summary } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "vscode".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: instructions_url.clone(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }

            // opencode
            if let Some(opencode) = &entry.opencode {
                match opencode {
                    OpencodePluginDescriptor::Install { module, install_url } => {
                        let outcome = crate::plugin::install_opencode_plugin(module);
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "opencode".into(),
                            outcome,
                            identifier: module.clone(),
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    OpencodePluginDescriptor::ConfigWrite { plugin_uri, install_url } => {
                        let outcome = crate::ancillary::write_opencode_plugin_uri(target, plugin_uri);
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "opencode".into(),
                            outcome,
                            identifier: plugin_uri.clone(),
                            bundle: bundle.name.clone(),
                            install_url: install_url.clone(),
                            instructions_url: None,
                            summary: None,
                        });
                    }
                    OpencodePluginDescriptor::Instructions { instructions_url, summary } => {
                        results.push(PluginResult {
                            name: plugin_name.clone(),
                            client: "opencode".into(),
                            outcome: crate::plugin::PluginOutcome::ManualInstructions,
                            identifier: instructions_url.clone(),
                            bundle: bundle.name.clone(),
                            install_url: None,
                            instructions_url: Some(instructions_url.clone()),
                            summary: summary.clone(),
                        });
                    }
                }
            }
        }
    }

    results
}
```

- [ ] **Step 2: Update `PluginResult` struct to include new fields**

```rust
pub struct PluginResult {
    pub name: String,
    pub client: String,
    pub outcome: crate::plugin::PluginOutcome,
    pub identifier: String,
    pub bundle: String,
    pub install_url: Option<String>,
    /// Instructions URL for instructions-only descriptors.
    pub instructions_url: Option<String>,
    /// Summary text for instructions-only descriptors.
    pub summary: Option<String>,
}
```

- [ ] **Step 3: Update the call site to pass `target`**

The function signature now takes `target: &Path`. Update the call at line ~299:

```rust
let plugin_results = install_plugins_from_bundles(&report.bundles, plugin_scope, target);
```

- [ ] **Step 4: Update lockfile recording to handle ManualInstructions**

In the lockfile recording loop (around line 333), add the new variant:

```rust
let status = match &pr.outcome {
    PluginOutcome::Success => PluginInstallStatus::Installed,
    PluginOutcome::CliNotFound => PluginInstallStatus::Skipped,
    PluginOutcome::ManualInstructions => PluginInstallStatus::Instructions,
    PluginOutcome::Failed { .. } => continue,
};
```

- [ ] **Step 5: Update doctor to handle Instructions status**

In the doctor function's plugin reconciliation loop, add the `Instructions` arm:

```rust
match &plugin.status {
    PluginInstallStatus::Skipped => {
        report.skipped_plugins.push(SkippedPlugin { /* ... */ });
    }
    PluginInstallStatus::Instructions => {
        // Instructions-only: informational, no reconciliation needed.
        // Treat same as skipped for reporting purposes.
        report.skipped_plugins.push(SkippedPlugin {
            name: plugin.name.clone(),
            client: plugin.client.clone(),
            identifier: plugin.identifier.clone(),
            bundle: plugin.bundle.clone(),
        });
    }
    PluginInstallStatus::Installed => {
        // existing reconciliation logic unchanged
    }
}
```

- [ ] **Step 6: Verify compilation passes**

Run: `cargo check`
Expected: May still fail in `main.rs` or tests (will fix in subsequent tasks). Core library should compile.

- [ ] **Step 7: Commit**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): handle instructions-only and config-write plugin variants (#168)

Updates install_plugins_from_bundles to match on enum variants.
Instructions-only produces ManualInstructions outcome. Config-write
calls ancillary::write_opencode_plugin_uri. Lockfile records
Instructions status for the new variant."
```

---

### Task 4: Lockfile — Add Instructions Status

**Files:**

- Modify: `src/lockfile.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn instructions_plugin_roundtrips_through_save_and_load() {
    let tmp = tempfile::tempdir().unwrap();
    let mut lock = Lockfile::new();
    lock.upsert_plugin(LockedPlugin {
        name: "superpowers".into(),
        client: "opencode".into(),
        identifier: "https://github.com/obra/superpowers/INSTALL.md".into(),
        scope: None,
        bundle: "superpowers".into(),
        status: PluginInstallStatus::Instructions,
    });
    lock.save(tmp.path()).expect("save");
    let loaded = Lockfile::load(tmp.path()).expect("load");
    assert_eq!(loaded.plugins[0].status, PluginInstallStatus::Instructions);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib -- instructions_plugin_roundtrips`
Expected: FAIL — `PluginInstallStatus::Instructions` does not exist yet.

- [ ] **Step 3: Add Instructions variant**

In `src/lockfile.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginInstallStatus {
    #[default]
    Installed,
    Skipped,
    /// Instructions-only — no automated install performed; consumer must
    /// follow manual steps. Recorded so doctor can surface the URL.
    Instructions,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib -- instructions_plugin_roundtrips`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lockfile.rs
git commit -m "feat(lockfile): add Instructions variant to PluginInstallStatus (#168)"
```

---

### Task 5: Ancillary — opencode.json Plugin Write/Remove

**Files:**

- Modify: `src/ancillary.rs`

- [ ] **Step 1: Write failing tests**

Add at the bottom of `src/ancillary.rs` tests module:

```rust
#[test]
fn write_opencode_plugin_uri_creates_file_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
    assert_eq!(outcome, crate::plugin::PluginOutcome::Success);

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap())
            .unwrap();
    let plugins = content["plugin"].as_array().unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0], "sp@git+https://example.com");
}

#[test]
fn write_opencode_plugin_uri_appends_to_existing() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("opencode.json"),
        r#"{"instructions": [".agents/rules/**/RULE.md"], "plugin": ["existing@foo"]}"#,
    )
    .unwrap();

    let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
    assert_eq!(outcome, crate::plugin::PluginOutcome::Success);

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap())
            .unwrap();
    let plugins = content["plugin"].as_array().unwrap();
    assert_eq!(plugins.len(), 2);
    assert_eq!(plugins[0], "existing@foo");
    assert_eq!(plugins[1], "sp@git+https://example.com");
    // instructions preserved
    assert!(content["instructions"].is_array());
}

#[test]
fn write_opencode_plugin_uri_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("opencode.json"),
        r#"{"plugin": ["sp@git+https://example.com"]}"#,
    )
    .unwrap();

    let outcome = write_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com");
    assert_eq!(outcome, crate::plugin::PluginOutcome::Success);

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap())
            .unwrap();
    let plugins = content["plugin"].as_array().unwrap();
    assert_eq!(plugins.len(), 1); // not duplicated
}

#[test]
fn remove_opencode_plugin_uri_removes_entry() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("opencode.json"),
        r#"{"plugin": ["sp@git+https://example.com", "other@foo"]}"#,
    )
    .unwrap();

    remove_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com").unwrap();

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap())
            .unwrap();
    let plugins = content["plugin"].as_array().unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0], "other@foo");
}

#[test]
fn remove_opencode_plugin_uri_noop_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("opencode.json"),
        r#"{"plugin": ["other@foo"]}"#,
    )
    .unwrap();

    remove_opencode_plugin_uri(tmp.path(), "sp@git+https://example.com").unwrap();

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(tmp.path().join("opencode.json")).unwrap())
            .unwrap();
    let plugins = content["plugin"].as_array().unwrap();
    assert_eq!(plugins.len(), 1); // unchanged
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib -- write_opencode_plugin_uri`
Expected: FAIL — function does not exist.

- [ ] **Step 3: Implement `write_opencode_plugin_uri`**

Add to `src/ancillary.rs`:

```rust
use crate::plugin::PluginOutcome;

/// Write a plugin URI to the `plugin[]` array in `<target>/opencode.json`.
///
/// - File absent → creates `{"plugin": ["<uri>"]}`.
/// - File present, URI absent → appends to `plugin[]`.
/// - File present, URI already in array → no-op (idempotent).
/// - File present but malformed → returns `PluginOutcome::Failed`.
///
/// Other keys in the JSON are preserved.
pub fn write_opencode_plugin_uri(target: &Path, plugin_uri: &str) -> PluginOutcome {
    let path = target.join(OPENCODE_JSON);

    let mut config: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                return PluginOutcome::Failed {
                    exit_code: None,
                    stderr: format!("parse {}: {e}", path.display()),
                };
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_json::json!({})
        }
        Err(e) => {
            return PluginOutcome::Failed {
                exit_code: None,
                stderr: format!("read {}: {e}", path.display()),
            };
        }
    };

    let obj = match config.as_object_mut() {
        Some(o) => o,
        None => {
            return PluginOutcome::Failed {
                exit_code: None,
                stderr: format!("{} is not a JSON object", path.display()),
            };
        }
    };

    let plugins = obj
        .entry("plugin")
        .or_insert_with(|| serde_json::json!([]));

    let arr = match plugins.as_array_mut() {
        Some(a) => a,
        None => {
            return PluginOutcome::Failed {
                exit_code: None,
                stderr: format!(
                    "{}: `plugin` is not an array",
                    path.display()
                ),
            };
        }
    };

    // Idempotent: check if already present
    if arr.iter().any(|v| v.as_str() == Some(plugin_uri)) {
        return PluginOutcome::Success;
    }

    arr.push(serde_json::Value::String(plugin_uri.to_string()));

    match write_pretty_json(&path, &config) {
        Ok(()) => PluginOutcome::Success,
        Err(e) => PluginOutcome::Failed {
            exit_code: None,
            stderr: format!("write {}: {e}", path.display()),
        },
    }
}

/// Remove a plugin URI from the `plugin[]` array in `<target>/opencode.json`.
///
/// No-op if the file doesn't exist or the URI isn't in the array.
pub fn remove_opencode_plugin_uri(target: &Path, plugin_uri: &str) -> anyhow::Result<()> {
    let path = target.join(OPENCODE_JSON);
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };

    let mut config: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;

    if let Some(arr) = config.get_mut("plugin").and_then(|v| v.as_array_mut()) {
        let before = arr.len();
        arr.retain(|v| v.as_str() != Some(plugin_uri));
        if arr.len() < before {
            write_pretty_json(&path, &config)?;
        }
    }

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib -- opencode_plugin_uri`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ancillary.rs
git commit -m "feat(ancillary): add opencode.json plugin URI write/remove (#168)

write_opencode_plugin_uri appends a plugin URI to the plugin[] array
idempotently. remove_opencode_plugin_uri removes it. Both preserve
other keys in the JSON. Follows the same pattern as the existing
ensure_opencode_rules_registered function."
```

---

### Task 6: CLI Output — Update main.rs

**Files:**

- Modify: `src/main.rs`

- [ ] **Step 1: Update `print_plugin_results` to handle ManualInstructions**

Add a fourth category in the function for instructions-only results:

```rust
fn print_plugin_results(report: &InstallReport) {
    if style::is_quiet() || report.plugin_results.is_empty() {
        return;
    }

    use upskill::plugin::PluginOutcome;

    let successes: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| r.outcome.is_success())
        .collect();
    let skipped: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| r.outcome.is_cli_not_found())
        .collect();
    let instructions: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| r.outcome.is_manual_instructions())
        .collect();
    let failures: Vec<&PluginResult> = report
        .plugin_results
        .iter()
        .filter(|r| {
            !r.outcome.is_success()
                && !r.outcome.is_cli_not_found()
                && !r.outcome.is_manual_instructions()
        })
        .collect();

    if !successes.is_empty() {
        println!(
            "{} {} plugin(s) installed",
            style::success("Plugins:"),
            successes.len()
        );
        for r in &successes {
            println!(
                "  {} {} ({})",
                style::dim("plugin"),
                style::name(&r.name),
                r.client
            );
        }
    }

    for r in &instructions {
        let url = r
            .instructions_url
            .as_deref()
            .unwrap_or(&r.identifier);
        let summary_lines = r
            .summary
            .as_deref()
            .map(|s| format!("\n        {}", s.trim().replace('\n', "\n        ")))
            .unwrap_or_default();
        eprintln!(
            "{} plugin {} ({}) \u{2014} manual step required{}",
            style::info("info:"),
            style::name(&r.name),
            r.client,
            summary_lines
        );
        eprintln!("        Instructions: {url}");
    }

    for r in &skipped {
        let url_hint = r
            .install_url
            .as_deref()
            .map(|u| format!(" \u{2014} install manually: {u}"))
            .unwrap_or_default();
        eprintln!(
            "{} plugin {} skipped \u{2014} {} CLI not found{}",
            style::warn("warning:"),
            style::name(&r.name),
            r.client,
            url_hint
        );
    }

    for r in &failures {
        if let PluginOutcome::Failed { exit_code, stderr } = &r.outcome {
            eprintln!(
                "{} plugin {} ({}) failed (exit {:?}): {}",
                style::warn("warning:"),
                style::name(&r.name),
                r.client,
                exit_code,
                stderr.trim()
            );
        }
    }
}
```

- [ ] **Step 2: Add `style::info` helper if it doesn't exist**

Check if `style::info` exists. If not, add it alongside the existing `style::success` and `style::warn` functions:

```rust
pub fn info(text: &str) -> String {
    format!("\x1b[36m{text}\x1b[0m")  // cyan
}
```

- [ ] **Step 3: Verify full compilation passes**

Run: `cargo check`
Expected: PASS (or fix remaining compilation issues).

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): display instructions-only plugin notices during install (#168)

Instructions-only plugins show as 'info:' notices with summary text
and instructions URL. Config-write successes appear in the normal
'Plugins: N installed' section."
```

---

### Task 7: Fix Existing Tests — parse/bundle.rs

**Files:**

- Modify: `src/parse/bundle.rs`

- [ ] **Step 1: Update existing plugin parsing tests to use enum matching**

The existing tests in `src/parse/bundle.rs` (starting around line 389) access struct fields like `claude.source`, `claude.plugin`. These need to be updated to match the enum variants:

```rust
#[test]
fn load_parses_plugins_with_all_clients() {
    // ... existing YAML fixture unchanged ...

    let sp = &bundle.plugins["superpowers"];
    let claude = sp.claude.as_ref().expect("claude block");
    match claude {
        ClaudePluginDescriptor::Install { source, plugin, install_url } => {
            assert_eq!(source, "anthropics/claude-plugins");
            assert_eq!(plugin, "superpowers");
            assert_eq!(install_url.as_deref(), Some("https://github.com/obra/superpowers#install"));
        }
        _ => panic!("expected Install variant"),
    }

    let copilot = sp.copilot.as_ref().expect("copilot block");
    match copilot {
        CopilotPluginDescriptor::Install { source, plugin, install_url } => {
            assert_eq!(source, "obra/superpowers-marketplace");
            assert_eq!(plugin, "superpowers");
            assert_eq!(install_url.as_deref(), Some("https://github.com/obra/superpowers#install"));
        }
        _ => panic!("expected Install variant"),
    }

    let vscode = sp.vscode.as_ref().expect("vscode block");
    match vscode {
        VscodePluginDescriptor::Install { extension, install_url } => {
            assert_eq!(extension, "anthropic.superpowers");
            assert_eq!(
                install_url.as_deref(),
                Some("https://marketplace.visualstudio.com/items?itemName=anthropic.superpowers")
            );
        }
        _ => panic!("expected Install variant"),
    }

    let opencode = sp.opencode.as_ref().expect("opencode block");
    match opencode {
        OpencodePluginDescriptor::Install { module, install_url } => {
            assert_eq!(module, "superpowers-opencode");
            assert_eq!(install_url.as_deref(), Some("https://opencode.ai/plugins/superpowers"));
        }
        _ => panic!("expected Install variant"),
    }
}
```

- [ ] **Step 2: Add tests for instructions-only and config-write bundle parsing**

```rust
#[test]
fn load_parses_instructions_only_plugin() {
    let content = "schema: 1
name: with-instructions
description: Plugin with instructions only
items:
  rules: []
plugins:
  superpowers:
    opencode:
      instructions_url: https://github.com/obra/superpowers/INSTALL.md
      summary: Add plugin URI to opencode.json
";
    let tmp = tempfile::tempdir().unwrap();
    let path = write_file(tmp.path(), "with-instructions.bundle.yaml", content);

    let bundle = load(&path).expect("load");
    let sp = &bundle.plugins["superpowers"];
    let opencode = sp.opencode.as_ref().expect("opencode block");
    match opencode {
        OpencodePluginDescriptor::Instructions { instructions_url, summary } => {
            assert_eq!(instructions_url, "https://github.com/obra/superpowers/INSTALL.md");
            assert_eq!(summary.as_deref(), Some("Add plugin URI to opencode.json"));
        }
        _ => panic!("expected Instructions variant"),
    }
}

#[test]
fn load_parses_config_write_plugin() {
    let content = "schema: 1
name: with-config-write
description: Plugin with config-write
items:
  rules: []
plugins:
  superpowers:
    opencode:
      plugin_uri: \"superpowers@git+https://github.com/obra/superpowers.git\"
      install_url: https://github.com/obra/superpowers/INSTALL.md
";
    let tmp = tempfile::tempdir().unwrap();
    let path = write_file(tmp.path(), "with-config-write.bundle.yaml", content);

    let bundle = load(&path).expect("load");
    let sp = &bundle.plugins["superpowers"];
    let opencode = sp.opencode.as_ref().expect("opencode block");
    match opencode {
        OpencodePluginDescriptor::ConfigWrite { plugin_uri, install_url } => {
            assert_eq!(plugin_uri, "superpowers@git+https://github.com/obra/superpowers.git");
            assert_eq!(install_url.as_deref(), Some("https://github.com/obra/superpowers/INSTALL.md"));
        }
        _ => panic!("expected ConfigWrite variant"),
    }
}
```

- [ ] **Step 3: Run all bundle tests**

Run: `cargo test --lib parse::bundle`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/parse/bundle.rs
git commit -m "test(parse): update bundle plugin tests for enum descriptors (#168)

Existing tests match on Install variant. New tests verify
Instructions and ConfigWrite variants parse correctly from YAML."
```

---

### Task 8: Remove — Handle Config-Write Plugins

**Files:**

- Modify: `src/pipeline.rs`

- [ ] **Step 1: Update remove to handle config-write plugin cleanup**

In the `remove` function, after removing lockfile entries, add plugin cleanup:

```rust
// Remove plugins associated with removed bundles (by name).
// For config-write (opencode) plugins, also remove from opencode.json.
let removed_bundle_names: std::collections::BTreeSet<&str> = match &filter {
    RemoveFilter::ByNames(names) => {
        lock.bundles
            .iter()
            .filter(|b| names.contains(&b.name))
            .map(|b| b.name.as_str())
            .collect()
    }
    RemoveFilter::BySource(source) => {
        lock.bundles
            .iter()
            .filter(|b| &b.source == source)
            .map(|b| b.name.as_str())
            .collect()
    }
};

// Remove plugins from bundles being removed.
let plugins_to_remove: Vec<crate::lockfile::LockedPlugin> = lock
    .plugins
    .iter()
    .filter(|p| removed_bundle_names.contains(p.bundle.as_str()))
    .cloned()
    .collect();

for plugin in &plugins_to_remove {
    // For config-write opencode plugins, remove URI from opencode.json.
    if plugin.client == "opencode" && plugin.status == PluginInstallStatus::Installed {
        let _ = crate::ancillary::remove_opencode_plugin_uri(target, &plugin.identifier);
    }
    lock.remove_plugins_by_name(&plugin.name);
}
```

- [ ] **Step 2: Run compilation check**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): remove config-write plugins from opencode.json on uninstall (#168)

When removing a bundle that declared config-write plugins, the
corresponding plugin_uri entries are removed from opencode.json."
```

---

### Task 9: Full Test Suite Pass

**Files:**

- All modified files

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`
Expected: Some tests may fail due to the changed struct → enum in existing integration tests.

- [ ] **Step 2: Fix any remaining failures**

Common issues:

- Tests in `tests/pipeline_lockfile.rs` that construct `PluginResult` directly
- Tests that reference `descriptor.source` etc. as struct fields
- Integration tests that parse bundle YAML

Fix each by updating to use the new enum shapes.

- [ ] **Step 3: Run full suite again**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 4: Run lint**

Run: `just lint`
Expected: PASS (no warnings)

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: update remaining tests for enum-based plugin descriptors (#168)"
```

---

### Task 10: Format-Spec Documentation Update

**Files:**

- Modify: `docs/format-spec.md`

- [ ] **Step 1: Add instructions-only and config-write forms to §3.7**

After the existing per-client descriptor fields table, add:

````markdown
**Additional descriptor forms:**

A client block MAY use one of these alternative forms instead of the standard
auto-install form. The mode is determined by which required field is present:

| Mode              | Discriminating fields                      | Available for | Behavior                               |
| ----------------- | ------------------------------------------ | ------------- | -------------------------------------- |
| Auto-install      | `source`+`plugin` / `extension` / `module` | all clients   | Shell out to client CLI                |
| Instructions-only | `instructions_url`                         | all clients   | Print info notice; no automated action |
| Config-write      | `plugin_uri`                               | opencode only | Write URI to project `opencode.json`   |

**Instructions-only form:**

```yaml
plugins:
  example:
    opencode:
      instructions_url: https://example.com/install-docs
      summary: |
        Add "example@git+https://example.com/repo.git" to the
        plugin[] array in opencode.json, then restart opencode.
```

| Field              | Type   | Required | Description                                    |
| ------------------ | ------ | -------- | ---------------------------------------------- |
| `instructions_url` | string | YES      | URL to manual install documentation.           |
| `summary`          | string | no       | Short text (1-5 lines) printed during install. |

**Config-write form (opencode only):**

```yaml
plugins:
  example:
    opencode:
      plugin_uri: "example@git+https://example.com/repo.git"
      install_url: https://example.com/install-docs
```

| Field         | Type   | Required | Description                                              |
| ------------- | ------ | -------- | -------------------------------------------------------- |
| `plugin_uri`  | string | YES      | Plugin URI appended to `opencode.json` `plugin[]` array. |
| `install_url` | string | no       | URL shown in output for reference.                       |
````

- [ ] **Step 2: Update lockfile status table**

Add `"instructions"` row:

```markdown
| `status`         | Meaning                                                                      |
| ---------------- | ---------------------------------------------------------------------------- |
| `"installed"`    | Plugin successfully installed via the client CLI or config-write.            |
| `"skipped"`      | Client CLI was not on PATH at install time (warn-skip).                      |
| `"instructions"` | Instructions-only — no automated install; consumer must follow manual steps. |
```

- [ ] **Step 3: Update §3.7 implementation rules**

Replace the "MUST shell out" rule with:

```markdown
- MUST shell out to the native client CLI for auto-install plugin descriptors — MUST NOT
  manipulate client config files directly except for the `opencode` config-write mode, which
  writes to project-scope `opencode.json` per the documented plugin registration path.
```

- [ ] **Step 4: Run `just fmt`**

Run: `just fmt`
Expected: Markdown formatting applied.

- [ ] **Step 5: Commit**

```bash
git add docs/format-spec.md
git commit -m "docs(format-spec): document instructions-only and config-write plugin forms (#168)

Updates §3.7 with the two new descriptor modes, their YAML schemas,
lockfile status values, and disambiguation rules. Narrows the §3.7
'no config manipulation' rule with an exception for opencode
config-write mode."
```

---

### Task 11: Final Verification

- [ ] **Step 1: Run `just verify`**

Run: `just verify`
Expected: ALL PASS (tests + lint + commit check)

- [ ] **Step 2: Review git log**

Run: `git log --oneline -10`
Expected: Clean commit history with conventional commit messages.
