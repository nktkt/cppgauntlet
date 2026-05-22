# Configuration

CppGauntlet can read project defaults from `cppgauntlet.yaml`.

Create a starter file:

```bash
cppgauntlet init
```

Overwrite an existing file:

```bash
cppgauntlet init --force
```

Use a non-default path:

```bash
cppgauntlet check main.cpp --config path/to/cppgauntlet.yaml
```

## Precedence

Values are resolved in this order:

1. CLI arguments
2. `cppgauntlet.yaml`
3. built-in defaults

This lets teams commit shared project defaults while individual CI jobs or local runs override specific fields.

## Current Format

```yaml
standard: c++20
compiler: clang++
artifact_dir: .cppgauntlet
timeout_seconds: 30

sanitizers:
  enabled:
    - address
    - undefined

report:
  path: .cppgauntlet/cppgauntlet-report.json

test:
  ctest: false

static_analysis:
  clang_tidy: false
  clang_tidy_bin: clang-tidy
  clang_tidy_checks: null

coverage:
  enabled: false
  llvm_cov_bin: llvm-cov
  llvm_profdata_bin: llvm-profdata
```

## Fields

- `standard`: `c++17`, `c++20`, or `c++23`
- `compiler`: compiler executable, currently expected to be Clang-compatible
- `artifact_dir`: generated build artifacts and default report location
- `timeout_seconds`: per-command timeout
- `sanitizers.enabled`: `address`, `undefined`, `asan`, `ubsan`, or an empty list
- `report.path`: explicit JSON report path
- `test.ctest`: for CMake projects, build and run CTest after compile checks
- `static_analysis.clang_tidy`: run `clang-tidy` after compile checks
- `static_analysis.clang_tidy_bin`: `clang-tidy` executable
- `static_analysis.clang_tidy_checks`: optional checks expression passed as `--checks=<value>`
- `coverage.enabled`: collect source-based coverage for single-file checks
- `coverage.llvm_cov_bin`: `llvm-cov` executable
- `coverage.llvm_profdata_bin`: `llvm-profdata` executable

If `report.path` is omitted, CppGauntlet writes to `<artifact_dir>/cppgauntlet-report.json`.
