# Markdown Reports

CppGauntlet can render check results as Markdown for CI job summaries, pull request comments, or saved build artifacts.

## Stdout Format

Print the current check report as Markdown:

```bash
cppgauntlet --format markdown check main.cpp
```

The JSON report is still written to `.cppgauntlet/cppgauntlet-report.json` unless `--report` changes the path.

## Report Artifact

Write a Markdown report file:

```bash
cppgauntlet check main.cpp --markdown-report .cppgauntlet/cppgauntlet-report.md
```

The Markdown artifact includes:

- target metadata
- warning, error, diagnostic, timeout, coverage, and baseline summary metrics
- stage status table
- diagnostic details grouped by stage

## Configuration

```yaml
report:
  path: .cppgauntlet/cppgauntlet-report.json
  markdown_path: .cppgauntlet/cppgauntlet-report.md
```

CLI arguments take precedence over configuration values.
