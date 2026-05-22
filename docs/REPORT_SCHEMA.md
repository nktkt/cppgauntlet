# Report Schema

CppGauntlet reports are JSON documents intended for both human inspection and CI automation.

The current schema version is `2`.

## Top-Level Fields

```json
{
  "schema_version": 2,
  "tool": {},
  "target": {},
  "status": "passed",
  "summary": {},
  "stages": [],
  "report_path": ".cppgauntlet/cppgauntlet-report.json",
  "markdown_report_path": ".cppgauntlet/cppgauntlet-report.md",
  "html_report_path": ".cppgauntlet/cppgauntlet-report.html",
  "sarif_report_path": ".cppgauntlet/cppgauntlet-report.sarif.json"
}
```

## Status Values

Report and stage status values use snake case:

- `passed`
- `failed`
- `skipped`

## Summary

```json
{
  "warnings": 1,
  "errors": 0,
  "diagnostics": 1,
  "failed_stages": 0,
  "timed_out_stages": 0,
  "coverage": {
    "lines": { "count": 10, "covered": 9, "percent": 90.0 },
    "functions": { "count": 2, "covered": 2, "percent": 100.0 },
    "regions": { "count": 6, "covered": 5, "percent": 83.33 },
    "changed_lines": { "count": 2, "covered": 1, "percent": 50.0 }
  },
  "baseline": {
    "path": ".cppgauntlet/baseline.json",
    "baseline_unique_diagnostics": 1,
    "current_unique_diagnostics": 1,
    "new_unique_diagnostics": 0,
    "new_diagnostic_occurrences": 0,
    "resolved_unique_diagnostics": 0
  }
}
```

Warnings do not fail a report unless the `policy.max_warnings` gate is configured. Compile failures, runtime failures, sanitizer failures, policy failures, and timeouts do fail a report.
`clang-tidy` warnings do not fail a report unless the `clang-tidy` process itself exits non-zero.
The `coverage` object is omitted when coverage is not enabled or when coverage collection fails before a summary can be parsed. If all earlier stages pass, a configured `policy.min_line_coverage` gate fails when this summary is unavailable. The optional `coverage.changed_lines` metric is present when changed lines were supplied and LLVM coverage file data was available.
The `baseline` object is omitted when no baseline report is configured.
The top-level `markdown_report_path`, `html_report_path`, and `sarif_report_path` fields are omitted when those artifacts are not configured.

## Stage

Each stage describes one command or one skipped pipeline step:

```json
{
  "name": "compile",
  "status": "passed",
  "command": ["clang++", "-std=c++20", "main.cpp", "-o", ".cppgauntlet/build/main"],
  "exit_code": 0,
  "timed_out": false,
  "warnings": 0,
  "errors": 0,
  "diagnostics": [],
  "stdout": "",
  "stderr": "",
  "artifact": ".cppgauntlet/build/main"
}
```

Current stage names:

- `cmake_configure`
- `cmake_build`
- `clang_tidy`
- `clang_tidy:<source path>`
- `compile`
- `compile:<source path>`
- `coverage_compile`
- `coverage_compile:<source path>`
- `coverage_cmake_configure`
- `coverage_cmake_build`
- `coverage_ctest`
- `coverage_run`
- `coverage_test_command`
- `coverage_merge`
- `coverage_report`
- `ctest`
- `policy`
- `run`
- `sanitize_compile`
- `sanitize_run`
- `test_command`

`compile:<source path>`, `clang_tidy:<source path>`, and `coverage_compile:<source path>` are used for project checks created from `compile_commands.json`.

## Diagnostics

Diagnostics are extracted from compiler, sanitizer, and `clang-tidy` stdout/stderr output.

```json
{
  "severity": "warning",
  "message": "unused variable 'unused' [-Wunused-variable]",
  "raw": "main.cpp:2:9: warning: unused variable 'unused' [-Wunused-variable]",
  "baseline_status": "existing"
}
```

Current severity values:

- `warning`
- `error`

When a baseline report is configured, `baseline_status` is either `existing` or `new`. The field is omitted when no baseline was used.

CppGauntlet keeps the full raw stdout and stderr for every stage because diagnostic parsing will become more precise over time.
