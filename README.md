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
- structured diagnostics extracted from compiler and sanitizer output
- raw stdout and stderr
- generated artifact path

See [docs/REPORT_SCHEMA.md](docs/REPORT_SCHEMA.md) for the current report contract.
See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for the current configuration file format.
See [docs/DOCTOR.md](docs/DOCTOR.md) for environment diagnostics.
See [docs/COMPILATION_DATABASE.md](docs/COMPILATION_DATABASE.md) for project checks through `compile_commands.json`.
See [docs/CMAKE.md](docs/CMAKE.md) for CMake project checks.

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

- CMake project detection.
- `compile_commands.json` support.
- `clang-tidy` integration.
- `ctest` integration.
- `llvm-cov` and `llvm-profdata` coverage reports.
- HTML reports.
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
  formats:
    - text
    - json
```

## Project Status

CppGauntlet is in early implementation. The repository currently includes the Rust CLI skeleton and the first single-file `check` workflow.

## License

License information will be added before the first release.
