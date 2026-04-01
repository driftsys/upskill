use assert_cmd::Command;
use predicates::str::contains;
use std::io::{Read, Write};
use tempfile::tempdir;

fn mock_skills_server(json_body: &'static str) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                json_body.len(),
                json_body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    addr
}

const MOCK_RESPONSE: &str = r#"{
  "query": "rust",
  "searchType": "fuzzy",
  "skills": [
    {
      "id": "github/awesome-copilot/rust-mcp-server-generator",
      "skillId": "rust-mcp-server-generator",
      "name": "rust-mcp-server-generator",
      "installs": 7608,
      "source": "github/awesome-copilot"
    },
    {
      "id": "github/anthropics/rust-analyzer",
      "skillId": "rust-analyzer",
      "name": "rust-analyzer",
      "installs": 3200,
      "source": "github/anthropics/skills"
    }
  ],
  "count": 2,
  "duration_ms": 12
}"#;

const EMPTY_RESPONSE: &str = r#"{
  "query": "zzznomatch",
  "searchType": "fuzzy",
  "skills": [],
  "count": 0,
  "duration_ms": 5
}"#;

#[test]
fn search_requires_query() {
    let cwd = tempdir().expect("must create temp dir");
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .args(["search"])
        .assert()
        .code(2);
}

#[test]
fn search_help_exits_zero() {
    let cwd = tempdir().expect("must create temp dir");
    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .args(["search", "--help"])
        .assert()
        .code(0);
}

#[test]
fn search_returns_results() {
    let addr = mock_skills_server(MOCK_RESPONSE);
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .env("UPSKILL_REGISTRY_URL", format!("http://{}", addr))
        .args(["search", "rust"])
        .assert()
        .code(0)
        .stdout(contains("rust-mcp-server-generator"))
        .stdout(contains("awesome-copilot"))
        .stdout(contains("rust-analyzer"));
}

#[test]
fn search_shows_install_command() {
    let addr = mock_skills_server(MOCK_RESPONSE);
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .env("UPSKILL_REGISTRY_URL", format!("http://{}", addr))
        .args(["search", "rust"])
        .assert()
        .code(0)
        .stdout(contains(
            "upskill add awesome-copilot --skill rust-mcp-server-generator",
        ));
}

#[test]
fn search_empty_results() {
    let addr = mock_skills_server(EMPTY_RESPONSE);
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .env("UPSKILL_REGISTRY_URL", format!("http://{}", addr))
        .args(["search", "zzznomatch"])
        .assert()
        .code(0)
        .stdout(contains("no skills found"));
}

#[test]
fn search_unreachable_registry_exits_one() {
    let cwd = tempdir().expect("must create temp dir");

    Command::cargo_bin("upskill")
        .unwrap()
        .current_dir(cwd.path())
        .env("UPSKILL_REGISTRY_URL", "http://127.0.0.1:1")
        .args(["search", "rust"])
        .assert()
        .code(1)
        .stderr(contains("error:"));
}
