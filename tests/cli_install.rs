#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn install_sh() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("install.sh")
}

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    Command::new("sh")
        .arg(install_sh())
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage"))
        .stdout(contains("UPSKILL_VERSION"));
}

#[test]
fn help_works_without_home_env() {
    Command::new("sh")
        .arg(install_sh())
        .arg("--help")
        .env_remove("HOME")
        .env_remove("UPSKILL_INSTALL_DIR")
        .assert()
        .success()
        .stdout(contains("Usage"));
}

#[test]
fn unsupported_platform_without_home_still_hints() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_uname = tmp.path().join("uname");
    fs::write(
        &fake_uname,
        "#!/bin/sh\ncase \"$1\" in\n  -s) echo Linux ;;\n  -m) echo riscv64 ;;\n  *) echo unknown ;;\nesac\n",
    )
    .unwrap();
    let mut perm = fs::metadata(&fake_uname).unwrap().permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&fake_uname, perm).unwrap();

    let orig_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", tmp.path().display(), orig_path);

    Command::new("sh")
        .arg(install_sh())
        .env("PATH", new_path)
        .env_remove("HOME")
        .env_remove("UPSKILL_INSTALL_DIR")
        .assert()
        .failure()
        .stderr(contains("cargo install upskill"));
}

#[test]
fn unsupported_platform_fails_with_cargo_hint() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_uname = tmp.path().join("uname");
    fs::write(
        &fake_uname,
        "#!/bin/sh\ncase \"$1\" in\n  -s) echo Linux ;;\n  -m) echo riscv64 ;;\n  *) echo unknown ;;\nesac\n",
    )
    .unwrap();
    let mut perm = fs::metadata(&fake_uname).unwrap().permissions();
    perm.set_mode(0o755);
    fs::set_permissions(&fake_uname, perm).unwrap();

    let orig_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", tmp.path().display(), orig_path);

    Command::new("sh")
        .arg(install_sh())
        .env("PATH", new_path)
        .assert()
        .failure()
        .stderr(contains("cargo install upskill"));
}
