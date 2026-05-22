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
  "timed_out_stages": 0
}
```

Warnings do not currently fail a report. Compile failures, runtime failures, sanitizer failures, and timeouts do fail a report.

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
- `compile`
- `compile:<source path>`
- `run`
- `sanitize_compile`
- `sanitize_run`

`compile:<source path>` is used for project checks created from `compile_commands.json`.

## Diagnostics

Diagnostics are extracted from compiler and sanitizer stderr output.

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
