# GitHub Actions

CppGauntlet keeps copyable GitHub Actions examples under `examples/github-actions/`.

## Compile Database Checks

Use [compile-database.yml](../examples/github-actions/compile-database.yml) when the project already has a `compile_commands.json` file or can generate one before running CppGauntlet.

The workflow:

- installs Clang, `clang-tidy`, CMake, and Ninja
- optionally runs `CPPGAUNTLET_CONFIGURE_COMMAND` to generate `build/compile_commands.json`
- runs `cppgauntlet check "$CPPGAUNTLET_TARGET"`
- optionally enables `--clang-tidy`
- uploads JSON and Markdown report artifacts
- fails the job after artifacts are uploaded when CppGauntlet fails

For a committed compile database, set:

```yaml
CPPGAUNTLET_TARGET: compile_commands.json
CPPGAUNTLET_CONFIGURE_COMMAND: ""
```

For a CMake-generated compile database, keep a configure command similar to:

```yaml
CPPGAUNTLET_TARGET: build/compile_commands.json
CPPGAUNTLET_CONFIGURE_COMMAND: cmake -S . -B build -G Ninja -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
```

## CMake Coverage

Use [cmake-coverage.yml](../examples/github-actions/cmake-coverage.yml) when the repository has a CMake project with CTest tests.

The workflow:

- installs Clang, LLVM coverage tools, CMake, and Ninja
- runs `cppgauntlet check "$CPPGAUNTLET_TARGET" --coverage`
- applies `--min-line-coverage` from `CPPGAUNTLET_MIN_LINE_COVERAGE`
- optionally narrows coverage with `CPPGAUNTLET_COVERAGE_SOURCE`
- uploads report files, `.cppgauntlet/cmake-coverage-build/**`, and `.cppgauntlet/coverage/cmake/**`
- fails the job after artifacts are uploaded when CppGauntlet fails

CppGauntlet creates a separate coverage build directory and runs CTest from that instrumented build.

## Target Matrix

Use [target-matrix.yml](../examples/github-actions/target-matrix.yml) when one repository needs several CppGauntlet checks in parallel.

The example matrix includes:

- a single-file smoke check
- a generated `compile_commands.json` check with `--clang-tidy`
- a CMake coverage check with `--min-line-coverage`

Each matrix entry owns:

- `name`: stable job and report directory suffix
- `target`: path passed to `cppgauntlet check`
- `configure`: optional setup command, such as CMake configure
- `args`: extra CppGauntlet flags
- `artifact`: uploaded artifact name

For arguments that need shell quoting, prefer moving options into `cppgauntlet.yaml` and keep the matrix `args` value simple.

## Fuzz Crash Artifacts

Use [fuzz-crash-artifacts.yml](../examples/github-actions/fuzz-crash-artifacts.yml) when fuzz smoke checks should preserve crash files for later inspection.

The workflow runs `cppgauntlet check --fuzz`, uploads `.cppgauntlet/fuzz/artifacts/**` and `.cppgauntlet/fuzz/summaries/**`, then fails the job after upload if the fuzz gate failed.

## Other Examples

- [target-matrix.yml](../examples/github-actions/target-matrix.yml): run several CppGauntlet targets in parallel
- [fuzz-crash-artifacts.yml](../examples/github-actions/fuzz-crash-artifacts.yml): upload libFuzzer crash artifacts and per-target summaries
- [changed-line-coverage.yml](../examples/github-actions/changed-line-coverage.yml): gate pull requests on coverage for changed lines
- [baseline-review.yml](../examples/github-actions/baseline-review.yml): compare against a diagnostic baseline and upload a candidate baseline
- [code-scanning.yml](../examples/github-actions/code-scanning.yml): upload SARIF results to GitHub Code Scanning
