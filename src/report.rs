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
