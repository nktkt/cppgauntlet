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
  markdown_path: null
  html_path: null
  sarif_path: null

test:
  ctest: false
  command: null

static_analysis:
  clang_tidy: false
  clang_tidy_bin: clang-tidy
  clang_tidy_checks: null

coverage:
  enabled: false
  llvm_cov_bin: llvm-cov
  llvm_profdata_bin: llvm-profdata
  sources: []
  objects: []

baseline:
  path: null

policy:
  max_warnings: null
  max_analyzer_findings: null
  min_line_coverage: null
  min_changed_line_coverage: null
  changed_lines: []
  fail_on_new_diagnostics: false
```

## Fields

- `standard`: `c++17`, `c++20`, or `c++23`
- `compiler`: compiler executable, currently expected to be Clang-compatible
- `artifact_dir`: generated build artifacts and default report location
- `timeout_seconds`: per-command timeout
- `sanitizers.enabled`: `address`, `undefined`, `asan`, `ubsan`, or an empty list
- `report.path`: explicit JSON report path
- `report.markdown_path`: optional Markdown report path
- `report.html_path`: optional HTML report path
- `report.sarif_path`: optional SARIF report path
- `test.ctest`: for CMake projects, build and run CTest after compile checks
- `test.command`: custom shell command to run after compile and analyzer checks
- `static_analysis.clang_tidy`: run `clang-tidy` after compile checks
- `static_analysis.clang_tidy_bin`: `clang-tidy` executable
- `static_analysis.clang_tidy_checks`: optional checks expression passed as `--checks=<value>`
- `coverage.enabled`: collect source-based coverage for single-file, `compile_commands.json`, and CMake checks
- `coverage.llvm_cov_bin`: `llvm-cov` executable
- `coverage.llvm_profdata_bin`: `llvm-profdata` executable
- `coverage.sources`: optional source paths passed to `llvm-cov export` as `--sources`
- `coverage.objects`: optional coverage objects or executables passed to `llvm-cov export`
- `baseline.path`: previous CppGauntlet JSON report used to classify diagnostics as new or existing
- `policy.max_warnings`: fail the report when total warnings exceed this number
- `policy.max_analyzer_findings`: fail the report when analyzer diagnostics exceed this number
- `policy.min_line_coverage`: fail the report when line coverage is below this percentage
- `policy.min_changed_line_coverage`: fail the report when changed-line coverage is below this percentage
- `policy.changed_lines`: changed source lines in `<path>:<line>` form used by changed-line coverage
- `policy.fail_on_new_diagnostics`: fail the report when diagnostics are not present in the baseline report

If `report.path` is omitted, CppGauntlet writes to `<artifact_dir>/cppgauntlet-report.json`. Markdown, HTML, and SARIF reports are written only when their report paths or CLI flags are set.
