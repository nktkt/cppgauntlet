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
    pub failed_stages: usize,
    pub timed_out_stages: usize,
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

pub fn count_warnings(stderr: &str) -> usize {
    stderr.matches("warning:").count()
}

pub fn count_errors(stderr: &str) -> usize {
    stderr.matches("error:").count()
}

pub fn stage_from_result(
    name: impl Into<String>,
    result: crate::runner::CommandResult,
    artifact: Option<&Path>,
) -> StageReport {
    let warnings = count_warnings(&result.stderr);
    let errors = count_errors(&result.stderr);
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
        stdout: result.stdout,
        stderr: result.stderr,
        artifact: artifact.map(Path::to_path_buf),
    }
}
