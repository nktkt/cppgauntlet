# Policy Gates

CppGauntlet can add a final `policy` stage that turns collected diagnostics and coverage data into CI-friendly pass/fail gates.

## CLI Usage

Fail when any warning is found:

```bash
cppgauntlet check main.cpp --max-warnings 0
```

Fail when analyzer findings are found:

```bash
cppgauntlet check main.cpp --max-analyzer-findings 0
```

Fail when line coverage is below 80 percent:

```bash
cppgauntlet check main.cpp --coverage --min-line-coverage 80
```

Fail when coverage for supplied changed lines is below 90 percent:

```bash
cppgauntlet check main.cpp --changed-line src/main.cpp:42 --min-changed-line-coverage 90
```

Combine both gates:

```bash
cppgauntlet check main.cpp --coverage --max-warnings 0 --min-line-coverage 80
```

Fail when diagnostics are not present in a baseline report:

```bash
cppgauntlet check main.cpp --baseline .cppgauntlet/baseline.json --fail-on-new-diagnostics
```

## Configuration

Policy gates can also be stored in `cppgauntlet.yaml`:

```yaml
policy:
  max_warnings: 0
  max_analyzer_findings: 0
  min_line_coverage: 80.0
  min_changed_line_coverage: 90.0
  changed_lines:
    - src/main.cpp:42
  fail_on_new_diagnostics: true
```

CLI arguments take precedence over configuration values.

## Behavior

When at least one policy is configured, CppGauntlet appends a `policy` stage after compile, analysis, sanitizer, test, and coverage stages.

- `max_warnings` counts warnings across all previous stages.
- `max_analyzer_findings` counts diagnostics from enabled analyzer stages such as `clang_tidy`.
- `min_line_coverage` checks `summary.coverage.lines.percent`.
- `min_changed_line_coverage` checks `summary.coverage.changed_lines.percent`.
- `fail_on_new_diagnostics` checks `summary.baseline.new_diagnostic_occurrences`.
- If an earlier stage has failed, the `policy` stage is skipped so the report keeps the original failure as the primary signal.
- If all earlier stages pass and `min_line_coverage` is configured but no coverage summary is available, the `policy` stage fails.
- If all earlier stages pass and `min_changed_line_coverage` is configured but no changed-line coverage summary is available, the `policy` stage fails.
- `fail_on_new_diagnostics` requires a baseline report from `--baseline` or `baseline.path`.

Warnings still appear in the report summary even when no policy gate is configured. Use `max_warnings` when warnings should fail CI.
Setting `max_analyzer_findings` enables `clang-tidy`; use `static_analysis.clang_tidy_bin` and `static_analysis.clang_tidy_checks` to customize that analyzer.
`--changed-line` accepts `<path>:<line>` and can be repeated. Non-coverable changed lines that do not appear in LLVM's coverage file data are ignored; if no supplied changed lines are coverable, changed-line coverage is reported as 100 percent.
