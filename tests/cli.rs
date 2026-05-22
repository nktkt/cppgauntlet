use std::fs;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn help_works() {
    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Put C++ code through"));
}

#[test]
fn missing_file_reports_error() {
    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();

    cmd.args(["check", "missing.cpp"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("target path does not exist"));
}

#[test]
fn check_hello_writes_json_report() {
    if StdCommand::new("clang++")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }

    let temp = tempdir().unwrap();
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/hello.cpp");
    let source = temp.path().join("hello.cpp");
    fs::copy(fixture, &source).unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp", "--sanitizers", "none"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: PASSED"));

    let report_path = temp.path().join(".cppgauntlet/cppgauntlet-report.json");
    let report = fs::read_to_string(report_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&report).unwrap();

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["status"], "passed");
    assert_eq!(value["target"]["standard"], "c++20");
}
