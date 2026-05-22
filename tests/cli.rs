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
#[cfg(unix)]
fn doctor_reports_custom_required_tool_available() {
    let temp = tempdir().unwrap();
    make_fake_tool(temp.path(), "fake-clang", "fake-clang 1.2.3");
    make_fake_tool(temp.path(), "fake-cmake", "fake-cmake 4.5.6");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.env("PATH", path_with_prefix(temp.path()))
        .args([
            "doctor",
            "--required-tool",
            "fake-clang",
            "--optional-tool",
            "fake-cmake",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: PASSED"))
        .stdout(predicate::str::contains("fake-clang: found"))
        .stdout(predicate::str::contains("fake-cmake: found"));
}

#[test]
fn doctor_fails_when_required_tool_is_missing() {
    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();

    cmd.args([
        "doctor",
        "--required-tool",
        "cppgauntlet-missing-required-tool",
    ])
    .assert()
    .failure()
    .stdout(predicate::str::contains("Status: FAILED"))
    .stdout(predicate::str::contains(
        "Required missing: cppgauntlet-missing-required-tool",
    ));
}

#[test]
#[cfg(unix)]
fn doctor_json_reports_tool_availability() {
    let temp = tempdir().unwrap();
    make_fake_tool(temp.path(), "fake-clang-json", "fake-clang-json 7.8.9");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    let assert = cmd
        .env("PATH", path_with_prefix(temp.path()))
        .args([
            "--format",
            "json",
            "doctor",
            "--required-tool",
            "fake-clang-json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["status"], "passed");
    assert_eq!(value["tools"][0]["name"], "fake-clang-json");
    assert_eq!(value["tools"][0]["available"], true);
    assert_eq!(value["tools"][0]["version"], "fake-clang-json 7.8.9");
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
fn check_directory_uses_compile_commands_json() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_project_source(
        temp.path(),
        "src/good.cpp",
        "int add(int a, int b) { return a + b; }\n",
    );
    write_project_source(
        temp.path(),
        "src/warning.cpp",
        "int warning_case() { int unused = 42; return 0; }\n",
    );
    write_compile_commands(
        temp.path(),
        &[
            serde_json::json!({
                "directory": temp.path(),
                "file": "src/good.cpp",
                "arguments": ["clang++", "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-c", "src/good.cpp"]
            }),
            serde_json::json!({
                "directory": temp.path(),
                "file": "src/warning.cpp",
                "command": "clang++ -std=c++20 -Wall -Wextra -Wpedantic -c src/warning.cpp"
            }),
        ],
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: PASSED"))
        .stdout(predicate::str::contains("Warnings: 1"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(value["target"]["standard"], "from compilation database");
    assert_eq!(value["stages"].as_array().unwrap().len(), 2);
    assert!(stage_name_exists(&value, "compile:"));
    assert_eq!(value["summary"]["warnings"], 1);
}

#[test]
fn check_compile_commands_file_target_works() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_project_source(temp.path(), "src/good.cpp", "int good() { return 1; }\n");
    write_compile_commands(
        temp.path(),
        &[serde_json::json!({
            "directory": temp.path(),
            "file": "src/good.cpp",
            "arguments": ["clang++", "-std=c++20", "-c", "src/good.cpp"]
        })],
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "compile_commands.json"])
        .assert()
        .success();

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(value["target"]["path"], "compile_commands.json");
}

#[test]
fn check_directory_reports_compile_database_failure() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_project_source(
        temp.path(),
        "src/broken.cpp",
        "int broken() { return missing_symbol; }\n",
    );
    write_compile_commands(
        temp.path(),
        &[serde_json::json!({
            "directory": temp.path(),
            "file": "src/broken.cpp",
            "arguments": ["clang++", "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-c", "src/broken.cpp"]
        })],
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "."])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(value["summary"]["failed_stages"], 1);
    assert_eq!(value["summary"]["errors"], 1);
}

#[test]
fn check_directory_without_compile_commands_reports_error() {
    let temp = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "."])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "could not find compile_commands.json",
        ));
}

#[test]
fn check_cmake_project_generates_compile_commands() {
    if !clang_available() || !cmake_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_cmake_project(
        temp.path(),
        "int cmake_fixture_add(int a, int b) { return a + b; }\n",
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", ".", "--timeout-seconds", "60"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: PASSED"))
        .stdout(predicate::str::contains("cmake_configure"));

    assert!(temp
        .path()
        .join(".cppgauntlet/cmake-build/compile_commands.json")
        .exists());

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(stage(&value, "cmake_configure")["status"], "passed");
    assert!(stage_name_exists(&value, "compile:"));
}

#[test]
fn check_cmake_configure_failure_writes_report() {
    if !cmake_available() {
        return;
    }

    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.16)\nthis_is_not_a_cmake_command()\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", ".", "--timeout-seconds", "30"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"))
        .stdout(predicate::str::contains("cmake_configure"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "cmake_configure")["status"], "failed");
    assert_eq!(value["summary"]["failed_stages"], 1);
}

#[test]
fn check_cmake_project_can_run_ctest() {
    if !clang_available() || !cmake_available() || !ctest_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_cmake_test_project(temp.path(), "int main() { return 0; }\n");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", ".", "--ctest", "--timeout-seconds", "60"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cmake_build"))
        .stdout(predicate::str::contains("ctest"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(stage(&value, "cmake_build")["status"], "passed");
    assert_eq!(stage(&value, "ctest")["status"], "passed");
}

#[test]
fn check_cmake_project_reports_ctest_failure() {
    if !clang_available() || !cmake_available() || !ctest_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_cmake_test_project(temp.path(), "int main() { return 3; }\n");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", ".", "--ctest", "--timeout-seconds", "60"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "cmake_build")["status"], "passed");
    assert_eq!(stage(&value, "ctest")["status"], "failed");
}

#[test]
fn check_cmake_project_can_enable_ctest_from_config() {
    if !clang_available() || !cmake_available() || !ctest_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_cmake_test_project(temp.path(), "int main() { return 0; }\n");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        "timeout_seconds: 60\ntest:\n  ctest: true\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "."])
        .assert()
        .success();

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "ctest")["status"], "passed");
}

#[test]
fn check_ctest_requires_cmake_project() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp", "--ctest"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--ctest is only supported when checking a CMake project directory",
        ));
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

fn cmake_available() -> bool {
    StdCommand::new("cmake").arg("--version").output().is_ok()
}

fn ctest_available() -> bool {
    StdCommand::new("ctest").arg("--version").output().is_ok()
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

fn stage_name_exists(value: &serde_json::Value, prefix: &str) -> bool {
    value["stages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|stage| stage["name"].as_str().unwrap().starts_with(prefix))
}

fn write_project_source(dir: &std::path::Path, relative_path: &str, contents: &str) {
    let path = dir.join(relative_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn write_compile_commands(dir: &std::path::Path, entries: &[serde_json::Value]) {
    fs::write(
        dir.join("compile_commands.json"),
        serde_json::to_string_pretty(entries).unwrap(),
    )
    .unwrap();
}

fn write_cmake_project(dir: &std::path::Path, source: &str) {
    write_project_source(dir, "src/lib.cpp", source);
    fs::write(
        dir.join("CMakeLists.txt"),
        r#"cmake_minimum_required(VERSION 3.16)
project(CppGauntletFixture LANGUAGES CXX)
add_library(cppgauntlet_fixture src/lib.cpp)
target_compile_features(cppgauntlet_fixture PRIVATE cxx_std_17)
target_compile_options(cppgauntlet_fixture PRIVATE -Wall -Wextra -Wpedantic)
"#,
    )
    .unwrap();
}

fn write_cmake_test_project(dir: &std::path::Path, source: &str) {
    write_project_source(dir, "src/test.cpp", source);
    fs::write(
        dir.join("CMakeLists.txt"),
        r#"cmake_minimum_required(VERSION 3.16)
project(CppGauntletCTestFixture LANGUAGES CXX)
enable_testing()
add_executable(cppgauntlet_fixture_test src/test.cpp)
target_compile_features(cppgauntlet_fixture_test PRIVATE cxx_std_17)
target_compile_options(cppgauntlet_fixture_test PRIVATE -Wall -Wextra -Wpedantic)
add_test(NAME cppgauntlet_fixture_test COMMAND cppgauntlet_fixture_test)
"#,
    )
    .unwrap();
}

#[cfg(unix)]
fn make_fake_tool(dir: &std::path::Path, name: &str, version: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\necho '{version}'\n")).unwrap();

    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(unix)]
fn path_with_prefix(dir: &std::path::Path) -> std::ffi::OsString {
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let paths = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&current_path));
    std::env::join_paths(paths).unwrap()
}
