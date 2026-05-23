# CppGauntlet

CppGauntlet is a Rust-powered command-line tool for putting C++ code through a practical verification pipeline.

The goal is to make it easy to run a consistent set of checks across C++ files and projects: compile diagnostics, strict warnings, sanitizer builds, test execution, coverage, and machine-readable reports.

## Vision

CppGauntlet treats C++ validation as a gauntlet:

1. Detect the project layout.
2. Build with Clang and strict warnings.
3. Run static analysis where available.
4. Rebuild with sanitizers.
5. Execute tests.
6. Collect coverage.
7. Produce a clear score and report.

It is not intended to prove that a program is correct. It is intended to make common failures easier to expose, reproduce, and track over time.

## Planned CLI

```bash
cppgauntlet check main.cpp
cppgauntlet check ./src
cppgauntlet test ./project
cppgauntlet sanitize ./project --sanitizers address,undefined
cppgauntlet coverage ./project
cppgauntlet report ./project --format json
```

## Current CLI

The first implementation supports a single-file check workflow:

```bash
cargo run -- check main.cpp
cargo run -- check ./project
cargo run -- check ./project/compile_commands.json
cargo run -- check ./cmake-project
cargo run -- check ./cmake-project --ctest
cargo run -- check main.cpp --clang-tidy
cargo run -- check main.cpp --coverage
cargo run -- check main.cpp --coverage --coverage-source src/main.cpp
cargo run -- check ./project/compile_commands.json --coverage --test-command "./scripts/test.sh"
cargo run -- check ./cmake-project --coverage
cargo run -- check ./project --test-command "make test"
cargo run -- check main.cpp --max-warnings 0
cargo run -- check main.cpp --max-analyzer-findings 0
cargo run -- check main.cpp --coverage --min-line-coverage 80
cargo run -- check main.cpp --changed-line src/main.cpp:42 --min-changed-line-coverage 90
cargo run -- check ./project --changed-lines-diff .cppgauntlet/changed.diff --min-changed-line-coverage 80
cargo run -- check fuzz_target.cpp --fuzz --fuzz-seconds 5
cargo run -- check ./project --fuzz --fuzz-seconds 5
cargo run -- check main.cpp --baseline .cppgauntlet/baseline.json --fail-on-new-diagnostics
cargo run -- baseline update --report .cppgauntlet/cppgauntlet-report.json --output .cppgauntlet/baseline.json
cargo run -- baseline update --report current.json --previous .cppgauntlet/baseline.json --output .cppgauntlet/baseline.json
cargo run -- --format markdown check main.cpp
cargo run -- --format html check main.cpp
cargo run -- check main.cpp --markdown-report .cppgauntlet/cppgauntlet-report.md
cargo run -- check main.cpp --html-report .cppgauntlet/cppgauntlet-report.html
cargo run -- check main.cpp --sarif-report .cppgauntlet/cppgauntlet-report.sarif.json
cargo run -- init
cargo run -- doctor
cargo run -- --format json check main.cpp
cargo run -- check main.cpp --standard c++23 --sanitizers address,undefined
cargo run -- check main.cpp --config cppgauntlet.yaml
```

Generated artifacts are written to `.cppgauntlet/` by default, including `cppgauntlet-report.json`.

The current JSON report schema is version `3`. Each stage records:

- command arguments
- exit code
- timeout state
- warning and error counts
- structured diagnostics extracted from compiler, sanitizer, and analyzer output
- optional coverage summary
- changed-line coverage from explicit lines or unified diffs
- raw stdout and stderr
- generated artifact path

See [docs/REPORT_SCHEMA.md](docs/REPORT_SCHEMA.md) for the current report contract.
See [docs/REPORT_SCHEMA_MIGRATIONS.md](docs/REPORT_SCHEMA_MIGRATIONS.md) for schema compatibility and migration guidance.
See [docs/INSTALLATION.md](docs/INSTALLATION.md) for install options and tool requirements.
See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the current configuration file format.
See [docs/DOCTOR.md](docs/DOCTOR.md) for environment diagnostics.
See [docs/COMPILATION_DATABASE.md](docs/COMPILATION_DATABASE.md) for project checks through `compile_commands.json`.
See [docs/CMAKE.md](docs/CMAKE.md) for CMake project checks.
See [docs/CLANG_TIDY.md](docs/CLANG_TIDY.md) for static analysis with `clang-tidy`.
See [docs/COVERAGE.md](docs/COVERAGE.md) for source-based coverage with LLVM tools.
See [docs/GITHUB_CHANGED_LINE_COVERAGE.md](docs/GITHUB_CHANGED_LINE_COVERAGE.md) for GitHub Actions changed-line coverage gates.
See [docs/GITHUB_ACTIONS.md](docs/GITHUB_ACTIONS.md) for reusable GitHub Actions examples.
See [docs/FUZZING.md](docs/FUZZING.md) for libFuzzer smoke workflows.
See [docs/TESTING.md](docs/TESTING.md) for CTest and custom test commands.
See [docs/POLICY.md](docs/POLICY.md) for CI policy gates.
See [docs/BASELINE.md](docs/BASELINE.md) for diagnostic baselines.
See [docs/GITHUB_BASELINE_AUTOMATION.md](docs/GITHUB_BASELINE_AUTOMATION.md) for reviewable baseline update artifacts in GitHub Actions.
See [docs/ARTIFACT_REPORTS.md](docs/ARTIFACT_REPORTS.md) for Markdown and HTML reports.
See [docs/SARIF.md](docs/SARIF.md) for SARIF output.
See [docs/GITHUB_CODE_SCANNING.md](docs/GITHUB_CODE_SCANNING.md) for GitHub Code Scanning integration.
See [docs/RELEASE.md](docs/RELEASE.md) for the release checklist.
See [docs/CONTRIBUTOR_AUTOMATION.md](docs/CONTRIBUTOR_AUTOMATION.md) for issue labeling and pull request checks.

## MVP Scope

The first version will focus on a small, useful workflow:

- Compile a single C++ source file with `clang++`.
- Support C++17, C++20, and C++23 modes.
- Enable strict warnings by default.
- Rebuild with AddressSanitizer and UndefinedBehaviorSanitizer.
- Run the produced executable.
- Emit a terminal summary.
- Write a JSON report.

## Future Scope

- CI-friendly exit codes.
- Quality scoring for trend tracking.

See [ROADMAP.md](ROADMAP.md) for the long-term product roadmap.

## Configuration Sketch

```yaml
standard: c++20
compiler: clang++

warnings:
  level: strict
  flags:
    - -Wall
    - -Wextra
    - -Wpedantic

sanitizers:
  enabled:
    - address
    - undefined

report:
  path: .cppgauntlet/cppgauntlet-report.json
  markdown_path: null
  html_path: null
  sarif_path: null

test:
  ctest: false
  command: null

static_analysis:
  clang_tidy: false
  clang_tidy_bin: clang-tidy

coverage:
  enabled: false
  llvm_cov_bin: llvm-cov
  llvm_profdata_bin: llvm-profdata
  sources: []
  objects: []

fuzz:
  enabled: false
  seconds: 5
  corpus: []

baseline:
  path: null

policy:
  max_warnings: null
  max_analyzer_findings: null
  min_line_coverage: null
  min_changed_line_coverage: null
  changed_lines: []
  changed_lines_diff: null
  fail_on_new_diagnostics: false
```

## Project Status

CppGauntlet is in early implementation. The repository currently includes single-file checks, compilation database checks, CMake configuration, optional CTest execution, custom test commands, optional `clang-tidy` analysis, single-file, compilation database, and CMake/CTest LLVM coverage, changed-line coverage from explicit lines or unified diffs, reusable GitHub Actions examples for compile database and CMake coverage checks, single-file and project-discovered libFuzzer smoke workflows with per-target artifact summaries, diagnostic baselines with changed-diagnostic update summaries, CI policy gates including changed-line coverage, baseline review artifacts and PR comments for GitHub Actions, environment diagnostics, JSON reports with diagnostic fingerprints and parsed source locations, schema compatibility tests for older report and baseline files, Markdown reports, HTML reports, SARIF output, GitHub Code Scanning examples, contributor templates, contributor automation for issue labels and pull request checks, release packaging metadata, and automated macOS/Linux release builds.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local setup, validation commands, issue guidance, and pull request expectations. Contributors are expected to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

CppGauntlet is licensed under the [MIT License](LICENSE).
