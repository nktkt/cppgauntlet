# Artifact Reports

CppGauntlet can render check results as Markdown or standalone HTML for CI job summaries, pull request comments, or saved build artifacts.

## Stdout Formats

Print the current check report as Markdown:

```bash
cppgauntlet --format markdown check main.cpp
```

Print the current check report as HTML:

```bash
cppgauntlet --format html check main.cpp
```

The JSON report is still written to `.cppgauntlet/cppgauntlet-report.json` unless `--report` changes the path.

## Report Artifacts

Write Markdown and HTML report files:

```bash
cppgauntlet check main.cpp \
  --markdown-report .cppgauntlet/cppgauntlet-report.md \
  --html-report .cppgauntlet/cppgauntlet-report.html
```

The rendered artifacts include:

- target metadata
- warning, error, diagnostic, timeout, coverage, and baseline summary metrics
- stage status table
- diagnostic details grouped by stage

The HTML artifact is self-contained and does not require external assets.

## Configuration

```yaml
report:
  path: .cppgauntlet/cppgauntlet-report.json
  markdown_path: .cppgauntlet/cppgauntlet-report.md
  html_path: .cppgauntlet/cppgauntlet-report.html
```

CLI arguments take precedence over configuration values.
