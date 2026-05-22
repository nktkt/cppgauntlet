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
cargo run -- check ./cmake-project --coverage
cargo run -- check ./project --test-command "make test"
cargo run -- check main.cpp --max-warnings 0
cargo run -- check main.cpp --coverage --min-line-coverage 80
cargo run -- check main.cpp --baseline .cppgauntlet/baseline.json --fail-on-new-diagnostics
cargo run -- baseline update --report .cppgauntlet/cppgauntlet-report.json --output .cppgauntlet/baseline.json
cargo run -- --format markdown check main.cpp
cargo run -- --format html check main.cpp
cargo run -- check main.cpp --markdown-report .cppgauntlet/cppgauntlet-report.md
cargo run -- check main.cpp --html-report .cppgauntlet/cppgauntlet-report.html
cargo run -- init
cargo run -- doctor
cargo run -- --format json check main.cpp
cargo run -- check main.cpp --standard c++23 --sanitizers address,undefined
cargo run -- check main.cpp --config cppgauntlet.yaml
```

Generated artifacts are written to `.cppgauntlet/` by default, including `cppgauntlet-report.json`.

The current JSON report schema is version `2`. Each stage records:

- command arguments
- exit code
- timeout state
- warning and error counts
- structured diagnostics extracted from compiler, sanitizer, and analyzer output
- optional coverage summary
- raw stdout and stderr
- generated artifact path

See [docs/REPORT_SCHEMA.md](docs/REPORT_SCHEMA.md) for the current report contract.
See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the current configuration file format.
See [docs/DOCTOR.md](docs/DOCTOR.md) for environment diagnostics.
See [docs/COMPILATION_DATABASE.md](docs/COMPILATION_DATABASE.md) for project checks through `compile_commands.json`.
See [docs/CMAKE.md](docs/CMAKE.md) for CMake project checks.
See [docs/CLANG_TIDY.md](docs/CLANG_TIDY.md) for static analysis with `clang-tidy`.
See [docs/COVERAGE.md](docs/COVERAGE.md) for source-based coverage with LLVM tools.
See [docs/TESTING.md](docs/TESTING.md) for CTest and custom test commands.
See [docs/POLICY.md](docs/POLICY.md) for CI policy gates.
See [docs/BASELINE.md](docs/BASELINE.md) for diagnostic baselines.
See [docs/ARTIFACT_REPORTS.md](docs/ARTIFACT_REPORTS.md) for Markdown and HTML reports.

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

- `compile_commands.json` coverage workflows.
- SARIF output.
- libFuzzer workflows.
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

baseline:
  path: null

policy:
  max_warnings: null
  min_line_coverage: null
  fail_on_new_diagnostics: false
```

## Project Status

CppGauntlet is in early implementation. The repository currently includes single-file checks, compilation database checks, CMake configuration, optional CTest execution, custom test commands, optional `clang-tidy` analysis, single-file and CMake/CTest LLVM coverage, diagnostic baselines and baseline updates, CI policy gates, environment diagnostics, JSON reports, Markdown reports, and HTML reports.

## License

License information will be added before the first release.
