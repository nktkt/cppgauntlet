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
fn github_code_scanning_example_is_documented() {
    let workflow = fs::read_to_string("examples/github-actions/code-scanning.yml").unwrap();
    assert!(workflow.contains("github/codeql-action/upload-sarif@v4"));
    assert!(workflow.contains("security-events: write"));
    assert!(workflow.contains("--sarif-report .cppgauntlet/cppgauntlet.sarif.json"));
    assert!(workflow.contains("category: cppgauntlet"));

    let docs = fs::read_to_string("docs/GITHUB_CODE_SCANNING.md").unwrap();
    assert!(docs.contains("../examples/github-actions/code-scanning.yml"));
    assert!(docs.contains("github/codeql-action/upload-sarif@v4"));
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

    assert_eq!(value["schema_version"], 3);
    assert_eq!(value["status"], "passed");
    assert_eq!(value["target"]["standard"], "c++20");
}

#[test]
fn check_markdown_format_outputs_report() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "--format",
            "markdown",
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# CppGauntlet Check Report"))
        .stdout(predicate::str::contains("| Status | passed |"))
        .stdout(predicate::str::contains("No diagnostics recorded."));
}

#[test]
fn check_html_format_outputs_report() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "--format",
            "html",
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("<!doctype html>"))
        .stdout(predicate::str::contains(
            "<title>CppGauntlet Check Report</title>",
        ))
        .stdout(predicate::str::contains("No diagnostics recorded."));
}

#[test]
fn check_can_write_markdown_report_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--markdown-report",
            "report.md",
        ])
        .assert()
        .success();

    let markdown = fs::read_to_string(temp.path().join("report.md")).unwrap();
    assert!(markdown.contains("# CppGauntlet Check Report"));
    assert!(markdown.contains("| compile | passed |"));
    assert!(markdown.contains("unused variable"));

    let value = read_report(temp.path());
    assert_eq!(value["markdown_report_path"], "report.md");
}

#[test]
fn check_can_write_html_report_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--html-report",
            "report.html",
        ])
        .assert()
        .success();

    let html = fs::read_to_string(temp.path().join("report.html")).unwrap();
    assert!(html.contains("<!doctype html>"));
    assert!(html.contains("<td>compile</td>"));
    assert!(html.contains("unused variable"));

    let value = read_report(temp.path());
    assert_eq!(value["html_report_path"], "report.html");
}

#[test]
fn check_can_write_sarif_report_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--sarif-report",
            "report.sarif.json",
        ])
        .assert()
        .success();

    let sarif = fs::read_to_string(temp.path().join("report.sarif.json")).unwrap();
    let sarif: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(sarif["version"], "2.1.0");
    assert_eq!(sarif["runs"][0]["tool"]["driver"]["name"], "CppGauntlet");
    assert_eq!(
        sarif["runs"][0]["results"][0]["ruleId"],
        "cppgauntlet/warning"
    );
    assert_eq!(sarif["runs"][0]["results"][0]["level"], "warning");
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
            ["uri"],
        "warning.cpp"
    );
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startLine"],
        2
    );
    assert_eq!(
        sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startColumn"],
        9
    );

    let value = read_report(temp.path());
    assert_eq!(value["sarif_report_path"], "report.sarif.json");
    let diagnostic = &stage(&value, "compile")["diagnostics"][0];
    assert_eq!(diagnostic["location"]["uri"], "warning.cpp");
    assert_eq!(diagnostic["location"]["start_line"], 2);
    assert_eq!(diagnostic["location"]["start_column"], 9);
    let fingerprint = diagnostic["fingerprint"].as_str().unwrap();
    assert_eq!(fingerprint.len(), 16);
    assert_eq!(
        sarif["runs"][0]["results"][0]["partialFingerprints"]["cppgauntletDiagnosticV1"],
        fingerprint
    );
}

#[test]
fn check_config_can_write_markdown_report_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        r#"sanitizers:
  enabled: []
report:
  markdown_path: configured-report.md
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success();

    let markdown = fs::read_to_string(temp.path().join("configured-report.md")).unwrap();
    assert!(markdown.contains("# CppGauntlet Check Report"));
}

#[test]
fn check_config_can_write_html_report_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        r#"sanitizers:
  enabled: []
report:
  html_path: configured-report.html
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success();

    let html = fs::read_to_string(temp.path().join("configured-report.html")).unwrap();
    assert!(html.contains("<title>CppGauntlet Check Report</title>"));
}

#[test]
fn check_config_can_write_sarif_report_file() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        r#"sanitizers:
  enabled: []
report:
  sarif_path: configured-report.sarif.json
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "warning.cpp"])
        .assert()
        .success();

    let sarif = fs::read_to_string(temp.path().join("configured-report.sarif.json")).unwrap();
    let sarif: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    assert_eq!(
        sarif["runs"][0]["results"][0]["ruleId"],
        "cppgauntlet/warning"
    );
}

#[test]
#[cfg(unix)]
fn check_source_runs_clang_tidy() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let clang_tidy = make_fake_script(
        temp.path(),
        "fake-clang-tidy",
        "echo 'hello.cpp:1:1: warning: fake tidy warning [fake-check]'\n",
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
            "--clang-tidy",
            "--clang-tidy-bin",
            clang_tidy.to_str().unwrap(),
            "--clang-tidy-checks",
            "modernize-*",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("clang_tidy"))
        .stdout(predicate::str::contains("Warnings: 1"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(stage(&value, "clang_tidy")["status"], "passed");
    assert_eq!(stage(&value, "clang_tidy")["warnings"], 1);
    assert!(stage(&value, "clang_tidy")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "--checks=modernize-*"));
}

#[test]
#[cfg(unix)]
fn check_source_reports_clang_tidy_failure() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let clang_tidy = make_fake_script(
        temp.path(),
        "fake-clang-tidy-fail",
        "echo 'hello.cpp:1:1: error: fake tidy failure [fake-check]'\nexit 2\n",
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
            "--clang-tidy-bin",
            clang_tidy.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "clang_tidy")["status"], "failed");
    assert_eq!(stage(&value, "clang_tidy")["errors"], 1);
}

#[test]
#[cfg(unix)]
fn check_config_can_enable_clang_tidy() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let clang_tidy = make_fake_script(temp.path(), "configured-clang-tidy", "exit 0\n");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        format!(
            r#"sanitizers:
  enabled: []
static_analysis:
  clang_tidy: true
  clang_tidy_bin: "{}"
  clang_tidy_checks: "bugprone-*"
"#,
            clang_tidy.display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success();

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "clang_tidy")["status"], "passed");
    assert!(stage(&value, "clang_tidy")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "--checks=bugprone-*"));
}

#[test]
#[cfg(unix)]
fn check_source_can_collect_coverage() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "coverage-clang++");
    let llvm_profdata = make_fake_profdata(temp.path(), "coverage-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov(temp.path(), "coverage-llvm-cov");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--coverage",
            "--llvm-profdata-bin",
            llvm_profdata.to_str().unwrap(),
            "--llvm-cov-bin",
            llvm_cov.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Line Coverage: 100.00%"))
        .stdout(predicate::str::contains("coverage_report"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(stage(&value, "coverage_compile")["status"], "passed");
    assert_eq!(stage(&value, "coverage_run")["status"], "passed");
    assert_eq!(stage(&value, "coverage_merge")["status"], "passed");
    assert_eq!(stage(&value, "coverage_report")["status"], "passed");
    assert_eq!(value["summary"]["coverage"]["lines"]["percent"], 100.0);
    assert!(stage(&value, "coverage_compile")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "-fprofile-instr-generate"));
    assert!(stage(&value, "coverage_run")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg.as_str().unwrap().starts_with("LLVM_PROFILE_FILE=")));
    assert!(temp
        .path()
        .join(".cppgauntlet/coverage/coverage-summary.json")
        .exists());
}

#[test]
#[cfg(unix)]
fn check_coverage_can_filter_sources_and_objects() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "filtered-coverage-clang++");
    let llvm_profdata = make_fake_profdata(temp.path(), "filtered-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov(temp.path(), "filtered-llvm-cov");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--coverage",
            "--coverage-source",
            "src/interesting.cpp",
            "--coverage-object",
            "build/interesting.o",
            "--coverage-object",
            "build/helper.o",
            "--llvm-profdata-bin",
            llvm_profdata.to_str().unwrap(),
            "--llvm-cov-bin",
            llvm_cov.to_str().unwrap(),
        ])
        .assert()
        .success();

    let value = read_report(temp.path());
    let command = stage(&value, "coverage_report")["command"]
        .as_array()
        .unwrap();
    assert!(command.iter().any(|arg| arg == "build/interesting.o"));
    assert!(command.iter().any(|arg| arg == "--object=build/helper.o"));
    assert!(command.iter().any(|arg| arg == "--sources"));
    assert!(command.iter().any(|arg| arg == "src/interesting.cpp"));
    assert!(!command
        .iter()
        .any(|arg| { arg.as_str().unwrap().ends_with("hello.cpp") }));
}

#[test]
#[cfg(unix)]
fn check_config_can_enable_coverage() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "configured-coverage-clang++");
    let llvm_profdata = make_fake_profdata(temp.path(), "configured-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov(temp.path(), "configured-llvm-cov");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        format!(
            r#"compiler: "{}"
sanitizers:
  enabled: []
coverage:
  enabled: true
  llvm_cov_bin: "{}"
  llvm_profdata_bin: "{}"
  sources:
    - configured-source.cpp
  objects:
    - configured-object.o
"#,
            compiler.display(),
            llvm_cov.display(),
            llvm_profdata.display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success();

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "coverage_report")["status"], "passed");
    assert_eq!(value["summary"]["coverage"]["functions"]["covered"], 1);
    assert!(stage(&value, "coverage_report")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "configured-object.o"));
    assert!(stage(&value, "coverage_report")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "configured-source.cpp"));
}

#[test]
#[cfg(unix)]
fn check_compile_commands_can_run_custom_test_command() {
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
        .args(["check", ".", "--test-command", "test -f src/good.cpp"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test_command"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "test_command")["status"], "passed");
    assert!(stage(&value, "test_command")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "test -f src/good.cpp"));
}

#[test]
#[cfg(unix)]
fn check_custom_test_command_failure_fails_report() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "test-command-clang++");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--test-command",
            "exit 4",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "test_command")["status"], "failed");
    assert_eq!(stage(&value, "test_command")["exit_code"], 4);
}

#[test]
#[cfg(unix)]
fn check_config_can_enable_custom_test_command() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "configured-test-command-clang++");
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        format!(
            r#"compiler: "{}"
sanitizers:
  enabled: []
test:
  command: "test -f hello.cpp"
"#,
            compiler.display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success();

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "test_command")["status"], "passed");
}

#[test]
fn check_policy_can_fail_on_warning_threshold() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--max-warnings",
            "0",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("policy: failed"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "policy")["status"], "failed");
    assert!(stage(&value, "policy")["stderr"]
        .as_str()
        .unwrap()
        .contains("exceed configured maximum"));
}

#[test]
#[cfg(unix)]
fn check_policy_can_fail_on_analyzer_findings() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "policy-analyzer-clang++");
    let clang_tidy = make_fake_script(
        temp.path(),
        "policy-clang-tidy",
        "echo 'hello.cpp:1:1: warning: analyzer finding [fake-check]'\n",
    );

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--clang-tidy-bin",
            clang_tidy.to_str().unwrap(),
            "--max-analyzer-findings",
            "0",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("clang_tidy"))
        .stdout(predicate::str::contains("policy: failed"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "clang_tidy")["status"], "passed");
    assert_eq!(stage(&value, "policy")["status"], "failed");
    assert!(stage(&value, "policy")["stderr"]
        .as_str()
        .unwrap()
        .contains("analyzer findings 1 exceed configured maximum 0"));
}

#[test]
#[cfg(unix)]
fn check_config_can_enable_analyzer_policy() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "configured-analyzer-policy-clang++");
    let clang_tidy = make_fake_script(
        temp.path(),
        "configured-policy-clang-tidy",
        "echo 'hello.cpp:1:1: warning: analyzer finding [fake-check]'\n",
    );
    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        format!(
            r#"compiler: "{}"
sanitizers:
  enabled: []
static_analysis:
  clang_tidy_bin: "{}"
policy:
  max_analyzer_findings: 1
"#,
            compiler.display(),
            clang_tidy.display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp"])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy: passed"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "clang_tidy")["status"], "passed");
    assert_eq!(stage(&value, "policy")["status"], "passed");
    assert!(stage(&value, "policy")["stdout"]
        .as_str()
        .unwrap()
        .contains("analyzer findings 1 <= 1"));
}

#[test]
#[cfg(unix)]
fn check_policy_can_pass_coverage_threshold() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "policy-coverage-clang++");
    let llvm_profdata = make_fake_profdata(temp.path(), "policy-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov_with_line_percent(temp.path(), "policy-llvm-cov", 95.0);

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--coverage",
            "--llvm-profdata-bin",
            llvm_profdata.to_str().unwrap(),
            "--llvm-cov-bin",
            llvm_cov.to_str().unwrap(),
            "--min-line-coverage",
            "90",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy: passed"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "policy")["status"], "passed");
    assert_eq!(value["summary"]["coverage"]["lines"]["percent"], 95.0);
}

#[test]
#[cfg(unix)]
fn check_policy_fails_when_coverage_threshold_has_no_summary() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "policy-no-coverage-clang++");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--min-line-coverage",
            "80",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("policy: failed"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "policy")["status"], "failed");
    assert!(stage(&value, "policy")["stderr"]
        .as_str()
        .unwrap()
        .contains("line coverage summary is unavailable"));
}

#[test]
#[cfg(unix)]
fn check_policy_can_fail_on_changed_line_coverage() {
    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    let compiler = make_fake_compiler(temp.path(), "changed-line-coverage-clang++");
    let llvm_profdata = make_fake_profdata(temp.path(), "changed-line-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov_with_changed_lines(temp.path(), "changed-line-llvm-cov");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--compiler",
            compiler.to_str().unwrap(),
            "--sanitizers",
            "none",
            "--changed-line",
            "hello.cpp:1",
            "--changed-line",
            "hello.cpp:2",
            "--min-changed-line-coverage",
            "60",
            "--llvm-profdata-bin",
            llvm_profdata.to_str().unwrap(),
            "--llvm-cov-bin",
            llvm_cov.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("policy: failed"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "coverage_report")["status"], "passed");
    assert_eq!(value["summary"]["coverage"]["changed_lines"]["covered"], 1);
    assert_eq!(value["summary"]["coverage"]["changed_lines"]["count"], 2);
    assert_eq!(
        value["summary"]["coverage"]["changed_lines"]["percent"],
        50.0
    );
    assert!(!stage(&value, "coverage_report")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "--summary-only"));
    assert!(stage(&value, "policy")["stderr"]
        .as_str()
        .unwrap()
        .contains("changed-line coverage 50.00% is below configured minimum 60.00%"));
}

#[test]
fn check_baseline_marks_existing_diagnostics_without_failing() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");
    let baseline_path = temp.path().join("baseline.json");
    let current_path = temp.path().join("current.json");

    let mut baseline = Command::cargo_bin("cppgauntlet").unwrap();
    baseline
        .current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--report",
            baseline_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--report",
            current_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline: 0 new"));

    let value = read_report_at(current_path);
    assert_eq!(value["status"], "passed");
    assert_eq!(
        value["summary"]["baseline"]["new_diagnostic_occurrences"],
        0
    );
    assert_eq!(
        stage(&value, "compile")["diagnostics"][0]["baseline_status"],
        "existing"
    );
}

#[test]
fn check_policy_fails_on_new_diagnostics_against_baseline() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    copy_fixture(temp.path(), "warning.cpp");
    let baseline_path = temp.path().join("baseline.json");
    let current_path = temp.path().join("current.json");

    let mut baseline = Command::cargo_bin("cppgauntlet").unwrap();
    baseline
        .current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
            "--report",
            baseline_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--fail-on-new-diagnostics",
            "--report",
            current_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("policy: failed"));

    let value = read_report_at(current_path);
    assert_eq!(value["status"], "failed");
    assert_eq!(
        value["summary"]["baseline"]["new_diagnostic_occurrences"],
        1
    );
    assert_eq!(
        stage(&value, "compile")["diagnostics"][0]["baseline_status"],
        "new"
    );
    assert!(stage(&value, "policy")["stderr"]
        .as_str()
        .unwrap()
        .contains("new diagnostics 1 exceed baseline"));
}

#[test]
fn check_config_can_enable_baseline_policy() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");
    copy_fixture(temp.path(), "warning.cpp");

    let mut baseline = Command::cargo_bin("cppgauntlet").unwrap();
    baseline
        .current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
            "--report",
            "baseline.json",
        ])
        .assert()
        .success();

    fs::write(
        temp.path().join("cppgauntlet.yaml"),
        r#"sanitizers:
  enabled: []
baseline:
  path: baseline.json
policy:
  fail_on_new_diagnostics: true
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "warning.cpp"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("policy: failed"));

    let value = read_report(temp.path());
    assert_eq!(stage(&value, "policy")["status"], "failed");
    assert_eq!(
        value["summary"]["baseline"]["new_diagnostic_occurrences"],
        1
    );
}

#[test]
fn baseline_update_writes_reusable_baseline_report() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");

    let mut check = Command::cargo_bin("cppgauntlet").unwrap();
    check
        .current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--report",
            "current.json",
            "--markdown-report",
            "current.md",
            "--html-report",
            "current.html",
            "--sarif-report",
            "current.sarif.json",
        ])
        .assert()
        .success();

    let mut update = Command::cargo_bin("cppgauntlet").unwrap();
    update
        .current_dir(temp.path())
        .args([
            "baseline",
            "update",
            "--report",
            "current.json",
            "--output",
            "baseline.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: UPDATED"))
        .stdout(predicate::str::contains("Unique diagnostics: 1"));

    let baseline = read_report_at(temp.path().join("baseline.json"));
    assert_eq!(baseline["report_path"], "baseline.json");
    assert!(baseline["markdown_report_path"].is_null());
    assert!(baseline["html_report_path"].is_null());
    assert!(baseline["sarif_report_path"].is_null());
    assert!(baseline["summary"]["baseline"].is_null());
    assert!(stage(&baseline, "compile")["diagnostics"][0]["baseline_status"].is_null());

    let mut verify = Command::cargo_bin("cppgauntlet").unwrap();
    verify
        .current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--baseline",
            "baseline.json",
            "--fail-on-new-diagnostics",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("policy: passed"));
}

#[test]
fn baseline_update_supports_json_and_markdown_output() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");

    let mut check = Command::cargo_bin("cppgauntlet").unwrap();
    check
        .current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
            "--report",
            "current.json",
        ])
        .assert()
        .success();

    let mut json = Command::cargo_bin("cppgauntlet").unwrap();
    let assert = json
        .current_dir(temp.path())
        .args([
            "--format",
            "json",
            "baseline",
            "update",
            "--report",
            "current.json",
            "--output",
            "baseline-json.json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["output"], "baseline-json.json");

    let mut markdown = Command::cargo_bin("cppgauntlet").unwrap();
    markdown
        .current_dir(temp.path())
        .args([
            "--format",
            "markdown",
            "baseline",
            "update",
            "--report",
            "current.json",
            "--output",
            "baseline-md.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# CppGauntlet Baseline"))
        .stdout(predicate::str::contains("| Status | updated |"));
}

#[test]
fn baseline_update_reports_previous_baseline_changes() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "warning.cpp");
    copy_fixture(temp.path(), "hello.cpp");

    let mut previous_check = Command::cargo_bin("cppgauntlet").unwrap();
    previous_check
        .current_dir(temp.path())
        .args([
            "check",
            "warning.cpp",
            "--sanitizers",
            "none",
            "--report",
            "previous-report.json",
        ])
        .assert()
        .success();

    let mut previous_update = Command::cargo_bin("cppgauntlet").unwrap();
    previous_update
        .current_dir(temp.path())
        .args([
            "baseline",
            "update",
            "--report",
            "previous-report.json",
            "--output",
            "previous-baseline.json",
        ])
        .assert()
        .success();

    let mut current_check = Command::cargo_bin("cppgauntlet").unwrap();
    current_check
        .current_dir(temp.path())
        .args([
            "check",
            "hello.cpp",
            "--sanitizers",
            "none",
            "--report",
            "current-report.json",
        ])
        .assert()
        .success();

    let mut update = Command::cargo_bin("cppgauntlet").unwrap();
    let assert = update
        .current_dir(temp.path())
        .args([
            "--format",
            "json",
            "baseline",
            "update",
            "--report",
            "current-report.json",
            "--previous",
            "previous-baseline.json",
            "--output",
            "updated-baseline.json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value["previous_baseline"], "previous-baseline.json");
    assert_eq!(value["previous_unique_diagnostics"], 1);
    assert_eq!(value["unique_diagnostics"], 0);
    assert_eq!(value["new_unique_diagnostics"], 0);
    assert_eq!(value["resolved_unique_diagnostics"], 1);
    assert_eq!(value["unchanged_unique_diagnostics"], 0);

    let updated = read_report_at(temp.path().join("updated-baseline.json"));
    assert_eq!(
        stage(&updated, "compile")["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn baseline_update_rejects_invalid_report() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("broken.json"), "not json").unwrap();

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "baseline",
            "update",
            "--report",
            "broken.json",
            "--output",
            "baseline.json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse report"));
}

#[test]
fn check_fail_on_new_diagnostics_requires_baseline() {
    if !clang_available() {
        return;
    }

    let temp = tempdir().unwrap();
    copy_fixture(temp.path(), "hello.cpp");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args(["check", "hello.cpp", "--fail-on-new-diagnostics"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires --baseline"));
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
#[cfg(unix)]
fn check_compile_commands_can_run_clang_tidy() {
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
    let clang_tidy = make_fake_script(temp.path(), "project-clang-tidy", "exit 0\n");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            ".",
            "--clang-tidy",
            "--clang-tidy-bin",
            clang_tidy.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("clang_tidy:"));

    let value = read_report(temp.path());
    let tidy_stage = stage_with_prefix(&value, "clang_tidy:");
    assert_eq!(tidy_stage["status"], "passed");
    assert!(tidy_stage["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "-p"));
}

#[test]
#[cfg(unix)]
fn check_compile_commands_can_collect_coverage() {
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
            "arguments": ["clang++", "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-c", "src/good.cpp"]
        })],
    );
    let llvm_profdata = make_fake_profdata(temp.path(), "compdb-coverage-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov(temp.path(), "compdb-coverage-llvm-cov");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            ".",
            "--coverage",
            "--test-command",
            r#"test -f src/good.cpp; if [ -n "$LLVM_PROFILE_FILE" ]; then : > "$LLVM_PROFILE_FILE"; fi"#,
            "--llvm-profdata-bin",
            llvm_profdata.to_str().unwrap(),
            "--llvm-cov-bin",
            llvm_cov.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage_test_command"))
        .stdout(predicate::str::contains("Line Coverage: 100.00%"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    let coverage_compile = stage_with_prefix(&value, "coverage_compile:");
    assert_eq!(coverage_compile["status"], "passed");
    assert!(coverage_compile["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "-fprofile-instr-generate"));
    assert_eq!(stage(&value, "coverage_test_command")["status"], "passed");
    assert_eq!(stage(&value, "coverage_merge")["status"], "passed");
    assert_eq!(stage(&value, "coverage_report")["status"], "passed");
    assert_eq!(value["summary"]["coverage"]["lines"]["percent"], 100.0);
    assert!(stage(&value, "coverage_test_command")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg.as_str().unwrap().starts_with("LLVM_PROFILE_FILE=")));
    assert!(temp
        .path()
        .join(".cppgauntlet/coverage/compilation-database/coverage-summary.json")
        .exists());
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
        .args([
            "check",
            ".",
            "--timeout-seconds",
            "30",
            "--max-warnings",
            "0",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Status: FAILED"))
        .stdout(predicate::str::contains("cmake_configure"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert_eq!(stage(&value, "cmake_configure")["status"], "failed");
    assert_eq!(stage(&value, "policy")["status"], "skipped");
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
#[cfg(unix)]
fn check_cmake_project_can_collect_coverage() {
    if !clang_available() || !cmake_available() || !ctest_available() {
        return;
    }

    let temp = tempdir().unwrap();
    write_cmake_test_project(temp.path(), "int main() { return 0; }\n");
    write_compile_commands(
        temp.path(),
        &[serde_json::json!({
            "directory": temp.path(),
            "file": "src/test.cpp",
            "arguments": ["clang++", "-std=c++20", "-c", "src/test.cpp"]
        })],
    );
    let llvm_profdata = make_fake_profdata(temp.path(), "cmake-coverage-llvm-profdata");
    let llvm_cov = make_fake_llvm_cov(temp.path(), "cmake-coverage-llvm-cov");

    let mut cmd = Command::cargo_bin("cppgauntlet").unwrap();
    cmd.current_dir(temp.path())
        .args([
            "check",
            ".",
            "--coverage",
            "--llvm-profdata-bin",
            llvm_profdata.to_str().unwrap(),
            "--llvm-cov-bin",
            llvm_cov.to_str().unwrap(),
            "--timeout-seconds",
            "60",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage_cmake_configure"))
        .stdout(predicate::str::contains("coverage_ctest"))
        .stdout(predicate::str::contains("Line Coverage: 100.00%"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "passed");
    assert_eq!(
        stage(&value, "coverage_cmake_configure")["status"],
        "passed"
    );
    assert_eq!(stage(&value, "coverage_cmake_build")["status"], "passed");
    assert_eq!(stage(&value, "coverage_ctest")["status"], "passed");
    assert_eq!(stage(&value, "coverage_merge")["status"], "passed");
    assert_eq!(stage(&value, "coverage_report")["status"], "passed");
    assert_eq!(value["summary"]["coverage"]["lines"]["percent"], 100.0);
    assert!(stage(&value, "coverage_ctest")["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg.as_str().unwrap().starts_with("LLVM_PROFILE_FILE=")));
    assert!(temp
        .path()
        .join(".cppgauntlet/coverage/cmake/coverage-summary.json")
        .exists());
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
fn check_compile_commands_coverage_requires_test_command() {
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
        .args(["check", ".", "--coverage"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("coverage_test_command"));

    let value = read_report(temp.path());
    assert_eq!(value["status"], "failed");
    assert!(stage(&value, "coverage_test_command")["stderr"]
        .as_str()
        .unwrap()
        .contains("requires --test-command"));
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

fn stage_with_prefix<'a>(value: &'a serde_json::Value, prefix: &str) -> &'a serde_json::Value {
    value["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|stage| stage["name"].as_str().unwrap().starts_with(prefix))
        .unwrap()
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
    make_fake_script(dir, name, &format!("echo '{version}'\n"));
}

#[cfg(unix)]
fn make_fake_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();

    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

#[cfg(unix)]
fn make_fake_compiler(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    make_fake_script(
        dir,
        name,
        r#"out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
  fi
  shift
done
if [ -z "$out" ]; then
  echo "missing output path" >&2
  exit 2
fi
cat > "$out" <<'SCRIPT'
#!/bin/sh
exit 0
SCRIPT
chmod +x "$out"
"#,
    )
}

#[cfg(unix)]
fn make_fake_profdata(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    make_fake_script(
        dir,
        name,
        r#"out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
  fi
  shift
done
if [ -z "$out" ]; then
  echo "missing output path" >&2
  exit 2
fi
: > "$out"
"#,
    )
}

#[cfg(unix)]
fn make_fake_llvm_cov(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    make_fake_llvm_cov_with_line_percent(dir, name, 100.0)
}

#[cfg(unix)]
fn make_fake_llvm_cov_with_line_percent(
    dir: &std::path::Path,
    name: &str,
    line_percent: f64,
) -> std::path::PathBuf {
    let line_covered = line_percent.round() as u64;
    let body = format!(
        r#"cat <<'JSON'
{{
  "data": [
    {{
      "totals": {{
        "lines": {{ "count": 100, "covered": {line_covered}, "percent": {line_percent} }},
        "functions": {{ "count": 1, "covered": 1, "percent": 100.0 }},
        "regions": {{ "count": 2, "covered": 2, "percent": 100.0 }}
      }}
    }}
  ],
  "type": "llvm.coverage.json.export",
  "version": "2.0.1"
}}
JSON
"#
    );
    make_fake_script(dir, name, &body)
}

#[cfg(unix)]
fn make_fake_llvm_cov_with_changed_lines(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let body = r#"cat <<'JSON'
{
  "data": [
    {
      "files": [
        {
          "filename": "hello.cpp",
          "segments": [
            [1, 1, 1, true, true, false],
            [2, 1, 0, true, true, false]
          ],
          "summary": {
            "lines": { "count": 2, "covered": 1, "percent": 50.0 },
            "functions": { "count": 1, "covered": 1, "percent": 100.0 },
            "regions": { "count": 2, "covered": 1, "percent": 50.0 }
          }
        }
      ],
      "totals": {
        "lines": { "count": 2, "covered": 1, "percent": 50.0 },
        "functions": { "count": 1, "covered": 1, "percent": 100.0 },
        "regions": { "count": 2, "covered": 1, "percent": 50.0 }
      }
    }
  ],
  "type": "llvm.coverage.json.export",
  "version": "2.0.1"
}
JSON
"#;
    make_fake_script(dir, name, body)
}

#[cfg(unix)]
fn path_with_prefix(dir: &std::path::Path) -> std::ffi::OsString {
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let paths = std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&current_path));
    std::env::join_paths(paths).unwrap()
}
