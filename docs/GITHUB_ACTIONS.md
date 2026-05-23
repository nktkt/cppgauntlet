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

## Sanitizer and Standard Matrix

Use [sanitizer-standard-matrix.yml](../examples/github-actions/sanitizer-standard-matrix.yml) when a single-file C++ entry point should be checked across language standards and sanitizer sets.

The example matrix runs:

- C++ standards: `c++17`, `c++20`, and `c++23`
- sanitizer sets: `none`, `address`, `undefined`, and `address,undefined`

Each matrix entry runs:

```bash
cppgauntlet check "$CPPGAUNTLET_TARGET" \
  --standard "$CPPGAUNTLET_STANDARD" \
  --sanitizers "$CPPGAUNTLET_SANITIZERS"
```

Use this workflow for fast compatibility coverage on standalone examples, smoke-test binaries, or small command-line entry points. For project-wide build-system coverage, prefer `compile-database.yml`, `cmake-coverage.yml`, or `target-matrix.yml`.

## Fuzz Crash Artifacts

Use [fuzz-crash-artifacts.yml](../examples/github-actions/fuzz-crash-artifacts.yml) when fuzz smoke checks should preserve crash files for later inspection.

The workflow runs `cppgauntlet check --fuzz`, uploads `.cppgauntlet/fuzz/artifacts/**` and `.cppgauntlet/fuzz/summaries/**`, then fails the job after upload if the fuzz gate failed.

## Fuzz Corpus Retention

Use [fuzz-corpus-retention.yml](../examples/github-actions/fuzz-corpus-retention.yml) for scheduled or manually triggered fuzz jobs that should keep corpus growth between runs.

The workflow restores `.cppgauntlet/fuzz/corpus` with `actions/cache/restore@v5`, runs a longer fuzz pass, saves the updated corpus with `actions/cache/save@v5` when the fuzz gate passes, and uploads the retained corpus as a 30-day workflow artifact.

## Other Examples

- [target-matrix.yml](../examples/github-actions/target-matrix.yml): run several CppGauntlet targets in parallel
- [sanitizer-standard-matrix.yml](../examples/github-actions/sanitizer-standard-matrix.yml): run a single-file C++ target across C++ standards and sanitizer sets
- [fuzz-crash-artifacts.yml](../examples/github-actions/fuzz-crash-artifacts.yml): upload libFuzzer crash artifacts and per-target summaries
- [fuzz-corpus-retention.yml](../examples/github-actions/fuzz-corpus-retention.yml): retain libFuzzer corpus inputs across scheduled long-running jobs
- [changed-line-coverage.yml](../examples/github-actions/changed-line-coverage.yml): gate pull requests on coverage for changed lines
- [baseline-review.yml](../examples/github-actions/baseline-review.yml): compare against a diagnostic baseline and upload a candidate baseline
- [code-scanning.yml](../examples/github-actions/code-scanning.yml): upload SARIF results to GitHub Code Scanning
