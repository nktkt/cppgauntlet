# Coverage

CppGauntlet can collect LLVM source-based coverage for single-file checks.

```bash
cppgauntlet check main.cpp --coverage
```

Current coverage support is limited to single C++ source files. Project-level coverage for CMake and `compile_commands.json` workflows will build on the same report model later.

## Pipeline

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

## Options

```bash
cppgauntlet check main.cpp \
  --coverage \
  --llvm-cov-bin llvm-cov \
  --llvm-profdata-bin llvm-profdata
```

- `--coverage`: enable coverage collection
- `--llvm-cov-bin`: override the `llvm-cov` executable path
- `--llvm-profdata-bin`: override the `llvm-profdata` executable path

Passing either tool override also enables coverage.

## Configuration

```yaml
coverage:
  enabled: true
  llvm_cov_bin: llvm-cov
  llvm_profdata_bin: llvm-profdata
```

If an earlier stage fails, coverage stages are skipped. If `llvm-profdata` or `llvm-cov` fails, the report fails and keeps the raw stdout/stderr for diagnosis.
