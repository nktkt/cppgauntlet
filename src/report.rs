use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub tool: ToolInfo,
    pub target: TargetInfo,
    pub status: ReportStatus,
    pub summary: Summary,
    pub stages: Vec<StageReport>,
    pub report_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sarif_report_path: Option<PathBuf>,
}

impl Report {
    pub fn is_success(&self) -> bool {
        self.status == ReportStatus::Passed
    }

    pub fn render_text(&self) -> String {
        let mut lines = vec![
            "CppGauntlet Check".to_string(),
            format!("Target: {}", self.target.path.display()),
            format!("Status: {}", self.status.as_str().to_uppercase()),
            format!("Standard: {}", self.target.standard),
            format!("Compiler: {}", self.target.compiler),
            format!("Warnings: {}", self.summary.warnings),
            format!("Errors: {}", self.summary.errors),
            coverage_line(&self.summary),
            baseline_line(&self.summary),
            format!("Report: {}", self.report_path.display()),
            String::new(),
            "Stages:".to_string(),
        ];

        for stage in &self.stages {
            let exit = stage
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "n/a".to_string());
            lines.push(format!(
                "- {}: {} (exit: {}, warnings: {}, errors: {})",
                stage.name,
                stage.status.as_str(),
                exit,
                stage.warnings,
                stage.errors
            ));
        }

        lines.join("\n")
    }

    pub fn render_markdown(&self) -> String {
        let mut lines = vec![
            "# CppGauntlet Check Report".to_string(),
            String::new(),
            "## Target".to_string(),
            String::new(),
            "| Field | Value |".to_string(),
            "| --- | --- |".to_string(),
            format!(
                "| Path | `{}` |",
                markdown_cell(&self.target.path.display().to_string())
            ),
            format!("| Status | {} |", self.status.as_str()),
            format!("| Standard | {} |", markdown_cell(&self.target.standard)),
            format!("| Compiler | `{}` |", markdown_cell(&self.target.compiler)),
            format!(
                "| JSON report | `{}` |",
                markdown_cell(&self.report_path.display().to_string())
            ),
        ];

        if let Some(path) = &self.markdown_report_path {
            lines.push(format!(
                "| Markdown report | `{}` |",
                markdown_cell(&path.display().to_string())
            ));
        }

        lines.extend([
            String::new(),
            "## Summary".to_string(),
            String::new(),
            "| Metric | Value |".to_string(),
            "| --- | ---: |".to_string(),
            format!("| Warnings | {} |", self.summary.warnings),
            format!("| Errors | {} |", self.summary.errors),
            format!("| Diagnostics | {} |", self.summary.diagnostics),
            format!("| Failed stages | {} |", self.summary.failed_stages),
            format!("| Timed out stages | {} |", self.summary.timed_out_stages),
        ]);

        if let Some(coverage) = &self.summary.coverage {
            lines.extend([
                format!("| Line coverage | {:.2}% |", coverage.lines.percent),
                format!("| Function coverage | {:.2}% |", coverage.functions.percent),
                format!("| Region coverage | {:.2}% |", coverage.regions.percent),
            ]);
        }

        if let Some(baseline) = &self.summary.baseline {
            lines.extend([
                format!(
                    "| Baseline unique diagnostics | {} |",
                    baseline.baseline_unique_diagnostics
                ),
                format!(
                    "| Current unique diagnostics | {} |",
                    baseline.current_unique_diagnostics
                ),
                format!(
                    "| New diagnostics | {} |",
                    baseline.new_diagnostic_occurrences
                ),
                format!(
                    "| Resolved diagnostics | {} |",
                    baseline.resolved_unique_diagnostics
                ),
            ]);
        }

        lines.extend([
            String::new(),
            "## Stages".to_string(),
            String::new(),
            "| Stage | Status | Exit | Warnings | Errors | Timed out |".to_string(),
            "| --- | --- | ---: | ---: | ---: | --- |".to_string(),
        ]);

        for stage in &self.stages {
            let exit = stage
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "n/a".to_string());
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} |",
                markdown_cell(&stage.name),
                stage.status.as_str(),
                exit,
                stage.warnings,
                stage.errors,
                stage.timed_out
            ));
        }

        lines.extend([String::new(), "## Diagnostics".to_string()]);
        let diagnostics = self
            .stages
            .iter()
            .flat_map(|stage| {
                stage
                    .diagnostics
                    .iter()
                    .map(move |diagnostic| (stage.name.as_str(), diagnostic))
            })
            .collect::<Vec<_>>();

        if diagnostics.is_empty() {
            lines.extend([String::new(), "No diagnostics recorded.".to_string()]);
        } else {
            for (stage_name, diagnostic) in diagnostics {
                let baseline = diagnostic
                    .baseline_status
                    .map(|status| format!(" / {}", status.as_str()))
                    .unwrap_or_default();
                lines.extend([
                    String::new(),
                    format!(
                        "- **{}{}** in `{}`: {}",
                        diagnostic.severity.as_str(),
                        baseline,
                        markdown_cell(stage_name),
                        markdown_cell(&diagnostic.message)
                    ),
                    format!("  - Raw: `{}`", markdown_cell(&diagnostic.raw)),
                ]);
            }
        }

        lines.join("\n")
    }

    pub fn render_html(&self) -> String {
        let mut target_rows = vec![
            html_table_row("Path", &self.target.path.display().to_string()),
            html_table_row("Status", self.status.as_str()),
            html_table_row("Standard", &self.target.standard),
            html_table_row("Compiler", &self.target.compiler),
            html_table_row("JSON report", &self.report_path.display().to_string()),
        ];
        if let Some(path) = &self.markdown_report_path {
            target_rows.push(html_table_row(
                "Markdown report",
                &path.display().to_string(),
            ));
        }
        if let Some(path) = &self.html_report_path {
            target_rows.push(html_table_row("HTML report", &path.display().to_string()));
        }
        if let Some(path) = &self.sarif_report_path {
            target_rows.push(html_table_row("SARIF report", &path.display().to_string()));
        }

        let mut summary_rows = vec![
            html_table_row("Warnings", &self.summary.warnings.to_string()),
            html_table_row("Errors", &self.summary.errors.to_string()),
            html_table_row("Diagnostics", &self.summary.diagnostics.to_string()),
            html_table_row("Failed stages", &self.summary.failed_stages.to_string()),
            html_table_row(
                "Timed out stages",
                &self.summary.timed_out_stages.to_string(),
            ),
        ];
        if let Some(coverage) = &self.summary.coverage {
            summary_rows.extend([
                html_table_row("Line coverage", &format!("{:.2}%", coverage.lines.percent)),
                html_table_row(
                    "Function coverage",
                    &format!("{:.2}%", coverage.functions.percent),
                ),
                html_table_row(
                    "Region coverage",
                    &format!("{:.2}%", coverage.regions.percent),
                ),
            ]);
        }
        if let Some(baseline) = &self.summary.baseline {
            summary_rows.extend([
                html_table_row(
                    "Baseline unique diagnostics",
                    &baseline.baseline_unique_diagnostics.to_string(),
                ),
                html_table_row(
                    "Current unique diagnostics",
                    &baseline.current_unique_diagnostics.to_string(),
                ),
                html_table_row(
                    "New diagnostics",
                    &baseline.new_diagnostic_occurrences.to_string(),
                ),
                html_table_row(
                    "Resolved diagnostics",
                    &baseline.resolved_unique_diagnostics.to_string(),
                ),
            ]);
        }

        let stage_rows = self
            .stages
            .iter()
            .map(|stage| {
                let exit = stage
                    .exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "n/a".to_string());
                format!(
                    r#"<tr><td>{}</td><td><span class="status status-{}">{}</span></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
                    html_escape(&stage.name),
                    stage.status.as_str(),
                    stage.status.as_str(),
                    html_escape(&exit),
                    stage.warnings,
                    stage.errors,
                    stage.timed_out
                )
            })
            .collect::<String>();

        let diagnostics = self
            .stages
            .iter()
            .flat_map(|stage| {
                stage
                    .diagnostics
                    .iter()
                    .map(move |diagnostic| (stage.name.as_str(), diagnostic))
            })
            .collect::<Vec<_>>();
        let diagnostics_html = if diagnostics.is_empty() {
            "<p>No diagnostics recorded.</p>".to_string()
        } else {
            diagnostics
                .iter()
                .map(|(stage_name, diagnostic)| {
                    let baseline = diagnostic
                        .baseline_status
                        .map(|status| format!(" / {}", status.as_str()))
                        .unwrap_or_default();
                    format!(
                        r#"<article class="diagnostic"><h3>{}{}</h3><p><strong>Stage:</strong> <code>{}</code></p><p>{}</p><pre>{}</pre></article>"#,
                        diagnostic.severity.as_str(),
                        html_escape(&baseline),
                        html_escape(stage_name),
                        html_escape(&diagnostic.message),
                        html_escape(&diagnostic.raw)
                    )
                })
                .collect::<String>()
        };

        format!(
            r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CppGauntlet Check Report</title>
<style>
:root {{ color-scheme: light; --bg: #f6f7f9; --panel: #ffffff; --text: #182230; --muted: #5d6b7a; --border: #d7dde5; --pass: #0f7b46; --fail: #b42318; --skip: #6f5c00; }}
body {{ margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: var(--text); background: var(--bg); }}
main {{ max-width: 1180px; margin: 0 auto; padding: 32px 20px 48px; }}
header {{ margin-bottom: 24px; }}
h1 {{ margin: 0 0 8px; font-size: 28px; }}
h2 {{ margin-top: 28px; font-size: 18px; }}
.status-line {{ color: var(--muted); }}
section {{ background: var(--panel); border: 1px solid var(--border); padding: 18px; margin: 16px 0; }}
table {{ width: 100%; border-collapse: collapse; }}
th, td {{ border-bottom: 1px solid var(--border); padding: 9px 10px; text-align: left; vertical-align: top; }}
th {{ color: var(--muted); font-weight: 600; }}
code, pre {{ background: #eef1f5; }}
code {{ padding: 1px 4px; }}
pre {{ padding: 10px; overflow-x: auto; }}
.status {{ font-weight: 700; }}
.status-passed {{ color: var(--pass); }}
.status-failed {{ color: var(--fail); }}
.status-skipped {{ color: var(--skip); }}
.diagnostic {{ border-top: 1px solid var(--border); padding-top: 14px; margin-top: 14px; }}
.diagnostic h3 {{ margin: 0 0 8px; font-size: 16px; }}
</style>
</head>
<body>
<main>
<header>
<h1>CppGauntlet Check Report</h1>
<div class="status-line">Status: <strong>{}</strong></div>
</header>
<section>
<h2>Target</h2>
<table><tbody>{}</tbody></table>
</section>
<section>
<h2>Summary</h2>
<table><tbody>{}</tbody></table>
</section>
<section>
<h2>Stages</h2>
<table>
<thead><tr><th>Stage</th><th>Status</th><th>Exit</th><th>Warnings</th><th>Errors</th><th>Timed out</th></tr></thead>
<tbody>{}</tbody>
</table>
</section>
<section>
<h2>Diagnostics</h2>
{}
</section>
</main>
</body>
</html>"#,
            self.status.as_str(),
            target_rows.join(""),
            summary_rows.join(""),
            stage_rows,
            diagnostics_html
        )
    }

    pub fn render_sarif(&self) -> String {
        serde_json::to_string_pretty(&self.render_sarif_value())
            .expect("SARIF value should be serializable")
    }

    fn render_sarif_value(&self) -> serde_json::Value {
        serde_json::json!({
            "$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [
                {
                    "tool": {
                        "driver": {
                            "name": self.tool.name,
                            "semanticVersion": self.tool.version,
                            "informationUri": "https://github.com/nktkt/cppgauntlet",
                            "rules": sarif_rules()
                        }
                    },
                    "results": self.sarif_results()
                }
            ]
        })
    }

    fn sarif_results(&self) -> Vec<serde_json::Value> {
        self.stages
            .iter()
            .flat_map(|stage| {
                stage
                    .diagnostics
                    .iter()
                    .map(move |diagnostic| self.sarif_result(stage, diagnostic))
            })
            .collect()
    }

    fn sarif_result(&self, stage: &StageReport, diagnostic: &Diagnostic) -> serde_json::Value {
        let mut properties = serde_json::Map::from_iter([
            (
                "stage".to_string(),
                serde_json::Value::String(stage.name.clone()),
            ),
            (
                "raw".to_string(),
                serde_json::Value::String(diagnostic.raw.clone()),
            ),
        ]);
        if let Some(status) = diagnostic.baseline_status {
            properties.insert(
                "baselineStatus".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        serde_json::json!({
            "ruleId": diagnostic.sarif_rule_id(),
            "level": diagnostic.severity.sarif_level(),
            "message": {
                "text": diagnostic.message
            },
            "locations": [
                {
                    "physicalLocation": sarif_physical_location(
                        &diagnostic.raw,
                        &self.target.path.display().to_string()
                    )
                }
            ],
            "properties": properties
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetInfo {
    pub path: PathBuf,
    pub standard: String,
    pub compiler: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Summary {
    pub warnings: usize,
    pub errors: usize,
    pub diagnostics: usize,
    pub failed_stages: usize,
    pub timed_out_stages: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<BaselineSummary>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoverageSummary {
    pub lines: CoverageMetric,
    pub functions: CoverageMetric,
    pub regions: CoverageMetric,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CoverageMetric {
    pub count: u64,
    pub covered: u64,
    pub percent: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BaselineSummary {
    pub path: PathBuf,
    pub baseline_unique_diagnostics: usize,
    pub current_unique_diagnostics: usize,
    pub new_unique_diagnostics: usize,
    pub new_diagnostic_occurrences: usize,
    pub resolved_unique_diagnostics: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StageReport {
    pub name: String,
    pub status: StageStatus,
    pub command: Vec<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub warnings: usize,
    pub errors: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub stdout: String,
    pub stderr: String,
    pub artifact: Option<PathBuf>,
}

impl StageReport {
    pub fn skipped(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StageStatus::Skipped,
            command: Vec::new(),
            exit_code: None,
            timed_out: false,
            warnings: 0,
            errors: 0,
            diagnostics: Vec::new(),
            stdout: String::new(),
            stderr: String::new(),
            artifact: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Passed,
    Failed,
}

impl ReportStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Passed,
    Failed,
    Skipped,
}

impl StageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub raw: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_status: Option<DiagnosticBaselineStatus>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

impl DiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub fn sarif_level(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticBaselineStatus {
    Existing,
    New,
}

impl DiagnosticBaselineStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Existing => "existing",
            Self::New => "new",
        }
    }
}

pub fn parse_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    stderr
        .lines()
        .filter_map(|line| {
            diagnostic_from_marker(line, "runtime error:", DiagnosticSeverity::Error)
                .or_else(|| diagnostic_from_marker(line, "ERROR:", DiagnosticSeverity::Error))
                .or_else(|| diagnostic_from_marker(line, "warning:", DiagnosticSeverity::Warning))
                .or_else(|| diagnostic_from_marker(line, "error:", DiagnosticSeverity::Error))
        })
        .collect()
}

fn diagnostic_from_marker(
    line: &str,
    marker: &str,
    severity: DiagnosticSeverity,
) -> Option<Diagnostic> {
    let (_, message) = line.split_once(marker)?;
    Some(Diagnostic {
        severity,
        message: message.trim().to_string(),
        raw: line.to_string(),
        baseline_status: None,
    })
}

pub fn stage_from_result(
    name: impl Into<String>,
    result: crate::runner::CommandResult,
    artifact: Option<&Path>,
) -> StageReport {
    let diagnostic_output = format!("{}\n{}", result.stdout, result.stderr);
    let diagnostics = parse_diagnostics(&diagnostic_output);
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
        .count();
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .count();
    let status = if result.success() {
        StageStatus::Passed
    } else {
        StageStatus::Failed
    };

    StageReport {
        name: name.into(),
        status,
        command: result.command,
        exit_code: result.exit_code,
        timed_out: result.timed_out,
        warnings,
        errors,
        diagnostics,
        stdout: result.stdout,
        stderr: result.stderr,
        artifact: artifact.map(Path::to_path_buf),
    }
}

fn coverage_line(summary: &Summary) -> String {
    summary
        .coverage
        .as_ref()
        .map(|coverage| format!("Line Coverage: {:.2}%", coverage.lines.percent))
        .unwrap_or_else(|| "Line Coverage: n/a".to_string())
}

fn baseline_line(summary: &Summary) -> String {
    summary
        .baseline
        .as_ref()
        .map(|baseline| {
            format!(
                "Baseline: {} new, {} resolved ({})",
                baseline.new_diagnostic_occurrences,
                baseline.resolved_unique_diagnostics,
                baseline.path.display()
            )
        })
        .unwrap_or_else(|| "Baseline: n/a".to_string())
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn html_table_row(label: &str, value: &str) -> String {
    format!(
        "<tr><th>{}</th><td>{}</td></tr>",
        html_escape(label),
        html_escape(value)
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

impl Diagnostic {
    fn sarif_rule_id(&self) -> &'static str {
        match self.severity {
            DiagnosticSeverity::Warning => "cppgauntlet/warning",
            DiagnosticSeverity::Error => "cppgauntlet/error",
        }
    }
}

fn sarif_rules() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": "cppgauntlet/warning",
            "name": "CppGauntlet warning",
            "shortDescription": {
                "text": "C++ warning reported by CppGauntlet"
            },
            "defaultConfiguration": {
                "level": "warning"
            }
        }),
        serde_json::json!({
            "id": "cppgauntlet/error",
            "name": "CppGauntlet error",
            "shortDescription": {
                "text": "C++ error reported by CppGauntlet"
            },
            "defaultConfiguration": {
                "level": "error"
            }
        }),
    ]
}

fn sarif_physical_location(raw: &str, fallback_uri: &str) -> serde_json::Value {
    if let Some(location) = parse_diagnostic_location(raw) {
        let mut region = serde_json::Map::from_iter([(
            "startLine".to_string(),
            serde_json::Value::from(location.start_line),
        )]);
        if let Some(start_column) = location.start_column {
            region.insert(
                "startColumn".to_string(),
                serde_json::Value::from(start_column),
            );
        }

        serde_json::json!({
            "artifactLocation": {
                "uri": location.uri
            },
            "region": region
        })
    } else {
        serde_json::json!({
            "artifactLocation": {
                "uri": fallback_uri
            }
        })
    }
}

#[derive(Debug)]
struct DiagnosticLocation {
    uri: String,
    start_line: u64,
    start_column: Option<u64>,
}

fn parse_diagnostic_location(raw: &str) -> Option<DiagnosticLocation> {
    let colon_positions = raw
        .char_indices()
        .filter_map(|(index, ch)| (ch == ':').then_some(index))
        .collect::<Vec<_>>();

    for window in colon_positions.windows(2) {
        let path_end = window[0];
        let line_start = path_end + 1;
        let line_end = window[1];
        let column_start = line_end + 1;
        let column_end = raw[column_start..]
            .find(':')
            .map(|offset| column_start + offset)
            .unwrap_or(raw.len());

        let line = raw[line_start..line_end].parse::<u64>().ok();
        let column = raw[column_start..column_end].parse::<u64>().ok();
        if let Some(start_line) = line {
            return Some(DiagnosticLocation {
                uri: raw[..path_end].to_string(),
                start_line,
                start_column: column,
            });
        }
    }

    None
}
