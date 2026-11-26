//! The fixture of the hosted-marketplace smoke test (technote T5) must stay
//! valid for `upskill lint` and `upskill pack --catalog`. The test starts no AI
//! client and makes no network call.

#![cfg(unix)]

mod common;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const REGISTRY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/plugin-smoke/registry"
);

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            // `fs::copy` keeps the permission bits, so the hook scripts stay
            // executable.
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Copies the fixture registry below `root` and packs every bundle with a
/// catalog. Returns the registry root, which also holds the output.
fn pack_fixture(root: &Path) -> std::path::PathBuf {
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let registry = root.join("registry");
    copy_dir(Path::new(REGISTRY), &registry);
    common::upskill_cmd(&home)
        .current_dir(&registry)
        .args([
            "pack",
            "--all",
            "--source",
            "driftsys/smoke-test",
            "--out",
            registry.join("plugins").to_str().unwrap(),
            "--catalog",
            registry.to_str().unwrap(),
            "--catalog-name",
            "smoke-market",
            "--catalog-owner",
            "driftsys",
        ])
        .assert()
        .success();

    registry
}

#[test]
fn lint_strict_accepts_the_fixture_items() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let out = common::upskill_cmd(&home)
        .current_dir(tmp.path())
        .args(["lint", "--strict", &format!("{REGISTRY}/items")])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "lint failed: {stdout}");
    assert!(
        stdout.contains("8 file(s) checked, 0 findings"),
        "unexpected lint output: {stdout}"
    );
}

#[test]
fn pack_catalog_writes_both_plugins_and_both_marketplace_files() {
    let tmp = tempfile::tempdir().unwrap();
    let registry = pack_fixture(tmp.path());

    for (client, source, init_flag) in [
        ("claude", ".claude-plugin/marketplace.json", "--claude"),
        ("copilot", ".github/plugin/marketplace.json", "--copilot"),
    ] {
        let plugin = registry.join(format!("plugins/smoke-{client}"));
        assert!(plugin.is_dir(), "missing plugin directory for {client}");
        let marketplace = fs::read_to_string(registry.join(source)).unwrap();
        assert!(marketplace.contains(&format!("./plugins/smoke-{client}")));
        assert!(marketplace.contains("\"smoke-market\""));

        let init = fs::read_to_string(plugin.join("skills/init/SKILL.md")).unwrap();
        let command =
            format!("upskill add driftsys/smoke-test smoke --kind rule --project {init_flag}");
        assert!(init.contains(&command), "init text of {client}: {init}");

        for skill in ["smoke-skill", "smoke-command", "init"] {
            assert!(plugin.join(format!("skills/{skill}/SKILL.md")).is_file());
        }
        for script in ["smoke-hook/marker.sh", "smoke-counter/counter.sh"] {
            assert!(plugin.join("hooks").join(script).is_file());
        }
    }

    let claude = registry.join("plugins/smoke-claude");
    let copilot = registry.join("plugins/smoke-copilot");
    assert!(claude.join("agents/smoke-agent.md").is_file());
    assert!(copilot.join("agents/smoke-agent.agent.md").is_file());
    assert!(claude.join(".mcp.json").is_file() && copilot.join(".mcp.json").is_file());
    assert!(claude.join(".lsp.json").is_file() && copilot.join("lsp.json").is_file());
    assert!(claude.join("hooks/hooks.json").is_file() && copilot.join("hooks.json").is_file());
    assert!(claude.join("hooks/context/session-start.json").is_file());

    let hooks = fs::read_to_string(copilot.join("hooks.json")).unwrap();
    assert!(hooks.contains("\"matcher\": \"bash\"") && hooks.contains("\"matcher\": \"edit\""));
}

/// True when `program` runs. A test that needs it prints a skip message
/// otherwise, so a skip is visible in the test output.
fn available(program: &str, test: &str) -> bool {
    // Only whether the program can be started counts, not its exit code:
    // some `sh` implementations reject `--version`.
    let ok = Command::new(program).arg("--version").output().is_ok();
    if !ok {
        eprintln!("SKIPPED {test}: `{program}` is not on PATH");
    }
    ok
}

/// Starts the server from a packed JSON file, writes `input` to its stdin and
/// returns its stdout. `TMPDIR` points at `tmpdir`.
fn run_server(file: &Path, section: &str, input: &[u8], tmpdir: &Path) -> String {
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    let server = json[section].as_object().unwrap().values().next().unwrap();
    let mut command = Command::new(server["command"].as_str().unwrap());
    for arg in server["args"].as_array().unwrap() {
        command.arg(arg.as_str().unwrap());
    }
    let mut child = command
        .env("TMPDIR", tmpdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "server failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn frame(value: &serde_json::Value) -> Vec<u8> {
    let body = value.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

/// Splits LSP output into JSON messages. Every message needs exactly one
/// `Content-Length: N` header, an empty line and N bytes; anything else is an
/// error, including bytes after the last message.
fn parse_frames(mut bytes: &[u8]) -> Result<Vec<serde_json::Value>, String> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        let end = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or("no header terminator")?;
        let header = std::str::from_utf8(&bytes[..end]).map_err(|e| e.to_string())?;
        let length: usize = header
            .strip_prefix("Content-Length: ")
            .ok_or_else(|| format!("bad header `{header}`"))?
            .parse()
            .map_err(|e| format!("bad length: {e}"))?;
        let body = bytes
            .get(end + 4..end + 4 + length)
            .ok_or("body shorter than Content-Length")?;
        messages.push(serde_json::from_slice(body).map_err(|e| e.to_string())?);
        bytes = &bytes[end + 4 + length..];
    }
    Ok(messages)
}

#[test]
fn the_frame_parser_accepts_frames_and_rejects_broken_output() {
    let good = frame(&serde_json::json!({"id": 1}));
    assert_eq!(parse_frames(&good).unwrap().len(), 1);
    assert!(parse_frames(br#"{"id": 1}"#).is_err(), "unframed output");
    let wrong = b"Content-Length: 50\r\n\r\n{\"id\": 1}";
    assert!(parse_frames(wrong).is_err(), "length longer than the body");
    let short = b"Content-Length: 2\r\n\r\n{\"id\": 1}";
    assert!(parse_frames(short).is_err(), "length shorter than the body");
    let mut trailing = good.clone();
    trailing.extend(b"x");
    assert!(parse_frames(&trailing).is_err(), "trailing bytes");
}

#[test]
fn packed_mcp_server_answers_initialize_and_the_marker_tool() {
    if !available(
        "python3",
        "packed_mcp_server_answers_initialize_and_the_marker_tool",
    ) {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let registry = pack_fixture(tmp.path());
    let input = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"smoke_marker"}}"#,
    ]
    .join("\n")
        + "\n";
    for file in [
        "plugins/smoke-claude/.mcp.json",
        "plugins/smoke-copilot/.mcp.json",
    ] {
        let out = run_server(
            &registry.join(file),
            "mcpServers",
            input.as_bytes(),
            tmp.path(),
        );
        let lines: Vec<serde_json::Value> = out
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 3, "{file}: {out}");
        assert_eq!(lines[0]["result"]["serverInfo"]["name"], "smoke-mcp");
        assert_eq!(lines[1]["result"]["tools"][0]["name"], "smoke_marker");
        assert_eq!(lines[2]["result"]["content"][0]["text"], "slate-newt-2468");
    }
}

#[test]
fn packed_lsp_server_answers_document_symbol_and_writes_its_marker() {
    let name = "packed_lsp_server_answers_document_symbol_and_writes_its_marker";
    if !available("python3", name) {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let registry = pack_fixture(tmp.path());
    let mut input = Vec::new();
    for message in [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"textDocument/documentSymbol","params":{}}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"workspace/unknown"}),
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"shutdown"}),
        serde_json::json!({"jsonrpc":"2.0","method":"exit"}),
    ] {
        input.extend(frame(&message));
    }
    for (file, section) in [
        ("plugins/smoke-claude/.lsp.json", "smoke-lsp"),
        ("plugins/smoke-copilot/lsp.json", "lspServers"),
    ] {
        let marker = tmp.path().join("upskill-smoke-lsp-marker");
        let _ = fs::remove_file(&marker);
        // The Claude Code file has the server at the top level.
        let path = registry.join(file);
        let out = if section == "lspServers" {
            run_server(&path, section, &input, tmp.path())
        } else {
            let json: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            let wrapped = tmp.path().join("claude-lsp.json");
            fs::write(
                &wrapped,
                serde_json::json!({"lspServers": json}).to_string(),
            )
            .unwrap();
            run_server(&wrapped, "lspServers", &input, tmp.path())
        };
        let responses = parse_frames(out.as_bytes()).unwrap_or_else(|e| panic!("{file}: {e}"));
        let by_id = |id: u64| {
            responses
                .iter()
                .find(|r| r["id"] == id)
                .unwrap_or_else(|| panic!("{file}: no response with id {id}"))
        };
        assert_eq!(responses.len(), 4, "{file}: {responses:?}");
        assert_eq!(
            by_id(1)["result"]["capabilities"]["documentSymbolProvider"],
            true
        );
        assert_eq!(by_id(2)["result"], serde_json::json!([]));
        assert!(has_null_result(by_id(3)), "{}", by_id(3));
        assert!(has_null_result(by_id(4)), "{}", by_id(4));
        let lines = fs::read_to_string(&marker).unwrap();
        assert!(
            lines.contains("started ") && lines.contains("documentSymbol "),
            "{lines}"
        );
        let pids: Vec<&str> = lines
            .lines()
            .map(|line| line.rsplit("pid=").next().unwrap())
            .collect();
        assert!(pids.len() == 2 && pids[0] == pids[1], "{lines}");
    }
}

#[test]
fn packed_hook_scripts_write_their_marker_files() {
    if !available("sh", "packed_hook_scripts_write_their_marker_files") {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let registry = pack_fixture(tmp.path());
    for (client, script, file) in [
        (
            "claude",
            "smoke-hook/marker.sh",
            "upskill-smoke-hook-marker",
        ),
        (
            "copilot",
            "smoke-counter/counter.sh",
            "upskill-smoke-session-counter",
        ),
        (
            "claude",
            "smoke-match-bash/match-bash.sh",
            "upskill-smoke-match-bash",
        ),
        (
            "copilot",
            "smoke-match-edit/match-edit.sh",
            "upskill-smoke-match-edit",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = registry.join(format!("plugins/smoke-{client}/hooks/{script}"));
        // The script runs through its own mode bits, as a client runs it.
        let out = Command::new(&path)
            .env("TMPDIR", dir.path())
            .env("UPSKILL_CLIENT", client)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(out.status.success(), "{script}");
        assert!(out.stdout.is_empty(), "{script} must print nothing");
        let written = fs::read_to_string(dir.path().join(file)).unwrap();
        assert!(
            written.starts_with(&format!("{client} 20")),
            "{script}: {written}"
        );
    }
}

/// A response with an explicit `"result": null` and no `error`. Indexing a
/// missing key gives `Null` too, so the key itself must be present.
fn has_null_result(response: &serde_json::Value) -> bool {
    response.get("result") == Some(&serde_json::Value::Null) && response.get("error").is_none()
}

#[test]
fn a_null_result_needs_the_result_key() {
    assert!(has_null_result(
        &serde_json::json!({"jsonrpc": "2.0", "id": 3, "result": null})
    ));
    assert!(!has_null_result(
        &serde_json::json!({"jsonrpc": "2.0", "id": 3})
    ));
    assert!(!has_null_result(
        &serde_json::json!({"jsonrpc": "2.0", "id": 3, "result": null, "error": {}})
    ));
}
