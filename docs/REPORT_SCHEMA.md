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
  "report_path": ".cppgauntlet/cppgauntlet-report.json"
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
    "regions": { "count": 6, "covered": 5, "percent": 83.33 }
  }
}
```

Warnings do not currently fail a report. Compile failures, runtime failures, sanitizer failures, and timeouts do fail a report.
`clang-tidy` warnings do not fail a report unless the `clang-tidy` process itself exits non-zero.
The `coverage` object is omitted when coverage is not enabled or when coverage collection fails before a summary can be parsed.

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
- `coverage_cmake_configure`
- `coverage_cmake_build`
- `coverage_ctest`
- `coverage_run`
- `coverage_merge`
- `coverage_report`
- `ctest`
- `run`
- `sanitize_compile`
- `sanitize_run`

`compile:<source path>` and `clang_tidy:<source path>` are used for project checks created from `compile_commands.json`.

## Diagnostics

Diagnostics are extracted from compiler, sanitizer, and `clang-tidy` stdout/stderr output.

```json
{
  "severity": "warning",
  "message": "unused variable 'unused' [-Wunused-variable]",
  "raw": "main.cpp:2:9: warning: unused variable 'unused' [-Wunused-variable]"
}
```

Current severity values:

- `warning`
- `error`

CppGauntlet keeps the full raw stdout and stderr for every stage because diagnostic parsing will become more precise over time.
