# Policy Gates

CppGauntlet can add a final `policy` stage that turns collected diagnostics and coverage data into CI-friendly pass/fail gates.

## CLI Usage

Fail when any warning is found:

```bash
cppgauntlet check main.cpp --max-warnings 0
```

Fail when line coverage is below 80 percent:

```bash
cppgauntlet check main.cpp --coverage --min-line-coverage 80
```

Combine both gates:

```bash
cppgauntlet check main.cpp --coverage --max-warnings 0 --min-line-coverage 80
```

## Configuration

Policy gates can also be stored in `cppgauntlet.yaml`:

```yaml
policy:
  max_warnings: 0
  min_line_coverage: 80.0
```

CLI arguments take precedence over configuration values.

## Behavior

When at least one policy is configured, CppGauntlet appends a `policy` stage after compile, analysis, sanitizer, test, and coverage stages.

- `max_warnings` counts warnings across all previous stages.
- `min_line_coverage` checks `summary.coverage.lines.percent`.
- If an earlier stage has failed, the `policy` stage is skipped so the report keeps the original failure as the primary signal.
- If all earlier stages pass and `min_line_coverage` is configured but no coverage summary is available, the `policy` stage fails.

Warnings still appear in the report summary even when no policy gate is configured. Use `max_warnings` when warnings should fail CI.
