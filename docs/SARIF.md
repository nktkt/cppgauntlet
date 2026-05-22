# SARIF Output

CppGauntlet can export diagnostics as SARIF 2.1.0 for code scanning platforms.

## Usage

Write a SARIF report:

```bash
cppgauntlet check main.cpp --sarif-report .cppgauntlet/cppgauntlet-report.sarif.json
```

The regular JSON report is still written unless `--report` changes the path.

## Configuration

```yaml
report:
  sarif_path: .cppgauntlet/cppgauntlet-report.sarif.json
```

CLI arguments take precedence over configuration values.

## Mapping

CppGauntlet emits a SARIF 2.1.0 log with one run and two rules:

- `cppgauntlet/warning`
- `cppgauntlet/error`

Each diagnostic becomes a SARIF result with:

- `ruleId` based on diagnostic severity
- `level` set to `warning` or `error`
- `message.text` from the parsed diagnostic message
- `locations[0].physicalLocation` from `file:line:column` diagnostic text when available
- `properties.stage`, `properties.raw`, and optional `properties.baselineStatus`

The emitted `$schema` points to the OASIS SARIF 2.1.0 schema.

See [GITHUB_CODE_SCANNING.md](GITHUB_CODE_SCANNING.md) for a GitHub Actions workflow that uploads this SARIF output to GitHub Code Scanning.
