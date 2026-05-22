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
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Warning,
    Error,
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
