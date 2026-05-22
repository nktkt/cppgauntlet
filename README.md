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
cargo run -- --format json check main.cpp
cargo run -- check main.cpp --standard c++23 --sanitizers address,undefined
```

Generated artifacts are written to `.cppgauntlet/` by default, including `cppgauntlet-report.json`.

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
