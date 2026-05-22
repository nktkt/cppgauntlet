# Diagnostic Baselines

CppGauntlet can compare the current run against a previous JSON report. This lets teams keep known diagnostic debt visible while failing CI only when new diagnostics appear.

## CLI Usage

Create a regular check report:

```bash
cppgauntlet check main.cpp --report .cppgauntlet/cppgauntlet-report.json
```

Export that report as a reusable baseline:

```bash
cppgauntlet baseline update --report .cppgauntlet/cppgauntlet-report.json --output .cppgauntlet/baseline.json
```

Compare a future run against that baseline:

```bash
cppgauntlet check main.cpp --baseline .cppgauntlet/baseline.json
```

Fail CI when the current run contains diagnostics that are not in the baseline:

```bash
cppgauntlet check main.cpp --baseline .cppgauntlet/baseline.json --fail-on-new-diagnostics
```

## Configuration

```yaml
baseline:
  path: .cppgauntlet/baseline.json

policy:
  fail_on_new_diagnostics: true
```

CLI arguments take precedence over configuration values.

## Updating Baselines

`baseline update` reads a CppGauntlet JSON report, validates it, removes transient baseline comparison fields, and writes a normalized JSON report that can be used with `--baseline`.

Default paths:

```bash
cppgauntlet baseline update
```

This reads `.cppgauntlet/cppgauntlet-report.json` and writes `.cppgauntlet/baseline.json`.

The command also supports machine-readable output:

```bash
cppgauntlet --format json baseline update --report current.json --output baseline.json
cppgauntlet --format markdown baseline update --report current.json --output baseline.json
```

## Report Output

When a baseline is configured, each current diagnostic receives a `baseline_status`:

- `existing`: the diagnostic fingerprint exists in the baseline report
- `new`: the diagnostic fingerprint was not found in the baseline report

The report summary also includes baseline counts:

```json
{
  "baseline": {
    "path": ".cppgauntlet/baseline.json",
    "baseline_unique_diagnostics": 1,
    "current_unique_diagnostics": 2,
    "new_unique_diagnostics": 1,
    "new_diagnostic_occurrences": 1,
    "resolved_unique_diagnostics": 0
  }
}
```

Diagnostic fingerprints currently use severity, message, and raw diagnostic text after whitespace normalization. This is intentionally transparent and will become more precise as the diagnostic model grows.
