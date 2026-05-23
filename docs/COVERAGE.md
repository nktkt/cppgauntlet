# Coverage

CppGauntlet can collect LLVM source-based coverage for single-file checks, raw `compile_commands.json` workflows, and CMake projects.

```bash
cppgauntlet check main.cpp --coverage
cppgauntlet check ./compile_commands.json --coverage --test-command "./scripts/test.sh"
cppgauntlet check ./my-cmake-project --coverage
```

## Single-File Pipeline

When coverage is enabled, CppGauntlet adds four stages after earlier enabled checks have passed:

- `coverage_compile`
- `coverage_run`
- `coverage_merge`
- `coverage_report`

The generated commands are equivalent to:

```bash
clang++ -fprofile-instr-generate -fcoverage-mapping main.cpp -o .cppgauntlet/build/main-coverage
LLVM_PROFILE_FILE=.cppgauntlet/coverage/main.profraw .cppgauntlet/build/main-coverage
llvm-profdata merge -sparse .cppgauntlet/coverage/main.profraw -o .cppgauntlet/coverage/main.profdata
llvm-cov export .cppgauntlet/build/main-coverage -instr-profile=.cppgauntlet/coverage/main.profdata --summary-only --sources main.cpp
```

The raw LLVM coverage JSON is written to:

```text
.cppgauntlet/coverage/coverage-summary.json
```

The parsed line, function, and region coverage totals are stored in `summary.coverage` in the main JSON report.

## Compilation Database Pipeline

For raw `compile_commands.json` targets, CppGauntlet replays each compilation database entry with Clang source-based coverage flags and writes the coverage objects under:

```text
.cppgauntlet/coverage/compilation-database/objects
```

Because `compile_commands.json` only describes translation-unit compilation, coverage mode requires `--test-command` or `test.command` to run binaries built with matching coverage instrumentation. The coverage test command runs with `LLVM_PROFILE_FILE` set to:

```text
.cppgauntlet/coverage/compilation-database/compdb-%p.profraw
```

It then runs:

- `coverage_compile:<source path>` for each compilation database entry
- `coverage_test_command`
- `coverage_merge`
- `coverage_report`

The raw LLVM coverage JSON is written to:

```text
.cppgauntlet/coverage/compilation-database/coverage-summary.json
```

## CMake Pipeline

For CMake projects, CppGauntlet keeps the normal CMake build directory unchanged and creates a separate coverage build:

```text
.cppgauntlet/cmake-coverage-build
```

It then runs:

- `coverage_cmake_configure`
- `coverage_cmake_build`
- `coverage_ctest`
- `coverage_merge`
- `coverage_report`

The coverage configure step adds Clang source-based coverage flags through CMake:

```bash
-DCMAKE_CXX_COMPILER=<configured compiler>
-DCMAKE_CXX_FLAGS="-fprofile-instr-generate -fcoverage-mapping"
-DCMAKE_EXE_LINKER_FLAGS=-fprofile-instr-generate
-DCMAKE_SHARED_LINKER_FLAGS=-fprofile-instr-generate
```

CTest runs with `LLVM_PROFILE_FILE` pointing to:

```text
.cppgauntlet/coverage/cmake/cmake-%p.profraw
```

The raw LLVM coverage JSON is written to:

```text
.cppgauntlet/coverage/cmake/coverage-summary.json
```

## Options

```bash
cppgauntlet check main.cpp \
  --coverage \
  --coverage-source src/main.cpp \
  --coverage-object .cppgauntlet/build/main-coverage \
  --llvm-cov-bin llvm-cov \
  --llvm-profdata-bin llvm-profdata
```

- `--coverage`: enable coverage collection
- `--coverage-source`: pass a source path to `llvm-cov export`; repeat to limit coverage output to selected files
- `--coverage-object`: pass an object or executable to `llvm-cov export`; repeat to override automatic object discovery
- `--changed-line`: pass a changed source line in `<path>:<line>` form; repeat to calculate changed-line coverage
- `--changed-lines-diff`: pass a unified diff file to discover changed source lines automatically
- `--llvm-cov-bin`: override the `llvm-cov` executable path
- `--llvm-profdata-bin`: override the `llvm-profdata` executable path

Passing either tool override, source filter, object override, changed-line input, or changed-lines diff also enables coverage.

When no sources are configured, CppGauntlet uses the checked source file for single-file checks, all compilation database entries for raw `compile_commands.json` checks, and no explicit source filter for CMake checks. When no objects are configured, CppGauntlet uses the generated single-file executable, generated compilation database coverage objects, or discovered CMake coverage objects.

When changed lines are supplied, CppGauntlet reads full `llvm-cov export` file data instead of `--summary-only` output and stores the result in `summary.coverage.changed_lines`. Non-coverable changed lines that are not present in LLVM coverage file data are ignored.

To discover changed lines from Git in CI, write a unified diff and pass it to CppGauntlet:

```bash
git diff -U0 origin/main...HEAD > .cppgauntlet/changed.diff
cppgauntlet check . --changed-lines-diff .cppgauntlet/changed.diff --min-changed-line-coverage 80
```

The diff parser records added lines from the new file side of each hunk. Deleted-only hunks do not add changed lines because they have no coverable line in the current source tree.

See [GITHUB_CHANGED_LINE_COVERAGE.md](GITHUB_CHANGED_LINE_COVERAGE.md) for a GitHub Actions workflow that gates pull requests on changed-line coverage and uploads review artifacts.

## Configuration

```yaml
coverage:
  enabled: true
  llvm_cov_bin: llvm-cov
  llvm_profdata_bin: llvm-profdata
  sources:
    - src/main.cpp
  objects:
    - .cppgauntlet/build/main-coverage
```

If an earlier stage fails, coverage stages are skipped. If `llvm-profdata` or `llvm-cov` fails, the report fails and keeps the raw stdout/stderr for diagnosis.
