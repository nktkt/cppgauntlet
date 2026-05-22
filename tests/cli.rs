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
fn missing_compiler_reports_error() {
    let temp = tempdir().unwrap();
    let source = copy_fixture(temp.path(), "hello.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            source.file_name().unwrap().to_str().unwrap(),
            "--compiler",
            "cppgauntlet-missing-compiler-for-test",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to run command"));
}

#[test]
fn init_writes_default_config() {
    let temp = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Created cppgauntlet.yaml"));

    let config = fs::read_to_string(temp.path().join("cppgauntlet.yaml")).unwrap();
    assert!(config.contains("standard: c++20"));
    assert!(config.contains("compiler: clang++"));
    assert!(config.contains("artifact_dir:"));
    assert!(config.contains(".cppgauntlet"));
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("cppgauntlet.yaml"), "standard: c++17\n").unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "configuration file already exists",
        ));
}

#[test]
fn init_force_overwrites_existing_config() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("cppgauntlet.yaml"), "standard: c++17\n").unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["init", "--force"])
        .assert()
        .success();

    let config = fs::read_to_string(temp.path().join("cppgauntlet.yaml")).unwrap();
    assert!(config.contains("standard: c++20"));
}

#[test]
fn check_hello_writes_json_report() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp", "--sanitizers", "none"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: PASSED"));

    let report_path = temp.path().join(".cppgauntlet/cppgauntlet-report.json");
    let report = fs::read_to_string(report_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&report).unwrap();

    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["status"], "passed");
    assert_eq!(value["target"]["standard"], "c++20");
}

#[test]
fn check_uses_default_yaml_config() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        r#"standard: c++17
compiler: clang++
artifact_dir: configured-artifacts
timeout_seconds: 30
sanitizers:
  enabled: []
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Standard: c++17"));

    let value = read_report_at(
        temp.path()
            .join("configured-artifacts/cppgauntlet-report.json"),
    );
    assert_eq!(value["target"]["standard"], "c++17");
    assert_eq!(stage(&value, "sanitize_compile")["status"], "skipped");
}

#[test]
fn check_cli_flags_override_config_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    fs::write(
        temp.path().join("custom.yaml"),
        r#"standard: c++17
artifact_dir: config-artifacts
sanitizers:
  enabled: []
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--config",
            "custom.yaml",
            "--standard",
            "c++23",
            "--artifact-dir",
            "cli-artifacts",
            "--sanitizers",
            "none",
        ])
        .assert()
        .success();

    let value = read_report_at(temp.path().join("cli-artifacts/cppgauntlet-report.json"));
    assert_eq!(value["target"]["standard"], "c++23");
    assert!(!temp
        .path()
        .join("config-artifacts/cppgauntlet-report.json")
        .exists());
}

#[test]
fn invalid_config_standard_reports_error() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    fs::write(temp.path().join("cppgauntlet.yaml"), "standard: c++14\n").unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported C++ standard"));
}

#[test]
fn compile_errors_are_reported_as_diagnostics() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "compile_error.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "compile_error.cpp", "--sanitizers", "none"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "compile")["status"], "failed");
    assert_eq!(stage(&value, "run")["status"], "skipped");
    assert!(value["summary"]["errors"].as_u64().unwrap() >= 1);
    assert_eq!(
        stage(&value, "compile")["diagnostics"][0]["severity"],
        "error"
    );
}

#[test]
fn warnings_are_reported_without_failing_the_check() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "warning.cpp", "--sanitizers", "none"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: PASSED"))
        .stdout(predicate::str::contains("Warnings: 1"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(value["summary"]["warnings"], 1);
    assert_eq!(
        stage(&value, "compile")["diagnostics"][0]["severity"],
        "warning"
    );
}

#[test]
fn runtime_failures_fail_the_report() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "runtime_fail.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "runtime_fail.cpp", "--sanitizers", "none"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "run")["status"], "failed");
    assert_eq!(stage(&value, "run")["exit_code"], 7);
}

#[test]
fn runtime_timeouts_fail_the_report() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "timeout.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "timeout.cpp",
            "--sanitizers",
            "none",
            "--timeout-seconds",
            "1",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "run")["status"], "failed");
    assert_eq!(stage(&value, "run")["timed_out"], true);
    assert_eq!(value["summary"]["timed_out_stages"], 1);
}

#[test]
fn undefined_behavior_sanitizer_failures_are_reported() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "undefined_behavior.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "undefined_behavior.cpp",
            "--sanitizers",
            "undefined",
            "--timeout-seconds",
            "10",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "sanitize_compile")["status"], "passed");
    assert_eq!(stage(&value, "sanitize_run")["status"], "failed");
    assert!(stage(&value, "sanitize_run")["stderr"]
        .as_str()
        .unwrap()
        .contains("runtime error"));
    assert_eq!(
        stage(&value, "sanitize_run")["diagnostics"][0]["severity"],
        "error"
    );
}

fn clang_available() -> bool {
    StdCommand::new("clang++").arg("--version").output().is_ok()
}

fn copy_fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    let source = dir.join(name);
    fs::copy(fixture, &source).unwrap();
    source
}

fn read_report(dir: &std::path::Path) -> serde_json::Value {
    read_report_at(dir.join(".cppgauntlet/cppgauntlet-report.json"))
}

fn read_report_at(path: impl AsRef<std::path::Path>) -> serde_json::Value {
    let report = fs::read_to_string(path).unwrap();
    serde_json::from_str(&report).unwrap()
}

fn stage<'a>(value: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    value["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["name"] == name)
        .unwrap()
}
